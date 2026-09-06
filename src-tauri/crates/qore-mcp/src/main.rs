// SPDX-License-Identifier: Apache-2.0

mod prompts;
mod resources;

use std::path::PathBuf;
use std::sync::Arc;

use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, Content, GetPromptRequestParams, GetPromptResult, Implementation,
    InitializeRequestParams, InitializeResult, ListPromptsResult, ListResourceTemplatesResult,
    ListResourcesResult, PaginatedRequestParams, ProtocolVersion, ReadResourceRequestParams,
    ReadResourceResult, ResourceContents, ServerCapabilities, ServerInfo,
};
use rmcp::service::RequestContext;
use rmcp::transport::stdio;
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler, ServiceExt, tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::sync::Mutex;

use qore_core::{Namespace, SessionId};
use qore_service::ServiceContext;
use qore_service::agent_access::{self, AgentSessions};
use qore_service::agent_tools::{self, AgentToolContext, PREVIEW_MAX_ROWS};
use qore_service::interceptor::QuerySource;
use qore_service::paths::{PROJECT_ID, QUERY_TIMEOUT_MS, config_dir};
use qore_service::vault::VaultStorage;
use qore_service::vault::backend::KeyringProvider;
use qore_service::vault::credentials::SavedConnection;

const INSTRUCTIONS: &str = "QoreDB gives read-only access to the database connections the user \
explicitly exposed to AI agents. Every session is forced read-only, the safety policy applies \
(row cap, timeout, rate limit) and each call is written to the audit log.\n\
\n\
Tools:\n\
- list_connections: the exposed connections (id, driver, host, environment). Start here.\n\
- list_namespaces: databases/schemas of a connection.\n\
- list_tables: tables or collections of a namespace, with an optional name filter.\n\
- describe_table: columns, primary key, foreign keys, indexes and row estimate of a table.\n\
- preview_table: a sample of rows (max 100) through the engine's cheapest path.\n\
- search_schema: find tables and columns whose name contains a pattern.\n\
- run_query: a read-only query, optionally scoped to a database/schema.\n\
- explain_query: the execution plan of a read-only query.\n\
\n\
Resources: qore://{connection_id}/{database}[/{schema}]/{table} returns the table schema as JSON.\n\
Prompts: audit_table, explain_slow_query, document_schema.\n\
\n\
Writes are never possible from this server; suggest DDL or DML to the user instead of trying.";

