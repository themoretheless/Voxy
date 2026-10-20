//! Original archive material words and source metadata; no image substitution.
use std::path::Path;
pub(super) fn load(
    root: &Path,
    meshes: &[voxy_render::archive_scene::ArchiveMeshSource],
    enabled: bool,
) -> Result<Vec<u32>, Box<dyn std::error::Error>> {
    if !enabled {
        return Ok(vec![0, 0, 24, 0]);
    }
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("manifest.json"))?)?;
    let materials = manifest["materials"]
        .as_array()
        .ok_or("material admission requires source materials")?;
    let mut words = vec![u32::try_from(materials.len())?, 1, 24, 0];
    for (index, material) in materials.iter().enumerate() {
        if material["id"].as_u64() != Some(index as u64) {
            return Err("noncanonical material identity".into());
        }
        let name = material["parameters"]["file"]
            .as_str()
            .ok_or("missing material payload")?;
        if Path::new(name).components().count() != 1 {
            return Err("invalid material payload path".into());
        }
        let bytes = std::fs::read(root.join(name))?;
        if bytes.len() != 80 || material["parameters"]["bytes"].as_u64() != Some(80) {
            return Err("material source ABI mismatch".into());
        }
        let parameters: voxy_render::archive_scene::ArchiveMaterial =
            bytemuck::pod_read_unaligned(&bytes);
        if bytemuck::cast_slice::<_, f32>(std::slice::from_ref(&parameters))
            .iter()
            .any(|v| !v.is_finite())
        {
            return Err("nonfinite material parameters".into());
        }
        let images = material["images"]
            .as_array()
            .ok_or("missing material image bindings")?;
        if images.len() != 4 {
            return Err("material image ABI mismatch".into());
        }
        let image_count = manifest["image_count"]
            .as_u64()
            .ok_or("missing source image count")?;
        for image in images {
            if !image.is_null() && image.as_u64().is_none_or(|i| i >= image_count) {
                return Err("invalid source material image".into());
            }
        }
        words.extend(
            bytes
                .chunks_exact(4)
                .map(|v| u32::from_le_bytes(v.try_into().unwrap())),
        );
        words.extend([
            images[0]
                .as_u64()
                .map(u32::try_from)
                .transpose()?
                .unwrap_or(u32::MAX),
            u32::from(
                material["double_sided"]
                    .as_bool()
                    .ok_or("missing sidedness")?,
            ),
            0,
            0,
        ]);
    }
    if meshes.iter().any(|mesh| mesh.material >= materials.len()) {
        return Err("mesh material identity out of range".into());
    }
    Ok(words)
}
pub(super) fn verify(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
    offset: u64,
    words: &[u32],
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = bytemuck::cast_slice::<u32, u8>(words);
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("original material word readback"),
        size: bytes.len() as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(buffer, offset, &read, 0, bytes.len() as u64);
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    read.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    rx.recv()??;
    let actual = read.slice(..).get_mapped_range()?;
    if actual.as_ref() != bytes {
        return Err("GPU material words changed".into());
    }
    if let Some(path) = std::env::var_os("VOXY_RAY_WORLD_MATERIAL_SOURCE_OUTPUT") {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)?;
        file.write_all(&actual)?;
        file.sync_all()?;
    }
    drop(actual);
    read.unmap();
    println!(
        "WORLD MATERIAL SOURCE PASS materials={} bytes={} source_words_unmodified=true",
        words[0],
        bytes.len()
    );
    Ok(())
}
