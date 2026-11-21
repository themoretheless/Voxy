//! Unified Voxy Engine & Editor MCP server supporting stdio and Streamable HTTP.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    voxy_editor::mcp::run_cli(std::env::args().skip(1))
}
