//! Loopback-only MCP `2025-06-18` Streamable HTTP + SSE transport (`std::net::TcpListener`).

use crate::context::{NotificationScope, OutboundSink};
use crate::router::{DispatchOutcome, McpHandler, Router};
use crate::types::{INVALID_REQUEST, PARSE_ERROR, SUPPORTED_PROTOCOL_VERSIONS, jsonrpc_error};
use serde_json::{Value, json};
use std::collections::{HashMap, VecDeque};
use std::io::{self, BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_HEADER_BYTES: usize = 64 * 1024;
const MAX_BODY_BYTES: usize = 8 * 1024 * 1024;
const MAX_HISTORY_EVENTS: usize = 256;

#[derive(Clone, Debug)]
struct SseEvent {
    id: u64,
    data: String,
}

enum PostStreamMsg {
    Event(SseEvent),
    Finish(Option<Value>),
}

#[derive(Debug)]
struct SseSubscriber {
    id: u64,
    tx: mpsc::Sender<SseEvent>,
}

#[derive(Debug)]
struct HttpSession<H> {
    id: String,
    router: Arc<Router<H>>,
    terminated: AtomicBool,
    event_seq: AtomicU64,
    subscriber_seq: AtomicU64,
    history: Mutex<VecDeque<SseEvent>>,
    subscribers: Mutex<Vec<SseSubscriber>>,
    pending_client: Mutex<HashMap<String, mpsc::Sender<Value>>>,
}

impl<H: McpHandler> HttpSession<H> {
    fn new(id: String, handler: Arc<H>) -> Self {
        Self {
            id,
            router: Arc::new(Router::new(handler)),
            terminated: AtomicBool::new(false),
            event_seq: AtomicU64::new(0),
            subscriber_seq: AtomicU64::new(0),
            history: Mutex::new(VecDeque::with_capacity(MAX_HISTORY_EVENTS)),
            subscribers: Mutex::new(Vec::new()),
            pending_client: Mutex::new(HashMap::new()),
        }
    }

    fn record_and_broadcast(&self, value: &Value) -> SseEvent {
        let id = self.event_seq.fetch_add(1, Ordering::Relaxed) + 1;
        let data = serde_json::to_string(value).unwrap_or_else(|_| "{}".to_owned());
        let event = SseEvent { id, data };
        if let Ok(mut history) = self.history.lock() {
            if history.len() >= MAX_HISTORY_EVENTS {
                history.pop_front();
            }
            history.push_back(event.clone());
        }
        if let Ok(mut subs) = self.subscribers.lock() {
            subs.retain(|sub| sub.tx.send(event.clone()).is_ok());
        }
        event
    }

    fn has_live_get_subscriber(&self) -> bool {
        self.subscribers.lock().is_ok_and(|subs| !subs.is_empty())
    }

    fn register_subscriber(
        self: &Arc<Self>,
        last_event_id: Option<u64>,
    ) -> (Vec<SseEvent>, mpsc::Receiver<SseEvent>, SubscriberGuard<H>) {
        let (tx, rx) = mpsc::channel();
        let sub_id = self.subscriber_seq.fetch_add(1, Ordering::Relaxed) + 1;
        let replay = self
            .history
            .lock()
            .map(|history| {
                history
                    .iter()
                    .filter(|ev| last_event_id.is_some_and(|after| ev.id > after))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        if let Ok(mut subs) = self.subscribers.lock() {
            subs.push(SseSubscriber { id: sub_id, tx });
        }
        (
            replay,
            rx,
            SubscriberGuard {
                session: Arc::clone(self),
                sub_id,
            },
        )
    }

    fn deliver_client_response(&self, msg: Value) {
        let Some(id) = msg.get("id") else {
            return;
        };
        if let Ok(mut pending) = self.pending_client.lock()
            && let Some(tx) = pending.remove(&id.to_string())
        {
            let _ = tx.send(msg);
        }
    }
}

struct SubscriberGuard<H: McpHandler> {
    session: Arc<HttpSession<H>>,
    sub_id: u64,
}

impl<H: McpHandler> Drop for SubscriberGuard<H> {
    fn drop(&mut self) {
        if let Ok(mut subs) = self.session.subscribers.lock() {
            subs.retain(|sub| sub.id != self.sub_id);
        }
    }
}

struct PostSink<H: McpHandler> {
    session: Arc<HttpSession<H>>,
    accepts_sse: bool,
    stream_tx: Mutex<Option<mpsc::Sender<PostStreamMsg>>>,
}

impl<H: McpHandler> PostSink<H> {
    fn new(
        session: Arc<HttpSession<H>>,
        accepts_sse: bool,
        stream_tx: Option<mpsc::Sender<PostStreamMsg>>,
    ) -> Self {
        Self {
            session,
            accepts_sse,
            stream_tx: Mutex::new(stream_tx),
        }
    }

    fn send_post_event(&self, event: SseEvent) -> bool {
        if !self.accepts_sse {
            return false;
        }
        let Ok(mut guard) = self.stream_tx.lock() else {
            return false;
        };
        if let Some(tx) = guard.as_ref() {
            if tx.send(PostStreamMsg::Event(event)).is_ok() {
                return true;
            }
            *guard = None;
        }
        false
    }
}

impl<H: McpHandler> OutboundSink for PostSink<H> {
    fn emit_notification(&self, scope: NotificationScope, method: &str, params: Value) {
        let payload = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        let event = self.session.record_and_broadcast(&payload);
        if scope == NotificationScope::Request {
            let _ = self.send_post_event(event);
        }
    }

    fn send_client_request(
        &self,
        id: Value,
        method: &str,
        params: Value,
    ) -> Result<mpsc::Receiver<Value>, String> {
        let key = id.to_string();
        let (tx, rx) = mpsc::channel();
        {
            let mut guard = self
                .session
                .pending_client
                .lock()
                .map_err(|_| "session client registry poisoned".to_owned())?;
            guard.insert(key, tx);
        }
        let payload = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let event = self.session.record_and_broadcast(&payload);
        if !self.session.has_live_get_subscriber() {
            let _ = self.send_post_event(event);
        }
        Ok(rx)
    }
}

#[derive(Debug)]
struct ServerState<H> {
    handler: Arc<H>,
    sessions: Mutex<HashMap<String, Arc<HttpSession<H>>>>,
    session_seq: AtomicU64,
    shutdown: AtomicBool,
}

impl<H: McpHandler> ServerState<H> {
    fn create_session(&self) -> Arc<HttpSession<H>> {
        let seq = self.session_seq.fetch_add(1, Ordering::Relaxed) + 1;
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let seed = format!("voxy-mcp:{}:{seq}:{nanos}", std::process::id());
        let session_id = blake3::hash(seed.as_bytes()).to_hex()[..32].to_owned();
        let session = Arc::new(HttpSession::new(
            session_id.clone(),
            Arc::clone(&self.handler),
        ));
        if let Ok(mut guard) = self.sessions.lock() {
            guard.insert(session_id, Arc::clone(&session));
        }
        session
    }

    fn get_session(&self, id: &str) -> Option<Arc<HttpSession<H>>> {
        let guard = self.sessions.lock().ok()?;
        guard
            .get(id)
            .filter(|s| !s.terminated.load(Ordering::Acquire))
            .cloned()
    }

    fn delete_session(&self, id: &str) -> bool {
        let removed = self
            .sessions
            .lock()
            .ok()
            .and_then(|mut guard| guard.remove(id));
        if let Some(session) = removed {
            session.terminated.store(true, Ordering::Release);
            if let Ok(mut subs) = session.subscribers.lock() {
                subs.clear();
            }
            true
        } else {
            false
        }
    }
}

#[derive(Debug)]
pub struct HttpServer<H: McpHandler> {
    addr: SocketAddr,
    state: Arc<ServerState<H>>,
    join: Option<JoinHandle<()>>,
}

impl<H: McpHandler + 'static> HttpServer<H> {
    pub fn spawn(handler: Arc<H>, addr: SocketAddr) -> io::Result<Self> {
        if !addr.ip().is_loopback() {
            return Err(io::Error::new(
                io::ErrorKind::AddrNotAvailable,
                format!("Streamable HTTP MCP binds loopback only (refused {addr})"),
            ));
        }
        let listener = TcpListener::bind(addr)?;
        let bound = listener.local_addr()?;
        listener.set_nonblocking(true)?;
        let state = Arc::new(ServerState {
            handler,
            sessions: Mutex::new(HashMap::new()),
            session_seq: AtomicU64::new(0),
            shutdown: AtomicBool::new(false),
        });
        let loop_state = Arc::clone(&state);
        let join = thread::spawn(move || accept_loop(listener, loop_state));
        Ok(Self {
            addr: bound,
            state,
            join: Some(join),
        })
    }

    #[must_use]
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn shutdown(&mut self) {
        self.state.shutdown.store(true, Ordering::Release);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }

    pub fn wait(mut self) {
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

impl<H: McpHandler> Drop for HttpServer<H> {
    fn drop(&mut self) {
        self.state.shutdown.store(true, Ordering::Release);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

pub fn serve_http<H: McpHandler + 'static>(handler: Arc<H>, addr: SocketAddr) -> io::Result<()> {
    let mut server = HttpServer::spawn(handler, addr)?;
    if let Some(join) = server.join.take() {
        let _ = join.join();
    }
    Ok(())
}

fn accept_loop<H: McpHandler + 'static>(listener: TcpListener, state: Arc<ServerState<H>>) {
    while !state.shutdown.load(Ordering::Acquire) {
        match listener.accept() {
            Ok((stream, _)) => {
                let conn_state = Arc::clone(&state);
                thread::spawn(move || handle_connection(stream, conn_state));
            }
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(20));
            }
            Err(_) => break,
        }
    }
}

struct ParsedRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

fn handle_connection<H: McpHandler + 'static>(mut stream: TcpStream, state: Arc<ServerState<H>>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let _ = stream.set_nonblocking(false);
    let Ok(cloned) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(cloned);
    let req = match read_http_request(&mut reader) {
        Ok(r) => r,
        Err((status, msg)) => {
            let _ = write_simple_response(&mut stream, status, "application/json", msg.as_bytes());
            return;
        }
    };
    if let Some((status, msg)) = validate_common_headers(&req) {
        let _ = write_simple_response(&mut stream, status, "application/json", msg.as_bytes());
        return;
    }
    match req.method.as_str() {
        "POST" => handle_post(&req, &mut stream, &state),
        "GET" => handle_get(&req, &mut stream, &state),
        "DELETE" => handle_delete(&req, &mut stream, &state),
        _ => {
            let _ = write_simple_response(
                &mut stream,
                "405 Method Not Allowed",
                "text/plain; charset=utf-8",
                b"Method not allowed",
            );
        }
    }
}

fn read_http_request<R: BufRead>(reader: &mut R) -> Result<ParsedRequest, (&'static str, String)> {
    let mut request_line = String::new();
    let n = reader.read_line(&mut request_line).map_err(|_| {
        (
            "400 Bad Request",
            r#"{"error":"bad request line"}"#.to_owned(),
        )
    })?;
    if n == 0 || request_line.len() > 8192 {
        return Err((
            "400 Bad Request",
            r#"{"error":"empty or oversized request line"}"#.to_owned(),
        ));
    }
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err((
            "400 Bad Request",
            r#"{"error":"malformed HTTP request line"}"#.to_owned(),
        ));
    }
    let path = parts[1].split('?').next().unwrap_or(parts[1]).to_owned();
    let headers = read_headers(reader)?;
    let body = read_body(reader, &headers)?;
    Ok(ParsedRequest {
        method: parts[0].to_ascii_uppercase(),
        path,
        headers,
        body,
    })
}

