//! MCP `2025-06-18` JSON-RPC 2.0 session router with cancellation, progress, pagination,
//! logging, resources/subscriptions, prompts, and completions.

use crate::context::{NoopSink, OutboundSink, RequestContext, SessionControls};
use crate::pagination::{CursorParams, paginate};
use crate::types::{
    CallToolResult, CompleteParams, CompleteResult, CompletionValues, GetPromptResult,
    INVALID_PARAMS, INVALID_REQUEST, InitializeParams, LATEST_PROTOCOL_VERSION, LogLevel,
    METHOD_NOT_FOUND, NOT_INITIALIZED, PARSE_ERROR, PromptDef, RESOURCE_NOT_FOUND,
    ReadResourceResult, ResourceContents, ResourceDef, ResourceTemplateDef,
    SUPPORTED_PROTOCOL_VERSIONS, ServerConfig, ToolDef, jsonrpc_error, jsonrpc_result,
    server_capabilities,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use voxy_core::CancelToken;

#[derive(Clone, Debug)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
}

impl RpcError {
    #[must_use]
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(INVALID_PARAMS, message)
    }

    #[must_use]
    pub fn resource_not_found(uri: &str) -> Self {
        Self::new(RESOURCE_NOT_FOUND, format!("resource not found: {uri}"))
    }
}

impl<E: std::fmt::Display> From<E> for RpcError {
    fn from(value: E) -> Self {
        Self::invalid_params(value.to_string())
    }
}

pub trait McpHandler: Send + Sync {
    fn config(&self) -> ServerConfig;
    fn list_tools(&self) -> Vec<ToolDef>;
    fn call_tool(
        &self,
        ctx: &RequestContext,
        name: &str,
        arguments: Value,
    ) -> Result<CallToolResult, String>;

    fn list_resources(&self) -> Vec<ResourceDef> {
        Vec::new()
    }

    fn list_resource_templates(&self) -> Vec<ResourceTemplateDef> {
        Vec::new()
    }

    fn read_resource(
        &self,
        _ctx: &RequestContext,
        uri: &str,
    ) -> Result<Vec<ResourceContents>, RpcError> {
        Err(RpcError::resource_not_found(uri))
    }

    fn list_prompts(&self) -> Vec<PromptDef> {
        Vec::new()
    }

    fn get_prompt(
        &self,
        _ctx: &RequestContext,
        name: &str,
        _arguments: Value,
    ) -> Result<GetPromptResult, RpcError> {
        Err(RpcError::invalid_params(format!("unknown prompt: {name}")))
    }

    fn complete(
        &self,
        _ctx: &RequestContext,
        _params: &CompleteParams,
    ) -> Result<CompletionValues, RpcError> {
        Ok(CompletionValues::default())
    }

    fn on_roots_changed(&self, _ctx: &RequestContext) {}
}

#[derive(Clone, Debug)]
pub enum DispatchOutcome {
    Reply(Value),
    Cancelled,
    None,
}

#[derive(Debug)]
pub struct Router<H> {
    handler: Arc<H>,
    controls: Arc<SessionControls>,
    initialized: AtomicBool,
    in_flight: Mutex<HashMap<String, CancelToken>>,
}

impl<H: McpHandler> Router<H> {
    #[must_use]
    pub fn new(handler: Arc<H>) -> Self {
        Self {
            handler,
            controls: Arc::new(SessionControls::new()),
            initialized: AtomicBool::new(false),
            in_flight: Mutex::new(HashMap::new()),
        }
    }

    #[must_use]
    pub fn controls(&self) -> &Arc<SessionControls> {
        &self.controls
    }

