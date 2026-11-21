//! Request execution context: cancellation, progress, logging, subscriptions, and client calls.

use crate::types::{ClientCapabilities, LogLevel, Root};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fmt;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
    mpsc,
};
use std::time::Duration;
use voxy_core::CancelToken;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotificationScope {
    Request,
    Session,
}

pub trait OutboundSink: Send + Sync {
    fn emit_notification(&self, scope: NotificationScope, method: &str, params: Value);

    fn send_client_request(
        &self,
        id: Value,
        method: &str,
        params: Value,
    ) -> Result<mpsc::Receiver<Value>, String>;
}

#[derive(Debug, Default)]
pub struct NoopSink;

impl OutboundSink for NoopSink {
    fn emit_notification(&self, _: NotificationScope, _: &str, _: Value) {}

    fn send_client_request(
        &self,
        _: Value,
        _: &str,
        _: Value,
    ) -> Result<mpsc::Receiver<Value>, String> {
        Err("client-bound requests require an active interactive transport".to_owned())
    }
}

#[derive(Debug, Default)]
pub struct SessionControls {
    log_level: Mutex<Option<LogLevel>>,
    subscriptions: Mutex<BTreeSet<String>>,
    list_changed_opt_in: std::sync::atomic::AtomicBool,
    client_caps: Mutex<ClientCapabilities>,
    client_req_seq: AtomicU64,
}

impl SessionControls {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_client_capabilities(&self, caps: ClientCapabilities) {
        if let Ok(mut guard) = self.client_caps.lock() {
            *guard = caps;
        }
    }

    #[must_use]
    pub fn client_capabilities(&self) -> ClientCapabilities {
        self.client_caps
            .lock()
            .map_or_else(|e| e.into_inner().clone(), |g| g.clone())
    }

    pub fn set_log_level(&self, level: LogLevel) {
        if let Ok(mut guard) = self.log_level.lock() {
            *guard = Some(level);
        }
    }

    #[must_use]
    pub fn should_log(&self, level: LogLevel) -> bool {
        self.log_level
            .lock()
            .ok()
            .and_then(|g| *g)
            .is_some_and(|min_level| level >= min_level)
    }

    pub fn subscribe(&self, uri: impl Into<String>) {
        self.opt_in_list_changed();
        if let Ok(mut guard) = self.subscriptions.lock() {
            guard.insert(uri.into());
        }
    }

    pub fn unsubscribe(&self, uri: &str) {
        if let Ok(mut guard) = self.subscriptions.lock() {
            guard.remove(uri);
        }
    }

    #[must_use]
    pub fn is_subscribed(&self, uri: &str) -> bool {
        self.subscriptions
            .lock()
            .is_ok_and(|guard| guard.contains(uri))
    }

    pub fn opt_in_list_changed(&self) {
        self.list_changed_opt_in.store(true, Ordering::Release);
    }

    #[must_use]
    pub fn list_changed_enabled(&self) -> bool {
        self.list_changed_opt_in.load(Ordering::Acquire)
    }

    #[must_use]
    pub fn next_client_request_id(&self) -> Value {
        let seq = self.client_req_seq.fetch_add(1, Ordering::Relaxed) + 1;
        Value::String(format!("voxy-srv-{seq}"))
    }
}

#[derive(Clone)]
pub struct RequestContext {
    controls: Arc<SessionControls>,
    sink: Arc<dyn OutboundSink>,
    cancel: CancelToken,
    progress_token: Option<Value>,
}

impl fmt::Debug for RequestContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RequestContext")
            .field("cancelled", &self.cancel.is_cancelled())
            .field("has_progress_token", &self.progress_token.is_some())
            .finish_non_exhaustive()
    }
}

impl RequestContext {
    #[must_use]
    pub fn new(
        controls: Arc<SessionControls>,
        sink: Arc<dyn OutboundSink>,
        cancel: CancelToken,
        progress_token: Option<Value>,
    ) -> Self {
        Self {
            controls,
            sink,
            cancel,
            progress_token,
        }
    }