fn read_headers<R: BufRead>(
    reader: &mut R,
) -> Result<HashMap<String, String>, (&'static str, String)> {
    let mut headers = HashMap::new();
    let mut bytes_read = 0usize;
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).map_err(|_| {
            (
                "400 Bad Request",
                r#"{"error":"header read failed"}"#.to_owned(),
            )
        })?;
        bytes_read += n;
        if n == 0 || bytes_read > MAX_HEADER_BYTES {
            return Err((
                "400 Bad Request",
                r#"{"error":"invalid headers"}"#.to_owned(),
            ));
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            return Ok(headers);
        }
        let Some((name, value)) = trimmed.split_once(':') else {
            return Err((
                "400 Bad Request",
                r#"{"error":"malformed header"}"#.to_owned(),
            ));
        };
        headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_owned());
    }
}

fn read_body<R: BufRead>(
    reader: &mut R,
    headers: &HashMap<String, String>,
) -> Result<Vec<u8>, (&'static str, String)> {
    if headers
        .get("transfer-encoding")
        .is_some_and(|v| v.to_ascii_lowercase().contains("chunked"))
    {
        return read_chunked_body(reader);
    }
    let len = headers
        .get("content-length")
        .map_or(Ok(0usize), |v| v.parse::<usize>())
        .map_err(|_| {
            (
                "400 Bad Request",
                r#"{"error":"invalid Content-Length"}"#.to_owned(),
            )
        })?;
    if len > MAX_BODY_BYTES {
        return Err((
            "413 Payload Too Large",
            r#"{"error":"body exceeds 8 MiB"}"#.to_owned(),
        ));
    }
    let mut body = vec![0u8; len];
    if len > 0 {
        reader.read_exact(&mut body).map_err(|_| {
            (
                "400 Bad Request",
                r#"{"error":"truncated request body"}"#.to_owned(),
            )
        })?;
    }
    Ok(body)
}

