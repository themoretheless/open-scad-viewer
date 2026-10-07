use crate::{EdgeCounts, EdgeUses, MeshView, Result, check};
use math_core::{cross, norm, sub};

#[derive(Debug, Clone)]
pub struct Inspection {
    pub triangle_count: usize,
    pub vertex_count: usize,
    pub boundary_edges: usize,
    pub non_manifold_edges: usize,
    pub orientation_conflicts: usize,
    pub degenerate_triangles: usize,
    pub closed: bool,
    pub signed_volume_mm3: f64,
}
impl MeshView<'_> {
    pub fn inspect(&self) -> Result<Inspection> {
        self.inspect_with_edges().map(|(report, _)| report)
    }
    pub fn inspect_with_edges(&self) -> Result<(Inspection, EdgeUses)> {
        self.validate()?;
        let reference = if self.positions.is_empty() {
            [0.; 3]
        } else {
            self.point(0)?
        };
        let mut volume = 0.;
        let mut volume_compensation = 0.;
        let mut degenerate = 0;
        for t in self.indices.as_chunks::<3>().0 {
            let a = self.point(t[0])?;
            let b = self.point(t[1])?;
            let c = self.point(t[2])?;
            let ab = sub(b, a);
            let ac = sub(c, a);
            let normal = cross(ab, ac);
            if norm(normal) <= f64::EPSILON * (norm(ab) * norm(ac)).max(1.) {
                degenerate += 1;
            }
            let ar = sub(a, reference);
            let br = sub(b, reference);
            let cr = sub(c, reference);
            let bc = cross(br, cr);
            let term = ar[0] * bc[0] + ar[1] * bc[1] + ar[2] * bc[2] - volume_compensation;
            let next = volume + term;
            volume_compensation = (next - volume) - term;
            volume = next;
        }
        check(
            volume.is_finite(),
            "Mesh volume exceeded finite numeric bounds.",
        )?;
        let edges = EdgeUses::new(self.indices);
        let EdgeCounts {
            boundary,
            non_manifold,
            orientation,
        } = edges.counts();
        Ok((
            Inspection {
                triangle_count: self.indices.len() / 3,
                vertex_count: self.positions.len() / 3,
                boundary_edges: boundary,
                non_manifold_edges: non_manifold,
                orientation_conflicts: orientation,
                degenerate_triangles: degenerate,
                closed: !self.indices.is_empty()
                    && boundary == 0
                    && non_manifold == 0
                    && orientation == 0
                    && degenerate == 0,
                signed_volume_mm3: volume / 6.,
            },
            edges,
        ))
    }
}
