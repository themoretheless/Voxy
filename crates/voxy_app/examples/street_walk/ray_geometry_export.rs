//! Lossless diagnostic input for full-world BVH qualification, before GPU upload.
//! This is geometry evidence, not a renderable package or a performance result.
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::Path,
};

fn payload(root: &Path, name: &str, bytes: &[u8]) -> io::Result<serde_json::Value> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(name))?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(
        serde_json::json!({"file":name,"bytes":bytes.len(),"blake3":blake3::hash(bytes).to_hex().to_string()}),
    )
}

fn write_image(
    root: &Path,
    id: usize,
    image: &voxy_render::archive_scene::ArchiveImageSource,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    use voxy_render::archive_scene::ArchiveImageSource;
    let descriptor = match image {
        ArchiveImageSource::Float {
            width,
            height,
            pixels,
        } => {
            if *width == 0
                || *height == 0
                || u64::from(*width) * u64::from(*height) * 4 != pixels.len() as u64
                || pixels.iter().any(|v| !v.is_finite())
            {
                return Err("invalid float image capture".into());
            }
            serde_json::json!({"id":id,"format":"rgba32float","width":width,"height":height,"source_mip_count":1,"mips":[payload(root,&format!("image-{id:04}-mip-0000.bin"),bytemuck::cast_slice(pixels))?],"generated_gpu_mips_exported":false})
        }
        ArchiveImageSource::Rgba(image) => {
            serde_json::json!({"id":id,"format":"rgba8unorm","width":image.width(),"height":image.height(),"source_mip_count":1,"mips":[payload(root,&format!("image-{id:04}-mip-0000.bin"),image.rgba())?],"generated_gpu_mips_exported":false})
        }
        ArchiveImageSource::Bc7(image) => {
            if image.width == 0
                || image.height == 0
                || image.mip_ranges.is_empty()
                || image.mip_ranges.len() > image.width.max(image.height).ilog2() as usize + 1
            {
                return Err("invalid BC7 capture dimensions or mip count".into());
            }
            let mut mips = Vec::new();
            for mip in 0..image.mip_ranges.len() {
                let bytes = image.mip_bytes(mip).ok_or("missing source BC7 mip")?;
                let expected = u64::from((image.width >> mip).max(1).div_ceil(4))
                    * u64::from((image.height >> mip).max(1).div_ceil(4))
                    * 16;
                if bytes.len() as u64 != expected {
                    return Err("invalid BC7 capture mip length".into());
                }
                mips.push(payload(
                    root,
                    &format!("image-{id:04}-mip-{mip:04}.bin"),
                    image.mip_bytes(mip).ok_or("missing source BC7 mip")?,
                )?);
            }
            let format = match image.format {
                wgpu::TextureFormat::Bc7RgbaUnorm => "bc7-rgba-unorm",
                wgpu::TextureFormat::Bc7RgbaUnormSrgb => "bc7-rgba-unorm-srgb",
                _ => return Err("unexpected compressed source format".into()),
            };
            serde_json::json!({"id":id,"format":format,"width":image.width,"height":image.height,"source_mip_count":mips.len(),"mips":mips,"generated_gpu_mips_exported":false})
        }
    };
    Ok(descriptor)
}

