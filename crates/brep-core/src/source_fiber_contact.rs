//! Sufficient allowed-contact proof on a natural source chart boundary.
//! Original restrictions and exact shared ownership are retained unchanged.
use crate::{
    source_contour_proposal::SourceRegion,
    source_fiber_boundary::{self, Locus},
    source_plane_fiber,
    source_shared_edge::SharedEdge,
    source_shell_incidence::{Address, Shell},
};
use nurbs_core::{Error, Result};
pub struct Certificate {
    faces: [usize; 2],
    regions: [SourceRegion; 2],
    edges: Vec<SharedEdge>,
    fiber: source_plane_fiber::Certificate,
}
impl Certificate {
    pub fn faces(&self) -> [usize; 2] {
        self.faces
    }
    pub fn regions(&self) -> &[SourceRegion; 2] {
        &self.regions
    }
    pub fn edges(&self) -> &[SharedEdge] {
        &self.edges
    }
    pub fn fiber(&self) -> &source_plane_fiber::Certificate {
        &self.fiber
    }
}
pub struct Report {
    pub certificate: Option<Certificate>,
    pub exact_work: u64,
    pub spans: usize,
    pub driver_cells: usize,
    pub linear_cells: usize,
    pub uncertain_boundary: Option<Address>,
    pub reason: &'static str,
}
/// First face must be planar. The second meets its plane only on a natural
/// chart edge. Every retained boundary fragment on that edge must be paired
/// with the first face; every possible endpoint contact must join such an edge.
/// Thus every possible material contact has exact shared source ownership.
pub fn certify(
    shell: &Shell,
    faces: [usize; 2],
    max_work: u64,
    max_spans: usize,
    max_driver_cells: usize,
) -> Result<Report> {
    certify_with_linear_chart(shell, faces, max_work, max_spans, max_driver_cells, 0)
}
/// Optional fresh oblique chart proof has a separate globally bounded budget.
pub fn certify_with_linear_chart(
    shell: &Shell,
    faces: [usize; 2],
    max_work: u64,
    max_spans: usize,
    max_driver_cells: usize,
    max_linear_cells: usize,
) -> Result<Report> {
    if faces[0] == faces[1]
        || faces.iter().any(|&f| f >= shell.faces().len())
        || !(1..=100_000_000).contains(&max_work)
        || !(1..=100000).contains(&max_spans)
        || !(1..=100000).contains(&max_driver_cells)
        || max_linear_cells > 100000
    {
        return Err(Error::new(
            "BREP_SOURCE_FIBER_CONTACT",
            "Choose distinct faces and bounded work",
        ));
    }
    let regions = shell.regions().ok_or_else(|| {
        Error::new(
            "BREP_SOURCE_FIBER_CONTACT",
            "Qualified material regions are required",
        )
    })?;
    let mut out = Report {
        certificate: None,
        exact_work: 0,
        spans: 0,
        driver_cells: 0,
        linear_cells: 0,
        uncertain_boundary: None,
        reason: "source-fiber-chart-unproven",
    };
    for &f in &faces {
        if out.spans == max_spans {
            return Ok(out);
        }
        let r = nurbs_core::surface_injectivity::certify_contraction(
            regions[f].loops()[0][0].surface(),
            max_spans - out.spans,
        )?;
        out.spans += r.spans;
        if !r.proven {
            if out.linear_cells == max_linear_cells {
                return Ok(out);
            }
            let linear = nurbs_core::surface_linear_monotonicity::inspect_candidate(
                regions[f].loops()[0][0].surface(),
                max_linear_cells - out.linear_cells,
            )?;
            out.linear_cells += linear.cells;
            if !linear.certified {
                return Ok(out);
            }
        }
    }
    out.reason = "source-fiber-planarity-unproven";
    let Some(plane) = crate::source_allowed_contact::plane(
        regions[faces[0]].loops()[0][0].surface(),
        &mut out.exact_work,
        max_work,
    )?
    else {
        return Ok(out);
    };
    let s = regions[faces[1]].loops()[0][0].surface();
    out.reason = "source-fiber-support-unproven";
    for axis in 0..2 {
        for upper in [false, true] {
            if out.exact_work == max_work {
                return Ok(out);
            }
            let r = source_plane_fiber::certify(s, plane, axis, upper, max_work - out.exact_work)?;
            out.exact_work += r.exact_work;
            let Some(fiber) = r.certificate else {
                continue;
            };
            let shared = shell
                .uses()
                .iter()
                .enumerate()
                .filter(|(_, uses)| uses.iter().all(|a| faces.contains(&a.face)))
                .collect::<Vec<_>>();
            let addresses = shared
                .iter()
                .map(|(_, uses)| *uses.iter().find(|a| a.face == faces[1]).unwrap())
                .collect::<Vec<_>>();
            out.reason = "source-fiber-ownership-unproven";
            for (wire, edges) in regions[faces[1]].loops().iter().enumerate() {
                for (edge, fragment) in edges.iter().enumerate() {
                    let address = Address {
                        face: faces[1],
                        wire,
                        edge,
                    };
                    out.uncertain_boundary = Some(address);
                    if out.driver_cells == max_driver_cells {
                        return Ok(out);
                    }
                    let report = source_fiber_boundary::inspect(
                        fragment,
                        &fiber,
                        max_driver_cells - out.driver_cells,
                    )?;
                    out.driver_cells += report.driver_cells;
                    let vertex_owned = |end: usize| {
                        let v = (edge + end) % edges.len();
                        addresses.iter().any(|a| {
                            a.wire == wire && (a.edge == v || (a.edge + 1) % edges.len() == v)
                        })
                    };
                    match report.locus {
                        Locus::EntireFragment if addresses.contains(&address) => {}
                        Locus::Endpoints(ends) if (0..2).all(|i| !ends[i] || vertex_owned(i)) => {}
                        Locus::Away => {}
                        _ => return Ok(out),
                    }
                }
            }
            out.uncertain_boundary = None;
            out.certificate = Some(Certificate {
                faces,
                regions: [regions[faces[0]].clone(), regions[faces[1]].clone()],
                edges: shared
                    .iter()
                    .map(|(i, _)| shell.edges()[*i].clone())
                    .collect(),
                fiber,
            });
            out.reason = "source-fiber-allowed-contact-proven";
            return Ok(out);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        source_contour_proposal::qualify_original_region,
        source_shell_incidence::{self, Pair},
        trimmed_face_recipe::{Boundary, Limits},
    };
    use nurbs_core::{curve::Curve, surface::Surface};
    fn arc(z: f64, swapped: bool, uv: bool) -> Curve {
        let points = [[1., 0.], [1., 1.], [0., 1.]];
        Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: points
                .iter()
                .map(|p| {
                    let [x, y] = if swapped { [p[1], p[0]] } else { *p };
                    if uv {
                        vec![x, y]
                    } else {
                        vec![x, y, z]
                    }
                })
                .collect(),
            weights: vec![1., 0.5_f64.sqrt(), 1.],
            periodic: false,
        }
    }
    fn reversed(c: &Curve) -> Curve {
        let mut r = c.clone();
        r.control_points.reverse();
        r.weights.reverse();
        r
    }
    fn plane(map: impl Fn(f64, f64) -> Vec<f64>) -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..2)
                .map(|u| (0..2).map(|v| map(u as f64, v as f64)).collect())
                .collect(),
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    fn wedge() -> Shell {
        let bottom = plane(|u, v| vec![u, v, 0.]);
        let top = plane(|u, v| vec![v, u, 1.]);
        let rim = arc(0., false, false);
        let side = Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: rim.knots.clone(),
            control_points: vec![
                rim.control_points.clone(),
                arc(1., false, false).control_points,
            ],
            weights: vec![rim.weights.clone(); 2],
            periodic_u: false,
            periodic_v: false,
        };
        let surfaces = [
            bottom,
            top,
            side,
            plane(|u, v| vec![0., u, v]),
            plane(|u, v| vec![u, 0., 1. - v]),
        ];
        let context = cad_predicates::ToleranceContext::default_valid();
        let mut regions = Vec::new();
        let mut boundaries = Vec::new();
        for (f, s) in surfaces.iter().enumerate() {
            let pcurves = if f < 2 {
                vec![
                    arc(0., false, true),
                    Curve::from_polyline(vec![vec![0., 1.], vec![0., 0.]]).unwrap(),
                    Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap(),
                ]
            } else {
                let points = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
                (0..4)
                    .map(|i| {
                        Curve::from_polyline(vec![points[i].to_vec(), points[(i + 1) % 4].to_vec()])
                            .unwrap()
                    })
                    .collect()
            };
            let wire = pcurves
                .into_iter()
                .enumerate()
                .map(|(i, pc)| {
                    let world = if f < 2 && i == 0 {
                        arc(f as f64, f == 1, false)
                    } else if f == 2 && i == 1 {
                        arc(1., false, false)
                    } else if f == 2 && i == 3 {
                        reversed(&arc(0., false, false))
                    } else {
                        let a = &pc.control_points[0];
                        let b = pc.control_points.last().unwrap();
                        Curve::from_polyline(vec![
                            s.evaluate(a[0], a[1]).unwrap().point.to_vec(),
                            s.evaluate(b[0], b[1]).unwrap().point.to_vec(),
                        ])
                        .unwrap()
                    };
                    Boundary {
                        curve: world,
                        pcurve: pc,
                        reversed: false,
                    }
                })
                .collect::<Vec<_>>();
            let r = qualify_original_region(
                &context,
                s,
                &[wire.clone()],
                1e-8,
                Limits {
                    pairs: 10000,
                    region_cells: 10000,
                    domain_cells: 10000,
                    agreement_cells: 10000,
                },
            )
            .unwrap();
            assert!(r.region.is_some(), "face {} {}", f, r.reason);
            regions.push(r.region.unwrap());
            boundaries.push(wire);
        }
        let mut unmatched = Vec::<(Address, Curve)>::new();
        let mut pairs = Vec::new();
        for (face, wire) in boundaries.iter().enumerate() {
            for (edge, b) in wire.iter().enumerate() {
                let address = Address {
                    face,
                    wire: 0,
                    edge,
                };
                if let Some(i) = unmatched.iter().position(|(_, c)| reversed(c) == b.curve) {
                    let (old, world) = unmatched.remove(i);
                    pairs.push(Pair {
                        uses: [old, address],
                        world,
                        world_reversed: [false, true],
                        cutters: [None, None],
                    });
                } else {
                    unmatched.push((address, b.curve.clone()));
                }
            }
        }
        assert!(
            unmatched.is_empty(),
            "{} unpaired boundaries",
            unmatched.len()
        );
        let r = source_shell_incidence::assemble_regions(&regions, &pairs, 100_000_000).unwrap();
        assert!(r.shell.is_some(), "{} {:?}", r.reason, r.uncertain_pair);
        r.shell.unwrap()
    }
    fn curved_strip() -> (Vec<SourceRegion>, Vec<Pair>) {
        let base = arc(0., false, false);
        let surface = |shifted: bool, top: bool, side: bool, inner: bool| {
            let net = (0..2)
                .map(|u| {
                    (0..3)
                        .map(|v| {
                            let p = &base.control_points[if top { 2 - v } else { v }];
                            let shift = if side {
                                usize::from(shifted) as f64
                            } else {
                                u as f64
                            };
                            let z = if side {
                                if inner {
                                    1. - u as f64
                                } else {
                                    u as f64
                                }
                            } else {
                                usize::from(top) as f64
                            };
                            vec![p[0] + shift, p[1] + shift, z]
                        })
                        .collect()
                })
                .collect();
            Surface {
                degree_u: 1,
                degree_v: 2,
                knots_u: vec![0., 0., 1., 1.],
                knots_v: base.knots.clone(),
                control_points: net,
                weights: vec![base.weights.clone(); 2],
                periodic_u: false,
                periodic_v: false,
            }
        };
        let surfaces = [
            surface(false, false, false, false),
            surface(false, true, false, false),
            surface(false, false, true, true),
            surface(true, false, true, false),
            plane(|u, v| vec![1. + u, u, 1. - v]),
            plane(|u, v| vec![u, 1. + u, v]),
        ];
        let context = cad_predicates::ToleranceContext::default_valid();
        let points = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
        let mut regions = Vec::new();
        let mut unmatched = Vec::<(Address, Curve)>::new();
        let mut pairs = Vec::new();
        for (face, s) in surfaces.iter().enumerate() {
            let wire = (0..4)
                .map(|edge| {
                    let a = points[edge];
                    let b = points[(edge + 1) % 4];
                    let pc = Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap();
                    let curve = if face < 4 && (edge == 1 || edge == 3) {
                        let (shift, z) = match face {
                            0 => (if edge == 1 { 1. } else { 0. }, 0.),
                            1 => (if edge == 1 { 1. } else { 0. }, 1.),
                            2 => (0., if edge == 1 { 0. } else { 1. }),
                            _ => (1., if edge == 1 { 1. } else { 0. }),
                        };
                        let mut c = arc(z, (face == 1) != (edge == 3), false);
                        for p in &mut c.control_points {
                            p[0] += shift;
                            p[1] += shift;
                        }
                        c
                    } else {
                        Curve::from_polyline(vec![
                            s.evaluate(a[0], a[1]).unwrap().point.to_vec(),
                            s.evaluate(b[0], b[1]).unwrap().point.to_vec(),
                        ])
                        .unwrap()
                    };
                    Boundary {
                        curve,
                        pcurve: pc,
                        reversed: false,
                    }
                })
                .collect::<Vec<_>>();
            let r = qualify_original_region(
                &context,
                s,
                &[wire.clone()],
                1e-8,
                Limits {
                    pairs: 10000,
                    region_cells: 10000,
                    domain_cells: 10000,
                    agreement_cells: 10000,
                },
            )
            .unwrap();
            assert!(r.region.is_some(), "strip face {} {}", face, r.reason);
            regions.push(r.region.unwrap());
            for (edge, b) in wire.iter().enumerate() {
                let address = Address {
                    face,
                    wire: 0,
                    edge,
                };
                if let Some(i) = unmatched.iter().position(|(_, c)| reversed(c) == b.curve) {
                    let (old, world) = unmatched.remove(i);
                    pairs.push(Pair {
                        uses: [old, address],
                        world,
                        world_reversed: [false, true],
                        cutters: [None, None],
                    });
                } else {
                    unmatched.push((address, b.curve.clone()));
                }
            }
        }
        assert!(unmatched.is_empty());
        (regions, pairs)
    }
    #[test]
    fn root_partitioned_curved_rim_keeps_exact_original_definitions_and_ownership() {
        use crate::source_boundary_fragment::Role;
        let (mut regions, mut pairs) = curved_strip();
        let index = pairs
            .iter()
            .position(|p| p.uses.iter().all(|a| [0, 2].contains(&a.face)))
            .unwrap();
        let old = pairs.remove(index);
        let original = old.world.clone();
        for address in old.uses {
            let fragment = &regions[address.face].loops()[address.wire][address.edge];
            let cut = Curve::from_polyline(vec![vec![-0.25, 0.625], vec![1.25, 0.625]]).unwrap();
            let selector = if address.face == 0 {
                [[0.3, 0.45], [0.1, 0.25]]
            } else {
                [[0.55, 0.7], [0.75, 0.9]]
            };
            let r = crate::source_contact_point::qualify(
                fragment.surface(),
                fragment.curve(),
                &cut,
                selector,
                10000,
            )
            .unwrap();
            assert!(r.point.is_some(), "root face {} {}", address.face, r.reason);
            regions[address.face] = regions[address.face]
                .split_boundary(
                    address.wire,
                    address.edge,
                    &r.point.unwrap(),
                    Role::Boundary,
                )
                .unwrap();
        }
        for pair in &mut pairs {
            for address in &mut pair.uses {
                for split in old.uses {
                    if address.face == split.face
                        && address.wire == split.wire
                        && address.edge > split.edge
                    {
                        address.edge += 1;
                    }
                }
            }
        }
        let [a, b] = old.uses;
        pairs.push(Pair {
            uses: [
                a,
                Address {
                    edge: b.edge + 1,
                    ..b
                },
            ],
            world: original.clone(),
            world_reversed: old.world_reversed,
            cutters: [None, None],
        });
        pairs.push(Pair {
            uses: [
                Address {
                    edge: a.edge + 1,
                    ..a
                },
                b,
            ],
            world: original.clone(),
            world_reversed: old.world_reversed,
            cutters: [None, None],
        });
        let shell_report =
            source_shell_incidence::assemble_regions(&regions, &pairs, 100_000_000).unwrap();
        assert!(
            shell_report.shell.is_some(),
            "{} {:?}",
            shell_report.reason,
            shell_report.uncertain_pair
        );
        let shell = shell_report.shell.unwrap();
        let r = certify_with_linear_chart(&shell, [0, 2], 100_000_000, 1000, 10000, 10000).unwrap();
        assert!(
            r.certificate.is_some(),
            "{} {} linear cells",
            r.reason,
            r.linear_cells
        );
        let c = r.certificate.unwrap();
        assert_eq!(c.edges().len(), 2);
        assert!(c.edges().iter().all(|e| e.world() == &original));
        assert!(c
            .edges()
            .iter()
            .all(|e| e
                .uses()
                .iter()
                .any(|f| f.endpoints().iter().any(|x| matches!(
                    x,
                    crate::source_boundary_fragment::Endpoint::Crossing { .. }
                )))));
        assert_eq!(c.fiber().boundary(), (0, true));
        assert!(r.linear_cells > 0);
        let limited =
            certify_with_linear_chart(&shell, [0, 2], 100_000_000, 1000, 10000, 1).unwrap();
        assert!(limited.certificate.is_none());
        assert!(limited.linear_cells <= 1);
        let mut damaged = original.clone();
        for p in &mut damaged.control_points {
            p[2] += 1e-12;
        }
        for pair in &mut pairs {
            if pair.world == original {
                pair.world = damaged.clone();
            }
        }
        assert!(
            source_shell_incidence::assemble_regions(&regions, &pairs, 100_000_000)
                .unwrap()
                .shell
                .is_none()
        );
    }
    #[test]
    fn nonplanar_rational_rim_has_exact_pair_ownership_with_fresh_oblique_chart() {
        let shell = wedge();
        let r = certify_with_linear_chart(&shell, [0, 2], 100_000_000, 100, 10000, 10000).unwrap();
        assert!(
            r.certificate.is_some(),
            "{} {} linear cells",
            r.reason,
            r.linear_cells
        );
        assert!(r.linear_cells > 0);
        let c = r.certificate.unwrap();
        assert_eq!(c.edges().len(), 1);
        assert_eq!(c.edges()[0].world().degree, 2);
        assert_eq!(c.fiber().boundary(), (0, false));
        let blocked = certify(&shell, [0, 2], 100_000_000, 100, 10000).unwrap();
        assert!(blocked.certificate.is_none());
        let audit = crate::source_face_contacts::inspect_shell_with_boundary_fibers_and_chart_work(
            &shell,
            1e-8,
            crate::face_contacts::Limits {
                pairs: 10,
                cells: 10000,
                domain_cells: 10000,
                cells_per_pair: 1000,
                domain_cells_per_pair: 1000,
            },
            100_000_000,
            1000,
            10000,
            10000,
        )
        .unwrap();
        assert!(audit
            .pairs
            .iter()
            .any(|p| p.faces == [0, 2] && p.fiber.is_some()));
    }
}