#[derive(Clone)]
struct QoreMcp {
    ctx: Arc<ServiceContext>,
    storage_dir: PathBuf,
    sessions: Arc<Mutex<AgentSessions>>,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct RunQueryReq {
    #[schemars(description = "ID of the saved connection to query")]
    connection_id: String,
    #[schemars(description = "Read-only SQL/query to execute")]
    query: String,
    #[schemars(
        description = "Database/namespace to run in (optional, defaults to the connection's)"
    )]
    #[serde(default)]
    database: Option<String>,
    #[schemars(description = "Schema name (optional, e.g. PostgreSQL schema)")]
    #[serde(default)]
    schema: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ConnReq {
    #[schemars(description = "ID of the saved connection")]
    connection_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ListTablesReq {
    #[schemars(description = "ID of the saved connection")]
    connection_id: String,
    #[schemars(description = "Database/namespace name")]
    database: String,
    #[schemars(description = "Schema name (optional, e.g. PostgreSQL schema)")]
    #[serde(default)]
    schema: Option<String>,
    #[schemars(description = "Optional name filter")]
    #[serde(default)]
    search: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DescribeTableReq {
    #[schemars(description = "ID of the saved connection")]
    connection_id: String,
    #[schemars(description = "Database/namespace name")]
    database: String,
    #[schemars(description = "Schema name (optional)")]
    #[serde(default)]
    schema: Option<String>,
    #[schemars(description = "Table/collection name")]
    table: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct PreviewTableReq {
    #[schemars(description = "ID of the saved connection")]
    connection_id: String,
    #[schemars(description = "Database/namespace name")]
    database: String,
    #[schemars(description = "Schema name (optional)")]
    #[serde(default)]
    schema: Option<String>,
    #[schemars(description = "Table/collection name")]
    table: String,
    #[schemars(description = "Number of rows to return (1-100, default 20)")]
    #[serde(default)]
    limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchSchemaReq {
    #[schemars(description = "ID of the saved connection")]
    connection_id: String,
    #[schemars(description = "Database/namespace name")]
    database: String,
    #[schemars(description = "Schema name (optional)")]
    #[serde(default)]
    schema: Option<String>,
    #[schemars(description = "Case-insensitive substring to look for in table and column names")]
    pattern: String,
}

fn text_result(result: Result<String, String>) -> CallToolResult {
    match result {
        Ok(json) => CallToolResult::success(vec![Content::text(json)]),
        Err(msg) => CallToolResult::error(vec![Content::text(msg)]),
    }
}

fn to_json<T: serde::Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| e.to_string())
}

fn namespace_of(database: &str, schema: Option<&str>) -> Namespace {
    Namespace {
        database: database.to_string(),
        schema: schema.map(str::to_string),
    }
}

#[tool_router]
impl QoreMcp {
    fn new(storage_dir: PathBuf) -> Self {
        Self {
            ctx: Arc::new(ServiceContext::new()),
            storage_dir,
            sessions: Arc::new(Mutex::new(AgentSessions::default())),
            tool_router: Self::tool_router(),
        }
    }

    fn storage(&self) -> VaultStorage {
        VaultStorage::new(
            PROJECT_ID,
            self.storage_dir.clone(),
            Box::new(KeyringProvider::new()),
        )
    }

    fn tool_ctx(&self) -> AgentToolContext {
        AgentToolContext::from_service(&self.ctx)
    }

    /// The safety policy's duration wins when it is stricter than the
    /// headless default.
    fn query_timeout(&self) -> u64 {
        self.ctx
            .policy
            .max_query_duration_ms
            .map_or(QUERY_TIMEOUT_MS, |ms| ms.min(QUERY_TIMEOUT_MS))
    }

    fn exposed_connections(&self) -> Result<Vec<SavedConnection>, String> {
        let all = self
            .storage()
            .list_connections_full()
            .map_err(|e| e.sanitized_message())?;
        Ok(agent_access::exposed_connections(all))
    }

    async fn close_idle_sessions(&self) {
        let idle = self.sessions.lock().await.take_idle();
        for session in idle {
            if let Err(err) = qore_service::connection::disconnect(
                &self.ctx.session_manager,
                &self.ctx.query_rate_limiter,
                session,
            )
            .await
            {
                tracing::warn!("failed to close idle session: {}", err.sanitized());
            }
        }
    }

    async fn ensure_session(&self, connection_id: &str) -> Result<SessionId, String> {
        self.close_idle_sessions().await;
        if let Some(session) = self.sessions.lock().await.get(connection_id) {
            return Ok(session);
        }

        let storage = self.storage();
        let saved = storage
            .get_connection(connection_id)
            .map_err(|e| e.sanitized_message())?;
        agent_access::require_exposed(&saved)?;
        let creds = storage
            .get_credentials(connection_id)
            .map_err(|e| e.sanitized_message())?;
        let config = agent_access::agent_connection_config(&saved, &creds)?;

        let session = qore_service::connection::connect(&self.ctx.session_manager, config)
            .await
            .map_err(|e| e.sanitized())?;
        self.sessions
            .lock()
            .await
            .insert(connection_id.to_string(), session);
        Ok(session)
    }

    async fn do_run_query(&self, req: &RunQueryReq) -> Result<String, String> {
        let session = self.ensure_session(&req.connection_id).await?;
        let namespace = req
            .database
            .as_deref()
            .map(|db| namespace_of(db, req.schema.as_deref()));
        let result = agent_tools::run_query(
            &self.tool_ctx(),
            session,
            &req.query,
            namespace.as_ref(),
            false,
            Some(self.query_timeout()),
            QuerySource::Mcp,
        )
        .await?;
        to_json(&result)
    }

    async fn do_explain_query(&self, req: &RunQueryReq) -> Result<String, String> {
        let session = self.ensure_session(&req.connection_id).await?;
        let namespace = req
            .database
            .as_deref()
            .map(|db| namespace_of(db, req.schema.as_deref()));
        let result = agent_tools::explain_query(
            &self.tool_ctx(),
            session,
            namespace.as_ref(),
            &req.query,
            Some(self.query_timeout()),
            QuerySource::Mcp,
        )
        .await?;
        to_json(&result)
    }

    async fn do_list_namespaces(&self, connection_id: &str) -> Result<String, String> {
        let session = self.ensure_session(connection_id).await?;
        let namespaces = agent_tools::list_namespaces(&self.tool_ctx(), session).await?;
        to_json(&namespaces)
    }

    async fn do_list_tables(&self, req: &ListTablesReq) -> Result<String, String> {
        let session = self.ensure_session(&req.connection_id).await?;
        let namespace = namespace_of(&req.database, req.schema.as_deref());
        let list =
            agent_tools::list_tables(&self.tool_ctx(), session, &namespace, req.search.clone())
                .await?;
        to_json(&list)
    }

    async fn do_describe_table(&self, req: &DescribeTableReq) -> Result<String, String> {
        let session = self.ensure_session(&req.connection_id).await?;
        let namespace = namespace_of(&req.database, req.schema.as_deref());
        let schema =
            agent_tools::describe_table(&self.tool_ctx(), session, &namespace, &req.table, None)
                .await?;
        to_json(&schema)
    }

    async fn do_preview_table(&self, req: &PreviewTableReq) -> Result<String, String> {
        let session = self.ensure_session(&req.connection_id).await?;
        let namespace = namespace_of(&req.database, req.schema.as_deref());
        let result = agent_tools::preview_table(
            &self.tool_ctx(),
            session,
            &namespace,
            &req.table,
            req.limit.unwrap_or(20).min(PREVIEW_MAX_ROWS),
        )
        .await?;
        to_json(&result)
    }

    async fn do_search_schema(&self, req: &SearchSchemaReq) -> Result<String, String> {
        let session = self.ensure_session(&req.connection_id).await?;
        let namespace = namespace_of(&req.database, req.schema.as_deref());
        let matches =
            agent_tools::search_schema(&self.tool_ctx(), session, &namespace, &req.pattern).await?;
        to_json(&matches)
    }

    #[tool(description = "List the saved connections exposed to AI agents (read-only access)")]
    async fn list_connections(&self) -> Result<CallToolResult, McpError> {
        let summary = self.exposed_connections().map(|connections| {
            connections
                .iter()
                .map(agent_access::connection_summary)
                .collect::<Vec<_>>()
        });
        Ok(text_result(summary.and_then(|s| to_json(&s))))
    }

    #[tool(
        description = "Run a read-only query against a saved connection and return the rows. \
                          Pass database/schema to target a namespace explicitly."
    )]
    async fn run_query(
        &self,
        Parameters(req): Parameters<RunQueryReq>,
    ) -> Result<CallToolResult, McpError> {
        Ok(text_result(self.do_run_query(&req).await))
    }

    #[tool(
        description = "Return the execution plan of a read-only query (EXPLAIN in the \
                          engine's dialect). Refused on engines without EXPLAIN."
    )]
    async fn explain_query(
        &self,
        Parameters(req): Parameters<RunQueryReq>,
    ) -> Result<CallToolResult, McpError> {
        Ok(text_result(self.do_explain_query(&req).await))
    }

    #[tool(description = "List databases/schemas (namespaces) for a saved connection")]
    async fn list_namespaces(
        &self,
        Parameters(req): Parameters<ConnReq>,
    ) -> Result<CallToolResult, McpError> {
        Ok(text_result(
            self.do_list_namespaces(&req.connection_id).await,
        ))
    }

    #[tool(description = "List tables/collections in a namespace")]
    async fn list_tables(
        &self,
        Parameters(req): Parameters<ListTablesReq>,
    ) -> Result<CallToolResult, McpError> {
        Ok(text_result(self.do_list_tables(&req).await))
    }

    #[tool(
        description = "Describe a table: columns, primary key, foreign keys, indexes, row estimate"
    )]
    async fn describe_table(
        &self,
        Parameters(req): Parameters<DescribeTableReq>,
    ) -> Result<CallToolResult, McpError> {
        Ok(text_result(self.do_describe_table(&req).await))
    }

    #[tool(
        description = "Return a sample of rows from a table (max 100) using the engine's \
                          cheapest read path; cached and free of charge on BigQuery"
    )]
    async fn preview_table(
        &self,
        Parameters(req): Parameters<PreviewTableReq>,
    ) -> Result<CallToolResult, McpError> {
        Ok(text_result(self.do_preview_table(&req).await))
    }

    #[tool(
        description = "Find tables and columns of a namespace whose name contains a pattern. \
                          Returns table, column and type; never searches row data."
    )]
    async fn search_schema(
        &self,
        Parameters(req): Parameters<SearchSchemaReq>,
    ) -> Result<CallToolResult, McpError> {
        Ok(text_result(self.do_search_schema(&req).await))
    }
}

