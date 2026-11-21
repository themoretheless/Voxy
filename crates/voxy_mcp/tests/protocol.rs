//! Integration tests covering MCP `2025-06-18` protocol features over `stdio` and Streamable HTTP.

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};
use voxy_mcp::{
    CallToolResult, CompleteParams, CompletionValues, ContentBlock, GetPromptResult, HttpServer,
    LogLevel, McpHandler, PromptArgument, PromptDef, PromptMessage, RequestContext,
    ResourceContents, ResourceDef, ResourceTemplateDef, RpcError, ServerConfig, ToolAnnotations,
    ToolDef, TransportMode, parse_transport_args, serve_streams,
};

#[derive(Debug, Default)]
struct DemoHandler {
    value: Mutex<String>,
    ticks_completed: AtomicUsize,
}

impl McpHandler for DemoHandler {
    fn config(&self) -> ServerConfig {
        ServerConfig::new("voxy-mcp-test", "0.1.0")
            .with_title("Voxy Test Server")
            .with_instructions("Test handler")
            .with_page_size(2)
    }

    fn list_tools(&self) -> Vec<ToolDef> {
        vec![
            ToolDef::new("set_value", "Update demo value", json!({"type": "object"}))
                .with_output_schema(json!({
                    "type": "object",
                    "properties": { "value": { "type": "string" } },
                    "required": ["value"]
                }))
                .with_annotations(ToolAnnotations {
                    read_only_hint: Some(false),
                    destructive_hint: Some(false),
                    idempotent_hint: Some(true),
                    open_world_hint: Some(false),
                }),
            ToolDef::new(
                "slow_work",
                "Cancellable progress task",
                json!({"type": "object"}),
            ),
            ToolDef::new(
                "ask_roots",
                "Request roots/list from client",
                json!({"type": "object"}),
            ),
            ToolDef::new(
                "ask_sampling",
                "Request sampling/createMessage",
                json!({"type": "object"}),
            ),
        ]
    }

    fn call_tool(
        &self,
        ctx: &RequestContext,
        name: &str,
        arguments: Value,
    ) -> Result<CallToolResult, String> {
        match name {
            "set_value" => {
                let next = arguments
                    .get("value")
                    .and_then(Value::as_str)
                    .ok_or("missing string value")?
                    .to_owned();
                self.value.lock().unwrap().clone_from(&next);
                ctx.log(LogLevel::Warning, "demo", json!({"updated": next}));
                ctx.log(
                    LogLevel::Debug,
                    "demo",
                    json!({"ignored_below_warning": true}),
                );
                ctx.notify_resource_updated("voxy://test/state");
                Ok(CallToolResult::structured(json!({ "value": next })))
            }
            "slow_work" => {
                let steps = arguments.get("steps").and_then(Value::as_u64).unwrap_or(5);
                let delay_ms = arguments
                    .get("delayMs")
                    .and_then(Value::as_u64)
                    .unwrap_or(10);
                for i in 1..=steps {
                    if ctx.is_cancelled() {
                        return Err("cancelled".to_owned());
                    }
                    ctx.report_progress(i as f64, Some(steps as f64), Some("working"));
                    self.ticks_completed.fetch_add(1, Ordering::Relaxed);
                    thread::sleep(Duration::from_millis(delay_ms));
                }
                Ok(CallToolResult::structured(
                    json!({ "done": true, "steps": steps }),
                ))
            }
            "ask_roots" => {
                let roots = ctx.list_roots()?;
                Ok(CallToolResult::structured(json!({ "roots": roots })))
            }
            "ask_sampling" => {
                let reply = ctx.create_sampling_message(json!({
                    "messages": [{ "role": "user", "content": { "type": "text", "text": "ping" } }],
                    "maxTokens": 16
                }))?;
                Ok(CallToolResult::structured(json!({ "sampling": reply })))
            }
            _ => Err(format!("unknown tool: {name}")),
        }
    }

