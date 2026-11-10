//! Native boundary partitions bound to qualified shell incidence.
use super::*;
#[path = "source_profile_shell_section_mesh.rs"]
mod section;
use crate::source_shared_edge::{BoundaryMeshLimits, BoundaryMeshReport};
pub struct ShellBoundaryPartition {
    pub pair: usize,
    /// Shell vertex IDs at first-use traversal endpoints.
    pub endpoints: [usize; 2],
    /// IDs corresponding to native carrier traversal endpoints.
    pub mesh_endpoints: [usize; 2],
    /// Which shell uses are the native carrier UV charts (not transition charts).
    pub native_chart_uses: [bool; 2],
    /// Index into the native mesh UV/parameter arrays, per shell use.
    pub native_chart_sides: [Option<usize>; 2],
    /// Interval UV coordinates in actual shell use order, per shared vertex.
    pub shell_uv: Option<Vec<[[[f64; 2]; 2]; 2]>>,
    pub mesh: Option<BoundaryMeshReport>,
    pub reason: &'static str,
}
pub struct ShellBoundaryMeshReport {
    pub partitions: Vec<ShellBoundaryPartition>,
    pub cells: usize,
    pub spans: usize,
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    /// Preserve every boundary pair, including those whose expression-valued
    /// carrier still needs a dedicated partition implementation.
    pub fn prepare_source_boundary_mesh(
        &self,
        limits: BoundaryMeshLimits,
    ) -> Result<ShellBoundaryMeshReport> {
        if !limits.max_deviation_mm.is_finite()
            || limits.max_deviation_mm <= 0.
            || limits.cells > 100000
            || limits.spans > 100000
            || limits.max_depth > 32
        {
            return Err(Error::new(
                "BREP_PROFILE_BOUNDARY_MESH",
                "Choose bounded native boundary work",
            ));
        }
        let original = self.assembly.patch.rails.face.boundary.rooted[0]
            .restricted
            .proposal
            .body
            .geometry()
            .shell();
        let mut report = ShellBoundaryMeshReport {
            partitions: Vec::new(),
            cells: 0,
            spans: 0,
        };
        for (pair, owned) in self.pairs.iter().enumerate() {
            let first = owned.uses[0];
            let endpoints = [
                self.boundary_vertex(first, 0).unwrap(),
                self.boundary_vertex(first, 1).unwrap(),
            ];
            let carrier = match owned.kind {
                ShellPairKind::Original(id) => Some(&original.edges()[id]),
                ShellPairKind::Outer(id) => Some(&self.assembly.edges[id]),
                ShellPairKind::Rail(id) => Some(&self.assembly.patch.rails.rails[id].edge),
                ShellPairKind::Section(_) => None,
            };
            let (mesh_endpoints, native_chart_uses) = if let ShellPairKind::Rail(id) = owned.kind {
                let rail = &self.assembly.patch.rails.rails[id];
                let neighbor = owned.uses[1];
                let side = rail.neighbor;
                (
                    [
                        self.boundary_vertex(neighbor, side).unwrap(),
                        self.boundary_vertex(neighbor, 1 - side).unwrap(),
                    ],
                    [false, true],
                )
            } else {
                (endpoints, [carrier.is_some(); 2])
            };
            let native_chart_sides = match owned.kind {
                ShellPairKind::Rail(id) => {
                    [None, Some(self.assembly.patch.rails.rails[id].neighbor)]
                }
                ShellPairKind::Section(_) => [None, None],
                _ => [Some(0), Some(1)],
            };
            let mesh = carrier
                .map(|edge| {
                    edge.prepare_boundary_mesh(BoundaryMeshLimits {
                        max_deviation_mm: limits.max_deviation_mm,
                        cells: limits.cells - report.cells,
                        spans: limits.spans - report.spans,
                        max_depth: limits.max_depth,
                    })
                })
                .transpose()?;
            let mesh = if let ShellPairKind::Section(end) = owned.kind {
                Some(section::prepare(
                    self,
                    end,
                    BoundaryMeshLimits {
                        max_deviation_mm: limits.max_deviation_mm,
                        cells: limits.cells - report.cells,
                        spans: limits.spans - report.spans,
                        max_depth: limits.max_depth,
                    },
                )?)
            } else {
                mesh
            };
            let shell_uv = if let Some(mesh) = &mesh {
                if let ShellPairKind::Rail(id) = owned.kind {
                    use nurbs_core::interval_eval::Interval as I;
                    let root = self.assembly.patch.rails.face.boundary.rooted[id];
                    let profile = root.restricted.proposal.profile(root.face).unwrap();
                    let range = profile.section(root.axis, root.parameters[0])?.range();
                    let rail = &self.assembly.patch.rails.rails[id];
                    let width = I::point(root.parameters[1]).sub(I::point(root.parameters[0]))?;
                    let mut mapped = Vec::new();
                    for vertex in &mesh.vertices {
                        let selected = vertex.uv[1 - rail.neighbor];
                        let across = I::new(selected[root.axis][0], selected[root.axis][1])?
                            .sub(I::point(root.parameters[0]))?
                            .div(width)?
                            .intersect(0., 1.)?;
                        mapped.push([
                            [[range[id]; 2], [across.lo, across.hi]],
                            vertex.uv[rail.neighbor],
                        ]);
                    }
                    Some(mapped)
                } else {
                    Some(mesh.vertices.iter().map(|v| v.uv).collect())
                }
            } else {
                None
            };
            let reason = match &mesh {
                None => "boundary-expression-carrier-partition-unproven",
                Some(mesh) if !mesh.pending.is_empty() => "boundary-source-partition-incomplete",
                Some(_) => "boundary-source-partition-qualified",
            };
            if let Some(mesh) = &mesh {
                report.cells += mesh.cells;
                report.spans += mesh.spans;
            }
            report.partitions.push(ShellBoundaryPartition {
                pair,
                endpoints,
                mesh_endpoints,
                native_chart_uses,
                native_chart_sides,
                shell_uv,
                mesh,
                reason,
            });
        }
        Ok(report)
    }
}
