use super::*;
use nurbs_core::{
    progressive_sweep::{Options, Orientation, Spacing, constant_vector_law},
    surface::{Axis, Surface},
};
fn path(rational: bool, spatial: bool) -> Curve {
    let mut points = vec![
        vec![0., 0., 0.],
        vec![1., 0., 0.],
        vec![2., 1., 0.],
        vec![3., 3., 1.],
        vec![2., 4., 6.],
        vec![-2., 4., -4.],
        vec![-3., 3., -1.],
        vec![-2., 1., 0.],
        vec![-1., 0., 0.],
        vec![0., 0., 0.],
    ];
    if !spatial {
        for p in &mut points {
            p[2] = 0.;
        }
    }
    Curve {
        degree: 9,
        knots: [vec![-3.; 10], vec![5.; 10]].concat(),
        control_points: points,
        weights: if rational {
            vec![1., 1., 1., 1., 1.25, 1.125, 1., 1., 1., 1.]
        } else {
            vec![1.; 10]
        },
        periodic: false,
    }
}
fn material(rational: bool, nonplanar: bool) -> (LoftCap, Vec<Curve>) {
    let r = 1. / 128.;
    let s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-1., -1., 3., 3.],
        knots_v: vec![2., 2., 6., 6.],
        control_points: vec![
            vec![vec![0., -r, -r], vec![0., -r, r]],
            vec![
                vec![0., r, -r],
                vec![if nonplanar { 1. / 65536. } else { 0. }, r, r],
            ],
        ],
        weights: if rational {
            vec![vec![1., 1.125], vec![1.25, 1.]]
        } else {
            vec![vec![1.; 2]; 2]
        },
        periodic_u: false,
        periodic_v: false,
    };
    let profiles = vec![
        s.iso(Axis::V, 2.).unwrap(),
        s.iso(Axis::U, 3.).unwrap(),
        s.iso(Axis::V, 6.).unwrap().reverse().unwrap(),
        s.iso(Axis::U, -1.).unwrap().reverse().unwrap(),
    ];
    let uv = [[-1., 2.], [3., 2.], [3., 6.], [-1., 6.]];
    let trims = vec![
        (0..4)
            .map(|i| Curve {
                degree: 1,
                knots: vec![7., 7., 11., 11.],
                control_points: vec![uv[i].to_vec(), uv[(i + 1) % 4].to_vec()],
                weights: vec![1.; 2],
                periodic: false,
            })
            .collect(),
    ];
    (LoftCap { surface: s, trims }, profiles)
}
fn limits() -> ClosedLimits {
    ClosedLimits {
        material: MaterialLimits {
            pairs: 10000,
            cells: 100000,
            domain_cells: 1000000,
        },
        ideal: IdealLimits {
            cells: 300000,
            exact_work: 1000000,
            projection: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        },
        wall_faces: 1024,
        boundary: Limits {
            source_cells: 100000,
            source_products: 1000000,
            source_exact_work: 1000000,
            regularity_cells: 100000,
            linear_cells: 20000,
            volume: volume_validity::Limits {
                boundary: crate::boundary_embedding::Limits {
                    exact_work: 1000000,
                    trim_pairs: 100000,
                    trim_cells: 100000,
                    trim_domain_cells: 1000000,
                    spans: 1000,
                    contacts: crate::face_contacts::Limits {
                        pairs: 100000,
                        cells: 100000,
                        domain_cells: 1000000,
                        cells_per_pair: 1000,
                        domain_cells_per_pair: 10000,
                    },
                },
                nesting_pairs: 10000,
                nesting_cells: 100000,
                nesting_domain_cells: 1000000,
                orientation_cells: 100000,
                orientation_domain_cells: 1000000,
                orientation_spans: 1000,
            },
        },
    }
}
fn options(spatial: bool, spacing: Spacing) -> Options {
    Options {
        normal: [0., 0., 1.],
        orientation: if spatial {
            Orientation::RotationMinimizing
        } else {
            Orientation::FixedNormal
        },
        spacing,
        initial_sections: 33,
        max_sections: 129,
        max_deviation: 0.05,
    }
}
#[test]
fn original_interior_fold_is_refused_even_when_the_same_retained_boundary_is_a_valid_solid() {
    let (mut material, _) = material(false, false);
    let r = 1. / 128.;
    // A cubic chart with straight, monotone square boundaries. The two
    // interiors below share every boundary coefficient, curve and preview.
    let grid = [-r, -r / 2., r / 2., r];
    material.surface = Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: grid
            .iter()
            .map(|y| grid.iter().map(|z| vec![0., *y, *z]).collect())
            .collect(),
        weights: vec![vec![1.; 4]; 4],
        periodic_u: false,
        periodic_v: false,
    };
    let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    material.trims = vec![
        (0..4)
            .map(|i| Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                control_points: vec![uv[i].to_vec(), uv[(i + 1) % 4].to_vec()],
                weights: vec![1.; 2],
                periodic: false,
            })
            .collect(),
    ];
    let profiles = vec![
        material.surface.iso(Axis::V, 0.).unwrap(),
        material.surface.iso(Axis::U, 1.).unwrap(),
        material
            .surface
            .iso(Axis::V, 1.)
            .unwrap()
            .reverse()
            .unwrap(),
        material
            .surface
            .iso(Axis::U, 0.)
            .unwrap()
            .reverse()
            .unwrap(),
    ];
    let path = path(false, true);
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        options(true, Spacing::Parameter),
    )
    .unwrap()
    .with_spatial_rmf_error_limits(1024, 100000, 1000000)
    .unwrap();
    let original_material = material.clone();
    let valid = assert_body(
        construct_closed_with_ideal_atlas(&sweep, &[4], 33, &material, 1e-8, 0.05, limits())
            .unwrap(),
    );
    assert!(valid.report().volume.as_ref().unwrap().proven);
    // Reverse the chart's transverse derivative in its interior while leaving
    // the entire authored boundary untouched. This is not a boundary defect.
    for j in 1..3 {
        material.surface.control_points[1][j][1] = 8. * r;
        material.surface.control_points[2][j][1] = -8. * r;
    }
    assert_eq!(
        original_material.surface.iso(Axis::V, 0.).unwrap(),
        material.surface.iso(Axis::V, 0.).unwrap()
    );
    assert_eq!(
        original_material.surface.iso(Axis::V, 1.).unwrap(),
        material.surface.iso(Axis::V, 1.).unwrap()
    );
    assert_eq!(
        original_material.surface.iso(Axis::U, 0.).unwrap(),
        material.surface.iso(Axis::U, 0.).unwrap()
    );
    assert_eq!(
        original_material.surface.iso(Axis::U, 1.).unwrap(),
        material.surface.iso(Axis::U, 1.).unwrap()
    );
    match construct_closed_with_ideal_atlas(&sweep, &[4], 33, &material, 1e-8, 0.05, limits())
        .unwrap()
    {
        ClosedAttempt::Certified(_) => panic!("folded original material received body credit"),
        ClosedAttempt::Refused(r) => {
            assert!(r.material.certified && r.material.bound_spans == 4);
            let ideal = r.ideal_volume.as_ref().unwrap();
            assert!(!ideal.embedding_certified && !ideal.parameter_topology_preserved);
            assert!(!r.certified && r.retained_walls.is_none() && r.volume.is_none());
            eprintln!(
                "folded original interior refused: {:?}; source cells={}",
                r.reason, ideal.cells
            );
        }
    }
}
fn assert_body(attempt: ClosedAttempt) -> Box<ClosedBody> {
    match attempt {
        ClosedAttempt::Certified(body) => {
            let r = body.report();
            assert!(r.certified && r.material.certified && r.continuous_bound && r.within_budget);
            assert!(r.ideal_volume.as_ref().unwrap().embedding_certified);
            assert!(r.ideal_volume.as_ref().unwrap().regularity_certified);
            assert!(
                r.ideal_volume
                    .as_ref()
                    .unwrap()
                    .parameter_topology_preserved
            );
            assert!(r.volume.as_ref().unwrap().proven);
            assert!(r.retained_walls.as_ref().unwrap().certified);
            assert_eq!(
                body.model().faces.len(),
                4 * (r.wall.as_ref().unwrap().sections - 1)
            );
            assert_eq!(
                r.retained_walls.as_ref().unwrap().inspected_faces,
                body.model().faces.len()
            );
            body.model().validate().unwrap();
            body
        }
        ClosedAttempt::Refused(r) => panic!(
            "closed refused: {:?}; material={} spans={} ideal={:?}; wall={:?}; E={:?}; owned={:?}; regular={:?}; volume={:?}",
            r.reason,
            r.material.certified,
            r.material.bound_spans,
            r.ideal_volume,
            r.wall.as_ref().map(|w| (
                w.continuous_bound,
                w.continuous_error_upper,
                w.error_certificate_reason
            )),
            r.boundary_error_upper,
            r.retained_walls.as_ref().map(|w| w.reason),
            r.regularity.as_ref().map(|r| &r.unresolved_faces),
            r.volume.as_ref().map(|v| (
                v.boundary.proven,
                v.boundary.intersections.pairs.next_pair,
                v.boundary
                    .intersections
                    .pairs
                    .pairs
                    .iter()
                    .filter(|p| p.reason == "pair-unresolved")
                    .map(|p| p.faces)
                    .collect::<Vec<_>>(),
                v.nesting.as_ref().map(|n| n.roles_consistent),
                v.orientations
                    .iter()
                    .map(|o| (o.expected_outward, o.outward))
                    .collect::<Vec<_>>()
            ))
        ),
    }
}
#[test]
fn closed_original_spatial_nonplanar_material_is_owned_by_a_certified_periodic_solid() {
    let (material, profiles) = material(false, true);
    let path = path(false, true);
    let original = (
        path.clone(),
        material.surface.clone(),
        material.trims.clone(),
        profiles.clone(),
    );
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        options(true, Spacing::Parameter),
    )
    .unwrap()
    .with_spatial_rmf_error_limits(1024, 100000, 1000000)
    .unwrap();
    let body = assert_body(
        construct_closed_with_ideal_atlas(&sweep, &[4], 33, &material, 1e-8, 0.05, limits())
            .unwrap(),
    );
    eprintln!(
        "closed material E={:?} source cells={} products={} ownership={} ideal={:?}",
        body.report().boundary_error_upper,
        body.report().source_cells,
        body.report().source_products,
        body.report().ownership_exact_work,
        body.report().ideal_volume
    );
    #[cfg(feature = "codec")]
    if let Ok(destination) = std::env::var("OSV_CLOSED_MATERIAL_FIXTURE") {
        let l = limits();
        let v = l.boundary.volume;
        let b = v.boundary;
        let packet = value_codec::json!({"op":"brep_sweep_closed_material_construct",
            "source":{"profiles":profiles,"path":path,"scale":scale,"twist":twist,
                "orientation":"rmf","spacing":"parameter","normal":[0.,0.,1.],
                "initial_sections":33,"max_sections":129,"max_deviation":0.05,
                "rmf_transport_steps":1024,"error_max_cells":100000,"error_max_products":1000000},
            "material":{"surface":material.surface,"trims":material.trims},
            "loopSizes":[4],"sections":33,"toleranceUv":1e-8,"maxDeviation":0.05,
            "idealBody":{"method":"atlas","projection":l.ideal.projection,"maxCells":l.ideal.cells,"maxExactWork":l.ideal.exact_work},
            "limits":{"sourceCells":l.boundary.source_cells,"sourceProducts":l.boundary.source_products,
                "sourceExactWork":l.boundary.source_exact_work,"regularityCells":l.boundary.regularity_cells,"linearCells":l.boundary.linear_cells,
                "maxWallFaces":l.wall_faces,"material":{"maxPairs":l.material.pairs,"maxCells":l.material.cells,"maxDomainCells":l.material.domain_cells},
                "volume":{"maxExactWork":b.exact_work,"maxTrimPairs":b.trim_pairs,"maxTrimCells":b.trim_cells,"maxTrimDomainCells":b.trim_domain_cells,
                    "maxSpans":b.spans,"maxPairs":b.contacts.pairs,"maxCells":b.contacts.cells,"maxDomainCells":b.contacts.domain_cells,
                    "cellsPerPair":b.contacts.cells_per_pair,"domainCellsPerPair":b.contacts.domain_cells_per_pair,
                    "nestingPairs":v.nesting_pairs,"nestingCells":v.nesting_cells,"nestingDomainCells":v.nesting_domain_cells,
                    "orientationCells":v.orientation_cells,"orientationDomainCells":v.orientation_domain_cells,"orientationSpans":v.orientation_spans}}});
        std::fs::write(destination, value_codec::to_string(&packet).unwrap()).unwrap();
    }
    assert_eq!(
        (
            path.clone(),
            material.surface.clone(),
            material.trims.clone(),
            profiles.clone()
        ),
        original
    );
}

