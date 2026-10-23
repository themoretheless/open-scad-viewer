use nurbs_core::{
    surface::Surface,
    worm_wheel::{
        AxialSource, Flank, RootPart, Spec, axial_instance, axial_root_chart,
        axial_stock_intersection, axial_tip_chart, place_axial_surfaces, qualify_axial_instance,
        regrid_axial_instance, regrid_axial_source,
    },
};
fn sources() -> Vec<AxialSource> {
    let s = Spec::default();
    let d = axial_stock_intersection(s, Flank::Descending, [-2., 2.], 21., 128).unwrap();
    let a = axial_stock_intersection(s, Flank::Ascending, [-2., 2.], 21., 128).unwrap();
    let tip = axial_tip_chart(&d, &a, 32).unwrap();
    vec![
        AxialSource::Root(
            axial_root_chart(s, RootPart::CrestEdge(Flank::Descending), [-2., 2.]).unwrap(),
        ),
        AxialSource::Working(d),
        AxialSource::Tip(tip),
        AxialSource::Working(a),
        AxialSource::Root(
            axial_root_chart(s, RootPart::CrestEdge(Flank::Ascending), [-2., 2.]).unwrap(),
        ),
        AxialSource::Root(axial_root_chart(s, RootPart::CrestCylinder, [-2., 2.]).unwrap()),
    ]
}
fn ys() -> Vec<f64> {
    (0..=16).map(|i| -2. + i as f64 / 4.).collect()
}
fn boundary(s: &[Surface], end: bool, iv: usize) -> &Vec<Vec<f64>> {
    let nv = 16;
    let iu = if end { s.len() / nv - 1 } else { 0 };
    &s[iu * nv + iv].control_points[if end { 5 } else { 0 }]
}
#[test]
fn six_common_grids_accept_actual_within_period_stitched_controls() {
    let sources = sources();
    let mut surfaces = Vec::new();
    let mut ts = Vec::new();
    for (i, source) in sources.iter().enumerate() {
        let t = if i == 1 || i == 3 {
            vec![0., 0.5, 1.]
        } else {
            vec![0., 1.]
        };
        let grid = regrid_axial_source(source, t.clone(), ys(), 1e-4, 128).unwrap();
        grid.reverify().unwrap();
        assert_eq!(grid.y_breaks(), ys());
        assert!(grid.certificate().error_upper <= 1e-4);
        ts.push(t);
        surfaces.push(grid.surfaces().to_vec());
    }
    for (left, le, right, re) in [
        (0, true, 1, false),
        (1, true, 2, false),
        (2, true, 3, true),
        (3, false, 4, true),
        (4, false, 5, false),
    ] {
        for iv in 0..16 {
            let row = boundary(&surfaces[left], le, iv).clone();
            let iu = if re {
                surfaces[right].len() / 16 - 1
            } else {
                0
            };
            surfaces[right][iu * 16 + iv].control_points[if re { 5 } else { 0 }] = row;
        }
    }
    for (i, source) in sources.iter().enumerate() {
        let instance = axial_instance(source, 0).unwrap();
        let grid = qualify_axial_instance(
            &instance,
            ts[i].clone(),
            ys(),
            surfaces[i].clone(),
            1e-4,
            128,
        )
        .unwrap();
        grid.reverify().unwrap();
        assert!(grid.certificate().error_upper <= 1e-4);
        eprintln!(
            "common family {i}: patches={} packed={} error_upper={}",
            grid.surfaces().len(),
            grid.packed_surfaces().len(),
            grid.certificate().error_upper
        );
    }
    for (left, le, right, re) in [
        (0, true, 1, false),
        (1, true, 2, false),
        (2, true, 3, true),
        (3, false, 4, true),
        (4, false, 5, false),
    ] {
        for iv in 0..16 {
            let a = boundary(&surfaces[left], le, iv);
            let b = boundary(&surfaces[right], re, iv);
            for (p, q) in a.iter().zip(b) {
                for (x, y) in p.iter().zip(q) {
                    assert_eq!(x.to_bits(), y.to_bits());
                }
            }
        }
    }
}

#[test]
fn saved_placement_is_admitted_against_exact_nominal_period_and_actual_world_poles() {
    let root = axial_root_chart(Spec::default(), RootPart::CrestCylinder, [-2., 2.]).unwrap();
    let source = AxialSource::Root(root.clone());
    let template = regrid_axial_source(&source, vec![0., 1.], ys(), 1e-4, 128).unwrap();
    let instance = axial_instance(&source, 7).unwrap();
    instance.reverify().unwrap();
    let placed = place_axial_surfaces(&instance, template.surfaces()).unwrap();
    let grid =
        qualify_axial_instance(&instance, vec![0., 1.], ys(), placed.clone(), 1e-4, 128).unwrap();
    grid.reverify().unwrap();
    assert_eq!(grid.instance().period(), 7);
    assert_eq!(
        grid.source().specification().axial_shift.to_bits(),
        0f64.to_bits()
    );
    let theta = -std::f64::consts::TAU * 7. / 40.;
    for p in grid.surfaces() {
        let t = (p.knots_u[5] + p.knots_u[6]) / 2.;
        let y = (p.knots_v[5] + p.knots_v[6]) / 2.;
        let q = root
            .position_bounds(nurbs_core::worm_wheel::AxialDomain {
                t: [t, t],
                y: [y, y],
            })
            .unwrap();
        let q = q.map(|v| (v[0] + v[1]) / 2.);
        let expected = [
            theta.cos() * q[0] - theta.sin() * q[2],
            q[1],
            theta.sin() * q[0] + theta.cos() * q[2],
        ];
        let actual = p.evaluate(t, y).unwrap().point;
        let error = (0..3)
            .map(|i| (actual[i] - expected[i]).powi(2))
            .sum::<f64>()
            .sqrt();
        assert!(error <= grid.certificate().error_upper + 1e-10);
    }
    let other = axial_instance(&source, 8).unwrap();
    assert!(qualify_axial_instance(&other, vec![0., 1.], ys(), placed.clone(), 1e-4, 128).is_err());
    let mut changed = placed.clone();
    changed[0].control_points[2][2][0] += 0.01;
    assert!(qualify_axial_instance(&instance, vec![0., 1.], ys(), changed, 1e-4, 128).is_err());
    let mut changed = placed;
    changed[0].control_points[2][0][1] += 1e-12;
    assert!(qualify_axial_instance(&instance, vec![0., 1.], ys(), changed, 1e-4, 128).is_err());
    let direct = regrid_axial_instance(&instance, vec![0., 1.], ys(), 1e-4, 128).unwrap();
    direct.reverify().unwrap();
    assert!(direct.certificate().error_upper <= 1e-4);
}