    fn list_resources(&self) -> Vec<ResourceDef> {
        vec![
            ResourceDef {
                uri: "voxy://test/state".into(),
                name: "state".into(),
                title: None,
                description: Some("Current demo value".into()),
                mime_type: Some("application/json".into()),
                size: None,
            },
            ResourceDef {
                uri: "voxy://test/a".into(),
                name: "a".into(),
                title: None,
                description: None,
                mime_type: Some("text/plain".into()),
                size: None,
            },
            ResourceDef {
                uri: "voxy://test/b".into(),
                name: "b".into(),
                title: None,
                description: None,
                mime_type: Some("text/plain".into()),
                size: None,
            },
        ]
    }

    fn list_resource_templates(&self) -> Vec<ResourceTemplateDef> {
        vec![ResourceTemplateDef {
            uri_template: "voxy://test/item/{id}".into(),
            name: "item".into(),
            title: None,
            description: Some("Templated test item".into()),
            mime_type: Some("application/json".into()),
        }]
    }

    fn read_resource(
        &self,
        _ctx: &RequestContext,
        uri: &str,
    ) -> Result<Vec<ResourceContents>, RpcError> {
        if uri == "voxy://test/state" {
            let val = self.value.lock().unwrap().clone();
            return Ok(vec![ResourceContents::json(uri, &json!({ "value": val }))]);
        }
        if let Some(item_id) = uri.strip_prefix("voxy://test/item/") {
            return Ok(vec![ResourceContents::json(uri, &json!({ "id": item_id }))]);
        }
        Err(RpcError::resource_not_found(uri))
    }

    fn list_prompts(&self) -> Vec<PromptDef> {
        vec![PromptDef {
            name: "inspect".into(),
            title: Some("Inspect Target".into()),
            description: Some("Inspect an entity".into()),
            arguments: vec![PromptArgument {
                name: "id".into(),
                description: Some("Target ID".into()),
                required: true,
            }],
        }]
    }

    fn get_prompt(
        &self,
        _ctx: &RequestContext,
        name: &str,
        arguments: Value,
    ) -> Result<GetPromptResult, RpcError> {
        if name != "inspect" {
            return Err(RpcError::invalid_params(format!("unknown prompt: {name}")));
        }
        let target = arguments
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("default");
        Ok(GetPromptResult {
            description: Some("Inspect prompt".into()),
            messages: vec![PromptMessage {
                role: "user".into(),
                content: ContentBlock::Text {
                    text: format!("Inspect {target}"),
                },
            }],
        })
    }

    fn complete(
        &self,
        _ctx: &RequestContext,
        params: &CompleteParams,
    ) -> Result<CompletionValues, RpcError> {
        let _ = &params.target;
        let candidates = ["alpha", "beta", "gamma", "delta"].map(str::to_owned);
        Ok(CompletionValues::from_candidates(
            candidates,
            &params.argument.value,
        ))
    }
}

fn send_http(
    addr: SocketAddr,
    method: &str,
    headers: &[(&str, &str)],
    body: Option<&str>,
) -> (u16, Vec<(String, String)>, String) {
    let mut stream = TcpStream::connect(addr).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let payload = body.unwrap_or("");
    write!(stream, "{method} /mcp HTTP/1.1\r\nHost: {addr}\r\n").unwrap();
    for (k, v) in headers {
        write!(stream, "{k}: {v}\r\n").unwrap();
    }
    if !payload.is_empty() {
        write!(stream, "Content-Type: application/json\r\n").unwrap();
    }
    write!(
        stream,
        "Content-Length: {}\r\nConnection: close\r\n\r\n{}",
        payload.len(),
        payload
    )
    .unwrap();
    stream.flush().unwrap();

    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    let (head, body_out) = raw.split_once("\r\n\r\n").unwrap();
    let mut lines = head.lines();
    let status: u16 = lines
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    let parsed_headers = lines
        .filter_map(|line| {
            line.split_once(':')
                .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_owned()))
        })
        .collect();
    (status, parsed_headers, body_out.to_owned())
}

