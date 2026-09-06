// SPDX-License-Identifier: Apache-2.0

//! Agent-facing data tools shared by the MCP server and the in-app AI agent.
//! Sessions are opened and scoped by the caller; read-only enforcement comes
//! from the session config and the preflight pipeline, not from this layer.

use std::sync::Arc;

use qore_core::{
    CollectionList, CollectionListOptions, Namespace, QueryResult, SessionId, TableSchema,
};
use qore_drivers::query_manager::QueryManager;
use qore_drivers::session_manager::SessionManager;

use crate::ServiceContext;
use crate::cache::QueryCache;
use crate::interceptor::{InterceptorPipeline, QuerySource};
use crate::policy::SafetyPolicy;
use crate::ratelimit::QueryRateLimiter;
use crate::virtual_relations::VirtualRelationStore;

/// Cheap snapshot of the service pieces the tools need. Callers that hold a
/// `ServiceContext` behind a lock can build one and release the lock; the
/// policy is a point-in-time copy.
#[derive(Clone)]
pub struct AgentToolContext {
    pub session_manager: Arc<SessionManager>,
    pub query_manager: Arc<QueryManager>,
    pub query_rate_limiter: Arc<QueryRateLimiter>,
    pub query_cache: Arc<QueryCache>,
    pub interceptor: Arc<InterceptorPipeline>,
    pub virtual_relations: Arc<VirtualRelationStore>,
    pub policy: SafetyPolicy,
}

