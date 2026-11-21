//! Shared MCP `2025-06-18` protocol router and dual transport (`stdio` + loopback `Streamable HTTP`).

pub mod context;
pub mod http;
pub mod pagination;
pub mod router;
pub mod stdio;
pub mod types;

use serde_json::Value;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};

pub use context::{NoopSink, NotificationScope, OutboundSink, RequestContext, SessionControls};
pub use http::{HttpServer, serve_http};
pub use router::{DispatchOutcome, McpHandler, Router, RpcError};
pub use stdio::{serve_stdio, serve_streams};
pub use types::{
    CallToolResult, CompleteParams, CompletionRef, CompletionValues, ContentBlock,
    DEFAULT_PAGE_SIZE, GetPromptResult, LATEST_PROTOCOL_VERSION, LogLevel, PromptArgument,
    PromptDef, PromptMessage, ResourceContents, ResourceDef, ResourceTemplateDef, Root,
    SUPPORTED_PROTOCOL_VERSIONS, ServerConfig, ToolAnnotations, ToolDef, base64_encode,
};
pub use voxy_core::CancelToken;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportMode {
    Stdio,
    Http(SocketAddr),
}

pub fn parse_transport_args<I, S>(args: I) -> Result<(TransportMode, Vec<String>), String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut mode = TransportMode::Stdio;
    let mut rest = Vec::new();
    let mut iter = args.into_iter().map(Into::into);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--http" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--http requires <127.0.0.1:port>".to_owned())?;
                let addr: SocketAddr = raw
                    .parse()
                    .map_err(|e| format!("invalid --http socket address `{raw}`: {e}"))?;
                if !addr.ip().is_loopback() {
                    return Err(format!("--http must bind a loopback address, got {addr}"));
                }
                mode = TransportMode::Http(addr);
            }
            "--http-port" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--http-port requires a TCP port number".to_owned())?;
                let port: u16 = raw
                    .parse()
                    .map_err(|e| format!("invalid --http-port `{raw}`: {e}"))?;
                mode = TransportMode::Http(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port));
            }
            _ => rest.push(arg),
        }
    }
    Ok((mode, rest))
}

pub fn serve<H: McpHandler + 'static>(handler: Arc<H>, mode: TransportMode) -> io::Result<()> {
    match mode {
        TransportMode::Stdio => serve_stdio(handler),
        TransportMode::Http(addr) => {
            let server = HttpServer::spawn(handler, addr)?;
            eprintln!("VOXY_MCP_HTTP_LISTEN http://{}/mcp", server.addr());
            server.wait();
            Ok(())
        }
    }
}

pub fn serve_blocking<H: McpHandler + 'static>(
    handler: Arc<H>,
    mode: TransportMode,
    on_http_ready: impl FnOnce(SocketAddr),
) -> io::Result<()> {
    match mode {
        TransportMode::Stdio => serve_stdio(handler),
        TransportMode::Http(addr) => {
            let server = HttpServer::spawn(handler, addr)?;
            on_http_ready(server.addr());
            server.wait();
            Ok(())
        }
    }
}

pub trait MutableMcpHandler: Send {
    fn config(&self) -> ServerConfig;
    fn list_tools(&self) -> Vec<ToolDef>;
    fn call_tool(
        &mut self,
        ctx: &RequestContext,
        name: &str,
        arguments: Value,
    ) -> Result<CallToolResult, String>;

    fn list_resources(&mut self) -> Vec<ResourceDef> {
        Vec::new()
    }

    fn list_resource_templates(&self) -> Vec<ResourceTemplateDef> {
        Vec::new()
    }

    fn read_resource(
        &mut self,
        _ctx: &RequestContext,
        uri: &str,
    ) -> Result<Vec<ResourceContents>, RpcError> {
        Err(RpcError::resource_not_found(uri))
    }

    fn list_prompts(&self) -> Vec<PromptDef> {
        Vec::new()
    }

    fn get_prompt(
        &mut self,
        _ctx: &RequestContext,
        name: &str,
        _arguments: Value,
    ) -> Result<GetPromptResult, RpcError> {
        Err(RpcError::invalid_params(format!("unknown prompt: {name}")))
    }

    fn complete(
        &mut self,
        _ctx: &RequestContext,
        _params: &CompleteParams,
    ) -> Result<CompletionValues, RpcError> {
        Ok(CompletionValues::default())
    }
}

#[derive(Debug)]
pub struct MutexMcpHandler<T> {
    inner: Mutex<T>,
    config: ServerConfig,
    tools: Vec<ToolDef>,
    templates: Vec<ResourceTemplateDef>,
    prompts: Vec<PromptDef>,
}

impl<T: MutableMcpHandler> MutexMcpHandler<T> {
    #[must_use]
    pub fn new(inner: T) -> Self {
        let config = inner.config();
        let tools = inner.list_tools();
        let templates = inner.list_resource_templates();
        let prompts = inner.list_prompts();
        Self {
            inner: Mutex::new(inner),
            config,
            tools,
            templates,
            prompts,
        }
    }

    pub fn with_inner<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        f(&mut guard)
    }
}

impl<T: MutableMcpHandler> McpHandler for MutexMcpHandler<T> {
    fn config(&self) -> ServerConfig {
        self.config.clone()
    }

    fn list_tools(&self) -> Vec<ToolDef> {
        self.tools.clone()
    }

    fn call_tool(
        &self,
        ctx: &RequestContext,
        name: &str,
        arguments: Value,
    ) -> Result<CallToolResult, String> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| "server state poisoned".to_owned())?;
        guard.call_tool(ctx, name, arguments)
    }

    fn list_resources(&self) -> Vec<ResourceDef> {
        self.inner
            .lock()
            .map_or_else(|_| Vec::new(), |mut g| g.list_resources())
    }

    fn list_resource_templates(&self) -> Vec<ResourceTemplateDef> {
        self.templates.clone()
    }

    fn read_resource(
        &self,
        ctx: &RequestContext,
        uri: &str,
    ) -> Result<Vec<ResourceContents>, RpcError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| RpcError::new(-32603, "server state poisoned"))?;
        guard.read_resource(ctx, uri)
    }

    fn list_prompts(&self) -> Vec<PromptDef> {
        self.prompts.clone()
    }

    fn get_prompt(
        &self,
        ctx: &RequestContext,
        name: &str,
        arguments: Value,
    ) -> Result<GetPromptResult, RpcError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| RpcError::new(-32603, "server state poisoned"))?;
        guard.get_prompt(ctx, name, arguments)
    }

    fn complete(
        &self,
        ctx: &RequestContext,
        params: &CompleteParams,
    ) -> Result<CompletionValues, RpcError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| RpcError::new(-32603, "server state poisoned"))?;
        guard.complete(ctx, params)
    }
}
