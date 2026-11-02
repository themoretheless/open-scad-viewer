use super::*;
use nurbs_core::{
    progressive_sweep::{Options, Orientation, Spacing, constant_vector_law},
    surface::Surface,
};
fn limits() -> OriginalClassifiedCappedMaterialLimits {
    OriginalClassifiedCappedMaterialLimits {
        volume: OriginalClassifiedVolumeLimits {
            charts: OriginalSmoothVolumeLimits {
                order: 2,
                cells: 1000000,
                exact_work: 1000000,
                products: 1000000,
            },
            steps: 1000000,
        },
        material: MaterialLimits {
            pairs: 100000,
            cells: 100000,
            domain_cells: 1000000,
        },
    }
}
fn source() -> (Vec<Curve>, [LoftCap; 2]) {
    let outer = [[0., 0.], [0.5, 0.], [1., 0.], [1., 1.], [0., 1.], [0., 0.]];
    let hole = [
        [0.25, 0.25],
        [0.25, 0.5],
        [0.25, 0.75],
        [0.75, 0.75],
        [0.75, 0.25],
        [0.25, 0.25],
    ];
    let profiles = [outer, hole]
        .iter()
        .flat_map(|r| {
            r.windows(2).map(|p| {
                nurbs_core::primitives::line([p[0][0], p[0][1], 0.], [p[1][0], p[1][1], 0.])
                    .unwrap()
            })
        })
        .collect();
    // Bernstein coefficients of 12 x(x-1/4)(x-3/4)(x-1). The end cap's
    // interior differs, but this polynomial vanishes on EVERY material trace.
    let q = [0., -9. / 16., 5. / 4., -9. / 16., 0.];
    let caps = std::array::from_fn(|end| {
        let (u, v) = if end == 0 {
            ([-3., 5.], [7., 11.])
        } else {
            ([17., 25.], [-7., 1.])
        };
        let surface = Surface {
            degree_u: 4,
            degree_v: 4,
            knots_u: [vec![u[0]; 5], vec![u[1]; 5]].concat(),
            knots_v: [vec![v[0]; 5], vec![v[1]; 5]].concat(),
            control_points: (0..5)
                .map(|i| {
                    (0..5)
                        .map(|j| {
                            vec![
                                i as f64 / 4.,
                                j as f64 / 4.,
                                if end == 0 { 0. } else { q[i] * q[j] / 64. },
                            ]
                        })
                        .collect()
                })
                .collect(),
            weights: vec![vec![if end == 0 { 1. } else { 2. }; 5]; 5],
            periodic_u: false,
            periodic_v: false,
        };
        let trims = [outer, hole]
            .iter()
            .map(|r| {
                r.windows(2)
                    .map(|p| Curve {
                        degree: 1,
                        knots: vec![3., 3., 7., 7.],
                        control_points: p
                            .iter()
                            .map(|p| vec![u[0] + (u[1] - u[0]) * p[0], v[0] + (v[1] - v[0]) * p[1]])
                            .collect(),
                        weights: vec![if end == 0 { 1. } else { 2. }; 2],
                        periodic: false,
                    })
                    .collect()
            })
            .collect();
        LoftCap { surface, trims }
    });
    (profiles, caps)
}
#[test]
fn different_nonplanar_caps_bind_the_same_hollow_material_and_exact_wall_jets() {
    let (profiles, caps) = source();
    assert_ne!(
        caps[0].surface.control_points,
        caps[1].surface.control_points
    );
    let before = (
        profiles.clone(),
        caps.each_ref()
            .map(|c| (c.surface.clone(), c.trims.clone())),
    );
    let path = nurbs_core::primitives::line([0.; 3], [0., 0., 2.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let axis = constant_vector_law([0., 0., 1.]).unwrap();
    let normal = constant_vector_law([1., 0., 0.]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        Options {
            normal: [1., 0., 0.],
            orientation: Orientation::Fixed,
            spacing: Spacing::Parameter,
            initial_sections: 5,
            max_sections: 5,
            max_deviation: 0.05,
        },
    )
    .unwrap()
    .with_frame_laws(&axis, &normal)
    .unwrap();
    let classes = [
        OriginalProfileClass::G2,
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
    ]
    .repeat(2);
    let projection = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    let run = |caps, limits| {
        certify_original_classified_capped_material(
            &sweep,
            &[5, 5],
            caps,
            &classes,
            projection,
            1e-8,
            limits,
        )
        .unwrap()
    };
    let body = match run([&caps[0], &caps[1]], limits()) {
        OriginalClassifiedCappedMaterialAttempt::Certified(body) => body,
        OriginalClassifiedCappedMaterialAttempt::Refused(r) => panic!(
            "capped material: {:?} {:?}, work={:?}",
            r.reason, r.volume, r.work
        ),
    };
    assert!(std::ptr::eq(body.caps()[0], &caps[0]) && std::ptr::eq(body.caps()[1], &caps[1]));
    assert!(std::ptr::eq(body.volume().sweep(), &sweep));
    for (end, bindings) in body.bindings().into_iter().enumerate() {
        assert!(std::ptr::eq(body.volume().caps()[end], &caps[end].surface));
        assert_eq!(
            body.material_reports()[end].trim.as_ref().unwrap().winding,
            vec![Some(1), Some(-1)]
        );
        assert_eq!(bindings.len(), 10);
        for (i, b) in bindings.iter().enumerate() {
            assert_eq!(
                (b.ring(), b.profile(), b.trim(), b.kind()),
                (
                    i / 5,
                    (i, 0),
                    (i % 5, 0),
                    MaterialSpanBindingKind::NormalizedMap
                )
            );
        }
    }
    assert_eq!(body.boundary_classes(), classes);
    assert_eq!(body.cap_wall_continuity(), "C0");
    let work = body.work();
    let exact = OriginalClassifiedCappedMaterialLimits {
        volume: OriginalClassifiedVolumeLimits {
            charts: OriginalSmoothVolumeLimits {
                cells: work.cells,
                exact_work: work.exact_work,
                products: work.products,
                ..limits().volume.charts
            },
            steps: body.volume().report().profiles.as_ref().unwrap().steps,
        },
        material: MaterialLimits {
            pairs: work.material_pairs,
            cells: work.material_cells,
            domain_cells: work.material_domain_cells,
        },
    };
    assert!(matches!(
        run([&caps[0], &caps[1]], exact),
        OriginalClassifiedCappedMaterialAttempt::Certified(_)
    ));
    for dimension in 0..6 {
        let mut short = exact;
        match dimension {
            0 => short.volume.charts.cells -= 1,
            1 => short.volume.charts.exact_work -= 1,
            2 => short.volume.charts.products -= 1,
            3 => short.material.pairs -= 1,
            4 => short.material.cells -= 1,
            _ => short.material.domain_cells -= 1,
        }
        let OriginalClassifiedCappedMaterialAttempt::Refused(r) = run([&caps[0], &caps[1]], short)
        else {
            panic!("short shared work admitted")
        };
        assert!(
            r.work.cells <= short.volume.charts.cells
                && r.work.exact_work <= short.volume.charts.exact_work
                && r.work.products <= short.volume.charts.products
                && r.work.material_pairs <= short.material.pairs
                && r.work.material_cells <= short.material.cells
                && r.work.material_domain_cells <= short.material.domain_cells
        );
    }
    let mut changed = LoftCap {
        surface: caps[1].surface.clone(),
        trims: caps[1].trims.clone(),
    };
    changed.trims[0][0].weights[1] *= 2.;
    let OriginalClassifiedCappedMaterialAttempt::Refused(r) = run([&caps[0], &changed], limits())
    else {
        panic!("independent cap trace clock admitted")
    };
    assert_eq!(r.reason, Some("original-material-boundary-unproved"));
    assert!(r.volume.is_none());
    // Each cap individually binds the complete SAME world boundary. The end
    // carrier expands XY by two while its material UV region shrinks by two.
    // These are different normalized regions: interpolation may NOT inherit
    // the original constant profiles from two independent boundary proofs.
    let mut different_region = LoftCap {
        surface: caps[1].surface.clone(),
        trims: caps[1].trims.clone(),
    };
    for p in different_region.surface.control_points.iter_mut().flatten() {
        p[0] *= 2.;
        p[1] *= 2.;
        p[2] = 0.;
    }
    for p in different_region
        .trims
        .iter_mut()
        .flatten()
        .flat_map(|c| &mut c.control_points)
    {
        p[0] = 17. + (p[0] - 17.) / 2.;
        p[1] = -7. + (p[1] + 7.) / 2.;
    }
    let OriginalClassifiedCappedMaterialAttempt::Refused(r) =
        run([&caps[0], &different_region], limits())
    else {
        panic!("independent material UV regions admitted")
    };
    assert_eq!(
        r.reason,
        Some("original-capped-material-common-region-unproved")
    );
    assert_eq!(r.materials.len(), 2);
    assert!(
        r.materials
            .iter()
            .all(|m| m.certified && m.bound_spans == 10)
    );
    assert!(r.volume.is_none());
    let mut material_short = limits();
    let first = body.material_reports()[0].trim.as_ref().unwrap();
    material_short.material.pairs = first.pairs;
    let OriginalClassifiedCappedMaterialAttempt::Refused(r) =
        run([&caps[0], &caps[1]], material_short)
    else {
        panic!("cap audit budget reset")
    };
    assert_eq!(r.reason, Some("original-material-region-work-limit"));
    assert!(r.work.material_pairs <= material_short.material.pairs && r.volume.is_none());
    assert_eq!(profiles, before.0);
    for (i, c) in caps.iter().enumerate() {
        assert_eq!((&c.surface, &c.trims), (&before.1[i].0, &before.1[i].1));
    }
    eprintln!("different original capped hollow material: {work:?}");
}