    #[must_use]
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::Acquire)
    }

    pub fn cancel_request_id(&self, request_id: &Value) {
        let key = request_id.to_string();
        if let Ok(guard) = self.in_flight.lock()
            && let Some(token) = guard.get(&key)
        {
            token.cancel();
        }
    }

    #[must_use]
    pub fn dispatch_sync(&self, message: &Value) -> Value {
        let sink: Arc<dyn OutboundSink> = Arc::new(NoopSink);
        match self.dispatch(message, &sink) {
            DispatchOutcome::Reply(reply) => reply,
            DispatchOutcome::Cancelled => jsonrpc_error(
                message.get("id").cloned().unwrap_or(Value::Null),
                INVALID_REQUEST,
                "request cancelled",
            ),
            DispatchOutcome::None => Value::Null,
        }
    }

    #[must_use]
    pub fn dispatch_raw(&self, raw: &str, sink: &Arc<dyn OutboundSink>) -> DispatchOutcome {
        let Ok(value) = serde_json::from_str::<Value>(raw) else {
            return DispatchOutcome::Reply(jsonrpc_error(
                Value::Null,
                PARSE_ERROR,
                "Parse error: invalid JSON",
            ));
        };
        self.dispatch(&value, sink)
    }

    #[must_use]
    pub fn dispatch(&self, message: &Value, sink: &Arc<dyn OutboundSink>) -> DispatchOutcome {
        let Some(object) = message.as_object() else {
            return DispatchOutcome::Reply(jsonrpc_error(
                Value::Null,
                INVALID_REQUEST,
                "Invalid Request: expected JSON object",
            ));
        };
        if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
            let id = object.get("id").cloned().unwrap_or(Value::Null);
            return DispatchOutcome::Reply(jsonrpc_error(
                id,
                INVALID_REQUEST,
                "Invalid Request: jsonrpc must be \"2.0\"",
            ));
        }
        let Some(method) = object.get("method").and_then(Value::as_str) else {
            let id = object.get("id").cloned().unwrap_or(Value::Null);
            return DispatchOutcome::Reply(jsonrpc_error(
                id,
                INVALID_REQUEST,
                "Invalid Request: missing method string",
            ));
        };
        let params = object.get("params").cloned().unwrap_or(Value::Null);
        let Some(id) = object.get("id").cloned() else {
            self.handle_notification(method, &params, sink);
            return DispatchOutcome::None;
        };
        let token = CancelToken::new();
        let progress = params
            .get("_meta")
            .and_then(|meta| meta.get("progressToken"))
            .filter(|v| v.is_string() || v.is_number())
            .cloned();
        let key = id.to_string();
        if let Ok(mut guard) = self.in_flight.lock() {
            guard.insert(key.clone(), token.clone());
        }
        let ctx = RequestContext::new(
            Arc::clone(&self.controls),
            Arc::clone(sink),
            token.clone(),
            progress,
        );
        let outcome = match self.handle_request(&ctx, method, params) {
            Ok(_) if token.is_cancelled() => DispatchOutcome::Cancelled,
            Ok(result) => DispatchOutcome::Reply(jsonrpc_result(id, result)),
            Err(_) if token.is_cancelled() => DispatchOutcome::Cancelled,
            Err(err) => DispatchOutcome::Reply(jsonrpc_error(id, err.code, err.message)),
        };
        if let Ok(mut guard) = self.in_flight.lock() {
            guard.remove(&key);
        }
        outcome
    }

    fn handle_notification(&self, method: &str, params: &Value, sink: &Arc<dyn OutboundSink>) {
        match method {
            "notifications/initialized" | "initialized" => {
                self.initialized.store(true, Ordering::Release);
            }
            "notifications/cancelled" => {
                if let Some(id) = params.get("requestId") {
                    self.cancel_request_id(id);
                }
            }
            "notifications/roots/list_changed" => {
                let ctx = RequestContext::new(
                    Arc::clone(&self.controls),
                    Arc::clone(sink),
                    CancelToken::new(),
                    None,
                );
                self.handler.on_roots_changed(&ctx);
            }
            _ => {}
        }
    }

    fn handle_request(
        &self,
        ctx: &RequestContext,
        method: &str,
        params: Value,
    ) -> Result<Value, RpcError> {
        match method {
            "initialize" => self.handle_initialize(params),
            "ping" => Ok(json!({})),
            _ if !self.is_initialized() => {
                Err(RpcError::new(NOT_INITIALIZED, "Initialize session first"))
            }
            "logging/setLevel" => self.handle_set_level(params),
            "tools/list" => self.handle_tools_list(params),
            "tools/call" => self.handle_tools_call(ctx, params),
            "resources/list" => self.handle_resources_list(params),
            "resources/templates/list" => self.handle_resource_templates_list(params),
            "resources/read" => self.handle_resources_read(ctx, params),
            "resources/subscribe" => self.handle_resources_subscribe(params, true),
            "resources/unsubscribe" => self.handle_resources_subscribe(params, false),
            "prompts/list" => self.handle_prompts_list(params),
            "prompts/get" => self.handle_prompts_get(ctx, params),
            "completion/complete" => self.handle_complete(ctx, params),
            _ => Err(RpcError::new(
                METHOD_NOT_FOUND,
                format!("Method not found: {method}"),
            )),
        }
    }

    fn handle_initialize(&self, params: Value) -> Result<Value, RpcError> {
        let parsed: InitializeParams = serde_json::from_value(params)
            .map_err(|e| RpcError::invalid_params(format!("invalid initialize params: {e}")))?;
        let negotiated = if SUPPORTED_PROTOCOL_VERSIONS.contains(&parsed.protocol_version.as_str())
        {
            parsed.protocol_version
        } else {
            LATEST_PROTOCOL_VERSION.to_owned()
        };
        self.controls.set_client_capabilities(parsed.capabilities);
        self.initialized.store(true, Ordering::Release);
        let cfg = self.handler.config();
        let mut server_info = json!({
            "name": cfg.name,
            "version": cfg.version,
        });
        if let Some(title) = cfg.title {
            server_info["title"] = Value::String(title);
        }
        let mut result = json!({
            "protocolVersion": negotiated,
            "capabilities": server_capabilities(),
            "serverInfo": server_info,
        });
        if let Some(instructions) = cfg.instructions {
            result["instructions"] = Value::String(instructions);
        }
        Ok(result)
    }

    fn handle_set_level(&self, params: Value) -> Result<Value, RpcError> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct SetLevelParams {
            level: LogLevel,
            #[serde(default, rename = "_meta")]
            _meta: Option<Value>,
        }
        let parsed: SetLevelParams = serde_json::from_value(params)
            .map_err(|e| RpcError::invalid_params(format!("invalid log level params: {e}")))?;
        self.controls.set_log_level(parsed.level);
        Ok(json!({}))
    }

    fn handle_tools_list(&self, params: Value) -> Result<Value, RpcError> {
        let cursor = parse_cursor_param(params)?;
        let tools = self.handler.list_tools();
        let page = paginate(&tools, cursor.as_deref(), self.handler.config().page_size)
            .map_err(RpcError::invalid_params)?;
        let mut reply = json!({ "tools": page.items });
        if let Some(next) = page.next_cursor {
            reply["nextCursor"] = Value::String(next);
        }
        Ok(reply)
    }

    fn handle_tools_call(&self, ctx: &RequestContext, params: Value) -> Result<Value, RpcError> {
        #[derive(Deserialize)]
        struct CallParams {
            name: String,
            #[serde(default)]
            arguments: Option<Value>,
        }
        let parsed: CallParams = serde_json::from_value(params)
            .map_err(|e| RpcError::invalid_params(format!("invalid tools/call params: {e}")))?;
        let arguments = match parsed.arguments {
            None | Some(Value::Null) => json!({}),
            Some(obj @ Value::Object(_)) => obj,
            Some(_) => {
                return Err(RpcError::invalid_params(
                    "tools/call arguments must be an object",
                ));
            }
        };
        let call_result = match self.handler.call_tool(ctx, &parsed.name, arguments) {
            Ok(result) => result,
            Err(message) => CallToolResult::error(message),
        };
        serde_json::to_value(call_result).map_err(|e| RpcError::new(-32603, e.to_string()))
    }

    fn handle_resources_list(&self, params: Value) -> Result<Value, RpcError> {
        let cursor = parse_cursor_param(params)?;
        let resources = self.handler.list_resources();
        let page = paginate(
            &resources,
            cursor.as_deref(),
            self.handler.config().page_size,
        )
        .map_err(RpcError::invalid_params)?;
        let mut reply = json!({ "resources": page.items });
        if let Some(next) = page.next_cursor {
            reply["nextCursor"] = Value::String(next);
        }
        Ok(reply)
    }

    fn handle_resource_templates_list(&self, params: Value) -> Result<Value, RpcError> {
        let cursor = parse_cursor_param(params)?;
        let templates = self.handler.list_resource_templates();
        let page = paginate(
            &templates,
            cursor.as_deref(),
            self.handler.config().page_size,
        )
        .map_err(RpcError::invalid_params)?;
        let mut reply = json!({ "resourceTemplates": page.items });
        if let Some(next) = page.next_cursor {
            reply["nextCursor"] = Value::String(next);
        }
        Ok(reply)
    }

    fn handle_resources_read(
        &self,
        ctx: &RequestContext,
        params: Value,
    ) -> Result<Value, RpcError> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct ReadParams {
            uri: String,
            #[serde(default, rename = "_meta")]
            _meta: Option<Value>,
        }
        let parsed: ReadParams = serde_json::from_value(params)
            .map_err(|e| RpcError::invalid_params(format!("invalid resources/read params: {e}")))?;
        let contents = self.handler.read_resource(ctx, &parsed.uri)?;
        serde_json::to_value(ReadResourceResult { contents })
            .map_err(|e| RpcError::new(-32603, e.to_string()))
    }

    fn handle_resources_subscribe(
        &self,
        params: Value,
        subscribe: bool,
    ) -> Result<Value, RpcError> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct SubParams {
            uri: String,
            #[serde(default, rename = "_meta")]
            _meta: Option<Value>,
        }
        let parsed: SubParams = serde_json::from_value(params)
            .map_err(|e| RpcError::invalid_params(format!("invalid subscription params: {e}")))?;
        if subscribe {
            self.controls.subscribe(parsed.uri);
        } else {
            self.controls.unsubscribe(&parsed.uri);
        }
        Ok(json!({}))
    }

    fn handle_prompts_list(&self, params: Value) -> Result<Value, RpcError> {
        let cursor = parse_cursor_param(params)?;
        let prompts = self.handler.list_prompts();
        let page = paginate(&prompts, cursor.as_deref(), self.handler.config().page_size)
            .map_err(RpcError::invalid_params)?;
        let mut reply = json!({ "prompts": page.items });
        if let Some(next) = page.next_cursor {
            reply["nextCursor"] = Value::String(next);
        }
        Ok(reply)
    }

    fn handle_prompts_get(&self, ctx: &RequestContext, params: Value) -> Result<Value, RpcError> {
        #[derive(Deserialize)]
        struct GetPromptParams {
            name: String,
            #[serde(default)]
            arguments: Option<Value>,
        }
        let parsed: GetPromptParams = serde_json::from_value(params)
            .map_err(|e| RpcError::invalid_params(format!("invalid prompts/get params: {e}")))?;
        let arguments = parsed.arguments.unwrap_or_else(|| json!({}));
        let result = self.handler.get_prompt(ctx, &parsed.name, arguments)?;
        serde_json::to_value(result).map_err(|e| RpcError::new(-32603, e.to_string()))
    }

    fn handle_complete(&self, ctx: &RequestContext, params: Value) -> Result<Value, RpcError> {
        let parsed: CompleteParams = serde_json::from_value(params).map_err(|e| {
            RpcError::invalid_params(format!("invalid completion/complete params: {e}"))
        })?;
        let completion = self.handler.complete(ctx, &parsed)?;
        serde_json::to_value(CompleteResult { completion })
            .map_err(|e| RpcError::new(-32603, e.to_string()))
    }
}

fn parse_cursor_param(params: Value) -> Result<Option<String>, RpcError> {
    if params.is_null() {
        return Ok(None);
    }
    let parsed: CursorParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("invalid pagination params: {e}")))?;
    Ok(parsed.cursor)
}
