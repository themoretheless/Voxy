//! Typed MCP `2025-06-18` protocol structures and JSON-RPC 2.0 constants.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const LATEST_PROTOCOL_VERSION: &str = "2025-06-18";
pub const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];
pub const DEFAULT_PAGE_SIZE: usize = 50;

pub const PARSE_ERROR: i64 = -32700;
pub const INVALID_REQUEST: i64 = -32600;
pub const METHOD_NOT_FOUND: i64 = -32601;
pub const INVALID_PARAMS: i64 = -32602;
pub const INTERNAL_ERROR: i64 = -32603;
pub const NOT_INITIALIZED: i64 = -32000;
pub const RESOURCE_NOT_FOUND: i64 = -32002;

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub name: String,
    pub title: Option<String>,
    pub version: String,
    pub instructions: Option<String>,
    pub page_size: usize,
}

impl ServerConfig {
    #[must_use]
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            title: None,
            version: version.into(),
            instructions: None,
            page_size: DEFAULT_PAGE_SIZE,
        }
    }

    #[must_use]
    pub fn with_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    #[must_use]
    pub fn with_page_size(mut self, page_size: usize) -> Self {
        self.page_size = page_size.max(1);
        self
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Implementation {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default)]
    pub version: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RootsCapability {
    #[serde(default, rename = "listChanged")]
    pub list_changed: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ClientCapabilities {
    #[serde(default)]
    pub roots: Option<RootsCapability>,
    #[serde(default)]
    pub sampling: Option<Value>,
    #[serde(default)]
    pub elicitation: Option<Value>,
}

impl ClientCapabilities {
    #[must_use]
    pub fn supports_roots(&self) -> bool {
        self.roots.is_some()
    }

    #[must_use]
    pub fn supports_sampling(&self) -> bool {
        self.sampling.is_some()
    }

    #[must_use]
    pub fn supports_elicitation(&self) -> bool {
        self.elicitation.is_some()
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct InitializeParams {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    #[serde(default)]
    pub capabilities: ClientCapabilities,
    #[serde(default, rename = "clientInfo")]
    pub client_info: Option<Implementation>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolAnnotations {
    #[serde(
        rename = "readOnlyHint",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub read_only_hint: Option<bool>,
    #[serde(
        rename = "destructiveHint",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub destructive_hint: Option<bool>,
    #[serde(
        rename = "idempotentHint",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub idempotent_hint: Option<bool>,
    #[serde(
        rename = "openWorldHint",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub open_world_hint: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
    #[serde(rename = "outputSchema", skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ToolAnnotations>,
}

impl ToolDef {
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: Value,
    ) -> Self {
        Self {
            name: name.into(),
            title: None,
            description: description.into(),
            input_schema,
            output_schema: None,
            annotations: None,
        }
    }

    #[must_use]
    pub fn with_output_schema(mut self, schema: Value) -> Self {
        self.output_schema = Some(schema);
        self
    }

    #[must_use]
    pub fn with_annotations(mut self, annotations: ToolAnnotations) -> Self {
        self.annotations = Some(annotations);
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
    #[serde(rename = "audio")]
    Audio {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
    #[serde(rename = "resource_link")]
    ResourceLink {
        uri: String,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
    },
    #[serde(rename = "resource")]
    Resource { resource: ResourceContents },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CallToolResult {
    pub content: Vec<ContentBlock>,
    #[serde(rename = "structuredContent", skip_serializing_if = "Option::is_none")]
    pub structured_content: Option<Value>,
    #[serde(rename = "isError")]
    pub is_error: bool,
}

impl CallToolResult {
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: vec![ContentBlock::Text { text: text.into() }],
            structured_content: None,
            is_error: false,
        }
    }

    #[must_use]
    pub fn structured(value: Value) -> Self {
        let text = serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string());
        Self {
            content: vec![ContentBlock::Text { text }],
            structured_content: Some(value),
            is_error: false,
        }
    }

    #[must_use]
    pub fn error(message: impl std::fmt::Display) -> Self {
        Self {
            content: vec![ContentBlock::Text {
                text: message.to_string(),
            }],
            structured_content: None,
            is_error: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceDef {
    pub uri: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceTemplateDef {
    #[serde(rename = "uriTemplate")]
    pub uri_template: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceContents {
    pub uri: String,
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
}

impl ResourceContents {
    #[must_use]
    pub fn text(
        uri: impl Into<String>,
        mime_type: impl Into<String>,
        text: impl Into<String>,
    ) -> Self {
        Self {
            uri: uri.into(),
            mime_type: Some(mime_type.into()),
            text: Some(text.into()),
            blob: None,
        }
    }

    #[must_use]
    pub fn json(uri: impl Into<String>, value: &Value) -> Self {
        let text = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
        Self::text(uri, "application/json", text)
    }

    #[must_use]
    pub fn blob(
        uri: impl Into<String>,
        mime_type: impl Into<String>,
        base64_blob: impl Into<String>,
    ) -> Self {
        Self {
            uri: uri.into(),
            mime_type: Some(mime_type.into()),
            text: None,
            blob: Some(base64_blob.into()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReadResourceResult {
    pub contents: Vec<ResourceContents>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PromptArgument {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PromptDef {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub arguments: Vec<PromptArgument>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PromptMessage {
    pub role: String,
    pub content: ContentBlock,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetPromptResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub messages: Vec<PromptMessage>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CompletionRef {
    #[serde(rename = "ref/prompt")]
    Prompt { name: String },
    #[serde(rename = "ref/resource")]
    Resource { uri: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompletionArgument {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CompleteParams {
    #[serde(rename = "ref")]
    pub target: CompletionRef,
    pub argument: CompletionArgument,
    #[serde(default)]
    pub context: Option<Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CompletionValues {
    pub values: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<usize>,
    #[serde(rename = "hasMore")]
    pub has_more: bool,
}

impl CompletionValues {
    #[must_use]
    pub fn from_candidates(candidates: impl IntoIterator<Item = String>, prefix: &str) -> Self {
        let mut filtered: Vec<String> = candidates
            .into_iter()
            .filter(|candidate| prefix.is_empty() || candidate.contains(prefix))
            .collect();
        filtered.sort();
        filtered.dedup();
        let total = filtered.len();
        let has_more = total > 100;
        filtered.truncate(100);
        Self {
            values: filtered,
            total: Some(total),
            has_more,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompleteResult {
    pub completion: CompletionValues,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug = 0,
    Info = 1,
    Notice = 2,
    Warning = 3,
    Error = 4,
    Critical = 5,
    Alert = 6,
    Emergency = 7,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Root {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[must_use]
pub fn server_capabilities() -> Value {
    json!({
        "tools": { "listChanged": true },
        "resources": { "subscribe": true, "listChanged": true },
        "prompts": { "listChanged": true },
        "completions": {},
        "logging": {}
    })
}

#[must_use]
pub fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(TABLE[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[must_use]
pub fn jsonrpc_result(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}

#[must_use]
pub fn jsonrpc_error(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message.into(),
        }
    })
}