pub(super) fn write(
    root: &Path,
    scene: &super::scene::StreetScene,
    seed: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    if cfg!(target_endian = "big") {
        return Err("geometry capture requires little endian".into());
    }
    // Validate everything before creating the directory. Empty batches are retained.
    for (id, mesh) in scene.geometry.iter().enumerate() {
        if mesh.material >= scene.materials.len()
            || mesh.indices.len() % 3 != 0
            || mesh
                .indices
                .iter()
                .any(|&i| i as usize >= mesh.vertices.len())
            || bytemuck::cast_slice::<_, f32>(&mesh.vertices)
                .iter()
                .any(|v| !v.is_finite())
            || mesh
                .instances
                .iter()
                .any(|instance| instance.world.iter().flatten().any(|v| !v.is_finite()))
        {
            return Err(format!("invalid source geometry mesh={id} material={} materials={} vertices={} indices={} instances={} nonfinite_vertex={} nonfinite_instance={}", mesh.material,scene.materials.len(),mesh.vertices.len(),mesh.indices.len(),mesh.instances.len(),bytemuck::cast_slice::<_, f32>(&mesh.vertices).iter().filter(|v| !v.is_finite()).count(),bytemuck::cast_slice::<_, f32>(&mesh.instances).iter().filter(|v| !v.is_finite()).count()).into());
        }
    }
    fs::create_dir(root)?; // Refuse overwriting any earlier evidence, including partial exports.
    let mut meshes = Vec::with_capacity(scene.geometry.len());
    let mut unique_triangles = 0u64;
    let mut instanced_triangles = 0u64;
    let mut instances = 0u64;
    for (id, mesh) in scene.geometry.iter().enumerate() {
        let triangles = (mesh.indices.len() / 3) as u64;
        unique_triangles += triangles;
        instanced_triangles += triangles * mesh.instances.len() as u64;
        instances += mesh.instances.len() as u64;
        meshes.push(serde_json::json!({
            "id":id,"material":mesh.material,"center":mesh.center.to_array(),
            "vertex_count":mesh.vertices.len(),"index_count":mesh.indices.len(),
            "instance_count":mesh.instances.len(),
            "nonfinite_normal_instances":mesh.instances.iter().filter(|instance| instance.normal.iter().flatten().any(|v| !v.is_finite())).count(),
            "vertices":payload(root,&format!("mesh-{id:04}-vertices.bin"),bytemuck::cast_slice(&mesh.vertices))?,
            "indices":payload(root,&format!("mesh-{id:04}-indices.bin"),bytemuck::cast_slice(&mesh.indices))?,
            "instances":payload(root,&format!("mesh-{id:04}-instances.bin"),bytemuck::cast_slice(&mesh.instances))?
        }));
    }
    let mut materials = Vec::with_capacity(scene.materials.len());
    for (id, material) in scene.materials.iter().enumerate() {
        materials.push(serde_json::json!({"id":id,"images":material.images,"double_sided":material.double_sided,
            "parameters":payload(root,&format!("material-{id:04}.bin"),bytemuck::bytes_of(&material.parameters))?}));
    }
    let export_images = std::env::var("VOXY_STREET_EXPORT_RAY_IMAGES").as_deref() == Ok("1");
    let mut images = Vec::new();
    if export_images {
        for (id, image) in scene.images.iter().enumerate() {
            images.push(write_image(root, id, image)?);
        }
    }
    let manifest = serde_json::json!({
        "schema":"voxy-full-world-ray-geometry-v1","seed":seed,
        "byte_order":"little","vertex_stride":48,"instance_stride":112,"index_stride":4,
        "matrix_layout":"column-major","indices":"local to each mesh",
        "mesh_count":meshes.len(),"instance_count":instances,
        "unique_triangles":unique_triangles,"instanced_triangles":instanced_triangles,
        "image_count":scene.images.len(),"images_exported":export_images,"renderable_package":false,
        "camera_culling":false,"geometry_quantization":false,"fps_qualified":false,
        "meshes":meshes,"materials":materials,"images":images,
        "image_capture":"original decoded RGBA or original BC7 blocks; no conversion; upload-generated mip levels are not captured"
    });
    // A manifest is the completion marker; incomplete directories have none.
    payload(
        root,
        "manifest.json",
        &serde_json::to_vec_pretty(&manifest)?,
    )?;
    eprintln!(
        "STREET WORLD GEOMETRY meshes={} instances={instances} unique_triangles={unique_triangles} instanced_triangles={instanced_triangles} directory={}",
        scene.geometry.len(),
        root.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxy_render::archive_scene::ArchiveImageSource;
    fn directory() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let value = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let seq = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "voxy-image-export-{}-{value}-{seq}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        root
    }
    #[test]
    fn float_capture_keeps_subnormal_signed_zero_and_low_mantissa_bits() {
        let root = directory();
        let words = [1u32, 0x80000000, 0x3f800001, 0x3f000001];
        let pixels = words.map(f32::from_bits).to_vec();
        let image = ArchiveImageSource::Float {
            width: 1,
            height: 1,
            pixels,
        };
        let descriptor = write_image(&root, 0, &image).unwrap();
        assert_eq!(descriptor["format"], "rgba32float");
        assert_eq!(descriptor["source_mip_count"], 1);
        assert_eq!(descriptor["generated_gpu_mips_exported"], false);
        assert_eq!(
            fs::read(root.join("image-0000-mip-0000.bin")).unwrap(),
            bytemuck::cast_slice::<u32, u8>(&words)
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn rgba_capture_keeps_straight_alpha_bytes() {
        let root = directory();
        let bytes = vec![7, 31, 123, 0, 245, 231, 101, 255];
        let image = ArchiveImageSource::Rgba(
            voxy_render::ImageAsset::from_rgba(2, 1, bytes.clone(), Default::default()).unwrap(),
        );
        let descriptor = write_image(&root, 2, &image).unwrap();
        assert_eq!(descriptor["format"], "rgba8unorm");
        assert_eq!(
            fs::read(root.join("image-0002-mip-0000.bin")).unwrap(),
            bytes
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn bc7_capture_preserves_every_authored_mip_block() {
        let root = directory();
        let mut bytes = vec![0u8; 148 + 96];
        bytes[..4].copy_from_slice(b"DDS ");
        bytes[84..88].copy_from_slice(b"DX10");
        for (offset, value) in [
            (4, 124u32),
            (76, 32),
            (12, 8),
            (16, 8),
            (28, 3),
            (128, 99),
            (132, 3),
            (140, 1),
        ] {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        for (i, word) in bytes[148..].iter_mut().enumerate() {
            *word = (i * 13 + 17) as u8;
        }
        let image = ArchiveImageSource::Bc7(
            voxy_render::Bc7ImageAsset::decode_dds(bytes.clone().into(), 1024, 8192).unwrap(),
        );
        let descriptor = write_image(&root, 3, &image).unwrap();
        assert_eq!(descriptor["format"], "bc7-rgba-unorm-srgb");
        assert_eq!(descriptor["source_mip_count"], 3);
        for (mip, range) in [(0, 148..212), (1, 212..228), (2, 228..244)] {
            assert_eq!(
                fs::read(root.join(format!("image-0003-mip-{mip:04}.bin"))).unwrap(),
                bytes[range]
            );
        }
        assert!(write_image(&root, 3, &image).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
