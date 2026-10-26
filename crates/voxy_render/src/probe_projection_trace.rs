//! Opt-in observation of the original projector. No extra dispatch or readback
//! occurs between faces; canonical production WGSL remains untouched.
use voxy_gpu_core::{ComputeMemoryBudget, ManagedBufferDescriptor};

const FACE_VECS: usize = 656;
const SAMPLES: usize = 65536;
#[derive(Debug)]
pub(super) struct ProjectionTrace {
    pub buffer: voxy_gpu_core::ComputeStorage,
    pub shader: String,
    count: usize,
    target: usize,
    output: std::path::PathBuf,
}
fn insert_after(source: &mut String, anchor: &str, addition: &str) {
    assert_eq!(source.matches(anchor).count(), 1, "trace anchor changed");
    *source = source.replacen(anchor, &format!("{anchor}\n{addition}"), 1);
}
fn shader(count: usize, target: usize) -> String {
    let mut s = include_str!("probe_gi.wgsl").to_owned();
    insert_after(
        &mut s,
        "var<workgroup> partial:array<array<vec3<f32>,4>,128>;",
        &format!(
            "@group(0) @binding(3) var<storage,read_write> trace:array<vec4<u32>>;\nconst TRACE_SAMPLE_BASE:u32={}u;\nconst TRACE_TARGET:u32={target}u;\nconst TRACE_FACE_STRIDE:u32=656u;",
            count * 6 * FACE_VECS
        ),
    );
    insert_after(
        &mut s,
        "let count=u32(size/step);",
        "let trace_base=(u32(bake.forward.w)*6u+u32(bake.right.w))*TRACE_FACE_STRIDE;\nvar observed=vec4<u32>(2166136261u);",
    );
    insert_after(
        &mut s,
        "sum[3]+=radiance*(0.488603*n.x*weight);",
        "observed=vec4<u32>((observed.xyz^bitcast<vec3<u32>>(radiance))*vec3<u32>(16777619u),observed.w);\nobserved.w=(observed.w^bitcast<u32>(weight))*16777619u;\nobserved.w=(observed.w^bitcast<u32>(n.x))*16777619u;\nobserved.w=(observed.w^bitcast<u32>(n.y))*16777619u;\nobserved.w=(observed.w^bitcast<u32>(n.z))*16777619u;\nif u32(bake.forward.w)==TRACE_TARGET {\nlet t=TRACE_SAMPLE_BASE+u32(bake.right.w)*131072u+i*2u;\ntrace[t]=bitcast<vec4<u32>>(vec4<f32>(radiance,weight));\ntrace[t+1u]=vec4<u32>(bitcast<vec3<u32>>(n),0x50425431u);\n}",
    );
    insert_after(
        &mut s,
        "partial[thread]=sum;",
        "trace[trace_base+528u+thread]=observed;\nfor(var k=0u;k<4u;k++){trace[trace_base+thread*4u+k]=bitcast<vec4<u32>>(vec4<f32>(sum[k],0.));}\nif thread==0u {\ntrace[trace_base+512u]=bitcast<vec4<u32>>(bake.forward);\ntrace[trace_base+513u]=bitcast<vec4<u32>>(bake.right);\ntrace[trace_base+514u]=bitcast<vec4<u32>>(bake.up);\ntrace[trace_base+515u]=bitcast<vec4<u32>>(bake.square);\n}",
    );
    insert_after(
        &mut s,
        "probes[base+k]=vec4<f32>(previous+partial[0][k],1.);",
        "trace[trace_base+516u+k]=bitcast<vec4<u32>>(vec4<f32>(previous,0.));\ntrace[trace_base+520u+k]=bitcast<vec4<u32>>(vec4<f32>(partial[0][k],0.));\ntrace[trace_base+524u+k]=bitcast<vec4<u32>>(probes[base+k]);",
    );
    s
}
impl ProjectionTrace {
    pub fn new(
        device: &wgpu::Device,
        count: usize,
        target: usize,
    ) -> Result<Self, crate::ComputeError> {
        let fail = |s: &str| crate::ComputeError::Validation(s.into());
        if count == 0 || count > super::MAX_PROBES || target >= count {
            return Err(fail("projector trace index outside grid"));
        }
        let output = std::env::var_os("VOXY_STREET_PROBE_PROJECTOR_TRACE_OUTPUT")
            .ok_or_else(|| fail("projector trace output required"))?
            .into();
        let size = ((count * 6 * FACE_VECS + 6 * 2 * SAMPLES) * 16) as u64;
        if size > device.limits().max_storage_buffer_binding_size
            || size > device.limits().max_buffer_size
        {
            return Err(fail("projector trace exceeds device buffer limits"));
        }
        let [buffer] = ComputeMemoryBudget::for_device(device).allocate_buffers([
            ManagedBufferDescriptor {
                label: "original projector operands",
                size,
                contents: None,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            },
        ])?;
        Ok(Self {
            buffer,
            shader: shader(count, target),
            count,
            target,
            output,
        })
    }
    pub fn validate_face(&self, index: usize, face: usize, size: f32, step: f32) {
        assert!(index < self.count && face < 6);
        assert!(
            (size / step).floor().powi(2) <= SAMPLES as f32,
            "projector trace sample capacity exceeded"
        );
    }
    pub fn save(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<(), crate::ComputeError> {
        let run = || -> Result<(), Box<dyn std::error::Error>> {
            std::fs::create_dir(&self.output)?;
            let capture = crate::archive_hdr_diagnostics::capture_storage(
                device,
                queue,
                &self.buffer,
                &self.output.join("operands.u32.bin"),
                true,
            )?;
            let metadata = serde_json::json!({"schema":2,"operand_hash":"ordered_fnv1a_u32_rgb_and_weight_normal","hash_equality_is_not_operand_equality_proof":true,"probe_count":self.count,"target_probe":self.target,
                "face_vec4_stride":FACE_VECS,"sample_capacity":SAMPLES,"sample_vec4_base":self.count*6*FACE_VECS,
                "canonical_shader_blake3":blake3::hash(include_bytes!("probe_gi.wgsl")).to_hex().to_string(),
                "traced_shader_blake3":blake3::hash(self.shader.as_bytes()).to_hex().to_string(),
                "capture":capture,"extra_per_face_dispatches":0,"extra_per_face_readbacks":0,"performance_qualified":false});
            std::fs::write(
                self.output.join("metadata.json"),
                serde_json::to_vec_pretty(&metadata)?,
            )?;
            std::fs::write(self.output.join("projector.wgsl"), &self.shader)?;
            Ok(())
        };
        run().map_err(|e| crate::ComputeError::Validation(format!("projector operand trace: {e}")))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn traced_shader_validates_and_keeps_original_statements_in_order() {
        let source = super::shader(372, 82);
        let module = naga::front::wgsl::parse_str(&source).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();
        let mut remaining = source.as_str();
        for line in include_str!("probe_gi.wgsl").lines() {
            let i = remaining
                .find(line)
                .expect("original shader line removed or reordered");
            remaining = &remaining[i + line.len()..];
        }
        assert_eq!(source.matches("textureLoad(").count(), 1);
        assert_eq!(source.matches("workgroupBarrier();").count(), 2);
    }
}
