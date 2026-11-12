//! Cropped original boundary contact with a whole source graph side.
//! Plane support confines one source to a natural curve. Exact homogeneous
//! restriction and strict scalar order clip it without reminting surfaces.
use crate::{
    source_boundary_fragment::{Endpoint, Fragment},
    source_contour_proposal::SourceRegion,
    source_paired_projection_contact as paired,
    source_seam_tangency::Carrier,
    source_shell_incidence::Shell,
};
use cad_predicates::{
    AuthoredScalar, Limits, PredicateContext, Sign, SourceArena, ToleranceContext,
};
use nurbs_core::{Error, Result};
pub struct Certificate<E> {
    faces: [usize; 2],
    regions: [SourceRegion; 2],
    curve_slot: usize,
    fiber: crate::source_pole_plane_image::Certificate,
    graph_side: (usize, bool),
    restriction: [f64; 2],
    axes: [usize; 2],
    edges: Vec<E>,
}
impl<E> Certificate<E> {
    pub fn faces(&self) -> [usize; 2] {
        self.faces
    }
    pub fn regions(&self) -> &[SourceRegion; 2] {
        &self.regions
    }
    pub fn curve_slot(&self) -> usize {
        self.curve_slot
    }
    pub fn fiber(&self) -> &crate::source_pole_plane_image::Certificate {
        &self.fiber
    }
    pub fn graph_side(&self) -> (usize, bool) {
        self.graph_side
    }
    pub fn restriction(&self) -> [f64; 2] {
        self.restriction
    }
    pub fn axes(&self) -> [usize; 2] {
        self.axes
    }
    pub fn edges(&self) -> &[E] {
        &self.edges
    }
}
pub struct Report<E> {
    pub certificate: Option<Certificate<E>>,
    pub exact_work: u64,
    pub driver_cells: usize,
    pub reason: &'static str,
}
fn side(f: &Fragment) -> Option<((usize, bool), [f64; 2])> {
    let c = f.curve();
    if c.degree != 1
        || c.control_points.len() != 2
        || c.periodic
        || c.knots.len() != 4
        || c.control_points.iter().any(|p| p.len() != 2)
        || c.weights.iter().any(|w| *w <= 0.)
    {
        return None;
    }
    let domain = c.domain();
    if c.knots[..2].iter().any(|t| *t != domain[0]) || c.knots[2..].iter().any(|t| *t != domain[1])
    {
        return None;
    }
    let ends = f.endpoints();
    let mut ids = [0; 2];
    for i in 0..2 {
        let Endpoint::Parameter(t) = ends[i] else {
            return None;
        };
        ids[i] = domain.iter().position(|v| *v == t)?;
    }
    if ids[0] == ids[1] {
        return None;
    }
    let a = &c.control_points[ids[0]];
    let b = &c.control_points[ids[1]];
    let s = f.surface();
    let domains = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    for axis in 0..2 {
        if a[axis] == b[axis] {
            if let Some(end) = domains[axis].iter().position(|v| *v == a[axis]) {
                let ends = [a[1 - axis], b[1 - axis]];
                if ends[0] != ends[1]
                    && ends
                        .iter()
                        .all(|v| *v >= domains[1 - axis][0] && *v <= domains[1 - axis][1])
                {
                    return Some(((axis, end == 1), ends));
                }
            }
        }
    }
    None
}
pub fn certify<E: Carrier>(
    shell: &Shell<E>,
    faces: [usize; 2],
    max_work: u64,
    max_driver: usize,
) -> Result<Report<E>> {
    let regions = shell
        .regions()
        .ok_or_else(|| Error::new("BREP_SOURCE_CURVED_CONTACT", "Original regions required"))?;
    if faces[0] == faces[1]
        || faces.iter().any(|f| *f >= regions.len())
        || !(1..=100000000).contains(&max_work)
        || max_driver > 100000
    {
        return Err(Error::new(
            "BREP_SOURCE_CURVED_CONTACT",
            "Choose bounded original pair work",
        ));
    }
    let mut out = Report {
        certificate: None,
        exact_work: 0,
        driver_cells: 0,
        reason: "source-curved-boundary-contact-unproven",
    };
    let shared = shell
        .uses()
        .iter()
        .enumerate()
        .filter(|(_, p)| p.iter().all(|a| faces.contains(&a.face)))
        .collect::<Vec<_>>();
    if shared.is_empty() || max_driver == 0 {
        return Ok(out);
    }
    let surfaces = faces.map(|f| regions[f].loops()[0][0].surface());
    if surfaces.iter().any(|s| !paired::clamped(s)) {
        return Ok(out);
    }
    for curve_slot in 0..2 {
        let graph_slot = 1 - curve_slot;
        let curve_face = faces[curve_slot];
        let graph_face = faces[graph_slot];
        let addresses = shared
            .iter()
            .map(|(_, p)| *p.iter().find(|a| a.face == graph_face).unwrap())
            .collect::<Vec<_>>();
        let mut census = paired::Report {
            certificate: None,
            exact_work: 0,
            driver_cells: 0,
            reason: "source-curved-boundary-driver-work-limit",
        };
        let candidates = paired::boundaries(
            shell,
            graph_face,
            &addresses,
            &mut census,
            max_driver - out.driver_cells,
        )?;
        out.driver_cells += census.driver_cells;
        let Some(candidates) = candidates else {
            return Ok(out);
        };
        if candidates.is_empty() {
            continue;
        }
        for (index, uses) in &shared {
            let ca = uses.iter().find(|a| a.face == curve_face).unwrap();
            let ga = uses.iter().find(|a| a.face == graph_face).unwrap();
            let cf = &regions[curve_face].loops()[ca.wire][ca.edge];
            let gf = &regions[graph_face].loops()[ga.wire][ga.edge];
            let Some((curve_side, restriction)) = side(cf) else {
                continue;
            };
            let Some((graph_side, ends)) = side(gf) else {
                continue;
            };
            if !candidates.contains(&graph_side) {
                continue;
            }
            let g = surfaces[graph_slot];
            let gd = if graph_side.0 == 0 {
                [g.knots_v[g.degree_v], g.knots_v[g.control_points[0].len()]]
            } else {
                [g.knots_u[g.degree_u], g.knots_u[g.control_points.len()]]
            };
            if !((ends == gd) || (ends == [gd[1], gd[0]])) {
                continue;
            }
            let c = shell.edges()[*index].world();
            let mut distinct = Vec::new();
            for p in &c.control_points {
                if !distinct.contains(&p) {
                    distinct.push(p);
                }
                if distinct.len() == 3 {
                    break;
                }
            }
            if distinct.len() != 3 {
                continue;
            }
            let plane = std::array::from_fn(|i| std::array::from_fn(|k| distinct[i][k]));
            if !crate::source_allowed_contact::independent(
                plane[0],
                plane[1],
                plane[2],
                &mut out.exact_work,
                max_work,
            )? {
                continue;
            }
            if out.exact_work == max_work {
                return Ok(out);
            }
            let r = crate::source_pole_plane_image::certify(
                surfaces[curve_slot],
                plane,
                curve_side.0,
                curve_side.1,
                max_work - out.exact_work,
            )?;
            out.exact_work += r.exact_work;
            let Some(fiber) = r.certificate else {
                continue;
            };
            if !fiber.poles().is_empty() {
                continue;
            }
            let mut supported = true;
            for p in g.control_points.iter().flatten() {
                let Some(sign) = crate::source_allowed_contact::orient(
                    &[plane[0], plane[1], plane[2], [p[0], p[1], p[2]]],
                    None,
                    &mut out.exact_work,
                    max_work,
                )?
                else {
                    return Ok(out);
                };
                if sign != Sign::Zero && sign == fiber.side() {
                    supported = false;
                    break;
                }
            }
            if !supported {
                continue;
            }
            let s = surfaces[curve_slot];
            let span = if curve_side.0 == 0 {
                [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]]
            } else {
                [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]]
            };
            let ordered = [surfaces[curve_slot], g];
            let mut values = Vec::new();
            for s in ordered {
                for (i, row) in s.control_points.iter().enumerate() {
                    for (j, p) in row.iter().enumerate() {
                        values.extend([p[0], p[1], p[2], s.weights[i][j]]);
                    }
                }
            }
            let offset = values.len();
            values.extend(span);
            values.extend(restriction);
            let arena = SourceArena::authored(
                "source-original-subarc-graph",
                1,
                values
                    .into_iter()
                    .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                    .collect(),
            )
            .map_err(|_| Error::new("BREP_SOURCE_CURVED_CONTACT", "Invalid original leaves"))?;
            let mut at = 0;
            let refs = ordered.map(|s| {
                s.control_points
                    .iter()
                    .map(|row| {
                        row.iter()
                            .map(|_| {
                                std::array::from_fn(|_| {
                                    let r = arena.leaf(at).unwrap();
                                    at += 1;
                                    r
                                })
                            })
                            .collect()
                    })
                    .collect::<Vec<_>>()
            });
            if out.exact_work == max_work {
                return Ok(out);
            }
            let before = out.exact_work;
            let tol = ToleranceContext::default_valid();
            let mut ctx = PredicateContext::new(
                &arena,
                &tol,
                Limits {
                    max_work: (max_work - before).min(cad_predicates::MAX_WORK),
                    ..Limits::default()
                },
                None,
            );
            for reverse in [false, true] {
                for axes in [[0, 1], [0, 2], [1, 0], [1, 2], [2, 0], [2, 1]] {
                    let r = cad_predicates::binary_curve_graph_projection(
                        &mut ctx,
                        [
                            cad_predicates::ProjectionSource {
                                net: &refs[0],
                                transverse: curve_side.0,
                                upper: curve_side.1,
                            },
                            cad_predicates::ProjectionSource {
                                net: &refs[1],
                                transverse: graph_side.0,
                                upper: graph_side.1,
                            },
                        ],
                        reverse,
                        axes,
                        [arena.leaf(offset).unwrap(), arena.leaf(offset + 1).unwrap()],
                        [
                            arena.leaf(offset + 2).unwrap(),
                            arena.leaf(offset + 3).unwrap(),
                        ],
                    )
                    .map_err(|_| {
                        Error::new(
                            "BREP_SOURCE_CURVED_CONTACT",
                            "Invalid exact source projection query",
                        )
                    })?;
                    out.exact_work = before + ctx.work_used();
                    if r.proven {
                        out.certificate = Some(Certificate {
                            faces,
                            regions: [regions[faces[0]].clone(), regions[faces[1]].clone()],
                            curve_slot,
                            fiber,
                            graph_side,
                            restriction,
                            axes,
                            edges: shared
                                .iter()
                                .map(|(i, _)| shell.edges()[*i].clone())
                                .collect(),
                        });
                        out.reason = "source-original-subarc-graph-contact-qualified";
                        return Ok(out);
                    }
                    if out.exact_work == max_work {
                        return Ok(out);
                    }
                }
            }
        }
    }
    Ok(out)
}
