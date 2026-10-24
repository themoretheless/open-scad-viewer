use super::*;
use nurbs_core::surface::Surface;
#[test]
fn global_oblique_plane_uses_signed_derivative_without_constant_coordinates() {
    let mut world=Curve{degree:2,knots:vec![2.,2.,2.,8.,8.,8.],
        control_points:vec![vec![0.,0.,0.],vec![1.,0.5,0.],vec![1.,0.,0.]],
        weights:vec![1.;3],periodic:false};
    let plane=[[0.,0.,0.],[1.,1.,0.],[0.,0.,1.]];
    // x-y=q: both coordinates vary and y reverses direction.
    let fresh=||Report{edge:None,work_used:0,root_checks:0,driver_cells:0,reason:"test"};
    let mut report=fresh();
    assert!(unique_world_plane(&world,plane,&mut report,1000000,1000).unwrap());
    assert!(report.driver_cells>0 && report.driver_cells<=1000);
    let mut stopped=fresh();
    assert!(!unique_world_plane(&world,plane,&mut stopped,1000000,0).unwrap());
    assert_eq!(stopped.driver_cells,0);
    world.control_points[2]=vec![0.,0.,0.];
    let mut turning=fresh();
    assert!(!unique_world_plane(&world,plane,&mut turning,1000000,128).unwrap());
    assert!(turning.driver_cells<=128);
    let mut degenerate=fresh();
    assert!(!unique_world_plane(&world,[[0.;3];3],&mut degenerate,1000000,128).unwrap());
}