fn assert_stdio_core_capabilities(tx: &mut impl Write, read_msg: &mut impl FnMut() -> Value) {
    writeln!(tx, r#"{{"jsonrpc":"2.0","id":1,"method":"tools/list"}}"#).unwrap();
    tx.flush().unwrap();
    assert_eq!(read_msg()["error"]["code"], -32000);

    writeln!(
        tx,
        "{}",
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": { "roots": { "listChanged": true } },
                "clientInfo": { "name": "test-client", "version": "1.0" }
            }
        })
    )
    .unwrap();
    writeln!(
        tx,
        r#"{{"jsonrpc":"2.0","method":"notifications/initialized"}}"#
    )
    .unwrap();
    tx.flush().unwrap();
    assert_eq!(read_msg()["result"]["protocolVersion"], "2025-06-18");

    writeln!(tx, r#"{{"jsonrpc":"2.0","id":3,"method":"tools/list"}}"#).unwrap();
    tx.flush().unwrap();
    let page1 = read_msg();
    assert_eq!(page1["result"]["tools"].as_array().unwrap().len(), 2);
    let cursor = page1["result"]["nextCursor"].as_str().unwrap().to_owned();
    writeln!(
        tx,
        "{}",
        json!({"jsonrpc":"2.0","id":4,"method":"tools/list","params":{"cursor":cursor}})
    )
    .unwrap();
    tx.flush().unwrap();
    let page2 = read_msg();
    assert_eq!(page2["result"]["tools"].as_array().unwrap().len(), 2);
    assert!(page2["result"].get("nextCursor").is_none());
}

fn assert_stdio_notifications_and_templates(
    tx: &mut impl Write,
    read_msg: &mut impl FnMut() -> Value,
) {
    writeln!(
        tx,
        r#"{{"jsonrpc":"2.0","id":5,"method":"logging/setLevel","params":{{"level":"warning"}}}}"#
    )
    .unwrap();
    tx.flush().unwrap();
    assert!(read_msg().get("result").is_some());

    writeln!(
        tx,
        r#"{{"jsonrpc":"2.0","id":6,"method":"resources/subscribe","params":{{"uri":"voxy://test/state"}}}}"#
    )
    .unwrap();
    tx.flush().unwrap();
    assert!(read_msg().get("result").is_some());

    writeln!(
        tx,
        r#"{{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{{"name":"set_value","arguments":{{"value":"hello"}}}}}}"#
    )
    .unwrap();
    tx.flush().unwrap();
    let msgs = [read_msg(), read_msg(), read_msg()];
    let methods: Vec<&str> = msgs
        .iter()
        .filter_map(|m| m.get("method").and_then(Value::as_str))
        .collect();
    assert_eq!(
        methods,
        vec!["notifications/message", "notifications/resources/updated"]
    );
    let reply = msgs.into_iter().find(|m| m["id"] == 7).unwrap();
    assert_eq!(reply["result"]["structuredContent"]["value"], "hello");

    writeln!(
        tx,
        r#"{{"jsonrpc":"2.0","id":8,"method":"resources/read","params":{{"uri":"voxy://test/item/42"}}}}"#
    )
    .unwrap();
    tx.flush().unwrap();
    assert!(
        read_msg()["result"]["contents"][0]["text"]
            .as_str()
            .unwrap()
            .contains("42")
    );

    writeln!(
        tx,
        r#"{{"jsonrpc":"2.0","id":9,"method":"completion/complete","params":{{"ref":{{"type":"ref/prompt","name":"inspect"}},"argument":{{"name":"id","value":"al"}}}}}}"#
    )
    .unwrap();
    tx.flush().unwrap();
    assert_eq!(
        read_msg()["result"]["completion"]["values"],
        json!(["alpha"])
    );
}

