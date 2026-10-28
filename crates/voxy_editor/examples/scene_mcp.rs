//! Project-scene companion to model_mcp. Local stdio only; no listening socket.
#[path = "../../../tools/mcp_stdio.rs"]
mod mcp_stdio;
#[path = "procedural_preview/render.rs"]
mod render;
#[path = "scene_mcp/server.rs"]
mod server;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: scene_mcp /absolute/path/project.voxy")?;
    let mut server = server::Server::open(std::path::Path::new(&path))?;
    mcp_stdio::serve(|request| server.dispatch(request))?;
    Ok(())
}
