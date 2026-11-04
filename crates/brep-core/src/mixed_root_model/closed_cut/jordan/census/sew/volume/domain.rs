use super::*;
#[derive(Clone, Copy)]
struct Known {
    rectangle: [[f64; 2]; 2],
    inside: bool,
    // None is a native winding seed; Some is an intersecting root-free cell.
    via: Option<usize>,
}
pub(super) struct Domains {
    pub roots: [[[f64; 2]; 2]; 2],
    bands: Vec<[[f64; 2]; 4]>,
    wires: [Vec<ss::OriginalSsRootWire>; 2],
    known: [Vec<Known>; 2],
}
fn rectangle_disjoint(a: [[f64; 2]; 2], b: [[f64; 2]; 2]) -> bool {
    (0..2).any(|k| a[k][1] < b[k][0] || b[k][1] < a[k][0])
}
impl Domains {
    pub fn new(
        boundary: &QualifiedPairedRootBoundary,
        limits: RootBoundaryVolumeLimits,
        out: &mut RootBoundaryVolumeReport,
    ) -> Result<Option<Self>> {
        let t = boundary.model.topology();
        if t.edges
            .len()
            .checked_mul(limits.arc_subdivisions)
            .is_none_or(|n| n > limits.band_queries)
        {
            return Ok(None);
        }
        let mut bands = Vec::new();
        let mut indices = Vec::new();
        for edge in &t.edges {
            let MixedEdge::Root(edge) = &edge.curve else {
                return Err(unproved("Volume domain contains an unowned rational rim"));
            };
            let d = edge.parameter_bounds();
            let natural = edge.curve().parameter_domain();
            let range = [
                clock(&d[0], true)?.max(natural[0]),
                clock(&d[1], false)?.min(natural[1]),
            ];
            let mut breaks: Vec<_> = (0..=limits.arc_subdivisions)
                .map(|i| {
                    range[0] + (range[1] - range[0]) * (i as f64 / limits.arc_subdivisions as f64)
                })
                .collect();
            breaks[0] = range[0];
            breaks[limits.arc_subdivisions] = range[1];
            let start = bands.len();
            for d in breaks.windows(2) {
                if !(range[0] <= d[0] && d[0] < d[1] && d[1] <= range[1]) {
                    return Err(unproved("Root band subdivision is not representable"));
                }
                out.band_queries += 1;
                bands.push(edge.curve().enclose([d[0], d[1]], limits.geometry)?.uv);
            }
            indices.push(start..bands.len());
        }
        let mut wires: [Vec<ss::OriginalSsRootWire>; 2] = [Vec::new(), Vec::new()];
        let mut roots = [[[f64::INFINITY, f64::NEG_INFINITY]; 2]; 2];
        for support in 0..2 {
            let face = &t.faces[support];
            for &l in &std::iter::once(face.outer)
                .chain(face.holes.iter().copied())
                .collect::<Vec<_>>()
            {
                let mut edges = Vec::new();
                for c in &t.loops[l].coedges {
                    let MixedEdge::Root(e) = &t.edges[c.edge].curve else {
                        return Err(unproved("Foreign volume boundary edge"));
                    };
                    edges.push(if c.reversed {
                        ss::OriginalSsRootEdge::new(e.end().clone(), e.start().clone())?
                    } else {
                        e.clone()
                    });
                    if l == face.outer {
                        for i in indices[c.edge].clone() {
                            for k in 0..2 {
                                roots[support][k][0] =
                                    roots[support][k][0].min(bands[i][2 * support + k][0]);
                                roots[support][k][1] =
                                    roots[support][k][1].max(bands[i][2 * support + k][1]);
                            }
                        }
                    }
                }
                wires[support].push(ss::OriginalSsRootWire::new(edges)?);
            }
        }
        Ok(Some(Self {
            roots,
            bands,
            wires,
            known: [Vec::new(), Vec::new()],
        }))
    }
    pub fn classify(
        &mut self,
        support: usize,
        rectangle: [[f64; 2]; 2],
        limits: RootBoundaryVolumeLimits,
        out: &mut RootBoundaryVolumeReport,
    ) -> Result<Option<bool>> {
        if self
            .bands
            .iter()
            .any(|b| !rectangle_disjoint(rectangle, [b[2 * support], b[2 * support + 1]]))
        {
            return Ok(None);
        }
        let via = self.known[support]
            .iter()
            .position(|k| !rectangle_disjoint(rectangle, k.rectangle));
        let inside = if let Some(parent) = via {
            out.propagations += 1;
            self.known[support][parent].inside
        } else {
            let point = rectangle.map(|d| (d[0] * 0.5 + d[1] * 0.5).max(d[0]).min(d[1]));
            let mut inside = true;
            for (i, wire) in self.wires[support].iter().enumerate() {
                if out.winding_queries == limits.winding_queries {
                    return Ok(None);
                }
                out.winding_queries += 1;
                let Ok(w) = ss::qualify_original_ss_winding(wire, support, point, limits.winding)
                else {
                    return Ok(None);
                };
                let w = w.winding();
                if i == 0 {
                    if w == 0 {
                        inside = false;
                        break;
                    }
                    if w != 1 {
                        return Ok(None);
                    }
                } else {
                    if w == -1 {
                        inside = false;
                        break;
                    }
                    if w != 0 {
                        return Ok(None);
                    }
                }
            }
            out.seeds += 1;
            inside
        };
        self.known[support].push(Known {
            rectangle,
            inside,
            via,
        });
        Ok(Some(inside))
    }
    pub fn propagation_links(&self) -> usize {
        self.known
            .iter()
            .flatten()
            .filter(|k| k.via.is_some())
            .count()
    }
}
