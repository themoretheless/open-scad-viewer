//! Exact canonical world ownership for original source edges with qualified restrictions.
use crate::source_boundary_fragment::{Endpoint, Fragment, Role};
use cad_predicates::BezierIdentity;
use nurbs_core::{curve::Curve, curve_surface_agreement, Error, Result};
#[derive(Clone)]
pub struct SharedEdge {
    world: Curve,
    uses: [Fragment; 2],
    reversed: [bool; 2],
}
impl SharedEdge {
    pub fn world(&self) -> &Curve {
        &self.world
    }
    pub fn uses(&self) -> &[Fragment; 2] {
        &self.uses
    }
    pub fn reversed(&self) -> [bool; 2] {
        self.reversed
    }
}
pub struct Report {
    pub edge: Option<SharedEdge>,
    pub work_used: u64,
    pub root_checks: usize,
    pub reason: &'static str,
}
/// `world_reversed` relates canonical world traversal to each forward UV curve.
/// No caller-supplied certificate, shared key or world proximity is authority.
pub fn qualify(
    world: &Curve,
    uses: [&Fragment; 2],
    world_reversed: [bool; 2],
    max_work: u64,
) -> Result<Report> {
    qualify_with_cutters(world, uses, world_reversed, [None, None], max_work)
}
/// Optional canonical crossing curves for endpoints of the first directed use.
/// Their full source compositions are independently checked before root identity.
pub fn qualify_with_cutters(
    world: &Curve,
    uses: [&Fragment; 2],
    world_reversed: [bool; 2],
    cutters: [Option<&Curve>; 2],
    max_work: u64,
) -> Result<Report> {
    if max_work == 0 || max_work > 100_000_000 {
        return Err(Error::new(
            "BREP_SOURCE_SHARED_EDGE",
            "Choose bounded exact identity work",
        ));
    }
    world.validate()?;
    if world.control_points[0].len() != 3 {
        return Err(Error::new(
            "BREP_SOURCE_SHARED_EDGE",
            "Canonical world edge must be 3D",
        ));
    }
    let mut out = Report {
        edge: None,
        work_used: 0,
        root_checks: 0,
        reason: "source-restriction-identity-unproven",
    };
    let complete = uses.iter().all(|edge| {
        let d = edge.curve().domain();
        let expected = if edge.reversed() { [d[1], d[0]] } else { d };
        (0..2).all(|i| matches!(&edge.endpoints()[i], Endpoint::Parameter(t) if t.to_bits() == expected[i].to_bits()))
    });
    if !complete {
        // Exact full-source identity uses normalized traversal. With identical
        // domains, reversal is an exact reflection of normalized traversal.
        // Root identity follows exact equations and a fresh common-root proof, not
        // overlapping isolating intervals. Different UV equations require
        // independent canonical cutter identity and projected root uniqueness.
        if uses.iter().any(|e| e.curve().domain() != world.domain()) {
            return Ok(out);
        }
        for i in 0..2 {
            let same = match (&uses[0].endpoints()[i], &uses[1].endpoints()[1 - i]) {
                (Endpoint::Parameter(a), Endpoint::Parameter(b)) => {
                    if world_reversed[0] == world_reversed[1] {
                        a.to_bits() == b.to_bits()
                    } else {
                        same_reflected_parameter(*a, *b, world.domain(), &mut out, max_work)?
                    }
                }
                (
                    Endpoint::Crossing { point: a, role: ar },
                    Endpoint::Crossing { point: b, role: br },
                ) => {
                    if world_reversed[0] != world_reversed[1]
                        || ar != br
                        || a.boundary() != b.boundary()
                        || a.contact() != b.contact()
                    {
                        if let Some(cutter) = cutters[i] {
                            if world_reversed != [false, false] {
                                return Ok(out);
                            }
                            common_world_root(
                                world,
                                cutter,
                                [(a, *ar), (b, *br)],
                                &mut out,
                                max_work,
                            )?
                        } else {
                            same_linear_parameter(
                                [(a, *ar), (b, *br)],
                                world_reversed,
                                &mut out,
                                max_work,
                            )?
                        }
                    } else if a.selector() == b.selector() {
                        true
                    } else {
                        let selector: [[f64; 2]; 2] = std::array::from_fn(|axis| {
                            [
                                a.selector()[axis][0].max(b.selector()[axis][0]),
                                a.selector()[axis][1].min(b.selector()[axis][1]),
                            ]
                        });
                        if selector.iter().any(|r| r[0] >= r[1]) {
                            false
                        } else {
                            // A freshly certified root inside both unique-root
                            // selectors is their common root. Overlap alone is
                            // never an identity argument. At most two fresh
                            // one-box queries are needed for this edge pair.
                            out.root_checks += 1;
                            nurbs_core::uv_curve_crossings::certify_box(
                                a.boundary(),
                                a.contact(),
                                selector,
                            )?
                            .state
                                == nurbs_core::uv_curve_crossings::State::Unique
                        }
                    }
                }
                _ => false,
            };
            if !same {
                return Ok(out);
            }
        }
    }
    let effective = [
        uses[0].reversed() ^ world_reversed[0],
        uses[1].reversed() ^ world_reversed[1],
    ];
    if effective[0] == effective[1] {
        out.reason = "source-edge-uses-not-opposite";
        return Ok(out);
    }
    for (i, edge) in uses.iter().enumerate() {
        if out.work_used == max_work {
            out.reason = "source-edge-identity-work-limit";
            return Ok(out);
        }
        // Fragment construction independently proves its whole restriction in
        // the positive-weight source chart. Formal homogeneous identity then
        // proves equality there, without requiring the unused UV tails inside.
        let Some(proof) = curve_surface_agreement::verify_exact_algebraic(
            world,
            edge.curve(),
            edge.surface(),
            world_reversed[i],
            (max_work - out.work_used).min(cad_predicates::MAX_WORK),
        )?
        else {
            out.reason = "source-world-identity-layout-unproven";
            return Ok(out);
        };
        out.work_used += proof.work_used;
        if proof.outcome != BezierIdentity::Equal {
            out.reason = "source-world-identity-unproven";
            return Ok(out);
        }
    }
    out.edge = Some(SharedEdge {
        world: world.clone(),
        uses: [uses[0].clone(), uses[1].clone()],
        reversed: effective,
    });
    out.reason = "source-shared-world-edge-qualified";
    Ok(out)
}
fn same_reflected_parameter(
    a: f64,
    b: f64,
    domain: [f64; 2],
    out: &mut Report,
    max_work: u64,
) -> Result<bool> {
    use cad_predicates::{
        AuthoredScalar, Limits, ParameterIdentity, PredicateContext, SourceArena, ToleranceContext,
    };
    if out.work_used == max_work {
        return Ok(false);
    }
    let arena = SourceArena::authored(
        "source-reflected-parameter",
        1,
        [a, b, domain[0], domain[1]]
            .iter()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .map_err(|_| {
        Error::new(
            "BREP_SOURCE_SHARED_EDGE",
            "Invalid reflected parameter source",
        )
    })?;
    let tol = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tol,
        Limits {
            max_work: (max_work - out.work_used).min(cad_predicates::MAX_WORK),
            ..Limits::default()
        },
        None,
    );
    let r = cad_predicates::reflected_parameter_identity(
        &mut ctx,
        std::array::from_fn(|i| arena.leaf(i).unwrap()),
    )
    .map_err(|_| {
        Error::new(
            "BREP_SOURCE_SHARED_EDGE",
            "Invalid reflected parameter request",
        )
    })?;
    out.work_used += r.work_used;
    Ok(r.outcome == ParameterIdentity::Equal)
}
// For transverse original linear Beziers, the UV equations themselves give
// an exact rational parameter. Comparing fractions does not round either root.
fn same_linear_parameter(
    points: [(&crate::source_contact_point::SourcePoint, Role); 2],
    reversed: [bool; 2],
    out: &mut Report,
    max_work: u64,
) -> Result<bool> {
    use cad_predicates::{
        AuthoredScalar, Limits, ParameterIdentity, PredicateContext, SourceArena, ToleranceContext,
    };
    let line = |c: &Curve| {
        let d = c.domain();
        c.degree == 1
            && c.control_points.len() == 2
            && c.knots[..2].iter().all(|&t| t == d[0])
            && c.knots[2..].iter().all(|&t| t == d[1])
    };
    let mut values = Vec::new();
    for (point, role) in points {
        let (main, other) = match role {
            Role::Boundary => (point.boundary(), point.contact()),
            Role::Contact => (point.contact(), point.boundary()),
        };
        if !line(main) || !line(other) {
            return Ok(false);
        }
        for (p, w) in main.control_points.iter().zip(&main.weights) {
            values.extend(p.iter().copied());
            values.push(*w);
        }
        for p in &other.control_points {
            values.extend(p.iter().copied());
        }
    }
    if out.work_used == max_work {
        return Ok(false);
    }
    let arena = SourceArena::authored(
        "source-crossing-parameter",
        1,
        values
            .iter()
            .map(|v: &f64| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .map_err(|_| Error::new("BREP_SOURCE_SHARED_EDGE", "Invalid exact root source"))?;
    let main = std::array::from_fn(|i| {
        std::array::from_fn(|j| std::array::from_fn(|k| arena.leaf(10 * i + 3 * j + k).unwrap()))
    });
    let cutter = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            std::array::from_fn(|k| arena.leaf(10 * i + 6 + 2 * j + k).unwrap())
        })
    });
    let tol = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tol,
        Limits {
            max_work: (max_work - out.work_used).min(cad_predicates::MAX_WORK),
            ..Limits::default()
        },
        None,
    );
    let proof =
        cad_predicates::line_crossing_parameter_identity_oriented(&mut ctx, main, cutter, reversed)
            .map_err(|_| {
                Error::new(
                    "BREP_SOURCE_SHARED_EDGE",
                    "Invalid exact root identity request",
                )
            })?;
    out.work_used += proof.work_used;
    Ok(proof.outcome == ParameterIdentity::Equal)
}
// Both known source roots must lie inside a box on which a projection of the
// canonical 3D crossing equations has exactly one root. Thus projection cannot
// introduce ambiguity between the two actual roots, even on different charts.
fn common_world_root(
    world: &Curve,
    cutter: &Curve,
    points: [(&crate::source_contact_point::SourcePoint, Role); 2],
    out: &mut Report,
    max_work: u64,
) -> Result<bool> {
    let mut ranges = [[[0.; 2]; 2]; 2];
    for (i, (point, role)) in points.iter().enumerate() {
        let (main, other, index) = match role {
            Role::Boundary => (point.boundary(), point.contact(), 0),
            Role::Contact => (point.contact(), point.boundary(), 1),
        };
        if main.domain() != world.domain() || other.domain() != cutter.domain() {
            return Ok(false);
        }
        if out.work_used == max_work {
            return Ok(false);
        }
        let Some(proof) = curve_surface_agreement::verify_exact_algebraic(
            cutter,
            other,
            point.surface(),
            false,
            (max_work - out.work_used).min(cad_predicates::MAX_WORK),
        )?
        else {
            return Ok(false);
        };
        out.work_used += proof.work_used;
        if proof.outcome != BezierIdentity::Equal {
            return Ok(false);
        }
        ranges[i] = [point.selector()[index], point.selector()[1 - index]];
    }
    let box_: [[f64; 2]; 2] = std::array::from_fn(|axis| {
        [
            ranges[0][axis][0].min(ranges[1][axis][0]),
            ranges[0][axis][1].max(ranges[1][axis][1]),
        ]
    });
    for axes in [[0, 1], [0, 2], [1, 2]] {
        let project = |c: &Curve| {
            let mut p = c.clone();
            p.control_points = c
                .control_points
                .iter()
                .map(|v| vec![v[axes[0]], v[axes[1]]])
                .collect();
            p
        };
        out.root_checks += 1;
        if nurbs_core::uv_curve_crossings::certify_box(&project(world), &project(cutter), box_)?
            .state
            == nurbs_core::uv_curve_crossings::State::Unique
        {
            return Ok(true);
        }
    }
    Ok(false)
}
#[cfg(test)]
mod tests {
    use super::*;
    use nurbs_core::surface::Surface;
    #[test]
    fn root_valued_rational_restriction_uses_original_composition_without_extrapolated_chart_admission(
    ) {
        let surface = |vertical: bool| Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![
                    if vertical {
                        vec![0., 0., -0.5]
                    } else {
                        vec![0., -0.5, 0.]
                    },
                    if vertical {
                        vec![0., 0., 0.5]
                    } else {
                        vec![0., 0.5, 0.]
                    },
                ],
                vec![
                    if vertical {
                        vec![1., 0., -0.5]
                    } else {
                        vec![1., -0.5, 0.]
                    },
                    if vertical {
                        vec![1., 0., 0.5]
                    } else {
                        vec![1., 0.5, 0.]
                    },
                ],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let surfaces = [surface(false), surface(true)];
        let mut p = Curve::from_polyline(vec![vec![-0.25, 0.5], vec![1.25, 0.5]]).unwrap();
        p.weights[1] = 0.75;
        let mut world =
            Curve::from_polyline(vec![vec![-0.25, 0., 0.], vec![1.25, 0., 0.]]).unwrap();
        world.weights = p.weights.clone();
        let ends = |s: &Surface| -> Vec<Endpoint> {
            [(0.125, [0.2, 0.4]), (0.875, [0.7, 0.9])]
                .iter()
                .map(|&(u, selector)| {
                    let boundary = Curve::from_polyline(vec![vec![u, 0.], vec![u, 1.]]).unwrap();
                    let r = crate::source_contact_point::qualify(
                        s,
                        &boundary,
                        &p,
                        [[0.4, 0.6], selector],
                        16,
                    )
                    .unwrap();
                    assert!(r.point.is_some(), "{}", r.reason);
                    Endpoint::Crossing {
                        point: r.point.unwrap(),
                        role: Role::Contact,
                    }
                })
                .collect()
        };
        let ea = ends(&surfaces[0]);
        let eb = ends(&surfaces[1]);
        let a = Fragment::new(&surfaces[0], &p, ea[0].clone(), ea[1].clone()).unwrap();
        let b = Fragment::new(&surfaces[1], &p, eb[1].clone(), eb[0].clone()).unwrap();
        assert!(
            curve_surface_agreement::verify_exact(&world, &p, &surfaces[0], false, 1_000_000)
                .unwrap()
                .is_none()
        );
        let r = qualify(&world, [&a, &b], [false, false], 1_000_000).unwrap();
        assert!(r.edge.is_some(), "{}", r.reason);
        assert_eq!(r.edge.unwrap().uses()[0].definition(), a.definition());
        assert!(Fragment::new(
            &surfaces[0],
            &p,
            Endpoint::Parameter(0.),
            Endpoint::Parameter(1.)
        )
        .is_err());
        world.control_points[0][2] += 1e-12;
        assert!(qualify(&world, [&a, &b], [false, false], 1_000_000)
            .unwrap()
            .edge
            .is_none());
    }
    #[test]
    fn adjacent_non_coplanar_faces_have_exact_common_root_without_shared_cutter() {
        let surface = |vertical: bool| Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![
                    vec![0., 0., 0.],
                    if vertical {
                        vec![0., 0., 1.]
                    } else {
                        vec![0., 1., 0.]
                    },
                ],
                vec![
                    vec![1., 0., 0.],
                    if vertical {
                        vec![1., 0., 1.]
                    } else {
                        vec![1., 1., 0.]
                    },
                ],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let p = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
        let cutter = |x: f64, extent: f64| {
            Curve::from_polyline(vec![vec![x, -extent], vec![x, extent]]).unwrap()
        };
        let q = [cutter(0.5, 0.2), cutter(0.5, 0.3)];
        let surfaces = [surface(false), surface(true)];
        let points: Vec<_> = (0..2)
            .map(|i| {
                crate::source_contact_point::qualify(&surfaces[i], &p, &q[i], [[0., 1.]; 2], 16)
                    .unwrap()
                    .point
                    .unwrap()
            })
            .collect();
        let a = Fragment::new(
            &surfaces[0],
            &p,
            Endpoint::Parameter(0.),
            Endpoint::Crossing {
                point: points[0].clone(),
                role: Role::Boundary,
            },
        )
        .unwrap();
        let b = Fragment::new(
            &surfaces[1],
            &p,
            Endpoint::Crossing {
                point: points[1].clone(),
                role: Role::Boundary,
            },
            Endpoint::Parameter(0.),
        )
        .unwrap();
        let world = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]]).unwrap();
        let r = qualify(&world, [&a, &b], [false, false], 1_000_000).unwrap();
        assert!(r.edge.is_some(), "{}", r.reason);
        let other = crate::source_contact_point::qualify(
            &surfaces[1],
            &p,
            &cutter(0.5 + 1e-12, 0.3),
            [[0., 1.]; 2],
            16,
        )
        .unwrap()
        .point
        .unwrap();
        let b = Fragment::new(
            &surfaces[1],
            &p,
            Endpoint::Crossing {
                point: other,
                role: Role::Boundary,
            },
            Endpoint::Parameter(0.),
        )
        .unwrap();
        assert!(qualify(&world, [&a, &b], [false, false], 1_000_000)
            .unwrap()
            .edge
            .is_none());
    }
    #[test]
    fn different_uv_equations_require_exact_world_cutter_and_unique_world_root() {
        let s = |swap: bool| Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..2)
                .map(|u| {
                    (0..2)
                        .map(|v| {
                            if swap {
                                vec![v as f64, u as f64, 0.]
                            } else {
                                vec![u as f64, v as f64, 0.]
                            }
                        })
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let line = |p| Curve::from_polyline(p).unwrap();
        let p = [
            line(vec![vec![0., 0.5], vec![1., 0.5]]),
            line(vec![vec![0.5, 0.], vec![0.5, 1.]]),
        ];
        let q = [
            line(vec![vec![0.5, 0.2], vec![0.5, 0.8]]),
            line(vec![vec![0.2, 0.5], vec![0.8, 0.5]]),
        ];
        let surface = [s(false), s(true)];
        let roots: Vec<_> = (0..2)
            .map(|i| {
                crate::source_contact_point::qualify(&surface[i], &p[i], &q[i], [[0., 1.]; 2], 16)
                    .unwrap()
                    .point
                    .unwrap()
            })
            .collect();
        let a = Fragment::new(
            &surface[0],
            &p[0],
            Endpoint::Parameter(0.),
            Endpoint::Crossing {
                point: roots[0].clone(),
                role: Role::Boundary,
            },
        )
        .unwrap();
        let b = Fragment::new(
            &surface[1],
            &p[1],
            Endpoint::Crossing {
                point: roots[1].clone(),
                role: Role::Boundary,
            },
            Endpoint::Parameter(0.),
        )
        .unwrap();
        let world = line(vec![vec![0., 0.5, 0.], vec![1., 0.5, 0.]]);
        let cutter = line(vec![vec![0.5, 0.2, 0.], vec![0.5, 0.8, 0.]]);
        assert!(qualify_with_cutters(
            &p[0],
            [&a, &b],
            [false, false],
            [None, Some(&cutter)],
            1_000_000
        )
        .is_err());
        assert!(qualify(&world, [&a, &b], [false, false], 1_000_000)
            .unwrap()
            .edge
            .is_some());
        let r = qualify_with_cutters(
            &world,
            [&a, &b],
            [false, false],
            [None, Some(&cutter)],
            1_000_000,
        )
        .unwrap();
        assert!(r.edge.is_some(), "{}", r.reason);
        assert_eq!(r.root_checks, 1);
        let mut wrong = cutter.clone();
        wrong.control_points[0][2] = 1e-12;
        assert!(qualify_with_cutters(
            &world,
            [&a, &b],
            [false, false],
            [None, Some(&wrong)],
            1_000_000
        )
        .unwrap()
        .edge
        .is_none());
    }
    #[test]
    fn different_surface_charts_share_only_exact_opposite_world_uses() {
        let surface = |z: f64| Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., z]],
                vec![vec![1., 0., 0.], vec![1., 1., z]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let p = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
        let a = Fragment::new(
            &surface(0.),
            &p,
            Endpoint::Parameter(0.),
            Endpoint::Parameter(1.),
        )
        .unwrap();
        let b = Fragment::new(
            &surface(1.),
            &p,
            Endpoint::Parameter(1.),
            Endpoint::Parameter(0.),
        )
        .unwrap();
        let c = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]]).unwrap();
        let r = qualify(&c, [&a, &b], [false, false], 1_000_000).unwrap();
        assert!(r.edge.is_some(), "{}", r.reason);
        assert_eq!(r.edge.unwrap().reversed(), [false, true]);
        assert!(qualify(&c, [&a, &a], [false, false], 1_000_000)
            .unwrap()
            .edge
            .is_none());
        let mut shifted = c.clone();
        shifted.control_points[0][2] = 1e-12;
        assert!(qualify(&shifted, [&a, &b], [false, false], 1_000_000)
            .unwrap()
            .edge
            .is_none());
        let cutter = Curve::from_polyline(vec![vec![0.5, -0.2], vec![0.5, 0.2]]).unwrap();
        let point = |surface: &Surface| {
            crate::source_contact_point::qualify(surface, &p, &cutter, [[0., 1.]; 2], 16)
                .unwrap()
                .point
                .unwrap()
        };
        let root_a = Endpoint::Crossing {
            point: point(a.surface()),
            role: crate::source_boundary_fragment::Role::Boundary,
        };
        let root_b = Endpoint::Crossing {
            point: point(b.surface()),
            role: crate::source_boundary_fragment::Role::Boundary,
        };
        let cut_a = Fragment::new(a.surface(), &p, Endpoint::Parameter(0.), root_a).unwrap();
        let cut_b = Fragment::new(b.surface(), &p, root_b, Endpoint::Parameter(0.)).unwrap();
        let shared = qualify(&c, [&cut_a, &cut_b], [false, false], 1_000_000).unwrap();
        assert!(shared.edge.is_some(), "{}", shared.reason);
        assert_eq!(
            shared.edge.unwrap().uses()[0].definition(),
            cut_a.definition()
        );
        let refined =
            crate::source_contact_point::qualify(b.surface(), &p, &cutter, [[0.2, 0.8]; 2], 16)
                .unwrap()
                .point
                .unwrap();
        let refined = Fragment::new(
            b.surface(),
            &p,
            Endpoint::Crossing {
                point: refined,
                role: crate::source_boundary_fragment::Role::Boundary,
            },
            Endpoint::Parameter(0.),
        )
        .unwrap();
        let independently_selected =
            qualify(&c, [&cut_a, &refined], [false, false], 1_000_000).unwrap();
        assert!(
            independently_selected.edge.is_some(),
            "{}",
            independently_selected.reason
        );
        assert_eq!(independently_selected.root_checks, 1);
        let mut other = cutter.clone();
        other.control_points[0][0] += 1e-12;
        other.control_points[1][0] += 1e-12;
        let other =
            crate::source_contact_point::qualify(b.surface(), &p, &other, [[0., 1.]; 2], 16)
                .unwrap()
                .point
                .unwrap();
        let mismatch = Fragment::new(
            b.surface(),
            &p,
            Endpoint::Crossing {
                point: other,
                role: crate::source_boundary_fragment::Role::Boundary,
            },
            Endpoint::Parameter(0.),
        )
        .unwrap();
        assert!(qualify(&c, [&cut_a, &mismatch], [false, false], 1_000_000)
            .unwrap()
            .edge
            .is_none());
        let partial = Fragment::new(
            a.surface(),
            &p,
            Endpoint::Parameter(0.2),
            Endpoint::Parameter(1.),
        )
        .unwrap();
        assert!(qualify(&c, [&partial, &b], [false, false], 1_000_000)
            .unwrap()
            .edge
            .is_none());
    }
}
