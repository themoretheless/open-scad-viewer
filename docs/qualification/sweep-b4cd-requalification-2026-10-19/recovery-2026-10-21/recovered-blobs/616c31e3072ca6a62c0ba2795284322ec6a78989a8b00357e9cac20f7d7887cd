use brep_core::worm::{WheelCapLimits, wheel_caps};
use nurbs_core::worm_wheel::{Spec, axial_side_network};
#[test]
fn actual_complete_wheel_end_profiles_receive_continuous_planar_cap_proofs() {
    let t = std::array::from_fn(|i| {
        if i == 1 || i == 3 {
            vec![0., 0.5, 1.]
        } else {
            vec![0., 1.]
        }
    });
    let y = (0..=16).map(|i| -2. + i as f64 / 4.).collect();
    let side = axial_side_network(Spec::default(), y, 21., t, 1e-4, 128).unwrap();
    let originals = side.end_contours();
    let owner = wheel_caps(side, 1e-4, WheelCapLimits::default()).unwrap();
    assert_eq!(owner.sides().teeth(), 40);
    for end in 0..2 {
        let cap = &owner.caps()[end];
        let trim = &owner.trims()[end];
        assert_eq!(trim.valid, Some(true));
        assert_eq!(trim.winding, vec![Some(1)]);
        assert_eq!(cap.boundaries.len(), 1);
        assert_eq!(cap.boundaries[0].len(), 240);
        let plane = if end == 0 { -2f64 } else { 2f64 };
        assert!(
            cap.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2].to_bits() == plane.to_bits())
        );
        for (source, actual) in originals[end].iter().zip(&cap.boundaries[0]) {
            assert_eq!(source.degree, actual.degree);
            assert_eq!(source.knots, actual.knots);
            assert_eq!(source.weights, actual.weights);
            for (p, q) in source.control_points.iter().zip(&actual.control_points) {
                assert_eq!(q[0].to_bits(), p[0].to_bits());
                assert_eq!(q[1].to_bits(), (-p[2]).to_bits());
                assert_eq!(q[2].to_bits(), p[1].to_bits());
            }
        }
        eprintln!(
            "wheel cap {end}: {} curves, trim={}, pairs={}, cells={}, domain={}",
            cap.boundaries[0].len(),
            trim.reason,
            trim.pairs,
            trim.cells,
            trim.domain_cells
        );
    }
    assert!(owner.agreement_cells() > 0);
    let flux = side_flux_volume(owner.sides());
    let sections = section_volume(owner.sides());
    assert!((4530. ..5550.).contains(&flux));
    assert!((flux - sections).abs() < 0.001 * flux);
    eprintln!("independent wheel volume: side flux={flux}, polygon sections={sections}");
    let body = brep_core::worm::wheel_body_candidate(owner).unwrap();
    let model = body.model();
    model.validate().unwrap();
    assert_eq!(model.faces.len(), 722);
    assert_eq!(model.edges.len(), 1680);
    assert_eq!(model.vertices.len(), 960);
    assert_eq!(model.shells.len(), 1);
    assert!(model.shells[0].closed);
    assert_eq!(model.bodies.len(), 1);
    assert!(!body.solid_geometry_certified());
    let closure = body.closure();
    assert_eq!(
        closure.closure.status,
        brep_core::watertight::Closure::WithinTolerance
    );
    eprintln!("wheel body: 722 faces, 1680 edges, 960 vertices, continuous seam closure qualified");
}

// Divergence theorem using F=(x/2,0,z/2): planar Y caps have zero flux.
// Eight-point Gauss integration resolves the polynomial degree of each actual
// quintic Bernstein side cell; no display tessellation or mass API is used.
fn side_flux_volume(side: &nurbs_core::worm_wheel::AxialSideNetwork) -> f64 {
    let nodes = [
        (0.1834346424956498, 0.3626837833783620),
        (0.5255324099163290, 0.3137066458778873),
        (0.7966664774136267, 0.2223810344533745),
        (0.9602898564975363, 0.1012285362903763),
    ];
    let mut volume = 0.;
    for (family, g) in side.grids().iter().enumerate() {
        let sign = if family % 6 == 3 || family % 6 == 4 {
            -1.
        } else {
            1.
        };
        for s in g.surfaces() {
            let (ua, ub) = (s.knots_u[5], s.knots_u[6]);
            let (va, vb) = (s.knots_v[5], s.knots_v[6]);
            for (x, wx) in nodes {
                for sx in [-1., 1.] {
                    for (y, wy) in nodes {
                        for sy in [-1., 1.] {
                            let q = s
                                .evaluate(
                                    (ua + ub) / 2. + sx * x * (ub - ua) / 2.,
                                    (va + vb) / 2. + sy * y * (vb - va) / 2.,
                                )
                                .unwrap();
                            let (a, b) = q.first_derivatives().unwrap();
                            let nx = a[1] * b[2] - a[2] * b[1];
                            let nz = a[0] * b[1] - a[1] * b[0];
                            volume += sign * (q.point[0] * nx + q.point[2] * nz) / 2.
                                * wx
                                * wy
                                * (ub - ua)
                                * (vb - va)
                                / 4.;
                        }
                    }
                }
            }
        }
    }
    volume
}
// Independent horizontal polygon sections with composite Simpson integration.
fn section_volume(side: &nurbs_core::worm_wheel::AxialSideNetwork) -> f64 {
    let mut areas = Vec::new();
    for k in 0..=16 {
        let y = -2. + k as f64 / 4.;
        let iv = k.min(15);
        let mut area = 0.;
        for (i, g) in side.grids().iter().enumerate() {
            let sign = if i % 6 == 3 || i % 6 == 4 { -1. } else { 1. };
            for iu in 0..g.t_breaks().len() - 1 {
                let s = &g.surfaces()[iu * 16 + iv];
                let (lo, hi) = (g.t_breaks()[iu], g.t_breaks()[iu + 1]);
                let mut a = s.evaluate(lo, y).unwrap().point;
                for j in 1..=64 {
                    let b = s
                        .evaluate(lo + (hi - lo) * j as f64 / 64., y)
                        .unwrap()
                        .point;
                    area -= sign * (a[0] * b[2] - b[0] * a[2]) / 2.;
                    a = b;
                }
            }
        }
        areas.push(area);
    }
    (areas[0]
        + areas[16]
        + (1..16)
            .map(|i| areas[i] * if i % 2 == 1 { 4. } else { 2. })
            .sum::<f64>())
        / 12.
}
