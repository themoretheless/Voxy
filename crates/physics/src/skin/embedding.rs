//! Renderer-independent displacement transfer from a skin shell to a detailed mesh.
use super::Point;
#[derive(Clone, Copy, Debug)]
pub struct SurfaceBinding {
    pub vertex: usize,
    pub triangle: usize,
    pub weights: [f64; 3],
}
#[derive(Clone, Debug)]
pub struct SkinEmbedding {
    render_count: usize,
    shell_count: usize,
    triangles: Vec<[usize; 3]>,
    bindings: Vec<SurfaceBinding>,
}
impl SkinEmbedding {
    /// One displacement binding per rendered vertex. Coincident seam vertices
    /// remain distinct and must each be covered when `require_complete` is true.
    /// Barycentric offsets preserve the detailed mesh exactly at the rest pose.
    /// # Errors
    /// Rejects invalid indices, repeated bindings, nonfinite/nonconvex weights,
    /// degenerate topology references, or missing full-surface coverage.
    pub fn new(
        render_count: usize,
        shell_count: usize,
        triangles: Vec<[usize; 3]>,
        bindings: Vec<SurfaceBinding>,
        require_complete: bool,
    ) -> Result<Self, &'static str> {
        if render_count == 0
            || shell_count < 3
            || triangles.is_empty()
            || triangles.iter().any(|t| {
                t.iter().any(|i| *i >= shell_count) || t[0] == t[1] || t[1] == t[2] || t[0] == t[2]
            })
        {
            return Err("invalid embedding mesh");
        }
        let mut covered = vec![false; render_count];
        for b in &bindings {
            if b.vertex >= render_count
                || b.triangle >= triangles.len()
                || b.weights
                    .iter()
                    .any(|w| !w.is_finite() || !(0.0..=1.0).contains(w))
                || (b.weights.iter().sum::<f64>() - 1.0).abs() > 1e-8
            {
                return Err("invalid surface binding");
            }
            if std::mem::replace(&mut covered[b.vertex], true) {
                return Err("duplicate surface binding");
            }
        }
        if require_complete && covered.iter().any(|v| !*v) {
            return Err("incomplete skin coverage");
        }
        Ok(Self {
            render_count,
            shell_count,
            triangles,
            bindings,
        })
    }
    #[must_use]
    pub fn bindings(&self) -> &[SurfaceBinding] {
        &self.bindings
    }
    /// Apply shell-minus-reference displacement to an independently posed render
    /// mesh. For animated attachments, reference is the posed skeleton surface,
    /// so skeleton motion is not added twice. No output is published on failure.
    /// # Errors
    /// Rejects wrong dimensions, nonfinite inputs and overflow.
    pub fn deform(
        &self,
        shell: &[Point],
        reference: &[Point],
        render: &[Point],
    ) -> Result<Vec<Point>, &'static str> {
        if shell.len() != self.shell_count
            || reference.len() != self.shell_count
            || render.len() != self.render_count
            || shell
                .iter()
                .chain(reference)
                .chain(render)
                .flatten()
                .any(|v| !v.is_finite())
        {
            return Err("invalid embedding pose");
        }
        let mut result = render.to_vec();
        for b in &self.bindings {
            let ids = self.triangles[b.triangle];
            for axis in 0..3 {
                let delta = (0..3)
                    .map(|j| b.weights[j] * (shell[ids[j]][axis] - reference[ids[j]][axis]))
                    .sum::<f64>();
                result[b.vertex][axis] += delta;
            }
        }
        if result.iter().flatten().any(|v| !v.is_finite()) {
            return Err("embedding overflow");
        }
        Ok(result)
    }
}