fn read_chunked_body<R: BufRead>(reader: &mut R) -> Result<Vec<u8>, (&'static str, String)> {
    let mut out = Vec::new();
    loop {
        let mut size_line = String::new();
        if reader.read_line(&mut size_line).is_err() {
            return Err((
                "400 Bad Request",
                r#"{"error":"invalid chunked body"}"#.to_owned(),
            ));
        }
        let hex = size_line.trim().split(';').next().unwrap_or("");
        let size = usize::from_str_radix(hex, 16).map_err(|_| {
            (
                "400 Bad Request",
                r#"{"error":"invalid chunk size"}"#.to_owned(),
            )
        })?;
        if size == 0 {
            let mut tail = String::new();
            let _ = reader.read_line(&mut tail);
            return Ok(out);
        }
        if out.len().saturating_add(size) > MAX_BODY_BYTES {
            return Err((
                "413 Payload Too Large",
                r#"{"error":"body exceeds 8 MiB"}"#.to_owned(),
            ));
        }
        let prev = out.len();
        out.resize(prev + size, 0);
        if reader.read_exact(&mut out[prev..]).is_err() {
            return Err((
                "400 Bad Request",
                r#"{"error":"truncated chunk"}"#.to_owned(),
            ));
        }
        let mut crlf = [0u8; 2];
        let _ = reader.read_exact(&mut crlf);
    }
}

