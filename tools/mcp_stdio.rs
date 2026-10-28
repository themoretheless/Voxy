//! Shared bounded newline-delimited MCP transport. stdout contains JSON-RPC only.
use serde_json::{Value, json};
use std::io::{self, BufRead, Read, Write};
const MAX_LINE: usize = 2 * 1024 * 1024;

pub fn serve(mut dispatch: impl FnMut(&Value) -> Value) -> io::Result<()> {
    run(io::stdin().lock(), io::stdout().lock(), &mut dispatch)
}
fn run(
    mut input: impl BufRead,
    mut output: impl Write,
    dispatch: &mut impl FnMut(&Value) -> Value,
) -> io::Result<()> {
    loop {
        let mut line = Vec::new();
        let n = (&mut input)
            .take((MAX_LINE + 1) as u64)
            .read_until(b'\n', &mut line)?;
        if n == 0 {
            return Ok(());
        }
        if n > MAX_LINE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "MCP request exceeds 2 MiB",
            ));
        }
        let response = match serde_json::from_slice::<Value>(&line) {
            Ok(request) => {
                if request.is_object()
                    && request.get("id").is_none()
                    && request["method"].is_string()
                {
                    continue;
                }
                if request.is_object()
                    && request.get("method").is_none()
                    && (request.get("result").is_some() || request.get("error").is_some())
                {
                    continue;
                }
                dispatch(&request)
            }
            Err(_) => {
                json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"Parse error"}})
            }
        };
        writeln!(output, "{response}")?;
        output.flush()?;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn notifications_are_silent_and_parse_errors_do_not_corrupt_the_next_request() {
        let input = b"{bad}\n{\"method\":\"notifications/initialized\",\"jsonrpc\":\"2.0\"}\n{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"ping\"}\n";
        let mut output = Vec::new();
        run(
            &input[..],
            &mut output,
            &mut |r| json!({"jsonrpc":"2.0","id":r["id"],"result":{}}),
        )
        .unwrap();
        let lines: Vec<Value> = String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|s| serde_json::from_str(s).unwrap())
            .collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0]["error"]["code"], -32700);
        assert_eq!(lines[1]["id"], 7);
    }
}
