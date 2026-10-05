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
        let (regions, pairs) = wedge_regions(0.5_f64.sqrt());
        let r = source_shell_incidence::assemble_regions(&regions, &pairs, 100_000_000).unwrap();
        assert!(r.shell.is_some(), "{} {:?}", r.reason, r.uncertain_pair);
        r.shell.unwrap()
    }
    fn wedge_regions(weight: f64) -> (Vec<SourceRegion>, Vec<Pair>) {
        let arc = |z, swapped, uv| {
            let mut c = arc(z, swapped, uv);
            c.weights[1] = weight;
            c
        };
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
        (regions, pairs)
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
        for edge in c.edges() { crate::source_edge_restriction::assert_replay(edge); }
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
    fn closed_curved_wedge_owns_nonlinear_root_ends_through_recomputed_planes() {
        use crate::source_boundary_fragment::Role;
        use source_shell_incidence::RootPlanes;
        let (mut regions, mut pairs) = wedge_regions(1.);
        let index = pairs
            .iter()
            .position(|p| p.uses.iter().all(|a| [0, 2].contains(&a.face)))
            .unwrap();
        let old = pairs.remove(index);
        let original = old.world.clone();
        for address in old.uses {
            let fragment = &regions[address.face].loops()[address.wire][address.edge];
            let (cut, selector) = if address.face == 0 {
                (
                    Curve::from_polyline(vec![vec![0.5, 1.25], vec![0.5, -0.25]]).unwrap(),
                    [[0.65, 0.8], [0.15, 0.3]],
                )
            } else {
                (
                    Curve {
                        degree: 2,
                        knots: vec![0., 0., 0., 1., 1., 1.],
                        control_points: vec![
                            vec![0.4375, -0.25],
                            vec![0.8125, 0.5],
                            vec![-1.0625, 1.25],
                        ],
                        weights: vec![1.; 3],
                        periodic: false,
                    },
                    [[0.2, 0.4], [0.55, 0.75]],
                )
            };
            let r = crate::source_contact_point::qualify(
                fragment.surface(),
                fragment.curve(),
                &cut,
                selector,
                10000,
            )
            .unwrap();
            assert!(
                r.point.is_some(),
                "nonlinear face {} {}",
                address.face,
                r.reason
            );
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
        let first = pairs.len();
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
        let plane = [[0.5, 0., 0.], [0.5, 1., 0.], [1.5, 0., 1.]];
        let specs = [
            RootPlanes {
                pair: first,
                planes: [None, Some(plane)],
            },
            RootPlanes {
                pair: first + 1,
                planes: [Some(plane), None],
            },
        ];
        assert!(
            source_shell_incidence::assemble_regions(&regions, &pairs, 100_000_000)
                .unwrap()
                .shell
                .is_none()
        );
        let r = source_shell_incidence::assemble_regions_with_root_planes(
            &regions,
            &pairs,
            &specs,
            100_000_000,
            10000,
        )
        .unwrap();
        assert!(r.shell.is_some(), "{} {:?}", r.reason, r.uncertain_pair);
        assert!(r.driver_cells > 0 && r.driver_cells <= 10000);
        let shell = r.shell.unwrap();
        assert!(
            shell
                .edges()
                .iter()
                .filter(|e| e.world() == &original)
                .count()
                == 2
        );
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
        assert!(audit.all_pairs_qualified);
        assert_eq!(audit.pairs.len(), 10);
        assert!(audit
            .pairs
            .iter()
            .any(|p| p.faces == [0, 2] && p.fiber.as_ref().is_some_and(|c| c.edges().len() == 2)));
        assert!(source_shell_incidence::assemble_regions_with_root_planes(
            &regions,
            &pairs,
            &specs,
            100_000_000,
            0
        )
        .unwrap()
        .shell
        .is_none());
        assert!(source_shell_incidence::assemble_regions_with_root_planes(
            &regions,
            &pairs,
            &[
                RootPlanes {
                    pair: first,
                    planes: [None, Some(plane)]
                },
                RootPlanes {
                    pair: first,
                    planes: [None, Some(plane)]
                }
            ],
            100_000_000,
            10000
        )
        .is_err());
    }
    #[test]
    fn closed_curved_wedge_preserves_mixed_root_and_fixed_vertex_ownership() {
        mixed_wedge(false);
    }
    #[test]
    fn closed_curved_wedge_uses_exact_root_candidates_without_planes() {
        mixed_wedge(true);
    }
    fn mixed_wedge(exact_candidates: bool) {
        use crate::source_boundary_fragment::{Endpoint, Role};
        use source_shell_incidence::RootPlanes;
        let (mut regions, mut pairs) = wedge_regions(1.);
        let index = pairs
            .iter()
            .position(|p| p.uses.iter().all(|a| [0, 2].contains(&a.face)))
            .unwrap();
        let old = pairs.remove(index);
        let original = old.world.clone();
        let a = old.uses[0];
        let b = old.uses[1];
        assert_eq!([a.face, b.face], [0, 2]);
        let fragment = &regions[a.face].loops()[a.wire][a.edge];
        let cut = Curve::from_polyline(if exact_candidates {
            vec![vec![0.609375, 1.], vec![0.609375, 0.]]
        } else { vec![vec![0.609375, 1.25], vec![0.609375, -0.25]] }).unwrap();
        let point = crate::source_contact_point::qualify(
            fragment.surface(),
            fragment.curve(),
            &cut,
            [[0.6, 0.65], if exact_candidates {[0.12,0.16]} else {[0.24,0.28]}],
            10000,
        )
        .unwrap();
        assert!(point.point.is_some(), "{}", point.reason);
        regions[a.face] = regions[a.face]
            .split_boundary(a.wire, a.edge, &point.point.unwrap(), Role::Boundary)
            .unwrap();
        let mut displaced = regions.clone();
        displaced[b.face] = displaced[b.face]
            .split_boundary_parameter(b.wire, b.edge, 0.375 + 1e-12)
            .unwrap();
        assert!(regions[b.face]
            .split_boundary_parameter(b.wire, b.edge, 0.)
            .is_err());
        assert!(regions[b.face]
            .split_boundary_parameter(b.wire, b.edge, 1.)
            .is_err());
        regions[b.face] = regions[b.face]
            .split_boundary_parameter(b.wire, b.edge, 0.375)
            .unwrap();
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
        let first = pairs.len();
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
        let plane = [[0.609375, 0., 0.], [0.609375, 1., 0.], [0.609375, 0., 1.]];
        let specs = [
            RootPlanes {
                pair: first,
                planes: [None, Some(plane)],
            },
            RootPlanes {
                pair: first + 1,
                planes: [Some(plane), None],
            },
        ];
        let r = source_shell_incidence::assemble_regions_with_root_planes(
            &regions,
            &pairs,
            &specs,
            100_000_000,
            10000,
        )
        .unwrap();
        assert!(r.shell.is_some(), "{} {:?}", r.reason, r.uncertain_pair);
        let mut shell = r.shell.unwrap();
        if exact_candidates {
            let witnesses = [
                source_shell_incidence::RootCandidates { pair: first,
                    candidates: [[None,Some([0.625,0.140625])],[None,None]] },
                source_shell_incidence::RootCandidates { pair: first+1,
                    candidates: [[Some([0.625,0.140625]),None],[None,None]] },
            ];
            let admitted=source_shell_incidence::assemble_regions_with_endpoint_inputs(
                &regions,&pairs,&[],&[],&witnesses,100_000_000,0).unwrap();
            assert!(admitted.shell.is_some(), "{} {:?}", admitted.reason, admitted.uncertain_pair);
            shell=admitted.shell.unwrap();
            let duplicate = [
                source_shell_incidence::RootCandidates {pair:first,candidates:witnesses[0].candidates},
                source_shell_incidence::RootCandidates {pair:first,candidates:witnesses[0].candidates},
            ];
            assert!(source_shell_incidence::assemble_regions_with_endpoint_inputs(
                &regions,&pairs,&[],&[],&duplicate,100_000_000,0).is_err());
            let invalid = [source_shell_incidence::RootCandidates {
                pair:pairs.len(), candidates:witnesses[0].candidates}];
            assert!(source_shell_incidence::assemble_regions_with_endpoint_inputs(
                &regions,&pairs,&[],&[],&invalid,100_000_000,0).is_err());
            assert!(source_shell_incidence::assemble_regions_with_endpoint_inputs(
                &displaced,&pairs,&[],&[],&witnesses,100_000_000,0).unwrap().shell.is_none());
        }
        let common = shell
            .edges()
            .iter()
            .filter(|e| e.world() == &original)
            .collect::<Vec<_>>();
        assert_eq!(common.len(), 2);
        assert!(common.iter().all(|e| e
            .uses()
            .iter()
            .flat_map(|f| f.endpoints())
            .filter(|x| matches!(x, Endpoint::Crossing { .. }))
            .count()
            == 1));
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
        assert!(audit.all_pairs_qualified);
        assert_eq!(audit.pairs.len(), 10);
        assert!(source_shell_incidence::assemble_regions_with_root_planes(
            &displaced,
            &pairs,
            &specs,
            100_000_000,
            10000
        )
        .unwrap()
        .shell
        .is_none());
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
    #[test]
    fn extended_canonical_carriers_preserve_closed_source_shell() {
        let (regions, mut pairs) = wedge_regions(1.);
        let mut maps = Vec::new();
        for (i, pair) in pairs.iter_mut().enumerate() {
            if pair.world.degree != 1 { continue; }
            let a = pair.world.control_points[0].clone();
            let b = pair.world.control_points[1].clone();
            pair.world.control_points = vec![
                a.iter().zip(&b).map(|(a,b)| 2.*a-b).collect(),
                b.iter().zip(&a).map(|(b,a)| 2.*b-a).collect(),
            ];
            maps.push(source_shell_incidence::AffineMaps {
                pair: i,
                ranges: pair.world_reversed.map(|r| if r {
                    [[2.,3.],[1.,3.]]
                } else { [[1.,3.],[2.,3.]] }),
            });
        }
        assert!(!maps.is_empty());
        assert!(source_shell_incidence::assemble_regions(&regions, &pairs, 100_000_000).unwrap().shell.is_none());
        let result = source_shell_incidence::assemble_regions_with_maps(
            &regions, &pairs, &[], &maps, 100_000_000, 10000).unwrap();
        assert!(result.shell.is_some(), "{} {:?}", result.reason, result.uncertain_pair);
        let shell = result.shell.unwrap();
        for spec in &maps {
            assert_eq!(shell.edges()[spec.pair].ranges(), Some(spec.ranges));
            assert!(!shell.edges()[spec.pair].covers_complete_canonical_source());
        }
        let audit = crate::source_face_contacts::inspect_shell_with_boundary_fibers_and_chart_work(
            &shell, 1e-8, crate::face_contacts::Limits {
                pairs: 10, cells: 10000, domain_cells: 10000,
                cells_per_pair: 1000, domain_cells_per_pair: 1000,
            }, 100_000_000, 1000, 10000, 10000).unwrap();
        assert!(audit.all_pairs_qualified);
        assert_eq!(audit.pairs.len(), 10);
        maps[0].ranges[0][0] = [1.,4.];
        assert!(source_shell_incidence::assemble_regions_with_maps(
            &regions, &pairs, &[], &maps, 100_000_000, 10000).unwrap().shell.is_none());
        maps.push(source_shell_incidence::AffineMaps { pair: maps[0].pair, ranges: maps[0].ranges });
        assert!(source_shell_incidence::assemble_regions_with_maps(
            &regions, &pairs, &[], &maps, 100_000_000, 10000).is_err());
    }

    #[test]
    fn root_clipped_curved_wedge_closes_with_new_planar_cap() {
        use crate::source_boundary_fragment::{Endpoint, Role};
        use source_shell_incidence::{Address, AffineMaps, RootCandidates};
        let (original, old_pairs) = wedge_regions(1.);
        let context = cad_predicates::ToleranceContext::default_valid();
        let limits = Limits {
            pairs: 10000,
            region_cells: 10000,
            domain_cells: 10000,
            agreement_cells: 10000,
        };
        let mut wires = Vec::new();
        for (face, r) in original.iter().enumerate() {
            let wire = r.loops()[0]
                .iter()
                .enumerate()
                .map(|(edge, f)| {
                    let address = Address {
                        face,
                        wire: 0,
                        edge,
                    };
                    let p = old_pairs
                        .iter()
                        .find(|p| p.uses.contains(&address))
                        .unwrap();
                    let use_index = p.uses.iter().position(|a| *a == address).unwrap();
                    Boundary {
                        curve: if p.world_reversed[use_index] {
                            reversed(&p.world)
                        } else {
                            p.world.clone()
                        },
                        pcurve: f.curve().clone(),
                        reversed: false,
                    }
                })
                .collect::<Vec<_>>();
            wires.push(wire);
        }
        let k = 0.859375;
        let contacts = [
            Curve::from_polyline(vec![vec![1.25, k], vec![-0.75, k]]).unwrap(),
            Curve::from_polyline(vec![vec![k, -0.75], vec![k, 1.25]]).unwrap(),
            Curve::from_polyline(vec![vec![1.25, 0.625], vec![-0.75, 0.625]]).unwrap(),
            Curve::from_polyline(vec![vec![k, -0.75], vec![k, 1.25]]).unwrap(),
        ];
        let cut_edges = [(0, 1, 0), (2, 0, 1), (1, 3, 0), (0, 2, 1)];
        let mut regions = original.clone();
        for face in 0..4 {
            let (start, end, driver) = cut_edges[face];
            let r = crate::source_contour_proposal::qualify_linear_region(
                &context,
                original[face].loops()[0][0].surface(),
                &[wires[face].clone()],
                &contacts[face],
                0,
                start,
                end,
                1e-8,
                limits,
                10000,
                [1e-8; 2],
                32,
                10000,
                driver,
                10000,
                10000,
                10000,
            )
            .unwrap();
            assert!(
                r.region.is_some(),
                "face {} {} / {} kept {} removed {}",
                face,
                r.reason,
                r.interior.reason,
                r.kept_side_proven,
                r.removed_side_proven
            );
            if face < 2 {
                assert!(r.driver_cells > 1);
            }
            if face == 0 {
                let arc = r.region.as_ref().unwrap().loops()[0]
                    .iter()
                    .find(|f| f.curve().degree == 2)
                    .unwrap();
                assert!(
                    crate::source_halfplane_side::prove(arc, &contacts[0], 1, true, 1)
                        .unwrap()
                        .proven
                );
                assert!(
                    !crate::source_halfplane_side::prove(arc, &contacts[0], 1, false, 1)
                        .unwrap()
                        .proven
                );
                assert!(
                    !crate::source_halfplane_side::prove(arc, &contacts[0], 1, true, 0)
                        .unwrap()
                        .proven
                );
                let opposite = crate::source_boundary_fragment::Fragment::new(
                    arc.surface(),
                    arc.curve(),
                    arc.endpoints()[1].clone(),
                    arc.endpoints()[0].clone(),
                )
                .unwrap();
                assert!(
                    crate::source_halfplane_side::prove(&opposite, &contacts[0], 1, true, 1)
                        .unwrap()
                        .proven
                );
                assert!(
                    !crate::source_halfplane_side::prove(arc, &contacts[1], 1, true, 100)
                        .unwrap()
                        .proven
                );
            }
            regions[face] = r.region.unwrap();
        }
        let cap = plane(|u, v| vec![0.609375 * u, k, v]);
        let corners = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
        let cap_wire = (0..4)
            .map(|i| {
                let a = corners[i];
                let b = corners[(i + 1) % 4];
                Boundary {
                    curve: Curve::from_polyline(vec![
                        vec![0.609375 * a[0], k, a[1]],
                        vec![0.609375 * b[0], k, b[1]],
                    ])
                    .unwrap(),
                    pcurve: Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap(),
                    reversed: false,
                }
            })
            .collect::<Vec<_>>();
        regions.push(
            qualify_original_region(&context, &cap, &[cap_wire.clone()], 1e-8, limits)
                .unwrap()
                .region
                .unwrap(),
        );
        let contact_world = [
            Curve::from_polyline(vec![vec![1.25, k, 0.], vec![-0.75, k, 0.]]).unwrap(),
            Curve::from_polyline(vec![vec![-0.75, k, 1.], vec![1.25, k, 1.]]).unwrap(),
            Curve::from_polyline(vec![vec![0.609375, k, 1.25], vec![0.609375, k, -0.75]]).unwrap(),
            Curve::from_polyline(vec![vec![0., k, -0.75], vec![0., k, 1.25]]).unwrap(),
        ];
        let candidate = |face: usize, e: &Endpoint| -> Option<[f64; 2]> {
            let Endpoint::Crossing { point, .. } = e else {
                return None;
            };
            let old = wires[face]
                .iter()
                .position(|b| &b.pcurve == point.boundary())
                .unwrap();
            Some(match (face, old) {
                (0, 0) => [0.625, 0.3203125],
                (0, 1) => [0.140625, 0.625],
                (1, 2) => [0.859375, 0.375],
                (1, 0) => [0.375, 0.6796875],
                (2, 1) => [0.625, 0.125],
                (2, 3) => [0.375, 0.625],
                (3, 0) => [0.859375, 0.375],
                (3, 2) => [0.140625, 0.875],
                _ => panic!("unexpected root"),
            })
        };
        let mut pending = Vec::<(Address, Curve, [Vec<f64>; 2], [Option<[f64; 2]>; 2])>::new();
        let mut pairs = Vec::new();
        let mut maps = Vec::new();
        let mut witnesses = Vec::new();
        for (face, r) in regions.iter().enumerate() {
            for (edge, f) in r.loops()[0].iter().enumerate() {
                let address = Address {
                    face,
                    wire: 0,
                    edge,
                };
                let world = if face == 5 {
                    cap_wire[edge].curve.clone()
                } else if face < 4 && f.curve() == &contacts[face] {
                    contact_world[face].clone()
                } else {
                    wires[face]
                        .iter()
                        .find(|b| &b.pcurve == f.curve())
                        .unwrap()
                        .curve
                        .clone()
                };
                let candidates = std::array::from_fn(|i| candidate(face, &f.endpoints()[i]));
                // Only fixture pairing suggestions use evaluated positions. Every
                // proposed pair and original root witness is rechecked below.
                let ends = std::array::from_fn(|i| {
                    let t = match &f.endpoints()[i] {
                        Endpoint::Parameter(t) => *t,
                        Endpoint::Crossing { role, .. } => {
                            candidates[i].unwrap()[if *role == Role::Boundary { 0 } else { 1 }]
                        }
                    };
                    let uv = f.curve().evaluate(t).unwrap().point;
                    f.surface().evaluate(uv[0], uv[1]).unwrap().point.to_vec()
                });
                if let Some(index) = pending
                    .iter()
                    .position(|(_, _, e, _)| e[0] == ends[1] && e[1] == ends[0])
                {
                    let (other, canonical, _, other_candidates) = pending.remove(index);
                    let range = if world == canonical {
                        [[0., 1.], [1., 1.]]
                    } else if world == reversed(&canonical) {
                        [[1., 1.], [0., 1.]]
                    } else {
                        assert_eq!(canonical.degree, 1);
                        assert_eq!(world.degree, 1);
                        let a = &canonical.control_points[0];
                        let b = &canonical.control_points[1];
                        let axis = (0..3).find(|&k| a[k] != b[k]).unwrap();
                        let denominator = b[axis] - a[axis];
                        std::array::from_fn(|i| {
                            let n = world.control_points[i][axis] - a[axis];
                            if denominator > 0. {
                                [n, denominator]
                            } else {
                                [-n, -denominator]
                            }
                        })
                    };
                    let pair = pairs.len();
                    pairs.push(Pair {
                        uses: [other, address],
                        world: canonical,
                        world_reversed: [false, true],
                        cutters: [None, None],
                    });
                    maps.push(AffineMaps {
                        pair,
                        ranges: [[[0., 1.], [1., 1.]], range],
                    });
                    witnesses.push(RootCandidates {
                        pair,
                        candidates: [other_candidates, candidates],
                    });
                } else {
                    pending.push((address, world, ends, candidates));
                }
            }
        }
        assert!(
            pending.is_empty(),
            "unpaired uses {:?}",
            pending
                .iter()
                .map(|(a, _, e, _)| (a, e))
                .collect::<Vec<_>>()
        );
        assert_eq!(pairs.len(), 12);
        let r = source_shell_incidence::assemble_regions_with_endpoint_inputs(
            &regions,
            &pairs,
            &[],
            &maps,
            &witnesses,
            100_000_000,
            0,
        )
        .unwrap();
        assert!(r.shell.is_some(), "{} {:?}", r.reason, r.uncertain_pair);
        let shell = r.shell.unwrap();
        assert_eq!(shell.faces().len(), 6);
        assert_eq!(shell.edges().len(), 12);
        assert!(shell
            .edges()
            .iter()
            .flat_map(|e| e.uses())
            .flat_map(|f| f.endpoints())
            .any(|e| matches!(e, Endpoint::Crossing { .. })));
        assert_eq!(shell.regions().unwrap()[5].loops()[0][0].surface(), &cap);
        let audit = crate::source_face_contacts::inspect_shell_with_boundary_fibers_and_chart_work(
            &shell,
            1e-8,
            crate::face_contacts::Limits {
                pairs: 15,
                cells: 100000,
                domain_cells: 100000,
                cells_per_pair: 5000,
                domain_cells_per_pair: 10000,
            },
            100_000_000,
            10000,
            10000,
            10000,
        )
        .unwrap();
        assert_eq!(audit.pairs.len(), 15);
        assert!(audit.all_pairs_qualified);
        let cap_side = audit.pairs.iter().find(|p| p.faces == [2, 5]).unwrap();
        assert!(cap_side.fiber.is_none() && cap_side.allowed.is_none());
        let contact = cap_side.interior_fiber.as_ref().unwrap();
        assert_eq!(contact.faces(), [5, 2]);
        assert_eq!(contact.fiber().coordinate(), (1, 0.625));
        assert_eq!(contact.edges().len(), 1);
        assert!(crate::source_interior_contact::certify_with_linear_chart(
            &shell,
            [5, 2],
            100_000_000,
            10000,
            1,
            10000
        )
        .unwrap()
        .certificate
        .is_none());
        println!("new cap: {} face pairs checked; complete contact qualification = {}; interior side/cap fiber qualified",audit.pairs.len(),audit.all_pairs_qualified);
        assert!(
            source_shell_incidence::assemble_regions_with_endpoint_inputs(
                &regions,
                &pairs,
                &[],
                &maps,
                &witnesses,
                1,
                0
            )
            .unwrap()
            .shell
            .is_none()
        );
        let displaced_cap = plane(|u, v| vec![0.609375 * u, k + 1e-12, v]);
        let mut displaced_wire = cap_wire.clone();
        for b in &mut displaced_wire {
            for p in &mut b.curve.control_points {
                p[1] += 1e-12;
            }
        }
        let mut displaced = regions.clone();
        displaced[5] =
            qualify_original_region(&context, &displaced_cap, &[displaced_wire], 1e-8, limits)
                .unwrap()
                .region
                .unwrap();
        assert!(
            source_shell_incidence::assemble_regions_with_endpoint_inputs(
                &displaced,
                &pairs,
                &[],
                &maps,
                &witnesses,
                100_000_000,
                0
            )
            .unwrap()
            .shell
            .is_none()
        );

        let geometry_limits = |corners, pair_count| crate::source_shell_geometry::Limits {
            tolerance_uv: 1e-8,
            corners,
            spans: 10000,
            linear_cells: 10000,
            pairs: crate::face_contacts::Limits {
                pairs: pair_count,
                cells: 100000,
                domain_cells: 100000,
                cells_per_pair: 5000,
                domain_cells_per_pair: 10000,
            },
            exact_work: 100_000_000,
            driver_cells: 10000,
        };
        let fresh_shell = || {
            source_shell_incidence::assemble_regions_with_endpoint_inputs(
                &regions,
                &pairs,
                &[],
                &maps,
                &witnesses,
                100_000_000,
                0,
            )
            .unwrap()
            .shell
            .unwrap()
        };
        let mut invalid = geometry_limits(24, 15);
        invalid.tolerance_uv = f64::NAN;
        assert!(crate::source_shell_geometry::qualify(fresh_shell(), invalid).is_err());
        let stopped =
            crate::source_shell_geometry::qualify(fresh_shell(), geometry_limits(23, 15)).unwrap();
        assert!(stopped.geometry.is_none());
        assert_eq!(stopped.corners, 23);
        assert!(stopped.uncertain_vertex.is_some());
        let stopped =
            crate::source_shell_geometry::qualify(fresh_shell(), geometry_limits(24, 14)).unwrap();
        assert!(stopped.geometry.is_none());
        assert_eq!(stopped.pairs, 14);
        assert_eq!(stopped.next_pair, Some([4, 5]));
        let qualified = crate::source_shell_geometry::qualify(shell, geometry_limits(24, 15)).unwrap();
        assert!(qualified.geometry.is_some(), "{}", qualified.reason);
        let geometry = qualified.geometry.unwrap();
        assert_eq!(geometry.topology().links.len(), 8);
        assert!(geometry.topology().links.iter().all(|v| v.cycle.len() == 3));
        assert_eq!(geometry.topology().euler_characteristic, Some(2));
        assert_eq!(geometry.topology().genus, Some(0));
        assert!(geometry.charts().all_injective);
        assert!(geometry.contacts().all_pairs_qualified);
        assert_eq!(geometry.contacts().pairs.len(), 15);
        assert_eq!(
            geometry.shell().regions().unwrap()[5].loops()[0][0].surface(),
            &cap
        );
        println!("embedded source shell: 8 vertex links, Euler=2, genus=0; all charts and 15 face pairs qualified");
        let volume_limits = |cells, domain_cells| crate::source_volume::Limits {
            axis: 2,
            origin: 0.,
            absolute_error: 0.02,
            tolerance_uv: 1e-8,
            cells,
            spans: 100000,
            domain_cells,
        };
        let stopped_geometry =
            crate::source_shell_geometry::qualify(fresh_shell(), geometry_limits(24, 15))
                .unwrap()
                .geometry
                .unwrap();
        let stopped =
            crate::source_volume::qualify(stopped_geometry, volume_limits(5, 1000000)).unwrap();
        assert!(stopped.body.is_none());
        assert!(stopped.signed_bounds.is_none());
        assert_eq!(stopped.uncertain_face, Some(5));
        let no_domain_geometry =
            crate::source_shell_geometry::qualify(fresh_shell(), geometry_limits(24, 15))
                .unwrap()
                .geometry
                .unwrap();
        let stopped = crate::source_volume::qualify(no_domain_geometry, volume_limits(100, 0)).unwrap();
        assert!(stopped.body.is_none());
        let bounds = stopped.signed_bounds.unwrap();
        assert!(bounds[0] <= 0. && bounds[1] >= 0.);
        let volume = crate::source_volume::qualify(geometry, volume_limits(10000, 1000000)).unwrap();
        println!(
            "source volume {:?}; cells {} spans {} domain {} reason {}",
            volume.signed_bounds, volume.cells, volume.spans, volume.domain_cells, volume.reason
        );
        assert!(
            volume.body.is_some(),
            "{} {:?}",
            volume.reason,
            volume.signed_bounds
        );
        let body = volume.body.unwrap();
        let bounds = body.volume();
        let analytic = 18995. / 24576.;
        assert!(bounds[0] <= analytic && analytic <= bounds[1]);
        assert!(bounds[1] - bounds[0] <= 0.02);
        assert!(body.reverse_orientation());
        assert_eq!(
            body.geometry().shell().regions().unwrap()[5].loops()[0][0].surface(),
            &cap
        );
    }
    #[test]
    fn rational_wedge_has_certified_source_volume_and_keeps_authored_weights() {
        let shell = wedge();
        let geometry = crate::source_shell_geometry::qualify(
            shell,
            crate::source_shell_geometry::Limits {
                tolerance_uv: 1e-8,
                corners: 1000,
                spans: 10000,
                linear_cells: 10000,
                pairs: crate::face_contacts::Limits {
                    pairs: 10,
                    cells: 100000,
                    domain_cells: 100000,
                    cells_per_pair: 10000,
                    domain_cells_per_pair: 10000,
                },
                exact_work: 100_000_000,
                driver_cells: 10000,
            },
        )
        .unwrap()
        .geometry
        .unwrap();
        let volume = crate::source_volume::qualify(
            geometry,
            crate::source_volume::Limits {
                axis: 2,
                origin: 0.,
                absolute_error: 0.05,
                tolerance_uv: 1e-8,
                cells: 10000,
                spans: 100000,
                domain_cells: 1000000,
            },
        )
        .unwrap();
        assert!(
            volume.body.is_some(),
            "{} {:?}",
            volume.reason,
            volume.signed_bounds
        );
        let body = volume.body.unwrap();
        let bounds = body.volume();
        // Authored binary64 weight approximates the circular reference; the
        // certificate encloses the unchanged authored rational definition.
        let reference = std::f64::consts::PI / 4.;
        assert!(bounds[0] <= reference && reference <= bounds[1]);
        assert!(body.reverse_orientation());
        assert!(body
            .geometry()
            .shell()
            .edges()
            .iter()
            .any(|e| e.world().weights.contains(&0.5_f64.sqrt())));
    }

    #[test]
    fn original_full_chart_cuboid_volume_is_precise_on_all_flux_axes() {
        let surfaces = [
            plane(|u, v| vec![2. * v, 3. * u, 10.]),
            plane(|u, v| vec![2. * u, 3. * v, 14.]),
            plane(|u, v| vec![2. * u, 0., 10. + 4. * v]),
            plane(|u, v| vec![2., 3. * u, 10. + 4. * v]),
            plane(|u, v| vec![2. * (1. - u), 3., 10. + 4. * v]),
            plane(|u, v| vec![0., 3. * (1. - u), 10. + 4. * v]),
        ];
        let corners = [[2., 3.], [4., 3.], [4., 7.], [2., 7.]];
        let context = cad_predicates::ToleranceContext::default_valid();
        let mut regions = Vec::new();
        let mut pairs = Vec::new();
        let mut pending = Vec::<(Address, Curve)>::new();
        for (face, mut s) in surfaces.into_iter().enumerate() {
            s.knots_u = vec![2., 2., 4., 4.];
            s.knots_v = vec![3., 3., 7., 7.];
            let wire = (0..4)
                .map(|edge| {
                    let a = corners[edge];
                    let b = corners[(edge + 1) % 4];
                    let world = Curve::from_polyline(vec![
                        s.evaluate(a[0], a[1]).unwrap().point.to_vec(),
                        s.evaluate(b[0], b[1]).unwrap().point.to_vec(),
                    ])
                    .unwrap();
                    let address = Address {
                        face,
                        wire: 0,
                        edge,
                    };
                    if let Some(index) = pending.iter().position(|(_, c)| reversed(c) == world) {
                        let (other, canonical) = pending.remove(index);
                        pairs.push(Pair {
                            uses: [other, address],
                            world: canonical,
                            world_reversed: [false, true],
                            cutters: [None, None],
                        });
                    } else {
                        pending.push((address, world.clone()));
                    }
                    Boundary {
                        curve: world,
                        pcurve: Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap(),
                        reversed: false,
                    }
                })
                .collect::<Vec<_>>();
            let region = qualify_original_region(
                &context,
                &s,
                &[wire],
                1e-8,
                Limits {
                    pairs: 10000,
                    region_cells: 10000,
                    domain_cells: 10000,
                    agreement_cells: 10000,
                },
            )
            .unwrap()
            .region
            .unwrap();
            assert!(region.whole_chart_material());
            regions.push(region);
        }
        assert!(pending.is_empty());
        for axis in 0..3 {
            let shell = source_shell_incidence::assemble_regions(&regions, &pairs, 100_000_000)
                .unwrap()
                .shell
                .unwrap();
            let geometry = crate::source_shell_geometry::qualify(
                shell,
                crate::source_shell_geometry::Limits {
                    tolerance_uv: 1e-8,
                    corners: 24,
                    spans: 10000,
                    linear_cells: 10000,
                    pairs: crate::face_contacts::Limits {
                        pairs: 15,
                        cells: 100000,
                        domain_cells: 100000,
                        cells_per_pair: 10000,
                        domain_cells_per_pair: 10000,
                    },
                    exact_work: 100_000_000,
                    driver_cells: 10000,
                },
            )
            .unwrap()
            .geometry
            .unwrap();
            let volume = crate::source_volume::qualify(
                geometry,
                crate::source_volume::Limits {
                    axis,
                    origin: 0.,
                    absolute_error: 1e-8,
                    tolerance_uv: 1e-8,
                    cells: 6,
                    spans: 6,
                    domain_cells: 0,
                },
            )
            .unwrap();
            assert!(
                volume.body.is_some(),
                "axis {} {} {:?}",
                axis,
                volume.reason,
                volume.signed_bounds
            );
            assert_eq!(volume.cells, 6);
            assert_eq!(volume.domain_cells, 0);
            let body = volume.body.unwrap();
            let bounds = body.volume();
            assert!(bounds[0] <= 24. && 24. <= bounds[1]);
            assert!(!body.reverse_orientation());
            assert!(bounds[1] - bounds[0] <= 1e-8);
        }
    }

}