impl AgentToolContext {
    pub fn from_service(ctx: &ServiceContext) -> Self {
        Self {
            session_manager: Arc::clone(&ctx.session_manager),
            query_manager: Arc::clone(&ctx.query_manager),
            query_rate_limiter: Arc::clone(&ctx.query_rate_limiter),
            query_cache: Arc::clone(&ctx.query_cache),
            interceptor: Arc::clone(&ctx.interceptor),
            virtual_relations: Arc::clone(&ctx.virtual_relations),
            policy: ctx.policy.clone(),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn run_query(
    ctx: &AgentToolContext,
    session: SessionId,
    query: &str,
    namespace: Option<&Namespace>,
    acknowledged: bool,
    timeout_ms: Option<u64>,
    source: QuerySource,
) -> Result<QueryResult, String> {
    let session_id = session.0.to_string();

    let pf = crate::query::preflight_with_source(
        &ctx.session_manager,
        &ctx.query_rate_limiter,
        &ctx.interceptor,
        &ctx.policy,
        session,
        &session_id,
        query,
        namespace,
        acknowledged,
        source,
    )
    .await?;

    let query_id = ctx.query_manager.register(session).await;
    let outcome = crate::query::execute(
        &ctx.query_manager,
        &ctx.query_cache,
        &ctx.interceptor,
        &ctx.policy,
        pf.driver,
        &pf.context,
        session,
        namespace.cloned(),
        query,
        query_id,
        pf.is_mutation,
        pf.connection_key.as_deref(),
        pf.safety_warning.as_deref(),
        timeout_ms,
        false,
        None,
        None,
        |_, _| {},
    )
    .await;

    if let Some(err) = outcome.error {
        return Err(err);
    }
    outcome
        .result
        .ok_or_else(|| "Query produced no result".to_string())
}

pub async fn list_namespaces(
    ctx: &AgentToolContext,
    session: SessionId,
) -> Result<Vec<Namespace>, String> {
    let driver = ctx
        .session_manager
        .get_driver(session)
        .await
        .map_err(|e| e.sanitized_message())?;
    driver
        .list_namespaces(session)
        .await
        .map_err(|e| e.sanitized_message())
}

pub async fn list_tables(
    ctx: &AgentToolContext,
    session: SessionId,
    namespace: &Namespace,
    search: Option<String>,
) -> Result<CollectionList, String> {
    let driver = ctx
        .session_manager
        .get_driver(session)
        .await
        .map_err(|e| e.sanitized_message())?;
    let options = CollectionListOptions {
        search,
        page: None,
        page_size: None,
    };
    driver
        .list_collections(session, namespace, options)
        .await
        .map_err(|e| e.sanitized_message())
}

pub async fn describe_table(
    ctx: &AgentToolContext,
    session: SessionId,
    namespace: &Namespace,
    table: &str,
    connection_id: Option<&str>,
) -> Result<TableSchema, String> {
    crate::query::describe_table(
        &ctx.session_manager,
        &ctx.virtual_relations,
        session,
        namespace,
        table,
        connection_id,
    )
    .await
    .map_err(|e| e.sanitized())
}

pub const PREVIEW_MAX_ROWS: u32 = 100;
pub const SEARCH_SCHEMA_MAX_RESULTS: usize = 200;

pub async fn preview_table(
    ctx: &AgentToolContext,
    session: SessionId,
    namespace: &Namespace,
    table: &str,
    limit: u32,
) -> Result<QueryResult, String> {
    crate::query::preview_table(
        &ctx.session_manager,
        &ctx.query_manager,
        &ctx.query_cache,
        &ctx.policy,
        session,
        namespace,
        table,
        limit.clamp(1, PREVIEW_MAX_ROWS),
        false,
    )
    .await
    .map_err(|e| e.sanitized())
}

pub async fn explain_query(
    ctx: &AgentToolContext,
    session: SessionId,
    namespace: Option<&Namespace>,
    query: &str,
    timeout_ms: Option<u64>,
    source: QuerySource,
) -> Result<QueryResult, String> {
    let driver = ctx
        .session_manager
        .get_driver(session)
        .await
        .map_err(|e| e.sanitized_message())?;
    let Some(prefix) = driver.explain_prefix() else {
        return Err(format!(
            "{} does not support EXPLAIN through this tool. Driver capabilities: {}",
            driver.driver_id(),
            serde_json::to_string(&driver.capabilities()).unwrap_or_default()
        ));
    };

    let trimmed = query.trim().trim_end_matches(';').trim();
    if trimmed.is_empty() {
        return Err("Query is empty".to_string());
    }
    let statement = if trimmed
        .get(..7)
        .is_some_and(|head| head.eq_ignore_ascii_case("explain"))
    {
        trimmed.to_string()
    } else {
        format!("{prefix} {trimmed}")
    };
    run_query(
        ctx, session, &statement, namespace, false, timeout_ms, source,
    )
    .await
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SchemaMatch {
    pub table: String,
    pub column: Option<String>,
    pub data_type: Option<String>,
}

/// Case-insensitive substring search over table and column names of one
/// namespace. Never touches row data. One catalogue query when the engine
/// has one; otherwise tables are described one by one.
pub async fn search_schema(
    ctx: &AgentToolContext,
    session: SessionId,
    namespace: &Namespace,
    pattern: &str,
) -> Result<Vec<SchemaMatch>, String> {
    let needle = pattern.trim().to_lowercase();
    if needle.is_empty() {
        return Err("Pattern is empty".to_string());
    }

    let driver = ctx
        .session_manager
        .get_driver(session)
        .await
        .map_err(|e| e.sanitized_message())?;
    let columns = match crate::schema_search::columns_query(driver.driver_id(), namespace) {
        Some(query) => {
            match catalogue_columns(ctx, driver.as_ref(), session, namespace, &query).await {
                Ok(columns) => columns,
                Err(err) => {
                    tracing::warn!("catalogue search failed, describing tables instead: {err}");
                    described_columns(ctx, session, namespace).await?
                }
            }
        }
        None => described_columns(ctx, session, namespace).await?,
    };

    let mut matches = Vec::new();
    let mut last_table: Option<&str> = None;
    for (table, column, data_type) in &columns {
        if last_table != Some(table.as_str()) {
            last_table = Some(table);
            if table.to_lowercase().contains(&needle) {
                matches.push(SchemaMatch {
                    table: table.clone(),
                    column: None,
                    data_type: None,
                });
            }
        }
        if !column.is_empty() && column.to_lowercase().contains(&needle) {
            matches.push(SchemaMatch {
                table: table.clone(),
                column: Some(column.clone()),
                data_type: Some(data_type.clone()),
            });
        }
        if matches.len() >= SEARCH_SCHEMA_MAX_RESULTS {
            break;
        }
    }
    Ok(matches)
}

type ColumnRow = (String, String, String);

async fn catalogue_columns(
    ctx: &AgentToolContext,
    driver: &dyn qore_core::DataEngine,
    session: SessionId,
    namespace: &Namespace,
    query: &str,
) -> Result<Vec<ColumnRow>, String> {
    let query_id = ctx.query_manager.register(session).await;
    let result = crate::governance::with_timeout(
        &ctx.policy,
        driver.execute_in_namespace(session, Some(namespace.clone()), query, query_id),
    )
    .await;
    ctx.query_manager.finish(query_id).await;
    let result = result?.map_err(|e| e.sanitized_message())?;
    let cell = |row: &qore_core::Row, i: usize| -> String {
        row.values
            .get(i)
            .and_then(|v| v.as_text())
            .unwrap_or_default()
            .to_string()
    };
    Ok(result
        .rows
        .iter()
        .map(|row| (cell(row, 0), cell(row, 1), cell(row, 2)))
        .collect())
}

/// Tables without columns still appear, as a row with an empty column name,
/// so table-name matches survive the flattening.
async fn described_columns(
    ctx: &AgentToolContext,
    session: SessionId,
    namespace: &Namespace,
) -> Result<Vec<ColumnRow>, String> {
    let list = list_tables(ctx, session, namespace, None).await?;
    let mut rows = Vec::new();
    for collection in list.collections {
        let schema = describe_table(ctx, session, namespace, &collection.name, None).await;
        let columns = schema.map(|s| s.columns).unwrap_or_default();
        if columns.is_empty() {
            rows.push((collection.name.clone(), String::new(), String::new()));
        }
        for column in columns {
            rows.push((collection.name.clone(), column.name, column.data_type));
        }
    }
    Ok(rows)
}

#[cfg(all(test, feature = "driver-sqlite"))]
mod tests {
    use super::*;
    use qore_core::{ConnectionConfig, DriverRegistry};
    use qore_drivers::drivers::sqlite::SqliteDriver;

    struct Fixture {
        ctx: AgentToolContext,
        session: SessionId,
        namespace: Namespace,
        _dir: tempfile::TempDir,
    }

    async fn fixture() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let mut registry = DriverRegistry::new();
        registry.register(Arc::new(SqliteDriver::new()));
        let registry = Arc::new(registry);
        let ctx = AgentToolContext {
            session_manager: Arc::new(SessionManager::new(registry)),
            query_manager: Arc::new(QueryManager::new()),
            query_rate_limiter: Arc::new(QueryRateLimiter::with_defaults()),
            query_cache: Arc::new(QueryCache::new()),
            interceptor: Arc::new(InterceptorPipeline::new(dir.path().join("interceptor"))),
            virtual_relations: Arc::new(VirtualRelationStore::new(dir.path().join("vr"))),
            policy: SafetyPolicy {
                prod_require_confirmation: true,
                prod_block_dangerous_sql: false,
                max_query_duration_ms: None,
                max_result_rows: None,
                max_concurrent_queries: None,
                query_rate_limit_enabled: false,
            },
        };

        let db_path = dir.path().join("agent_tools.db");
        let config = ConnectionConfig {
            options: Default::default(),
            driver: "sqlite".to_string(),
            host: db_path.to_string_lossy().to_string(),
            port: 0,
            username: String::new(),
            password: String::new(),
            database: None,
            ssl: false,
            ssl_mode: None,
            environment: "development".to_string(),
            read_only: false,
            ssh_tunnel: None,
            pool_acquire_timeout_secs: None,
            pool_max_connections: None,
            pool_min_connections: None,
            proxy: None,
            mssql_auth: None,
            clickhouse_cluster: None,
            search_auth_mode: None,
            ssl_ca_cert: None,
        };
        let session = crate::connection::connect(&ctx.session_manager, config)
            .await
            .unwrap();

        for statement in [
            "CREATE TABLE users (id INTEGER PRIMARY KEY, email TEXT NOT NULL, city TEXT)",
            "CREATE TABLE orders (id INTEGER PRIMARY KEY, user_id INTEGER, amount REAL)",
            "INSERT INTO users (email, city) VALUES ('a@x.io', 'Paris'), ('b@x.io', 'Lyon'), ('c@x.io', 'Nice')",
        ] {
            run_query(
                &ctx,
                session,
                statement,
                None,
                false,
                None,
                QuerySource::Mcp,
            )
            .await
            .unwrap();
        }

        let namespace = list_namespaces(&ctx, session).await.unwrap().remove(0);
        Fixture {
            ctx,
            session,
            namespace,
            _dir: dir,
        }
    }

    #[tokio::test]
    async fn run_query_targets_the_requested_namespace() {
        let f = fixture().await;
        let result = run_query(
            &f.ctx,
            f.session,
            "SELECT count(*) AS n FROM users",
            Some(&f.namespace),
            false,
            None,
            QuerySource::Mcp,
        )
        .await
        .unwrap();
        assert_eq!(result.rows.len(), 1);
        assert!(matches!(result.rows[0].values[0], qore_core::Value::Int(3)));
    }

    #[tokio::test]
    async fn preview_table_honours_and_caps_the_limit() {
        let f = fixture().await;
        let two = preview_table(&f.ctx, f.session, &f.namespace, "users", 2)
            .await
            .unwrap();
        assert_eq!(two.rows.len(), 2);

        let capped = preview_table(&f.ctx, f.session, &f.namespace, "users", 10_000)
            .await
            .unwrap();
        assert_eq!(capped.rows.len(), 3);
    }

    #[tokio::test]
    async fn explain_query_prefixes_the_dialect_keyword() {
        let f = fixture().await;
        let plan = explain_query(
            &f.ctx,
            f.session,
            Some(&f.namespace),
            "SELECT * FROM users WHERE email = 'a@x.io';",
            None,
            QuerySource::Mcp,
        )
        .await
        .unwrap();
        assert!(!plan.rows.is_empty());
        assert!(plan.columns.iter().any(|c| c.name == "detail"));
    }

    #[test]
    fn explain_prefix_comes_from_the_driver_capabilities() {
        use qore_core::DataEngine;
        let sqlite = SqliteDriver::new();
        assert_eq!(sqlite.explain_prefix(), Some("EXPLAIN QUERY PLAN"));
        assert_eq!(
            sqlite.capabilities().explain_prefix.as_deref(),
            Some("EXPLAIN QUERY PLAN")
        );
    }

    #[tokio::test]
    async fn search_schema_matches_tables_and_columns() {
        let f = fixture().await;
        let matches = search_schema(&f.ctx, f.session, &f.namespace, "MAIL")
            .await
            .unwrap();
        assert_eq!(
            matches,
            vec![SchemaMatch {
                table: "users".to_string(),
                column: Some("email".to_string()),
                data_type: Some("TEXT".to_string()),
            }]
        );

        let tables = search_schema(&f.ctx, f.session, &f.namespace, "order")
            .await
            .unwrap();
        assert!(
            tables
                .iter()
                .any(|m| m.table == "orders" && m.column.is_none())
        );

        let described = described_columns(&f.ctx, f.session, &f.namespace)
            .await
            .unwrap();
        let driver = f.ctx.session_manager.get_driver(f.session).await.unwrap();
        let catalogue = catalogue_columns(
            &f.ctx,
            driver.as_ref(),
            f.session,
            &f.namespace,
            &crate::schema_search::columns_query("sqlite", &f.namespace).unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(described, catalogue);
        assert!(
            search_schema(&f.ctx, f.session, &f.namespace, "  ")
                .await
                .is_err()
        );
    }
}
