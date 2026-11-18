//! Shared positions follow admitted incidence; coordinate overlap is only a check.
use super::*;
use nurbs_core::interval_eval::Interval as I;
pub struct BoundaryGraphVertex {
    pub bounds: Option<[[f64; 2]; 3]>,
    pub point: Option<[f64; 3]>,
}
pub struct BoundaryGraphSegment {
    pub pair: usize,
    pub vertices: [usize; 2],
    pub deviation_upper_mm: f64,
    /// Shell-use UV intervals at both segment ends.
    pub uv: [[[[f64; 2]; 2]; 2]; 2],
}
pub struct BoundaryGraphPending {
    pub pair: usize,
    pub vertices: Option<[usize; 2]>,
    pub reason: &'static str,
}
pub struct BoundaryGraphReport {
    pub vertices: Vec<BoundaryGraphVertex>,
    pub segments: Vec<BoundaryGraphSegment>,
    pub pending: Vec<BoundaryGraphPending>,
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn prepare_boundary_graph(
        &self,
        limits: crate::source_shared_edge::BoundaryMeshLimits,
    ) -> Result<BoundaryGraphReport> {
        let target = limits.max_deviation_mm;
        let partition = self.prepare_source_boundary_mesh(limits)?;
        let mut bounds: Vec<Option<[[f64; 2]; 3]>> = vec![None; self.vertices()];
        let mut conflict = vec![false; self.vertices()];
        let mut maps = Vec::new();
        for p in &partition.partitions {
            let mut map = Vec::new();
            if let Some(mesh) = &p.mesh {
                for (local, v) in mesh.vertices.iter().enumerate() {
                    let id = if local < 2 {
                        p.mesh_endpoints[local]
                    } else {
                        let id = bounds.len();
                        bounds.push(None);
                        conflict.push(false);
                        id
                    };
                    if !conflict[id] {
                        let merged: Option<[[f64; 2]; 3]> = if let Some(old) = bounds[id] {
                            let mut result = v.world;
                            let mut valid = true;
                            for k in 0..3 {
                                result[k] =
                                    [old[k][0].max(v.world[k][0]), old[k][1].min(v.world[k][1])];
                                valid &= result[k][0] <= result[k][1];
                            }
                            valid.then_some(result)
                        } else {
                            Some(v.world)
                        };
                        if merged.is_none() {
                            conflict[id] = true;
                        }
                        bounds[id] = merged;
                    }
                    map.push(id);
                }
            }
            maps.push(map);
        }
        let vertices: Vec<_> = bounds
            .into_iter()
            .map(|bounds| BoundaryGraphVertex {
                point: bounds.map(|b| std::array::from_fn(|k| b[k][0] * 0.5 + b[k][1] * 0.5)),
                bounds,
            })
            .collect();
        let mut result = BoundaryGraphReport {
            vertices,
            segments: vec![],
            pending: vec![],
        };
        for (p, map) in partition.partitions.iter().zip(&maps) {
            let (Some(mesh), Some(charts)) = (&p.mesh, &p.shell_uv) else {
                result.pending.push(BoundaryGraphPending {
                    pair: p.pair,
                    vertices: None,
                    reason: p.reason,
                });
                continue;
            };
            for pending in &mesh.pending {
                result.pending.push(BoundaryGraphPending {
                    pair: p.pair,
                    vertices: pending.vertices.map(|v| v.map(|i| map[i])),
                    reason: pending.reason,
                });
            }
            for segment in &mesh.segments {
                let ids = segment.vertices.map(|i| map[i]);
                let mut shift = 0_f64;
                let mut valid = true;
                for (local, id) in segment.vertices.into_iter().zip(ids) {
                    let Some(point) = result.vertices[id].point else {
                        valid = false;
                        break;
                    };
                    let mut distance = I::point(0.);
                    for k in 0..3 {
                        let d = I::point(point[k]).sub(I::point(mesh.vertices[local].point[k]))?;
                        distance = distance.add(I::point(d.lo.abs().max(d.hi.abs())))?;
                    }
                    shift = shift.max(distance.hi);
                }
                let error = I::point(segment.deviation_upper_mm)
                    .add(I::point(shift))?
                    .hi;
                if !valid || error > target {
                    result.pending.push(BoundaryGraphPending {
                        pair: p.pair,
                        vertices: Some(ids),
                        reason: if valid {
                            "boundary-shared-rounding-error-limit"
                        } else {
                            "boundary-shared-endpoint-enclosure-conflict"
                        },
                    });
                    continue;
                }
                result.segments.push(BoundaryGraphSegment {
                    pair: p.pair,
                    vertices: ids,
                    deviation_upper_mm: error,
                    uv: segment.vertices.map(|i| charts[i]),
                });
            }
        }
        Ok(result)
    }
}