fn validate_common_headers(req: &ParsedRequest) -> Option<(&'static str, String)> {
    if !matches!(req.path.as_str(), "/mcp" | "/mcp/" | "/") {
        return Some((
            "404 Not Found",
            r#"{"error":"endpoint not found"}"#.to_owned(),
        ));
    }
    if let Some(origin) = req.headers.get("origin")
        && !is_loopback_origin(origin)
    {
        return Some((
            "403 Forbidden",
            r#"{"error":"non-loopback Origin rejected"}"#.to_owned(),
        ));
    }
    if let Some(version) = req.headers.get("mcp-protocol-version")
        && !SUPPORTED_PROTOCOL_VERSIONS.contains(&version.as_str())
    {
        return Some((
            "400 Bad Request",
            format!(r#"{{"error":"unsupported MCP-Protocol-Version: {version}"}}"#),
        ));
    }
    None
}

fn is_loopback_origin(origin: &str) -> bool {
    let rest = origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"));
    let Some(host_port) = rest else {
        return false;
    };
    if host_port.contains(['/', '?', '#', '@']) {
        return false;
    }
    let host = host_port.split(':').next().unwrap_or("");
    matches!(host, "127.0.0.1" | "localhost")
}

fn accept_flags(req: &ParsedRequest) -> (bool, bool, bool) {
    let Some(accept) = req.headers.get("accept") else {
        return (true, false, false);
    };
    let lower = accept.to_ascii_lowercase();
    let wildcard = lower.contains("*/*");
    let json = wildcard || lower.contains("application/json");
    let sse = wildcard || lower.contains("text/event-stream");
    let prefers_sse = lower.contains("text/event-stream")
        && (!lower.contains("application/json")
            || lower.find("text/event-stream") < lower.find("application/json"));
    (json, sse, prefers_sse)
}

fn require_session<H: McpHandler>(
    req: &ParsedRequest,
    state: &ServerState<H>,
) -> Result<Arc<HttpSession<H>>, (&'static str, String)> {
    let Some(id) = req.headers.get("mcp-session-id") else {
        return Err((
            "400 Bad Request",
            r#"{"error":"missing Mcp-Session-Id header"}"#.to_owned(),
        ));
    };
    state.get_session(id).ok_or_else(|| {
        (
            "404 Not Found",
            r#"{"error":"unknown or terminated MCP session"}"#.to_owned(),
        )
    })
}

fn parse_post_json(req: &ParsedRequest, stream: &mut TcpStream) -> Option<Value> {
    let msg: Value = match serde_json::from_slice(&req.body) {
        Ok(v) => v,
        Err(_) => {
            let err = jsonrpc_error(Value::Null, PARSE_ERROR, "Parse error: invalid JSON");
            let _ = write_json_response(stream, "400 Bad Request", None, &err);
            return None;
        }
    };
    if !msg.is_object() {
        let err = jsonrpc_error(Value::Null, INVALID_REQUEST, "Invalid Request");
        let _ = write_json_response(stream, "400 Bad Request", None, &err);
        return None;
    }
    Some(msg)
}

fn handle_post<H: McpHandler + 'static>(
    req: &ParsedRequest,
    stream: &mut TcpStream,
    state: &Arc<ServerState<H>>,
) {
    let (accepts_json, accepts_sse, prefers_sse) = accept_flags(req);
    if !accepts_json && !accepts_sse {
        let _ = write_simple_response(
            stream,
            "406 Not Acceptable",
            "application/json",
            br#"{"error":"Accept must allow application/json or text/event-stream"}"#,
        );
        return;
    }
    let Some(msg) = parse_post_json(req, stream) else {
        return;
    };
    let obj = msg.as_object().expect("validated JSON object");
    if !obj.contains_key("method") && obj.contains_key("id") {
        match require_session(req, state) {
            Ok(session) => {
                session.deliver_client_response(msg);
                let _ = write_simple_response(stream, "202 Accepted", "application/json", b"");
            }
            Err((status, body)) => {
                let _ = write_simple_response(stream, status, "application/json", body.as_bytes());
            }
        }
        return;
    }
    if obj.get("method").and_then(Value::as_str) == Some("initialize") && obj.contains_key("id") {
        let session = state.create_session();
        let sink: Arc<dyn OutboundSink> =
            Arc::new(PostSink::new(Arc::clone(&session), false, None));
        match session.router.dispatch(&msg, &sink) {
            DispatchOutcome::Reply(reply) => {
                let _ = write_json_response(stream, "200 OK", Some(&session.id), &reply);
            }
            DispatchOutcome::Cancelled | DispatchOutcome::None => {
                let _ = write_simple_response(stream, "202 Accepted", "application/json", b"");
            }
        }
        return;
    }
    let session = match require_session(req, state) {
        Ok(s) => s,
        Err((status, body)) => {
            let _ = write_simple_response(stream, status, "application/json", body.as_bytes());
            return;
        }
    };
    if !obj.contains_key("id") {
        let sink: Arc<dyn OutboundSink> =
            Arc::new(PostSink::new(Arc::clone(&session), false, None));
        let _ = session.router.dispatch(&msg, &sink);
        let _ = write_simple_response(stream, "202 Accepted", "application/json", b"");
        return;
    }
    dispatch_post_request(
        stream,
        session,
        msg,
        accepts_sse && prefers_sse,
        accepts_sse,
    );
}

