//! Deterministic WGSL `// #include "..."` directive resolver and shared shader prelude library.
//!
//! Supports built-in shared modules (`dd_math.wgsl`, `fullscreen.wgsl`, `tonemap.wgsl`, `camera.wgsl`)
//! plus optional caller-supplied include slices, deduplicating multiple includes of the same header
//! (`#pragma once` semantics) and rejecting unknown or cyclic includes before `wgpu` compilation.

use crate::ComputeError;
use std::borrow::Cow;

pub const WGSL_DD_MATH: &str = include_str!("wgsl/dd_math.wgsl");
pub const WGSL_FULLSCREEN: &str = include_str!("wgsl/fullscreen.wgsl");
pub const WGSL_TONEMAP: &str = include_str!("wgsl/tonemap.wgsl");
pub const WGSL_CAMERA: &str = include_str!("wgsl/camera.wgsl");

/// Canonical `#[repr(C)]` host layout matching `SharedCameraUniform` in `wgsl/camera.wgsl`.
#[derive(Clone, Copy, Debug, Default, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct SharedCameraUniformPod {
    pub view_proj: [[f32; 4]; 4],
    pub inv_view_proj: [[f32; 4]; 4],
    pub camera_position: [f32; 4],
    pub viewport_size_and_inv: [f32; 4],
}

const BUILTIN_INCLUDES: &[(&str, &str)] = &[
    ("dd_math.wgsl", WGSL_DD_MATH),
    ("fullscreen.wgsl", WGSL_FULLSCREEN),
    ("tonemap.wgsl", WGSL_TONEMAP),
    ("camera.wgsl", WGSL_CAMERA),
];

/// Resolves `// #include "name.wgsl"` (or `#include "name.wgsl"`) directives deterministically.
///
/// Returns `Cow::Borrowed(source)` with zero allocations when `source` contains no `#include` lines.
///
/// # Errors
///
/// Returns [`ComputeError::Validation`] if an include target is unknown, malformed, or forms a cycle.
pub fn resolve_wgsl_includes<'a>(
    source: &'a str,
    extra_includes: &[(&str, &str)],
) -> Result<Cow<'a, str>, ComputeError> {
    if !source.contains("#include") {
        return Ok(Cow::Borrowed(source));
    }
    let mut out = String::with_capacity(source.len() + 512);
    let mut included_once: Vec<String> = Vec::new();
    let mut active_stack: Vec<String> = Vec::new();
    expand_into(
        source,
        extra_includes,
        &mut included_once,
        &mut active_stack,
        &mut out,
    )?;
    Ok(Cow::Owned(out))
}

fn parse_include_directive(line: &str) -> Result<Option<&str>, ComputeError> {
    let trimmed = line.trim();
    let rest = if let Some(after_slashes) = trimmed.strip_prefix("//") {
        let candidate = after_slashes.trim_start();
        let Some(after_inc) = candidate.strip_prefix("#include") else {
            return Ok(None);
        };
        after_inc.trim()
    } else if let Some(after_inc) = trimmed.strip_prefix("#include") {
        after_inc.trim()
    } else {
        return Ok(None);
    };
    let Some(unquoted) = rest.strip_prefix('"').and_then(|s| s.strip_suffix('"')) else {
        return Err(ComputeError::Validation(format!(
            "malformed WGSL #include directive: {trimmed}"
        )));
    };
    if unquoted.is_empty() {
        return Err(ComputeError::Validation(
            "empty WGSL #include target".into(),
        ));
    }
    Ok(Some(unquoted))
}

fn lookup_include<'a>(name: &str, extra_includes: &'a [(&str, &str)]) -> Option<&'a str> {
    extra_includes
        .iter()
        .find(|(k, _)| *k == name)
        .map(|(_, v)| *v)
        .or_else(|| {
            BUILTIN_INCLUDES
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| *v)
        })
}