fn assert_stdio_roots_and_cancel(
    tx: &mut impl Write,
    read_msg: &mut impl FnMut() -> Value,
    handler: &DemoHandler,
) {
    writeln!(
        tx,
        r#"{{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{{"name":"ask_roots","arguments":{{}}}}}}"#
    )
    .unwrap();
    tx.flush().unwrap();
    let req_from_srv = read_msg();
    assert_eq!(req_from_srv["method"], "roots/list");
    writeln!(
        tx,
        "{}",
        json!({
            "jsonrpc": "2.0",
            "id": req_from_srv["id"],
            "result": { "roots": [{ "uri": "file:///workspace/demo", "name": "demo" }] }
        })
    )
    .unwrap();
    tx.flush().unwrap();
    assert_eq!(
        read_msg()["result"]["structuredContent"]["roots"][0]["name"],
        "demo"
    );

    writeln!(
        tx,
        r#"{{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{{"_meta":{{"progressToken":"pt1"}},"name":"slow_work","arguments":{{"steps":40,"delayMs":15}}}}}}"#
    )
    .unwrap();
    tx.flush().unwrap();
    assert_eq!(read_msg()["method"], "notifications/progress");
    writeln!(
        tx,
        r#"{{"jsonrpc":"2.0","method":"notifications/cancelled","params":{{"requestId":11}}}}"#
    )
    .unwrap();
    writeln!(tx, r#"{{"jsonrpc":"2.0","id":12,"method":"ping"}}"#).unwrap();
    tx.flush().unwrap();

    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        assert!(Instant::now() < deadline);
        let msg = read_msg();
        assert_ne!(msg.get("id").and_then(Value::as_i64), Some(11));
        if msg.get("id").and_then(Value::as_i64) == Some(12) {
            break;
        }
    }
    assert!(handler.ticks_completed.load(Ordering::Relaxed) < 40);
}

#[test]
fn stdio_full_mcp_roundtrip_and_cancellation_and_roots() {
    let (client_to_srv_rx, client_to_srv_tx) = std::io::pipe().unwrap();
    let (srv_to_client_rx, srv_to_client_tx) = std::io::pipe().unwrap();

    let handler = Arc::new(DemoHandler::default());
    let srv_handler = Arc::clone(&handler);
    let server_thread = thread::spawn(move || {
        serve_streams(
            srv_handler,
            BufReader::new(client_to_srv_rx),
            srv_to_client_tx,
        )
        .unwrap();
    });

    let mut tx = client_to_srv_tx;
    let mut rx = BufReader::new(srv_to_client_rx);
    let mut read_msg = || -> Value {
        let mut line = String::new();
        rx.read_line(&mut line).unwrap();
        serde_json::from_str(line.trim()).unwrap()
    };

    assert_stdio_core_capabilities(&mut tx, &mut read_msg);
    assert_stdio_notifications_and_templates(&mut tx, &mut read_msg);
    assert_stdio_roots_and_cancel(&mut tx, &mut read_msg, &handler);

    drop(tx);
    server_thread.join().unwrap();
}

fn assert_http_security_and_initialize(addr: SocketAddr) -> String {
    let (status, _, _) = send_http(
        addr,
        "POST",
        &[
            ("Origin", "https://evil.example"),
            ("Accept", "application/json, text/event-stream"),
        ],
        Some(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#,
        ),
    );
    assert_eq!(status, 403);

    let (status, _, _) = send_http(
        addr,
        "POST",
        &[
            ("Origin", "http://127.0.0.1"),
            ("MCP-Protocol-Version", "1999-01-01"),
            ("Accept", "application/json, text/event-stream"),
        ],
        Some(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#),
    );
    assert_eq!(status, 400);

    let (status, headers, body) = send_http(
        addr,
        "POST",
        &[
            ("Origin", "http://localhost:8080"),
            ("Accept", "application/json, text/event-stream"),
        ],
        Some(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{"sampling":{}}}}"#,
        ),
    );
    assert_eq!(status, 200);
    let init_json: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(init_json["result"]["protocolVersion"], "2025-06-18");
    headers
        .iter()
        .find(|(k, _)| k == "mcp-session-id")
        .unwrap()
        .1
        .clone()
}