#[test]
fn closed_owner_refuses_invalid_original_material_and_each_incomplete_stage() {
    let (material, profiles) = material(false, true);
    let path = path(false, true);
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        options(true, Spacing::Parameter),
    )
    .unwrap()
    .with_spatial_rmf_error_limits(1024, 100000, 1000000)
    .unwrap();
    let full = assert_body(
        construct_closed_with_ideal_atlas(&sweep, &[4], 33, &material, 1e-8, 0.05, limits())
            .unwrap(),
    );
    let r = full.report();
    let mut cases = Vec::new();
    let mut l = limits();
    l.material.pairs = 0;
    cases.push(l);
    let mut l = limits();
    l.material.cells = 0;
    cases.push(l);
    let mut l = limits();
    l.material.domain_cells = 0;
    cases.push(l);
    let mut l = limits();
    l.ideal.cells = r.ideal_volume.as_ref().unwrap().cells - 1;
    cases.push(l);
    let mut l = limits();
    l.ideal.exact_work = r.ideal_volume.as_ref().unwrap().exact_work - 1;
    cases.push(l);
    let mut l = limits();
    l.boundary.source_cells = 0;
    cases.push(l);
    let mut l = limits();
    l.boundary.source_products = r.source_products - 1;
    cases.push(l);
    let mut l = limits();
    l.boundary.source_exact_work = r.ownership_exact_work - 1;
    cases.push(l);
    let mut l = limits();
    l.boundary.regularity_cells = 0;
    cases.push(l);
    let mut l = limits();
    l.boundary.volume.boundary.exact_work = 0;
    cases.push(l);
    let mut l = limits();
    l.boundary.volume.boundary.contacts.pairs = 1;
    cases.push(l);
    let mut l = limits();
    l.boundary.volume.orientation_cells = 0;
    cases.push(l);
    let mut l = limits();
    l.wall_faces = 127;
    cases.push(l);
    for l in cases {
        let a =
            construct_closed_with_ideal_atlas(&sweep, &[4], 33, &material, 1e-8, 0.05, l).unwrap();
        let ClosedAttempt::Refused(short) = a else {
            panic!("incomplete stage returned a Solid");
        };
        assert!(!short.certified && short.reason.is_some());
        assert!(short.source_cells <= l.boundary.source_cells);
        assert!(short.source_products <= l.boundary.source_products);
        assert!(short.ownership_exact_work <= l.boundary.source_exact_work);
    }
    let mut altered = material.clone();
    altered.surface.control_points[1][1][0] += 1. / 65536.;
    let ClosedAttempt::Refused(a) =
        construct_closed_with_ideal_atlas(&sweep, &[4], 33, &altered, 1e-8, 0.05, limits())
            .unwrap()
    else {
        panic!("different original carrier received a Solid");
    };
    assert!(!a.material.certified && a.ideal_volume.is_none());
    let mut gap = material.clone();
    gap.trims[0][0].control_points[1][0] -= 1e-12;
    let ClosedAttempt::Refused(a) =
        construct_closed_with_ideal_atlas(&sweep, &[4], 33, &gap, 1e-8, 0.05, limits()).unwrap()
    else {
        panic!("sub-tolerance original gap received a Solid");
    };
    assert!(!a.material.certified && a.ideal_volume.is_none());
    let mut near = path.clone();
    near.control_points[9][0] = 1e-15;
    let near_sweep = MultiSweep::new(
        &profiles,
        &near,
        &scale,
        &twist,
        options(true, Spacing::Parameter),
    )
    .unwrap()
    .with_spatial_rmf_error_limits(1024, 100000, 1000000)
    .unwrap();
    let ClosedAttempt::Refused(a) =
        construct_closed_with_ideal_atlas(&near_sweep, &[4], 33, &material, 1e-8, 0.05, limits())
            .unwrap()
    else {
        panic!("sub-tolerance source seam received a Solid");
    };
    assert!(
        a.material.certified
            && !a.ideal_volume.unwrap().embedding_certified
            && a.wall.is_none()
            && a.reason == Some("ideal-volume-exact-source-seam-unproved")
    );
    // Algebraic continuation outside the carrier chart is not a filled-region
    // certificate, even when that larger UV contour is simple and closed.
    let uv = [[-2., 1.], [4., 1.], [4., 7.], [-2., 7.]];
    let world = |p: [f64; 2]| {
        let u = (p[0] + 1.) / 4.;
        let v = (p[1] - 2.) / 4.;
        let r = 1. / 128.;
        [u * v / 65536., -r + 2. * r * u, -r + 2. * r * v]
    };
    let mut outside = material.clone();
    outside.trims = vec![
        (0..4)
            .map(|i| Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                control_points: vec![uv[i].to_vec(), uv[(i + 1) % 4].to_vec()],
                weights: vec![1.; 2],
                periodic: false,
            })
            .collect(),
    ];
    let outside_profiles = (0..4)
        .map(|i| nurbs_core::primitives::line(world(uv[i]), world(uv[(i + 1) % 4])).unwrap())
        .collect::<Vec<_>>();
    let outside_sweep = MultiSweep::new(
        &outside_profiles,
        &path,
        &scale,
        &twist,
        options(true, Spacing::Parameter),
    )
    .unwrap()
    .with_spatial_rmf_error_limits(1024, 100000, 1000000)
    .unwrap();
    let ClosedAttempt::Refused(a) =
        construct_closed_with_ideal_atlas(&outside_sweep, &[4], 33, &outside, 1e-8, 0.05, limits())
            .unwrap()
    else {
        panic!("outside-chart material received an ideal-body certificate");
    };
    assert_eq!(a.material.trim.unwrap().valid, Some(true));
    assert!(!a.material.certified && a.ideal_volume.is_none());
    for sizes in [vec![usize::MAX, 1], vec![3], vec![0, 4]] {
        assert!(
            construct_closed_with_ideal_atlas(&sweep, &sizes, 33, &material, 1e-8, 0.05, limits())
                .is_err()
        );
    }
}