fn expand_into(
    source: &str,
    extra_includes: &[(&str, &str)],
    included_once: &mut Vec<String>,
    active_stack: &mut Vec<String>,
    out: &mut String,
) -> Result<(), ComputeError> {
    for line in source.split_inclusive('\n') {
        if let Some(target) = parse_include_directive(line)? {
            if active_stack.iter().any(|s| s == target) {
                return Err(ComputeError::Validation(format!(
                    "cyclic WGSL #include detected for \"{target}\""
                )));
            }
            if included_once.iter().any(|s| s == target) {
                continue;
            }
            let Some(body) = lookup_include(target, extra_includes) else {
                return Err(ComputeError::Validation(format!(
                    "unknown WGSL #include target \"{target}\""
                )));
            };
            included_once.push(target.to_owned());
            active_stack.push(target.to_owned());
            expand_into(body, extra_includes, included_once, active_stack, out)?;
            if !out.ends_with('\n') {
                out.push('\n');
            }
            active_stack.pop();
        } else {
            out.push_str(line);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fast_path_borrows_source_when_no_includes_exist() {
        let src = "@compute @workgroup_size(64) fn main() {}";
        let resolved = resolve_wgsl_includes(src, &[]).unwrap();
        assert!(matches!(resolved, Cow::Borrowed(_)));
    }

    #[test]
    fn resolves_and_validates_all_builtin_wgsl_libraries_with_naga() {
        let shader = r#"
// #include "dd_math.wgsl"
// #include "fullscreen.wgsl"
// #include "tonemap.wgsl"
// #include "camera.wgsl"
// Duplicate include is deduplicated cleanly:
// #include "dd_math.wgsl"

@group(0) @binding(0) var<uniform> cam: SharedCameraUniform;

@fragment
fn fs_main(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let pair = dd_sqrt(dd_mul(vec2<f32>(cam.camera_position.x, 0.0), vec2<f32>(2.0, 0.0)));
    let mapped = tonemap_reinhard_safe(karis_weight_rgb(vec3<f32>(pair.x + in.uv.x)), 1.0);
    return vec4<f32>(mapped, 1.0);
}
"#;
        let expanded = resolve_wgsl_includes(shader, &[]).unwrap();
        let module = naga::front::wgsl::parse_str(&expanded).expect("valid WGSL");
        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        );
        validator.validate(&module).expect("naga validation passes");

        // Verify exact CPU #[repr(C)] Pod layout against naga's WGSL struct layout!
        let (_, cam_ty) = module
            .types
            .iter()
            .find(|(_, ty)| ty.name.as_deref() == Some("SharedCameraUniform"))
            .expect("SharedCameraUniform exists in module");
        let naga::TypeInner::Struct { members, span } = &cam_ty.inner else {
            panic!("expected struct");
        };
        assert_eq!(
            *span as usize,
            std::mem::size_of::<SharedCameraUniformPod>()
        );
        assert_eq!(
            members[0].offset as usize,
            std::mem::offset_of!(SharedCameraUniformPod, view_proj)
        );
        assert_eq!(
            members[1].offset as usize,
            std::mem::offset_of!(SharedCameraUniformPod, inv_view_proj)
        );
        assert_eq!(
            members[2].offset as usize,
            std::mem::offset_of!(SharedCameraUniformPod, camera_position)
        );
        assert_eq!(
            members[3].offset as usize,
            std::mem::offset_of!(SharedCameraUniformPod, viewport_size_and_inv)
        );
    }

    #[test]
    fn rejects_unknown_and_cyclic_includes() {
        assert!(resolve_wgsl_includes("// #include \"missing.wgsl\"\n", &[]).is_err());
        let extras = [
            ("a.wgsl", "// #include \"b.wgsl\"\n"),
            ("b.wgsl", "// #include \"a.wgsl\"\n"),
        ];
        assert!(resolve_wgsl_includes("// #include \"a.wgsl\"\n", &extras).is_err());
    }
}
