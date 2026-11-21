//! Bounded bidirectional newline-delimited JSON-RPC over stdio streams.

use crate::context::{NotificationScope, OutboundSink};
use crate::router::{DispatchOutcome, McpHandler, Router};
use crate::types::{INVALID_REQUEST, PARSE_ERROR, jsonrpc_error};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::sync::{Arc, Mutex, mpsc};

const MAX_LINE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug)]
struct StdioSink<W> {
    writer: Arc<Mutex<W>>,
    pending_client: Arc<Mutex<HashMap<String, mpsc::Sender<Value>>>>,
}

impl<W: Write + Send> StdioSink<W> {
    fn write_json(&self, value: &Value) {
        let Ok(encoded) = serde_json::to_string(value) else {
            return;
        };
        let Ok(mut out) = self.writer.lock() else {
            return;
        };
        let _ = writeln!(out, "{encoded}");
        let _ = out.flush();
    }
}

impl<W: Write + Send + 'static> OutboundSink for StdioSink<W> {
    fn emit_notification(&self, _scope: NotificationScope, method: &str, params: Value) {
        self.write_json(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        }));
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
                .pending_client
                .lock()
                .map_err(|_| "client response registry poisoned".to_owned())?;
            guard.insert(key, tx);
        }
        self.write_json(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }));
        Ok(rx)
    }
}

pub fn serve_stdio<H: McpHandler + 'static>(handler: Arc<H>) -> io::Result<()> {
    serve_streams(handler, io::stdin().lock(), io::stdout())
}

pub fn serve_streams<H, R, W>(handler: Arc<H>, reader: R, writer: W) -> io::Result<()>
where
    H: McpHandler + 'static,
    R: BufRead,
    W: Write + Send + 'static,
{
    let router = Arc::new(Router::new(handler));
    let pending_client = Arc::new(Mutex::new(HashMap::<String, mpsc::Sender<Value>>::new()));
    let writer = Arc::new(Mutex::new(writer));
    let sink: Arc<StdioSink<W>> = Arc::new(StdioSink {
        writer: Arc::clone(&writer),
        pending_client: Arc::clone(&pending_client),
    });
    let dyn_sink: Arc<dyn OutboundSink> = sink.clone();

    let (work_tx, work_rx) = mpsc::channel::<Value>();
    let worker_router = Arc::clone(&router);
    let worker_sink = Arc::clone(&dyn_sink);
    let worker_writer = Arc::clone(&sink);
    let worker = std::thread::spawn(move || {
        while let Ok(msg) = work_rx.recv() {
            match worker_router.dispatch(&msg, &worker_sink) {
                DispatchOutcome::Reply(reply) => worker_writer.write_json(&reply),
                DispatchOutcome::Cancelled | DispatchOutcome::None => {}
            }
        }
    });

    read_loop(reader, &router, &sink, &pending_client, &work_tx)?;
    drop(work_tx);
    let _ = worker.join();
    Ok(())
}

fn read_loop<R: BufRead, W: Write + Send>(
    mut reader: R,
    router: &Router<impl McpHandler>,
    sink: &StdioSink<W>,
    pending_client: &Mutex<HashMap<String, mpsc::Sender<Value>>>,
    work_tx: &mpsc::Sender<Value>,
) -> io::Result<()> {
    let mut buf = Vec::with_capacity(4096);
    loop {
        buf.clear();
        let read = read_bounded_line(&mut reader, &mut buf)?;
        if read == 0 {
            break;
        }
        if buf.len() > MAX_LINE_BYTES {
            sink.write_json(&jsonrpc_error(
                Value::Null,
                INVALID_REQUEST,
                "JSON-RPC message exceeds 4 MiB line limit",
            ));
            continue;
        }
        let Ok(line) = std::str::from_utf8(&buf) else {
            sink.write_json(&jsonrpc_error(
                Value::Null,
                PARSE_ERROR,
                "Parse error: invalid UTF-8",
            ));
            continue;
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        route_inbound_line(trimmed, router, sink, pending_client, work_tx);
    }
    Ok(())
}

fn read_bounded_line<R: BufRead>(reader: &mut R, out: &mut Vec<u8>) -> io::Result<usize> {
    let mut total = 0usize;
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok(total);
        }
        if let Some(pos) = available.iter().position(|&b| b == b'\n') {
            let take = pos + 1;
            if out.len() <= MAX_LINE_BYTES {
                let space = (MAX_LINE_BYTES + 1).saturating_sub(out.len());
                out.extend_from_slice(&available[..take.min(space)]);
            }
            reader.consume(take);
            total += take;
            return Ok(total);
        }
        let len = available.len();
        if out.len() <= MAX_LINE_BYTES {
            let space = (MAX_LINE_BYTES + 1).saturating_sub(out.len());
            out.extend_from_slice(&available[..len.min(space)]);
        }
        reader.consume(len);
        total += len;
    }
}

fn route_inbound_line<W: Write + Send>(
    trimmed: &str,
    router: &Router<impl McpHandler>,
    sink: &StdioSink<W>,
    pending_client: &Mutex<HashMap<String, mpsc::Sender<Value>>>,
    work_tx: &mpsc::Sender<Value>,
) {
    let Ok(msg) = serde_json::from_str::<Value>(trimmed) else {
        sink.write_json(&jsonrpc_error(
            Value::Null,
            PARSE_ERROR,
            "Parse error: invalid JSON",
        ));
        return;
    };
    let Some(obj) = msg.as_object() else {
        sink.write_json(&jsonrpc_error(
            Value::Null,
            INVALID_REQUEST,
            "Invalid Request: expected JSON object",
        ));
        return;
    };
    if let Some(method) = obj.get("method").and_then(Value::as_str) {
        if method == "notifications/cancelled" && !obj.contains_key("id") {
            if let Some(id) = obj.get("params").and_then(|p| p.get("requestId")) {
                router.cancel_request_id(id);
            }
            return;
        }
        let _ = work_tx.send(msg);
        return;
    }
    if let Some(id) = obj.get("id")
        && (obj.contains_key("result") || obj.contains_key("error"))
    {
        if let Ok(mut guard) = pending_client.lock()
            && let Some(sender) = guard.remove(&id.to_string())
        {
            let _ = sender.send(msg);
        }
        return;
    }
    let _ = work_tx.send(msg);
}