#[test]
fn incomplete_or_oversized_grids_and_outside_periods_never_evaluate_a_partial_source() {
    let source = AxialSource::Root(
        axial_root_chart(Spec::default(), RootPart::CrestCylinder, [-2., 2.]).unwrap(),
    );
    for t in [
        vec![],
        vec![0.],
        vec![0., 0.5],
        vec![0., 0.5, 0.5, 1.],
        vec![0., f64::NAN, 1.],
    ] {
        assert!(regrid_axial_source(&source, t, ys(), 1e-4, 128).is_err());
    }
    assert!(regrid_axial_source(&source, vec![0., 1.], vec![-2., 0.], 1e-4, 128).is_err());
    let large = (0..=4096).map(|i| i as f64 / 4096.).collect();
    assert!(regrid_axial_source(&source, large, ys(), 1e-4, 4096).is_err());
    assert!(axial_instance(&source, 40).is_err());
    assert!(regrid_axial_source(&source, vec![0., 1.], ys(), f64::NAN, 128).is_err());
}

#[test]
fn complete_forty_period_side_network_closes_the_last_first_seam() {
    let t = std::array::from_fn(|i| {
        if i == 1 || i == 3 {
            vec![0., 0.5, 1.]
        } else {
            vec![0., 1.]
        }
    });
    let network =
        nurbs_core::worm_wheel::axial_side_network(Spec::default(), ys(), 21., t, 1e-4, 128)
            .unwrap();
    assert_eq!(network.teeth(), 40);
    assert_eq!(network.grids().len(), 240);
    for (end, contour) in network.end_contours().iter().enumerate() {
        assert_eq!(contour.len(), 240);
        let plane = if end == 0 { -2f64 } else { 2f64 };
        let mut signed_area = 0.;
        for (i, curve) in contour.iter().enumerate() {
            curve.validate().unwrap();
            assert!(
                curve
                    .control_points
                    .iter()
                    .all(|p| p[1].to_bits() == plane.to_bits())
            );
            let a = curve.control_points.last().unwrap();
            let b = &contour[(i + 1) % contour.len()].control_points[0];
            assert!(
                a.iter()
                    .map(|x| x.to_bits())
                    .eq(b.iter().map(|x| x.to_bits()))
            );
            // Independent sampled winding check, not a continuous trim proof.
            let lo = curve.knots[curve.degree];
            let hi = curve.knots[curve.control_points.len()];
            for j in 0..8 {
                let a = curve
                    .evaluate(lo + (hi - lo) * j as f64 / 8.)
                    .unwrap()
                    .point;
                let b = curve
                    .evaluate(lo + (hi - lo) * (j + 1) as f64 / 8.)
                    .unwrap()
                    .point;
                signed_area += a[0] * b[2] - b[0] * a[2];
            }
        }
        assert!(signed_area < 0.);
    }
    assert_eq!(
        network
            .grids()
            .iter()
            .map(|g| g.packed_surfaces().len())
            .sum::<usize>(),
        720
    );
    for j in 0..40 {
        for iv in 0..16 {
            let a = boundary(network.grids()[j * 6 + 5].surfaces(), true, iv);
            let b = boundary(network.grids()[((j + 1) % 40) * 6].surfaces(), false, iv);
            assert!(
                a.iter()
                    .flatten()
                    .map(|x| x.to_bits())
                    .eq(b.iter().flatten().map(|x| x.to_bits()))
            );
        }
    }
    eprintln!(
        "complete wheel sides: 40 periods, 240 source owners, 720 packed supports, all 240 interfaces closed"
    );
}

#[test]
fn network_rejects_missing_families_and_excessive_total_work_before_construction() {
    use nurbs_core::worm_wheel::axial_side_network;
    let t = std::array::from_fn(|_| vec![0., 1.]);
    let mut missing = t.clone();
    missing[2] = vec![0., 0.5];
    assert!(axial_side_network(Spec::default(), ys(), 21., missing, 1e-4, 128).is_err());
    assert!(
        axial_side_network(
            Spec::default(),
            vec![-2., 2., 0.],
            21.,
            t.clone(),
            1e-4,
            128
        )
        .is_err()
    );
    assert!(axial_side_network(Spec::default(), ys(), 21., t.clone(), f64::NAN, 128).is_err());
    let s = Spec {
        teeth: 256,
        ..Spec::default()
    };
    assert!(axial_side_network(s, ys(), 21., t, 1e-4, 128).is_err());
}