fn qualify_spatial_case(rational: bool, spacing: Spacing) {
    let (material, profiles) = material(true, true);
    let original = (
        material.surface.clone(),
        material.trims.clone(),
        profiles.clone(),
    );
    let scalar = |points: [f64; 5], domain: [f64; 2]| Curve {
        degree: 4,
        knots: [vec![domain[0]; 5], vec![domain[1]; 5]].concat(),
        control_points: points.map(|x| vec![x, 0., 0.]).to_vec(),
        weights: vec![1.; 5],
        periodic: false,
    };
    let scale = scalar([1., 1., 1.125, 1., 1.], [2., 6.]);
    let twist = scalar([0., 0., 1. / 64., 0., 0.], [7., 11.]);
    let axes = constant_vector_law([1., 1.125, 0.875]).unwrap();
    let center = constant_vector_law([1. / 1024., 1. / 2048., 0.]).unwrap();
    let path = path(rational, true);
    let original_path = path.clone();
    let sweep = MultiSweep::new(&profiles, &path, &scale, &twist, options(true, spacing))
        .unwrap()
        .with_affine_laws(&axes, &center)
        .unwrap()
        .with_spatial_rmf_error_limits(2048, 100000, 1000000)
        .unwrap();
    let sections = if matches!(spacing, Spacing::ArcLength { .. }) {
        129
    } else {
        33
    };
    let body = assert_body(
        construct_closed_with_ideal_atlas(&sweep, &[4], sections, &material, 1e-8, 0.05, limits())
            .unwrap(),
    );
    assert_eq!(body.report().material.bound_spans, 4);
    eprintln!(
        "owned spatial rational={rational}/{spacing:?} E={:?} source={} ideal={:?}",
        body.report().boundary_error_upper,
        body.report().source_cells,
        body.report().ideal_volume
    );
    assert_eq!(body.model().bodies.len(), 1);
    assert_eq!(body.model().shells.len(), 1);
    assert!(body.model().bodies[0].inner_shells.is_empty());
    assert_eq!(
        (
            material.surface.clone(),
            material.trims.clone(),
            profiles.clone()
        ),
        original
    );
    assert_eq!(path, original_path);
}
#[test]
fn closed_owned_spatial_polynomial_parameter() {
    qualify_spatial_case(false, Spacing::Parameter);
}
#[test]
fn closed_owned_spatial_polynomial_arc() {
    qualify_spatial_case(
        false,
        Spacing::ArcLength {
            tolerance: 0.001,
            max_cells: 100000,
        },
    );
}
#[test]
fn closed_owned_spatial_rational_parameter() {
    qualify_spatial_case(true, Spacing::Parameter);
}
#[test]
fn closed_owned_spatial_rational_arc() {
    qualify_spatial_case(
        true,
        Spacing::ArcLength {
            tolerance: 0.001,
            max_cells: 100000,
        },
    );
}

