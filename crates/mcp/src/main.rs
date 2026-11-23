//! MCP server over stdio. Reads and writes the task database; the only tool that reaches the network is `import_linear_plan`.

mod tools;

use std::path::PathBuf;

use rmcp::{
    ServerHandler, ServiceExt,
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ListToolsResult,
        ServerCapabilities, ServerConfig,
    },
    service::RequestContext,
    transport::stdio,
};
use taskapp_core::Db;
use tools::Server;

impl ServerHandler for Server {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Cairn: reads and writes the local task database. The only tool that reaches the network is import_linear_plan, which reads Linear with this server's LINEAR_API_KEY; every other tool is local.",
        )
    }

    fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, rmcp::ErrorData>> + Send + '_ {
        std::future::ready(Ok(ListToolsResult::with_all_items(tools::definitions())))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<CallToolResponse, rmcp::ErrorData> {
        let args = serde_json::Value::Object(request.arguments.unwrap_or_default());
        // Failures are tool errors the model can read, never protocol errors or panics.
        let result = match self.call(&request.name, args).await {
            Ok(v) => CallToolResult::success(vec![ContentBlock::text(v.to_string())]),
            Err(msg) => CallToolResult::error(vec![ContentBlock::text(msg)]),
        };
        Ok(result.into())
    }
}

/// `--db <path>`, or the app's database. There is deliberately no other default.
fn db_path() -> Result<PathBuf, String> {
    let mut args = std::env::args().skip(1);
    match (args.next().as_deref(), args.next()) {
        (Some("--db"), Some(p)) => Ok(PathBuf::from(p)),
        (None, _) => {
            let home = std::env::var_os("HOME").ok_or("HOME is not set; pass --db <path>")?;
            Ok(PathBuf::from(home).join("Library/Application Support/taskapp/taskapp.db"))
        }
        _ => Err("usage: taskapp-mcp [--db <path>]".into()),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = db_path()?;
    let db = Db::open(&path).await?;
    let dir = path
        .parent()
        .map_or_else(PathBuf::new, |p| p.join("backups"));
    Box::pin(Server::new(db, dir).serve(stdio()))
        .await?
        .waiting()
        .await?;
    Ok(())
}