    #[must_use]
    pub fn standalone() -> Self {
        Self::new(
            Arc::new(SessionControls::new()),
            Arc::new(NoopSink),
            CancelToken::new(),
            None,
        )
    }

    #[must_use]
    pub fn cancel_token(&self) -> &CancelToken {
        &self.cancel
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    pub fn report_progress(&self, progress: f64, total: Option<f64>, message: Option<&str>) {
        let Some(token) = &self.progress_token else {
            return;
        };
        let mut params = json!({
            "progressToken": token,
            "progress": progress,
        });
        if let Some(total) = total {
            params["total"] = json!(total);
        }
        if let Some(message) = message {
            params["message"] = json!(message);
        }
        self.sink
            .emit_notification(NotificationScope::Request, "notifications/progress", params);
    }

    pub fn log(&self, level: LogLevel, logger: &str, data: Value) {
        if !self.controls.should_log(level) {
            return;
        }
        self.sink.emit_notification(
            NotificationScope::Session,
            "notifications/message",
            json!({
                "level": level,
                "logger": logger,
                "data": data,
            }),
        );
    }

    pub fn notify_resource_updated(&self, uri: &str) {
        if !self.controls.is_subscribed(uri) {
            return;
        }
        self.sink.emit_notification(
            NotificationScope::Session,
            "notifications/resources/updated",
            json!({ "uri": uri }),
        );
    }

    pub fn notify_resources_list_changed(&self) {
        if self.controls.list_changed_enabled() {
            self.sink.emit_notification(
                NotificationScope::Session,
                "notifications/resources/list_changed",
                json!({}),
            );
        }
    }

    pub fn notify_tools_list_changed(&self) {
        if self.controls.list_changed_enabled() {
            self.sink.emit_notification(
                NotificationScope::Session,
                "notifications/tools/list_changed",
                json!({}),
            );
        }
    }

    pub fn notify_prompts_list_changed(&self) {
        if self.controls.list_changed_enabled() {
            self.sink.emit_notification(
                NotificationScope::Session,
                "notifications/prompts/list_changed",
                json!({}),
            );
        }
    }

    pub fn call_client(
        &self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<Value, String> {
        let id = self.controls.next_client_request_id();
        let rx = self.sink.send_client_request(id, method, params)?;
        let reply = rx
            .recv_timeout(timeout)
            .map_err(|_| format!("timed out waiting for client response to {method}"))?;
        if let Some(err) = reply.get("error") {
            let msg = err
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("client rejected request");
            return Err(format!("client error for {method}: {msg}"));
        }
        Ok(reply.get("result").cloned().unwrap_or(Value::Null))
    }

    pub fn ping_client(&self) -> Result<(), String> {
        self.call_client("ping", json!({}), Duration::from_secs(5))?;
        Ok(())
    }

    pub fn list_roots(&self) -> Result<Vec<Root>, String> {
        if !self.controls.client_capabilities().supports_roots() {
            return Err("connected MCP client did not declare roots capability".to_owned());
        }
        let result = self.call_client("roots/list", json!({}), Duration::from_secs(10))?;
        let roots = result
            .get("roots")
            .cloned()
            .ok_or_else(|| "client roots/list reply missing roots array".to_owned())?;
        serde_json::from_value(roots).map_err(|e| format!("invalid client roots payload: {e}"))
    }

    pub fn create_sampling_message(&self, params: Value) -> Result<Value, String> {
        if !self.controls.client_capabilities().supports_sampling() {
            return Err("connected MCP client did not declare sampling capability".to_owned());
        }
        self.call_client("sampling/createMessage", params, Duration::from_secs(30))
    }

    pub fn create_elicitation(&self, params: Value) -> Result<Value, String> {
        if !self.controls.client_capabilities().supports_elicitation() {
            return Err("connected MCP client did not declare elicitation capability".to_owned());
        }
        self.call_client("elicitation/create", params, Duration::from_secs(30))
    }
}