fn dispatch_post_request<H: McpHandler + 'static>(
    stream: &mut TcpStream,
    session: Arc<HttpSession<H>>,
    msg: Value,
    force_sse: bool,
    accepts_sse: bool,
) {
    let has_progress = msg
        .get("params")
        .and_then(|p| p.get("_meta"))
        .and_then(|m| m.get("progressToken"))
        .is_some();
    let immediate_sse = accepts_sse && (force_sse || has_progress);
    let (tx, rx) = mpsc::channel();
    let sink: Arc<dyn OutboundSink> = Arc::new(PostSink::new(
        Arc::clone(&session),
        accepts_sse,
        accepts_sse.then_some(tx.clone()),
    ));
    let run_session = Arc::clone(&session);
    let worker = thread::spawn(move || {
        let reply = match run_session.router.dispatch(&msg, &sink) {
            DispatchOutcome::Reply(r) => Some(r),
            DispatchOutcome::Cancelled | DispatchOutcome::None => None,
        };
        let _ = tx.send(PostStreamMsg::Finish(reply));
    });

    let mut sse_started = false;
    if immediate_sse {
        if write_sse_headers(stream, Some(&session.id)).is_err() {
            let _ = worker.join();
            return;
        }
        sse_started = true;
    }
    while let Ok(item) = rx.recv() {
        match item {
            PostStreamMsg::Event(ev) => {
                if !sse_started {
                    if write_sse_headers(stream, Some(&session.id)).is_err() {
                        break;
                    }
                    sse_started = true;
                }
                if write_sse_event(stream, &ev).is_err() {
                    break;
                }
            }
            PostStreamMsg::Finish(Some(reply)) => {
                if sse_started {
                    let ev = session.record_and_broadcast(&reply);
                    let _ = write_sse_event(stream, &ev);
                } else {
                    let _ = write_json_response(stream, "200 OK", Some(&session.id), &reply);
                }
                break;
            }
            PostStreamMsg::Finish(None) => {
                if !sse_started {
                    let _ = write_simple_response(stream, "202 Accepted", "application/json", b"");
                }
                break;
            }
        }
    }
    let _ = worker.join();
}

