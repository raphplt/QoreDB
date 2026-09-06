// SPDX-License-Identifier: Apache-2.0

//! `qore://{connection_id}/{database}[/{schema}]/{table}` resources: one per
//! table of the exposed connections, each resolving to its `describe_table`.

use qore_core::Namespace;
use rmcp::model::{AnnotateAble, RawResource, RawResourceTemplate, Resource, ResourceTemplate};

pub const SCHEME: &str = "qore://";
pub const MIME_TYPE: &str = "application/json";
/// Keeps `resources/list` bounded on large estates; tables past the cap stay
/// reachable through the template and `read_resource`.
pub const MAX_LISTED: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableRef {
    pub connection_id: String,
    pub namespace: Namespace,
    pub table: String,
}

pub fn format_uri(connection_id: &str, namespace: &Namespace, table: &str) -> String {
    match namespace.schema.as_deref() {
        Some(schema) => format!(
            "{SCHEME}{connection_id}/{}/{schema}/{table}",
            namespace.database
        ),
        None => format!("{SCHEME}{connection_id}/{}/{table}", namespace.database),
    }
}

pub fn parse_uri(uri: &str) -> Result<TableRef, String> {
    let rest = uri
        .strip_prefix(SCHEME)
        .ok_or_else(|| format!("Unsupported resource URI: {uri}"))?;
    let parts: Vec<&str> = rest.split('/').collect();
    let valid = matches!(parts.len(), 3 | 4) && parts.iter().all(|p| !p.is_empty());
    if !valid {
        return Err(format!(
            "Invalid resource URI '{uri}': expected {SCHEME}{{connection_id}}/{{database}}[/{{schema}}]/{{table}}"
        ));
    }
    let schema = (parts.len() == 4).then(|| parts[2].to_string());
    Ok(TableRef {
        connection_id: parts[0].to_string(),
        namespace: Namespace {
            database: parts[1].to_string(),
            schema,
        },
        table: parts[parts.len() - 1].to_string(),
    })
}

pub fn table_resource(
    connection_id: &str,
    connection_name: &str,
    namespace: &Namespace,
    table: &str,
) -> Resource {
    let location = match namespace.schema.as_deref() {
        Some(schema) => format!("{}.{schema}.{table}", namespace.database),
        None => format!("{}.{table}", namespace.database),
    };
    let mut raw = RawResource::new(format_uri(connection_id, namespace, table), table)
        .with_title(format!("{connection_name}: {location}"));
    raw.description = Some(format!(
        "Columns, primary key, foreign keys and indexes of {location} (read-only)"
    ));
    raw.mime_type = Some(MIME_TYPE.to_string());
    raw.no_annotation()
}

pub fn template() -> ResourceTemplate {
    let mut raw = RawResourceTemplate::new(
        format!("{SCHEME}{{connection_id}}/{{database}}/{{table}}"),
        "table-schema",
    );
    raw.title = Some("Table schema".to_string());
    raw.description = Some(
        "Schema of one table on an exposed connection. Add a schema segment before the \
         table for engines that use schemas: qore://{connection_id}/{database}/{schema}/{table}"
            .to_string(),
    );
    raw.mime_type = Some(MIME_TYPE.to_string());
    raw.no_annotation()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uri_roundtrips_with_and_without_schema() {
        let plain = Namespace::new("shop");
        let uri = format_uri("conn_1", &plain, "orders");
        assert_eq!(uri, "qore://conn_1/shop/orders");
        assert_eq!(
            parse_uri(&uri).unwrap(),
            TableRef {
                connection_id: "conn_1".into(),
                namespace: plain,
                table: "orders".into(),
            }
        );

        let scoped = Namespace::with_schema("shop", "public");
        let uri = format_uri("conn_1", &scoped, "orders");
        assert_eq!(uri, "qore://conn_1/shop/public/orders");
        assert_eq!(parse_uri(&uri).unwrap().namespace, scoped);
    }

    #[test]
    fn malformed_uris_are_rejected() {
        assert!(parse_uri("file:///etc/passwd").is_err());
        assert!(parse_uri("qore://conn_1/shop").is_err());
        assert!(parse_uri("qore://conn_1//orders").is_err());
        assert!(parse_uri("qore://a/b/c/d/e").is_err());
    }
}