#[test]
fn mixed_root_and_fixed_endpoints_need_exact_point_membership_and_unique_world_root() {
    use crate::source_boundary_fragment::{Endpoint, Role};
    use nurbs_core::surface::Surface;
    let cap = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let side = Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 1.], vec![1., 1., 1.], vec![0., 1., 1.]],
        ],
        weights: vec![vec![1.; 3]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let main_a = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let main_b = Curve {
        degree: 1,
        knots: vec![10., 10., 18., 18.],
        control_points: vec![vec![0., 1.], vec![0., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let cut = Curve::from_polyline(vec![vec![0.609375, 1.25], vec![0.609375, -0.25]]).unwrap();
    let point = crate::source_contact_point::qualify(
        &cap,
        &main_a,
        &cut,
        [[0.6, 0.65], [0.24, 0.28]],
        10000,
    )
    .unwrap();
    assert!(point.point.is_some(), "{}", point.reason);
    let fa = Fragment::new(
        &cap,
        &main_a,
        Endpoint::Parameter(0.),
        Endpoint::Crossing {
            point: point.point.unwrap(),
            role: Role::Boundary,
        },
    )
    .unwrap();
    let fb = Fragment::new(
        &side,
        &main_b,
        Endpoint::Parameter(13.),
        Endpoint::Parameter(18.),
    )
    .unwrap();
    let world = Curve {
        degree: 2,
        knots: vec![-5., -5., -5., 3., 3., 3.],
        control_points: side.control_points[0].clone(),
        weights: vec![1.; 3],
        periodic: false,
    };
    let plane = [[0.609375, 0., 0.], [0.609375, 1., 0.], [0.609375, 0., 1.]];
    assert!(qualify(&world, [&fa, &fb], [false, true], 100_000_000)
        .unwrap()
        .edge
        .is_none());
    let r = qualify_with_planes(
        &world,
        [&fa, &fb],
        [false, true],
        [None, Some(plane)],
        100_000_000,
        10000,
    )
    .unwrap();
    assert!(r.edge.is_some(), "{}", r.reason);
    assert_eq!(r.root_checks, 1);
    assert!(r.driver_cells > 0);
    let e = r.edge.unwrap();
    crate::source_edge_restriction::assert_replay(&e);
    let restriction = crate::source_edge_restriction::Restriction::from_edge(&e);
    let b = restriction.parameter_bounds().unwrap()[1];
    assert!(b[0] <= 0.0 && 0.0 <= b[1]);
    let b = restriction.endpoint_boxes(100).unwrap()[1];
    for (axis, expected) in [0.609375, 0.859375, 0.].into_iter().enumerate() {
        assert!(b[axis][0] <= expected && expected <= b[axis][1]);
    }
    let restore = |definition, work| crate::source_shared_edge_restore::restore(definition,
        crate::source_shared_edge_restore::Limits {mapping_cells_per_use:10000,exact_work:work,driver_cells:10000}).unwrap();
    let restored = restore(e.definition(),100_000_000).edge.unwrap();
    assert_eq!(restored.definition(),e.definition());
    assert_eq!(restored.reversed(),e.reversed());
    assert!(!e.covers_complete_canonical_source());
    assert!(!restored.covers_complete_canonical_source());
    assert!(restore(e.definition(),1).edge.is_none());
    let mut missing_plane=e.definition();
    missing_plane["recipe"]["planes"][1]=value_codec::Value::Null;
    missing_plane["certificateQualified"]=value_codec::Value::Bool(true);
    assert!(restore(missing_plane,100_000_000).edge.is_none());

    assert_eq!(e.world(), &world);
    assert_eq!(e.uses()[1].curve(), &main_b);
    assert!(qualify_with_planes(
        &world,
        [&fb, &fa],
        [true, false],
        [Some(plane), None],
        100_000_000,
        10000
    )
    .unwrap()
    .edge
    .is_some());
    let wrong = Fragment::new(
        &side,
        &main_b,
        Endpoint::Parameter(13. + 1e-12),
        Endpoint::Parameter(18.),
    )
    .unwrap();
    assert!(qualify_with_planes(
        &world,
        [&fa, &wrong],
        [false, true],
        [None, Some(plane)],
        100_000_000,
        10000
    )
    .unwrap()
    .edge
    .is_none());
    assert!(qualify_with_planes(
        &world,
        [&fa, &fb],
        [false, true],
        [None, Some(plane)],
        100_000_000,
        0
    )
    .unwrap()
    .edge
    .is_none());
    assert!(qualify_with_planes(
        &world,
        [&fa, &fb],
        [false, true],
        [None, Some(plane)],
        1,
        10000
    )
    .unwrap()
    .edge
    .is_none());
}
#[test]
fn distinct_original_domains_share_exact_normalized_parameters_and_rational_roots() {
    use crate::source_boundary_fragment::{Endpoint, Role};
    use nurbs_core::surface::Surface;
    let surface = |vertical: bool| Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..2)
            .map(|u| {
                (0..2)
                    .map(|v| {
                        if vertical {
                            vec![u as f64, 0., v as f64]
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
    let sa = surface(false);
    let sb = surface(true);
    let a = Curve {
        degree: 1,
        knots: vec![2., 2., 4., 4.],
        control_points: vec![vec![0., 0.], vec![1., 0.]],
        weights: vec![1., 2.],
        periodic: false,
    };
    let b = Curve {
        degree: 1,
        knots: vec![10., 10., 18., 18.],
        control_points: vec![vec![1., 0.], vec![0., 0.]],
        weights: vec![2., 1.],
        periodic: false,
    };
    let world = Curve {
        degree: 1,
        knots: vec![-5., -5., 3., 3.],
        control_points: vec![vec![0., 0., 0.], vec![1., 0., 0.]],
        weights: vec![1., 2.],
        periodic: false,
    };
    let fa =
        Fragment::new(&sa, &a, Endpoint::Parameter(2.5), Endpoint::Parameter(3.5)).unwrap();
    let fb =
        Fragment::new(&sb, &b, Endpoint::Parameter(12.), Endpoint::Parameter(16.)).unwrap();
    let r = qualify(&world, [&fa, &fb], [false, true], 100_000_000).unwrap();
    assert!(r.edge.is_some(), "{}", r.reason);
    let edge = r.edge.unwrap();
    assert_eq!(edge.world(), &world);
    assert_eq!(edge.uses()[0].curve(), &a);
    assert_eq!(edge.uses()[1].curve(), &b);
    let wrong = Fragment::new(
        &sb,
        &b,
        Endpoint::Parameter(12.),
        Endpoint::Parameter(16. + 1e-12),
    )
    .unwrap();
    assert!(qualify(&world, [&fa, &wrong], [false, true], 100_000_000)
        .unwrap()
        .edge
        .is_none());
    let cut_a = Curve {
        degree: 1,
        knots: vec![20., 20., 24., 24.],
        control_points: vec![vec![0.5, -1.], vec![0.5, 1.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let cut_b = Curve {
        degree: 1,
        knots: vec![-4., -4., 2., 2.],
        control_points: cut_a.control_points.clone(),
        weights: vec![1.; 2],
        periodic: false,
    };
    let pa = crate::source_contact_point::qualify(
        &sa,
        &a,
        &cut_a,
        [[2.6, 2.8], [21.6, 22.4]],
        10000,
    )
    .unwrap();
    let pb = crate::source_contact_point::qualify(
        &sb,
        &b,
        &cut_b,
        [[15.2, 15.5], [-1.5, -0.5]],
        10000,
    )
    .unwrap();
    assert!(pa.point.is_some(), "{}", pa.reason);
    assert!(pb.point.is_some(), "{}", pb.reason);
    let ra = Fragment::new(
        &sa,
        &a,
        Endpoint::Parameter(2.),
        Endpoint::Crossing {
            point: pa.point.unwrap(),
            role: Role::Boundary,
        },
    )
    .unwrap();
    let rb = Fragment::new(
        &sb,
        &b,
        Endpoint::Crossing {
            point: pb.point.unwrap(),
            role: Role::Boundary,
        },
        Endpoint::Parameter(18.),
    )
    .unwrap();
    let root = qualify(&world, [&ra, &rb], [false, true], 100_000_000).unwrap();
    assert!(root.edge.is_some(), "{}", root.reason);
    assert_eq!(root.edge.unwrap().uses()[0].curve().domain(), [2., 4.]);
    assert!(qualify(&world, [&ra, &rb], [false, true], 1)
        .unwrap()
        .edge
        .is_none());
}
#[test]
fn two_world_plane_roots_never_become_one_shared_endpoint() {
    use crate::source_boundary_fragment::{Endpoint, Role};
    use nurbs_core::surface::Surface;
    let cap = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let side = Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![vec![0.5, 0., 0.], vec![0., 0.5, 0.], vec![0.5, 1., 0.]],
            vec![vec![0.5, 0., 1.], vec![0., 0.5, 1.], vec![0.5, 1., 1.]],
        ],
        weights: vec![vec![1.; 3]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let main_a = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0.5, 0.], vec![0., 0.5], vec![0.5, 1.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let main_b = Curve::from_polyline(vec![vec![0., 1.], vec![0., 0.]]).unwrap();
    let cut_a = Curve::from_polyline(vec![vec![0.375, -0.25], vec![0.375, 1.25]]).unwrap();
    let cut_b = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0.125, 0.], vec![-0.375, 0.5], vec![0.125, 1.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let a = crate::source_contact_point::qualify(
        &cap,
        &main_a,
        &cut_a,
        [[0.14, 0.16], [0.25, 0.28]],
        10000,
    )
    .unwrap();
    let b = crate::source_contact_point::qualify(
        &side,
        &main_b,
        &cut_b,
        [[0.14, 0.16], [0.84, 0.87]],
        10000,
    )
    .unwrap();
    assert!(a.point.is_some(), "{}", a.reason);
    assert!(b.point.is_some(), "{}", b.reason);
    let fa = Fragment::new(
        &cap,
        &main_a,
        Endpoint::Parameter(0.),
        Endpoint::Crossing {
            point: a.point.unwrap(),
            role: Role::Boundary,
        },
    )
    .unwrap();
    let fb = Fragment::new(
        &side,
        &main_b,
        Endpoint::Crossing {
            point: b.point.unwrap(),
            role: Role::Boundary,
        },
        Endpoint::Parameter(1.),
    )
    .unwrap();
    let world = Curve {
        degree: 2,
        knots: main_a.knots.clone(),
        control_points: side.control_points[0].clone(),
        weights: vec![1.; 3],
        periodic: false,
    };
    let plane = [[0.375, 0., 0.], [0.375, 1., 0.], [1.375, 0., 1.]];
    // Both exact crossing images lie in this plane, but its two world
    // roots own different canonical parameters. Plane membership alone
    // must never turn these restrictions into a shared edge.
    let r = qualify_with_planes(
        &world,
        [&fa, &fb],
        [false, true],
        [None, Some(plane)],
        100_000_000,
        100,
    )
    .unwrap();
    assert!(r.edge.is_none());
    assert!(r.driver_cells <= 100);
    let fixed_cut =
        Curve::from_polyline(vec![vec![0.3125, -0.25], vec![0.3125, 1.25]]).unwrap();
    let fixed_root = crate::source_contact_point::qualify(
        &cap,
        &main_a,
        &fixed_cut,
        [[0.2, 0.3], [0.3, 0.36]],
        10000,
    )
    .unwrap();
    assert!(fixed_root.point.is_some(), "{}", fixed_root.reason);
    let root_use = Fragment::new(
        &cap,
        &main_a,
        Endpoint::Parameter(0.),
        Endpoint::Crossing {
            point: fixed_root.point.unwrap(),
            role: Role::Boundary,
        },
    )
    .unwrap();
    let parameter_use = Fragment::new(
        &side,
        &main_b,
        Endpoint::Parameter(0.25),
        Endpoint::Parameter(1.),
    )
    .unwrap();
    let fixed_plane = [[0.3125, 0., 0.], [0.3125, 1., 0.], [1.3125, 0., 1.]];
    // Fixed q=.75 and crossing q=.25 both satisfy the same plane exactly.
    // Mixed endpoint admission still requires a unique canonical plane root.
    let mixed = qualify_with_planes(
        &world,
        [&root_use, &parameter_use],
        [false, true],
        [None, Some(fixed_plane)],
        100_000_000,
        100,
    )
    .unwrap();
    assert!(mixed.edge.is_none());
    assert!(mixed.driver_cells > 0 && mixed.driver_cells <= 100);
}
#[test]
fn nonlinear_local_crossings_share_only_a_fresh_unique_world_plane_root() {
    use crate::source_boundary_fragment::{Endpoint, Role};
    use nurbs_core::surface::Surface;
    let cap = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let side = Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 1.], vec![1., 1., 1.], vec![0., 1., 1.]],
        ],
        weights: vec![vec![1.; 3]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let main_a = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let main_b = Curve::from_polyline(vec![vec![0., 1.], vec![0., 0.]]).unwrap();
    let cut_a = Curve::from_polyline(vec![vec![0.5, 1.25], vec![0.5, -0.25]]).unwrap();
    let cut_b = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0.4375, -0.25], vec![0.8125, 0.5], vec![-1.0625, 1.25]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let a = crate::source_contact_point::qualify(
        &cap,
        &main_a,
        &cut_a,
        [[0.65, 0.8], [0.15, 0.3]],
        10000,
    )
    .unwrap();
    let b = crate::source_contact_point::qualify(
        &side,
        &main_b,
        &cut_b,
        [[0.2, 0.4], [0.55, 0.75]],
        10000,
    )
    .unwrap();
    assert!(a.point.is_some(), "{}", a.reason);
    assert!(b.point.is_some(), "{}", b.reason);
    let a = a.point.unwrap();
    let b = b.point.unwrap();
    let fa = Fragment::new(
        &cap,
        &main_a,
        Endpoint::Parameter(0.),
        Endpoint::Crossing {
            point: a,
            role: Role::Boundary,
        },
    )
    .unwrap();
    let fb = Fragment::new(
        &side,
        &main_b,
        Endpoint::Crossing {
            point: b,
            role: Role::Boundary,
        },
        Endpoint::Parameter(1.),
    )
    .unwrap();
    let world = Curve {
        degree: 2,
        knots: main_a.knots.clone(),
        control_points: vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let plane = [[0.5, 0., 0.], [0.5, 1., 0.], [1.5, 0., 1.]];
    assert!(qualify(&world, [&fa, &fb], [false, true], 100_000_000)
        .unwrap()
        .edge
        .is_none());
    let r = qualify_with_planes(
        &world,
        [&fa, &fb],
        [false, true],
        [None, Some(plane)],
        100_000_000,
        10000,
    )
    .unwrap();
    assert!(r.edge.is_some(), "{} {}", r.reason, r.work_used);
    assert_eq!(r.edge.unwrap().world(), &world);
    assert!(r.driver_cells > 0);
    let mut alternate = world.clone();
    alternate.knots = vec![-5., -5., -5., 3., 3., 3.];
    let remapped = qualify_with_planes(
        &alternate,
        [&fa, &fb],
        [false, true],
        [None, Some(plane)],
        100_000_000,
        10000,
    )
    .unwrap();
    assert!(remapped.edge.is_some(), "{}", remapped.reason);
    assert_eq!(remapped.edge.unwrap().world(), &alternate);

    let mut displaced = plane;
    for p in &mut displaced {
        p[0] += 1e-12;
    }
    assert!(qualify_with_planes(
        &world,
        [&fa, &fb],
        [false, true],
        [None, Some(displaced)],
        100_000_000,
        10000
    )
    .unwrap()
    .edge
    .is_none());
    assert!(qualify_with_planes(
        &world,
        [&fa, &fb],
        [false, true],
        [None, Some(plane)],
        100_000_000,
        0
    )
    .unwrap()
    .edge
    .is_none());
    assert!(qualify_with_planes(
        &world,
        [&fa, &fb],
        [false, true],
        [None, Some(plane)],
        1,
        10000
    )
    .unwrap()
    .edge
    .is_none());
    let mut wrong = world.clone();
    wrong.control_points[1][2] = 1e-12;
    assert!(qualify_with_planes(
        &wrong,
        [&fa, &fb],
        [false, true],
        [None, Some(plane)],
        100_000_000,
        10000
    )
    .unwrap()
    .edge
    .is_none());
}
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
    let mut mapped_world = world.clone();
    mapped_world.knots = vec![-5., -5., 3., 3.];
    let mut mapped_cutter = cutter.clone();
    mapped_cutter.knots = vec![10., 10., 18., 18.];
    let mapped = qualify_with_cutters(
        &mapped_world,
        [&a, &b],
        [false, false],
        [None, Some(&mapped_cutter)],
        1_000_000,
    )
    .unwrap();
    assert!(mapped.edge.is_some(), "{}", mapped.reason);
    assert_eq!(mapped.root_checks, 1);
    assert_eq!(mapped.edge.unwrap().world(), &mapped_world);
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
#[test]
fn curved_world_carrier_keeps_irrational_root_across_reflected_uv_charts() {
    let surface = |sign: f64| Surface {
        degree_u: 1, degree_v: 1, periodic_u: false, periodic_v: false,
        knots_u: vec![0.,0.,1.,1.], knots_v: vec![-1.,-1.,1.,1.],
        control_points: vec![
            vec![vec![0.,-sign,0.],vec![0.,sign,0.]],
            vec![vec![1.,-sign,0.],vec![1.,sign,0.]],
        ], weights: vec![vec![1.;2];2],
    };
    let main = Curve {degree:2,periodic:false,knots:vec![0.,0.,0.,1.,1.,1.],
        control_points:vec![vec![0.,0.],vec![0.5,0.25],vec![1.,0.]],weights:vec![1.,2.,1.]};
    let cutter=Curve {degree:2,periodic:false,knots:main.knots.clone(),
        control_points:vec![vec![0.,-0.5],vec![0.5,0.],vec![1.,0.5]],weights:vec![1.,2.,1.]};
    let mut other_main=main.clone();other_main.control_points.reverse();
    for p in &mut other_main.control_points {p[1] = -p[1];}
    let mut other_cutter=cutter.clone();
    for p in &mut other_cutter.control_points {p[1] = -p[1];}
    let a_surface=surface(1.);let b_surface=surface(-1.);
    let point = |surface:&Surface, m:&Curve, c:&Curve, selector| {
        let r=crate::source_contact_point::qualify(surface,m,c,selector,1000).unwrap();
        r.point.expect(r.reason)
    };
    let a=Fragment::new(&a_surface,&main,Endpoint::Parameter(0.),
        Endpoint::Crossing {point:point(&a_surface,&main,&cutter,[[0.7,0.71],[0.7,0.71]]),role:Role::Boundary}).unwrap();
    let b=Fragment::new(&b_surface,&other_main,
        Endpoint::Crossing {point:point(&b_surface,&other_main,&other_cutter,[[0.29,0.3],[0.7,0.71]]),role:Role::Boundary},Endpoint::Parameter(1.)).unwrap();
    let mut world=main.clone();for p in &mut world.control_points {p.push(0.);}
    let r=qualify(&world,[&a,&b],[false,true],1_000_000).unwrap();
    assert!(r.edge.is_some(),"{}",r.reason);
    assert!(r.root_checks>0);
    let edge=r.edge.unwrap();
    assert_eq!(edge.uses()[0].definition(),a.definition());
    assert_eq!(edge.uses()[1].definition(),b.definition());
    let range=edge.uses()[0].parameter_bounds()[1];
    assert!(range[0]<std::f64::consts::FRAC_1_SQRT_2 && range[1]>std::f64::consts::FRAC_1_SQRT_2);
    let mut damaged=world.clone();damaged.control_points[1][1]+=1e-12;
    assert!(qualify(&damaged,[&a,&b],[false,true],1_000_000).unwrap().edge.is_none());
}

#[test]
fn curved_support_charts_share_original_root_ended_world_edge() {
    // Distinct curved sheets use original polynomial and rational U rows.
    for middle_weight in [1.,2.] {
    let surface=|sign:f64|Surface{degree_u:2,degree_v:1,periodic_u:false,periodic_v:false,
        knots_u:vec![0.,0.,0.,1.,1.,1.],knots_v:vec![-1.,-1.,1.,1.],
        control_points:vec![vec![vec![0.,-sign,-1.],vec![0.,sign,1.]],
            vec![vec![0.5,-sign,-1.],vec![0.5,sign,1.]],vec![vec![1.,-sign,0.],vec![1.,sign,2.]]],
        weights:vec![vec![1.;2],vec![middle_weight;2],vec![1.;2]]};
    let main=Curve{degree:2,periodic:false,knots:vec![0.,0.,0.,1.,1.,1.],
        control_points:vec![vec![0.,0.],vec![0.5,0.],vec![1.,0.]],weights:vec![1.;3]};
    let cutter=Curve{control_points:vec![vec![0.,-0.5],vec![0.5,-0.5],vec![1.,0.5]],..main.clone()};
    let mut other_main=main.clone();other_main.control_points.reverse();
    let mut other_cutter=cutter.clone();for p in &mut other_cutter.control_points{p[1] = -p[1];}
    let sa=surface(1.);let sb=surface(-1.);
    let pa=crate::source_contact_point::qualify(&sa,&main,&cutter,[[0.7,0.71],[0.7,0.71]],1000).unwrap();
    let pb=crate::source_contact_point::qualify(&sb,&other_main,&other_cutter,[[0.29,0.3],[0.7,0.71]],1000).unwrap();
    let a=Fragment::new(&sa,&main,Endpoint::Parameter(0.),Endpoint::Crossing{point:pa.point.expect(pa.reason),role:Role::Boundary}).unwrap();
    let b=Fragment::new(&sb,&other_main,Endpoint::Crossing{point:pb.point.expect(pb.reason),role:Role::Boundary},Endpoint::Parameter(1.)).unwrap();
    let world=Curve{control_points:vec![vec![0.,0.,0.],vec![0.5,0.,0.],vec![1.,0.,1.]],weights:vec![1.,middle_weight,1.],..main.clone()};
    let r=qualify(&world,[&a,&b],[false,true],1_000_000).unwrap();
    let edge=r.edge.expect(r.reason);assert!(r.root_checks>0);
    assert!(!edge.covers_complete_canonical_source());
    let sample=edge.sample_interior(0.5,3).unwrap().sample.expect("shared curved interior sample");
    let bounds=sample.world_bounds();
    for (axis,value) in [0.5,0.,0.5/(1.+middle_weight)].into_iter().enumerate() {
        assert!(bounds[axis][0]<=value && value<=bounds[axis][1]);
    }
    assert!(edge.sample_interior(0.8,3).unwrap().sample.is_none());
    let endpoint=edge.sample_endpoint(1,3).unwrap().endpoint.expect("exact paired curved root endpoint");
    let canonical=endpoint.canonical_bounds();
    assert!(canonical[0]<std::f64::consts::FRAC_1_SQRT_2 && canonical[1]>std::f64::consts::FRAC_1_SQRT_2);
    assert_eq!(endpoint.parameter_bounds(),[a.parameter_bounds()[1],b.parameter_bounds()[0]]);
    assert!(endpoint.canonical_bounds()[0]<endpoint.canonical_bounds()[1]);
    for end in [0,1] {
        let refused=edge.sample_endpoint(end,0).unwrap();
        assert!(refused.endpoint.is_none());
        assert_eq!(refused.spans,0);
        assert_eq!(refused.reason,"boundary-endpoint-work-limit");
    }
    let restored=crate::source_shared_edge_restore::restore(edge.definition(),
        crate::source_shared_edge_restore::Limits {mapping_cells_per_use:10000,exact_work:100_000_000,driver_cells:10000}).unwrap();
    let restored=restored.edge.expect("root-ended curved edge replay");
    assert_eq!(restored.definition(),edge.definition());
    assert!(!restored.covers_complete_canonical_source());
    let replayed=restored.sample_endpoint(1,3).unwrap().endpoint.expect("replayed curved root endpoint");
    let replayed_canonical=replayed.canonical_bounds();
    assert!(replayed_canonical[0]<std::f64::consts::FRAC_1_SQRT_2 && replayed_canonical[1]>std::f64::consts::FRAC_1_SQRT_2);
    assert_eq!(replayed.parameter_bounds(),endpoint.parameter_bounds());
    let mesh=edge.prepare_boundary_mesh(BoundaryMeshLimits {max_deviation_mm:0.01,cells:4096,spans:100000,max_depth:24}).unwrap();
    assert!(mesh.pending.is_empty(), "{} pending leaves",mesh.pending.len());
    assert!(mesh.segments.len()>1);
    assert_eq!(mesh.vertices[1].canonical,canonical);
    let mut incidence=vec![0usize;mesh.vertices.len()];
    for segment in &mesh.segments {
        assert!(segment.deviation_upper_mm<=0.01);
        for id in segment.vertices {incidence[id]+=1;}
    }
    assert_eq!(incidence[0],1);assert_eq!(incidence[1],1);
    assert!(incidence[2..].iter().all(|n|*n==2));
    let limited=edge.prepare_boundary_mesh(BoundaryMeshLimits {max_deviation_mm:0.000001,cells:1,spans:100000,max_depth:24}).unwrap();
    assert!(!limited.pending.is_empty());
    assert_eq!(limited.cells,1);
    let range=edge.uses()[0].parameter_bounds()[1];
    assert!(range[0]<std::f64::consts::FRAC_1_SQRT_2&&range[1]>std::f64::consts::FRAC_1_SQRT_2);
    assert_eq!(edge.uses()[0].definition(),a.definition());assert_eq!(edge.uses()[1].definition(),b.definition());
    let mut wrong=world.clone();wrong.control_points[1][2]=1e-12;
    assert!(qualify(&wrong,[&a,&b],[false,true],1_000_000).unwrap().edge.is_none());
    }
}