fn hollow_material() -> (LoftCap, Vec<Curve>) {
    let (mut material, mut profiles) = material(false, true);
    let uv = [[-0.5, 3.], [1.5, 3.], [1.5, 5.], [-0.5, 5.]];
    let world = |p: [f64; 2]| {
        let u = (p[0] + 1.) / 4.;
        let v = (p[1] - 2.) / 4.;
        let r = 1. / 128.;
        [u * v / 65536., -r + 2. * r * u, -r + 2. * r * v]
    };
    let mut inner = Vec::new();
    for i in (0..4).rev() {
        let a = uv[(i + 1) % 4];
        let b = uv[i];
        profiles.push(nurbs_core::primitives::line(world(a), world(b)).unwrap());
        inner.push(Curve {
            degree: 1,
            knots: vec![7., 7., 11., 11.],
            control_points: vec![a.to_vec(), b.to_vec()],
            weights: vec![1.; 2],
            periodic: false,
        });
    }
    material.trims.push(inner);
    (material, profiles)
}
fn qualify_hollow_case(mode: usize, spacing: Spacing) {
    let (material, profiles) = hollow_material();
    let path = path(false, mode != 0);
    let axis = Curve {
        degree: 8,
        knots: [vec![2.; 9], vec![6.; 9]].concat(),
        control_points: path
            .control_points
            .windows(2)
            .map(|p| (0..3).map(|k| 9. * (p[1][k] - p[0][k])).collect())
            .collect(),
        weights: vec![1.; 9],
        periodic: false,
    };
    let mut guide = path.clone();
    for p in &mut guide.control_points {
        p[2] += 1.;
    }
    let normal = constant_vector_law([0., 0., 1.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let mut o = options(false, spacing);
    if mode == 1 {
        o.orientation = Orientation::Fixed;
    }
    let mut sweep = MultiSweep::new(&profiles, &path, &scale, &twist, o).unwrap();
    if mode == 1 {
        sweep = sweep.with_frame_laws(&axis, &normal).unwrap();
    }
    if mode == 2 {
        sweep = sweep.with_orientation_guide(&guide).unwrap();
    }
    let sections = if mode == 2 || matches!(spacing, Spacing::ArcLength { .. }) {
        129
    } else {
        33
    };
    let attempt = construct_closed_with_ideal_atlas(
        &sweep,
        &[4, 4],
        sections,
        &material,
        1e-8,
        0.05,
        limits(),
    )
    .unwrap();
    let body = match attempt {
        ClosedAttempt::Certified(body) => body,
        ClosedAttempt::Refused(r) => {
            eprintln!("hollow mode={mode}/{spacing:?}");
            assert_body(ClosedAttempt::Refused(r))
        }
    };
    let r = body.report();
    assert!(r.certified && r.material.certified && r.continuous_bound);
    assert_eq!(
        r.material.trim.as_ref().unwrap().winding,
        vec![Some(1), Some(-1)]
    );
    assert_eq!(r.material.bound_spans, 8);
    assert!(
        r.ideal_volume.as_ref().unwrap().embedding_certified && r.volume.as_ref().unwrap().proven
    );
    assert_eq!(body.model().faces.len(), 8 * (sections - 1));
    assert_eq!(
        r.retained_walls.as_ref().unwrap().inspected_faces,
        body.model().faces.len()
    );
    assert_eq!(body.model().shells.len(), 2);
    assert_eq!(body.model().bodies.len(), 1);
    assert_eq!(body.model().bodies[0].inner_shells.len(), 1);
    eprintln!(
        "owned hollow mode={mode}/{spacing:?} E={:?} source={}",
        r.boundary_error_upper, r.source_cells
    );
}
#[test]
fn closed_owned_hollow_fixed_parameter() {
    qualify_hollow_case(0, Spacing::Parameter);
}
#[test]
fn closed_owned_hollow_fixed_arc() {
    qualify_hollow_case(
        0,
        Spacing::ArcLength {
            tolerance: 0.001,
            max_cells: 100000,
        },
    );
}
#[test]
fn closed_owned_hollow_authored_parameter() {
    qualify_hollow_case(1, Spacing::Parameter);
}
#[test]
fn closed_owned_hollow_authored_arc() {
    qualify_hollow_case(
        1,
        Spacing::ArcLength {
            tolerance: 0.001,
            max_cells: 100000,
        },
    );
}
#[test]
fn closed_owned_hollow_guided_parameter() {
    qualify_hollow_case(2, Spacing::Parameter);
}
#[test]
fn closed_owned_hollow_guided_arc() {
    qualify_hollow_case(
        2,
        Spacing::ArcLength {
            tolerance: 0.001,
            max_cells: 100000,
        },
    );
}
#[test]
fn closed_original_material_reverse_roles_and_wrong_hole_orientation() {
    let (material, profiles) = hollow_material();
    let path = path(false, false);
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let mut reverse = material.clone();
    for ring in &mut reverse.trims {
        *ring = ring.iter().rev().map(|c| c.reverse().unwrap()).collect();
    }
    let reverse_profiles = profiles
        .chunks(4)
        .flat_map(|ring| ring.iter().rev().map(|c| c.reverse().unwrap()))
        .collect::<Vec<_>>();
    let sweep = MultiSweep::new(
        &reverse_profiles,
        &path,
        &scale,
        &twist,
        options(false, Spacing::Parameter),
    )
    .unwrap();
    let ClosedAttempt::Certified(body) =
        construct_closed_with_ideal_atlas(&sweep, &[4, 4], 33, &reverse, 1e-8, 0.05, limits())
            .unwrap()
    else {
        panic!("reversed original material roles must preserve the Solid");
    };
    assert_eq!(
        body.report().material.trim.as_ref().unwrap().winding,
        vec![Some(-1), Some(1)]
    );
    assert!(body.report().volume.as_ref().unwrap().proven);
    let mut wrong = material.clone();
    wrong.trims[1] = wrong.trims[1]
        .iter()
        .rev()
        .map(|c| c.reverse().unwrap())
        .collect();
    let wrong_profiles = profiles[..4]
        .iter()
        .cloned()
        .chain(profiles[4..].iter().rev().map(|c| c.reverse().unwrap()))
        .collect::<Vec<_>>();
    let sweep = MultiSweep::new(
        &wrong_profiles,
        &path,
        &scale,
        &twist,
        options(false, Spacing::Parameter),
    )
    .unwrap();
    let ClosedAttempt::Refused(r) =
        construct_closed_with_ideal_atlas(&sweep, &[4, 4], 33, &wrong, 1e-8, 0.05, limits())
            .unwrap()
    else {
        panic!("same-winding hole became a Solid");
    };
    assert!(!r.material.certified && r.ideal_volume.is_none());
}

#[test]
fn closed_original_multispan_world_and_uv_chains_keep_exact_material_ownership() {
    let (mut material, curves) = material(false, true);
    let mut uv = material.trims[0]
        .iter()
        .map(|c| c.control_points[0].clone())
        .collect::<Vec<_>>();
    uv.push(uv[0].clone());
    material.trims = vec![vec![Curve {
        degree: 1,
        knots: vec![7., 7., 8., 9., 10., 11., 11.],
        control_points: uv,
        weights: vec![1.; 5],
        periodic: false,
    }]];
    let mut points = curves
        .iter()
        .map(|c| c.control_points[0].clone())
        .collect::<Vec<_>>();
    points.push(points[0].clone());
    let profiles = vec![Curve {
        degree: 1,
        knots: vec![2., 2., 3., 4., 5., 6., 6.],
        control_points: points,
        weights: vec![1.; 5],
        periodic: false,
    }];
    let original = (profiles.clone(), material.trims.clone());
    let path = path(false, true);
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        options(true, Spacing::Parameter),
    )
    .unwrap()
    .with_spatial_rmf_error_limits(1024, 100000, 1000000)
    .unwrap();
    let body = assert_body(
        construct_closed_with_ideal_atlas(&sweep, &[1], 33, &material, 1e-8, 0.05, limits())
            .unwrap(),
    );
    assert_eq!(body.report().material.bound_spans, 4);
    assert_eq!(body.report().material.profiles.len(), 1);
    assert_eq!((profiles.clone(), material.trims.clone()), original);
}