impl QoreMcp {
    async fn table_resources(&self) -> Result<Vec<rmcp::model::Resource>, String> {
        let ctx = self.tool_ctx();
        let mut out = Vec::new();
        for connection in self.exposed_connections()? {
            let Ok(session) = self.ensure_session(&connection.id).await else {
                tracing::warn!("skipping unreachable connection {}", connection.id);
                continue;
            };
            let Ok(namespaces) = agent_tools::list_namespaces(&ctx, session).await else {
                continue;
            };
            for namespace in namespaces {
                let Ok(list) = agent_tools::list_tables(&ctx, session, &namespace, None).await
                else {
                    continue;
                };
                for table in list.collections {
                    out.push(resources::table_resource(
                        &connection.id,
                        &connection.name,
                        &namespace,
                        &table.name,
                    ));
                    if out.len() >= resources::MAX_LISTED {
                        return Ok(out);
                    }
                }
            }
        }
        Ok(out)
    }

    async fn read_table_resource(&self, uri: &str) -> Result<String, String> {
        let table = resources::parse_uri(uri)?;
        let session = self.ensure_session(&table.connection_id).await?;
        let schema = agent_tools::describe_table(
            &self.tool_ctx(),
            session,
            &table.namespace,
            &table.table,
            None,
        )
        .await?;
        to_json(&schema)
    }
}

