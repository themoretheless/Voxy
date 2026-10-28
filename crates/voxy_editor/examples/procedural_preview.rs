//! Offscreen preview using the same rendering entry point as scene_mcp.
#[path = "procedural_preview/render.rs"]
mod render;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: procedural_preview PROJECT OUTPUT.ppm".into());
    }
    let rgb = render::render(std::path::Path::new(&args[0]), None)?;
    let mut ppm = b"P6\n1024 1024\n255\n".to_vec();
    ppm.extend(rgb);
    std::fs::write(&args[1], ppm)?;
    Ok(())
}
