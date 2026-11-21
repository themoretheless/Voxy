//! Deterministic cursor pagination helpers for MCP `*/list` endpoints.

use serde::{Deserialize, Serialize};
use serde_json::Value;

const CURSOR_PREFIX: &str = "voxy-cursor:";

#[derive(Clone, Debug, Default, Deserialize)]
pub struct CursorParams {
    #[serde(default)]
    pub cursor: Option<String>,
    #[serde(default, rename = "_meta")]
    pub meta: Option<Value>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PaginatedItems<T> {
    pub items: Vec<T>,
    #[serde(rename = "nextCursor", skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

pub fn decode_cursor(cursor: Option<&str>, total: usize) -> Result<usize, String> {
    let Some(raw) = cursor else {
        return Ok(0);
    };
    let rest = raw
        .strip_prefix(CURSOR_PREFIX)
        .ok_or_else(|| "invalid pagination cursor".to_owned())?;
    let offset: usize = rest
        .parse()
        .map_err(|_| "malformed pagination cursor offset".to_owned())?;
    if offset > total || (total > 0 && offset == total) {
        return Err("pagination cursor out of bounds".to_owned());
    }
    Ok(offset)
}

#[must_use]
pub fn encode_cursor(offset: usize) -> String {
    format!("{CURSOR_PREFIX}{offset}")
}

pub fn paginate<T: Clone>(
    items: &[T],
    cursor: Option<&str>,
    page_size: usize,
) -> Result<PaginatedItems<T>, String> {
    let step = page_size.max(1);
    let start = decode_cursor(cursor, items.len())?;
    let end = start.saturating_add(step).min(items.len());
    let next_cursor = (end < items.len()).then(|| encode_cursor(end));
    Ok(PaginatedItems {
        items: items[start..end].to_vec(),
        next_cursor,
    })
}