fn handle_get<H: McpHandler>(
    req: &ParsedRequest,
    stream: &mut TcpStream,
    state: &Arc<ServerState<H>>,
) {
    let (_, accepts_sse, _) = accept_flags(req);
    if !accepts_sse {
        let _ = write_simple_response(
            stream,
            "406 Not Acceptable",
            "application/json",
            br#"{"error":"GET requires Accept: text/event-stream"}"#,
        );
        return;
    }
    let session = match require_session(req, state) {
        Ok(s) => s,
        Err((status, body)) => {
            let _ = write_simple_response(stream, status, "application/json", body.as_bytes());
            return;
        }
    };
    let last_event_id = req
        .headers
        .get("last-event-id")
        .and_then(|v| v.parse::<u64>().ok());
    let (replay, rx, _guard) = session.register_subscriber(last_event_id);
    if write_sse_headers(stream, Some(&session.id)).is_err() {
        return;
    }
    for ev in replay {
        if write_sse_event(stream, &ev).is_err() {
            return;
        }
    }
    let _ = stream.write_all(b": connected\n\n");
    let _ = stream.flush();
    while !state.shutdown.load(Ordering::Acquire) && !session.terminated.load(Ordering::Acquire) {
        match rx.recv_timeout(Duration::from_secs(2)) {
            Ok(ev) => {
                if write_sse_event(stream, &ev).is_err() {
                    break;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if stream.write_all(b": keepalive\n\n").is_err() || stream.flush().is_err() {
                    break;
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn handle_delete<H: McpHandler>(
    req: &ParsedRequest,
    stream: &mut TcpStream,
    state: &Arc<ServerState<H>>,
) {
    let Some(id) = req.headers.get("mcp-session-id") else {
        let _ = write_simple_response(
            stream,
            "400 Bad Request",
            "application/json",
            br#"{"error":"missing Mcp-Session-Id header"}"#,
        );
        return;
    };
    if state.delete_session(id) {
        let _ = write_simple_response(stream, "200 OK", "application/json", br#"{"closed":true}"#);
    } else {
        let _ = write_simple_response(
            stream,
            "404 Not Found",
            "application/json",
            br#"{"error":"unknown session"}"#,
        );
    }
}

fn write_simple_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    stream.flush()
}

fn write_json_response(
    stream: &mut TcpStream,
    status: &str,
    session_id: Option<&str>,
    body: &Value,
) -> io::Result<()> {
    let encoded = serde_json::to_vec(body).unwrap_or_else(|_| b"{}".to_vec());
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n",
        encoded.len()
    )?;
    if let Some(id) = session_id {
        write!(stream, "Mcp-Session-Id: {id}\r\n")?;
    }
    stream.write_all(b"\r\n")?;
    stream.write_all(&encoded)?;
    stream.flush()
}

fn write_sse_headers(stream: &mut TcpStream, session_id: Option<&str>) -> io::Result<()> {
    stream.write_all(
        b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\n",
    )?;
    if let Some(id) = session_id {
        write!(stream, "Mcp-Session-Id: {id}\r\n")?;
    }
    stream.write_all(b"\r\n")?;
    stream.flush()
}

fn write_sse_event(stream: &mut TcpStream, event: &SseEvent) -> io::Result<()> {
    write!(
        stream,
        "id: {}\nevent: message\ndata: {}\n\n",
        event.id, event.data
    )?;
    stream.flush()
}