#[tool_handler]
impl ServerHandler for QoreMcp {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.protocol_version = ProtocolVersion::V_2025_06_18;
        info.capabilities = ServerCapabilities::builder()
            .enable_tools()
            .enable_resources()
            .enable_prompts()
            .build();
        info.server_info = Implementation::from_build_env();
        info.instructions = Some(INSTRUCTIONS.to_string());
        info
    }

    async fn initialize(
        &self,
        _request: InitializeRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        Ok(self.get_info())
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        let resources = self
            .table_resources()
            .await
            .map_err(|e| McpError::internal_error(e, None))?;
        Ok(ListResourcesResult::with_all_items(resources))
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(ListResourceTemplatesResult::with_all_items(vec![
            resources::template(),
        ]))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        let text = self
            .read_table_resource(&request.uri)
            .await
            .map_err(|e| McpError::resource_not_found(e, None))?;
        Ok(ReadResourceResult::new(vec![
            ResourceContents::TextResourceContents {
                uri: request.uri,
                mime_type: Some(resources::MIME_TYPE.to_string()),
                text,
                meta: None,
            },
        ]))
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        Ok(ListPromptsResult::with_all_items(prompts::definitions()))
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        prompts::render(&request)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args()
        .skip(1)
        .any(|arg| arg == "--version" || arg == "-V")
    {
        println!("qore-mcp {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("starting qore-mcp (stdio)");

    let service = QoreMcp::new(config_dir()).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
