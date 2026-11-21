//! Project-scene MCP server supporting local stdio and loopback Streamable HTTP.
#[path = "procedural_preview/render.rs"]
mod render;
#[path = "scene_mcp/server.rs"]
mod server;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    voxy_editor::mcp::set_stdio_quiet(true);
    let (mode, rest) = voxy_mcp::parse_transport_args(std::env::args().skip(1))?;
    let path = rest
        .into_iter()
        .next()
        .ok_or("usage: scene_mcp /absolute/path/project.voxy [--http 127.0.0.1:port]")?;
    let server = server::Server::open(std::path::Path::new(&path))?;
    let handler = std::sync::Arc::new(voxy_mcp::MutexMcpHandler::new(server));
    voxy_mcp::serve(handler, mode)?;
    Ok(())
}
