use super::*;
#[derive(Clone, Copy)]
struct Known {
    rectangle: [[f64; 2]; 2],
    inside: bool,
    // None is a native winding seed; Some is an intersecting root-free cell.
    via: Option<usize>,
}
pub(super) struct Domains {
    pub roots: Vec<[[f64; 2]; 2]>,
    bands: Vec<[[f64; 2]; 4]>,
    faces: Vec<FluxDomain>,
}

struct FluxDomain {
    support: usize,
    root_outer: bool,
    wires: Vec<ss::OriginalSsRootWire>,
    known: Vec<Known>,
    rational: Option<crate::face_domain::FaceDomain>,
    rational_holes: Vec<nurbs_core::trim_domain::TrimDomain>,
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
        let faces = t.shells[0].faces.as_slice().try_into()
            .map_err(|_| unproved("Expected two paired root faces"))?;
        Self::from_faces(t, faces, limits, out)
    }

    pub(super) fn from_faces(
        t: &MixedTopology,
        faces: &[brep_topology::FaceUse; 2],
        limits: RootBoundaryVolumeLimits,
        out: &mut RootBoundaryVolumeReport,
    ) -> Result<Option<Self>> {
        Self::from_shell(t, faces, None, limits, out)
    }

    pub(super) fn from_shell(
        t: &MixedTopology,
        faces: &[brep_topology::FaceUse],
        rational_tolerance: Option<f64>,
        limits: RootBoundaryVolumeLimits,
        out: &mut RootBoundaryVolumeReport,
    ) -> Result<Option<Self>> {
        let mut used_edges = std::collections::BTreeSet::new();
        for use_ in faces {
            let face = &t.faces[use_.face];
            for l in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                for c in &t.loops[l].coedges {
                    if matches!(&c.pcurve, MixedPcurve::Root(_)) {
                        used_edges.insert(c.edge);
                    }
                }
            }
        }
        if used_edges.len().checked_mul(limits.arc_subdivisions)
            .is_none_or(|n| n > limits.band_queries)
        {
            return Ok(None);
        }
        let mut bands = Vec::new();
        let mut indices = std::collections::BTreeMap::new();
        for id in used_edges {
            let MixedEdge::Root(edge) = &t.edges[id].curve else {
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
            indices.insert(id, start..bands.len());
        }
        let mut roots = Vec::with_capacity(faces.len());
        let mut domains = Vec::with_capacity(faces.len());
        for use_ in faces {
            let face = &t.faces[use_.face];
            let mut root_bounds = [[f64::INFINITY, f64::NEG_INFINITY]; 2];
            let mut support = None;
            let mut root_outer = false;
            let mut wires = Vec::new();
            let mut rational = Vec::new();
            for l in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                let wire = &t.loops[l].coedges;
                if matches!(wire[0].pcurve, MixedPcurve::Rational(_)) {
                    let curves = wire.iter().map(|c| match &c.pcurve {
                        MixedPcurve::Rational(curve) => Ok(curve.clone()),
                        _ => Err(unproved("Flux loop mixes rational and SS edges")),
                    }).collect::<Result<Vec<_>>>()?;
                    rational.push(curves);
                    continue;
                }
                root_outer |= l == face.outer;
                let mut edges = Vec::new();
                for c in wire {
                    let (MixedEdge::Root(e), MixedPcurve::Root(p)) = (&t.edges[c.edge].curve, &c.pcurve) else {
                        return Err(unproved("Flux loop mixes SS and rational edges"));
                    };
                    if support.replace(p.support).is_some_and(|old| old != p.support) {
                        return Err(unproved("Flux face has inconsistent SS supports"));
                    }
                    edges.push(if c.reversed {
                        ss::OriginalSsRootEdge::new(e.end().clone(), e.start().clone())?
                    } else {
                        e.clone()
                    });
                    if l == face.outer {
                        for i in indices[&c.edge].clone() {
                            for (k, range) in root_bounds.iter_mut().enumerate() {
                                let uv = bands[i][2 * p.support + k];
                                range[0] = range[0].min(uv[0]);
                                range[1] = range[1].max(uv[1]);
                            }
                        }
                    }
                }
                wires.push(ss::OriginalSsRootWire::new(edges)?);
            }
            let surface = &face.surface;
            let mut rational_holes = Vec::new();
            let rational = if rational.is_empty() {
                None
            } else {
                let tolerance = rational_tolerance
                    .ok_or_else(|| unproved("Root flux domain contains a rational rim"))?;
                let mut loops = rational.into_iter();
                let outer = if root_outer { None } else { loops.next() };
                for hole in loops {
                    rational_holes.push(nurbs_core::trim_domain::TrimDomain::new(&[hole], tolerance)?);
                }
                outer.map(|outer| {
                    crate::face_domain::FaceDomain::from_pcurves(surface, &[outer], tolerance)
                }).transpose()?
            };
            roots.push(if root_outer {
                root_bounds
            } else {
                [
                    [surface.knots_u[surface.degree_u], surface.knots_u[surface.control_points.len()]],
                    [surface.knots_v[surface.degree_v], surface.knots_v[surface.control_points[0].len()]],
                ]
            });
            domains.push(FluxDomain {
                support: support.unwrap_or(0),
                root_outer,
                wires,
                known: Vec::new(),
                rational,
                rational_holes,
            });
        }
        Ok(Some(Self { roots, bands, faces: domains }))
    }

    pub fn classify(
        &mut self,
        face: usize,
        rectangle: [[f64; 2]; 2],
        limits: RootBoundaryVolumeLimits,
        out: &mut RootBoundaryVolumeReport,
    ) -> Result<Option<bool>> {
        let domain = &mut self.faces[face];
        let rational = domain.classify_rational(rectangle)?;
        if rational != Some(true) {
            return Ok(rational);
        }
        if domain.wires.is_empty() {
            return Ok(Some(true));
        }
        let support = domain.support;
        if self
            .bands
            .iter()
            .any(|b| !rectangle_disjoint(rectangle, [b[2 * support], b[2 * support + 1]]))
        {
            return Ok(None);
        }
        let via = domain.known
            .iter()
            .position(|k| !rectangle_disjoint(rectangle, k.rectangle));
        let inside = if let Some(parent) = via {
            out.propagations += 1;
            domain.known[parent].inside
        } else {
            let point = rectangle.map(|d| (d[0] * 0.5 + d[1] * 0.5).max(d[0]).min(d[1]));
            let mut inside = true;
            for (i, wire) in domain.wires.iter().enumerate() {
                if out.winding_queries == limits.winding_queries {
                    return Ok(None);
                }
                out.winding_queries += 1;
                let Ok(w) = ss::qualify_original_ss_winding(wire, support, point, limits.winding)
                else {
                    return Ok(None);
                };
                let w = w.winding();
                if domain.root_outer && i == 0 {
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
        domain.known.push(Known {
            rectangle,
            inside,
            via,
        });
        Ok(Some(inside))
    }
    /// A direct whole-box winding proof can resolve conservative band overlap.
    /// Unlike a witness alone, this certifies the ENTIRE query rectangle.
    pub fn classify_box(
        &mut self,
        face: usize,
        rectangle: [[f64; 2]; 2],
        limits: RootBoundaryVolumeLimits,
        out: &mut RootBoundaryVolumeReport,
    ) -> Result<Option<bool>> {
        let domain = &mut self.faces[face];
        let rational = domain.classify_rational(rectangle)?;
        if rational != Some(true) {
            return Ok(rational);
        }
        let support = domain.support;
        if let Some(parent) = domain.known.iter().position(|k| {
            (0..2).all(|a| {
                k.rectangle[a][0] <= rectangle[a][0] && rectangle[a][1] <= k.rectangle[a][1]
            })
        }) {
            let inside = domain.known[parent].inside;
            domain.known.push(Known {
                rectangle,
                inside,
                via: Some(parent),
            });
            out.propagations += 1;
            return Ok(Some(inside));
        }
        let mut inside = true;
        for (i, wire) in domain.wires.iter().enumerate() {
            if out.winding_queries == limits.winding_queries {
                return Ok(None);
            }
            out.winding_queries += 1;
            let Ok(proof) = wire.winding_box(support, rectangle, limits.winding) else {
                return Ok(None);
            };
            let w = proof.winding();
            if domain.root_outer && i == 0 {
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
        domain.known.push(Known {
            rectangle,
            inside,
            via: None,
        });
        out.seeds += 1;
        Ok(Some(inside))
    }
    pub fn propagation_links(&self) -> usize {
        self.faces
            .iter()
            .flat_map(|face| &face.known)
            .filter(|k| k.via.is_some())
            .count()
    }
}


impl FluxDomain {
    fn classify_rational(&self, rectangle: [[f64; 2]; 2]) -> Result<Option<bool>> {
        use nurbs_core::trim_domain::Location;
        if let Some(rational) = &self.rational {
            if !rational.contains_closed_rectangle(rectangle) {
                match rational.classify(rectangle, 100000)?.location {
                    Location::Inside => (),
                    Location::Outside => return Ok(Some(false)),
                    _ => return Ok(None),
                }
            }
        }
        // Each authored hole is a standalone winding domain. Only a box
        // proved outside every hole can use the retained SS-domain bound.
        let mut unresolved_hole = false;
        for hole in &self.rational_holes {
            match hole.classify(rectangle, 100000)?.location {
                Location::Inside => return Ok(Some(false)),
                Location::Outside => (),
                _ => unresolved_hole = true,
            }
        }
        if unresolved_hole {
            return Ok(None);
        }
        Ok(Some(true))
    }
}
