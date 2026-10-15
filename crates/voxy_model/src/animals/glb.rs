use super::{Breed, build_study};
use serde_json::json;
pub(super) fn encode(breeds: &[Breed], gallery: bool) -> Result<Vec<u8>, String> {
    let mut bin = Vec::new();
    let mut views = Vec::new();
    let mut accessors = Vec::new();
    let mut primitives = Vec::new();
    for (i, breed) in breeds.iter().enumerate() {
        let mut surface = build_study(breed.anatomy())?;
        if gallery {
            for p in &mut surface.positions {
                p[0] += (i % 3) as f32 * 1.2 - 1.2;
                p[2] += (i / 3) as f32 * 1.6;
            }
        }
        let mut attrs = Vec::new();
        for (data, position) in [(&surface.positions, true), (&surface.normals, false)] {
            let offset = bin.len();
            for v in data {
                for f in v {
                    bin.extend(f.to_le_bytes());
                }
            }
            let view = views.len();
            views.push(json!({"buffer":0,"byteOffset":offset,"byteLength":bin.len()-offset,"target":34962}));
            let mut accessor =
                json!({"bufferView":view,"componentType":5126,"count":data.len(),"type":"VEC3"});
            if position {
                let mut min = [f32::INFINITY; 3];
                let mut max = [f32::NEG_INFINITY; 3];
                for p in data {
                    for j in 0..3 {
                        min[j] = min[j].min(p[j]);
                        max[j] = max[j].max(p[j]);
                    }
                }
                accessor["min"] = json!(min);
                accessor["max"] = json!(max);
            }
            attrs.push(accessors.len());
            accessors.push(accessor);
        }
        let offset = bin.len();
        for index in &surface.indices {
            bin.extend(index.to_le_bytes());
        }
        let view = views.len();
        views.push(
            json!({"buffer":0,"byteOffset":offset,"byteLength":bin.len()-offset,"target":34963}),
        );
        let index = accessors.len();
        accessors.push(json!({"bufferView":view,"componentType":5125,"count":surface.indices.len(),"type":"SCALAR"}));
        primitives.push(json!({"attributes":{"POSITION":attrs[0],"NORMAL":attrs[1]},"indices":index,"material":0,"extras":{"breed":breed.id(),"stage":"neutral anatomy study; not final realistic asset"}}));
    }
    let root = json!({"asset":{"version":"2.0","generator":"Voxy original breed anatomy studies"},"scene":0,"scenes":[{"nodes":[0]}],"nodes":[{"mesh":0}],"meshes":[{"primitives":primitives}],"materials":[{"pbrMetallicRoughness":{"baseColorFactor":[0.42,0.42,0.42,1.0],"metallicFactor":0.0,"roughnessFactor":0.85}}],"buffers":[{"byteLength":bin.len()}],"bufferViews":views,"accessors":accessors});
    let mut json = serde_json::to_vec(&root).map_err(|e| e.to_string())?;
    while json.len() % 4 != 0 {
        json.push(b' ');
    }
    while bin.len() % 4 != 0 {
        bin.push(0);
    }
    let total = u32::try_from(28 + json.len() + bin.len()).map_err(|_| "GLB too large")?;
    let mut out = Vec::new();
    out.extend(b"glTF");
    out.extend(2_u32.to_le_bytes());
    out.extend(total.to_le_bytes());
    out.extend((json.len() as u32).to_le_bytes());
    out.extend(0x4e4f534a_u32.to_le_bytes());
    out.extend(json);
    out.extend((bin.len() as u32).to_le_bytes());
    out.extend(0x004e4942_u32.to_le_bytes());
    out.extend(bin);
    Ok(out)
}