fn read_sse_until_resource_updated(addr: SocketAddr, session_id: &str) -> String {
    let mut sse_stream = TcpStream::connect(addr).unwrap();
    sse_stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    write!(
        sse_stream,
        "GET /mcp HTTP/1.1\r\nHost: {addr}\r\nMcp-Session-Id: {session_id}\r\nAccept: text/event-stream\r\nLast-Event-ID: 0\r\n\r\n"
    )
    .unwrap();
    sse_stream.flush().unwrap();
    let mut sse_reader = BufReader::new(sse_stream);
    let mut collected = String::new();
    for _ in 0..30 {
        let mut line = String::new();
        if sse_reader.read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        collected.push_str(&line);
        if collected.contains("notifications/resources/updated") {
            break;
        }
    }
    collected
}

#[test]
fn streamable_http_session_sse_resumability_and_security() {
    let handler = Arc::new(DemoHandler::default());
    let mut server =
        HttpServer::spawn(handler, SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)).unwrap();
    let addr = server.addr();
    let session_id = assert_http_security_and_initialize(addr);

    let (status, post_headers, post_sse) = send_http(
        addr,
        "POST",
        &[
            ("Mcp-Session-Id", &session_id),
            ("MCP-Protocol-Version", "2025-06-18"),
            ("Accept", "application/json, text/event-stream"),
        ],
        Some(
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"_meta":{"progressToken":"hp"},"name":"slow_work","arguments":{"steps":3,"delayMs":1}}}"#,
        ),
    );
    assert_eq!(status, 200);
    assert!(
        post_headers
            .iter()
            .any(|(k, v)| k == "content-type" && v.contains("text/event-stream"))
    );
    assert!(post_sse.contains("notifications/progress"));
    assert!(post_sse.contains(r#""steps":3"#));

    let (status, _, _) = send_http(
        addr,
        "POST",
        &[
            ("Mcp-Session-Id", &session_id),
            ("Accept", "application/json"),
        ],
        Some(
            r#"{"jsonrpc":"2.0","id":3,"method":"resources/subscribe","params":{"uri":"voxy://test/state"}}"#,
        ),
    );
    assert_eq!(status, 200);
    let (status, _, _) = send_http(
        addr,
        "POST",
        &[
            ("Mcp-Session-Id", &session_id),
            ("Accept", "application/json"),
        ],
        Some(
            r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"set_value","arguments":{"value":"sse-replayed"}}}"#,
        ),
    );
    assert_eq!(status, 200);

    let collected = read_sse_until_resource_updated(addr, &session_id);
    assert!(collected.contains("notifications/resources/updated"));

    let (status, _, _) = send_http(addr, "DELETE", &[("Mcp-Session-Id", &session_id)], None);
    assert_eq!(status, 200);
    let (status, _, _) = send_http(
        addr,
        "POST",
        &[
            ("Mcp-Session-Id", &session_id),
            ("Accept", "application/json"),
        ],
        Some(r#"{"jsonrpc":"2.0","id":5,"method":"ping"}"#),
    );
    assert_eq!(status, 404);

    server.shutdown();
}

#[test]
fn cli_transport_arg_parsing_rejects_non_loopback() {
    let (mode, rest) =
        parse_transport_args(["proj.voxy", "--http-port", "8765"].map(str::to_owned)).unwrap();
    assert_eq!(
        mode,
        TransportMode::Http(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8765))
    );
    assert_eq!(rest, vec!["proj.voxy"]);
    assert!(parse_transport_args(["--http", "0.0.0.0:9000"].map(str::to_owned)).is_err());
}
