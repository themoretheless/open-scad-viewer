//! Certified divergence volume for variable-weight, lofted and bent-sweep solids.
use brep_core::analysis::{CERTIFIED_FREEFORM_VOLUME_CAPABILITY, certified_freeform_volume};
use brep_core::Model;
use nurbs_core::{curve::Curve, surface::Surface};
use std::f64::consts::{FRAC_1_SQRT_2, PI};

fn circles(stations: &[(f64, f64)]) -> Vec<Vec<Vec<Curve>>> {
    stations
        .iter()
        .map(|&(z, r)| vec![vec![nurbs_core::primitives::circle([0., 0., z], [0., 0., 1.], r).unwrap()]])
        .collect()
}

const STATIONS: [(f64, f64); 3] = [(0., 3.), (5., 4.), (10., 2.)];

fn frustum_stack() -> Model {
    brep_core::rational_section_loft(&circles(&STATIONS)).unwrap()
}

fn natural_loft() -> Model {
    brep_core::natural_section_loft(&circles(&STATIONS), &[0., 0.5, 1.]).unwrap()
}

/// Square section `[4,6]×[0,2]` (radius × height) swept a quarter turn about z.
fn bent_sweep() -> Model {
    let profile = [[4., 0.], [6., 0.], [6., 2.], [4., 2.]];
    let ring = |end: bool| -> Vec<Vec<Curve>> {
        let p = |q: [f64; 2]| if end { [0., q[0], q[1]] } else { [q[0], 0., q[1]] };
        vec![(0..4)
            .map(|i| nurbs_core::primitives::line(p(profile[i]), p(profile[(i + 1) % 4])).unwrap())
            .collect()]
    };
    let sides = vec![(0..4)
        .map(|i| {
            let row = |q: [f64; 2]| vec![vec![q[0], 0., q[1]], vec![q[0], q[0], q[1]], vec![0., q[0], q[1]]];
            Surface {
                degree_u: 1,
                degree_v: 2,
                knots_u: vec![0., 0., 1., 1.],
                knots_v: vec![0., 0., 0., 1., 1., 1.],
                control_points: vec![row(profile[i]), row(profile[(i + 1) % 4])],
                weights: vec![vec![1., FRAC_1_SQRT_2, 1.]; 2],
                periodic_u: false,
                periodic_v: false,
            }
        })
        .collect::<Vec<_>>()];
    brep_core::capped_loft_surfaces(&ring(false), &ring(true), &sides).unwrap()
}

/// Bump cuboid with a non-uniform interior weight: a variable-weight rational patch.
fn weighted_bump() -> Model {
    let mut model = brep_core::freeform_cuboid_with_bump_face([0., 0., 0.], [2., 2., 2.]).unwrap();
    model.faces[1].surface.weights[1][1] = 2.;
    model.faces[1].surface.weights[2][1] = 0.5;
    model
}

/// Non-certified dense reference: Gauss-Legendre divergence quadrature over
/// every full-chart face with central-difference tangents.
fn dense_divergence_volume(model: &Model) -> f64 {
    let nodes = [
        (-0.906_179_845_938_664, 0.236_926_885_056_189),
        (-0.538_469_310_105_683, 0.478_628_670_499_366),
        (0., 0.568_888_888_888_889),
        (0.538_469_310_105_683, 0.478_628_670_499_366),
        (0.906_179_845_938_664, 0.236_926_885_056_189),
    ];
    let mut total = 0.;
    for shell in &model.shells {
        for use_ in &shell.faces {
            let s = &model.faces[use_.face].surface;
            let (u0, u1) = (s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]);
            let (v0, v1) = (s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]);
            let e = |u: f64, v: f64| {
                let p = s.evaluate(u, v).unwrap().point;
                [p[0], p[1], p[2]]
            };
            let n = 64;
            let mut flux = 0.;
            for i in 0..n {
                for j in 0..n {
                    let (a, b) = (u0 + (u1 - u0) * i as f64 / n as f64, u0 + (u1 - u0) * (i + 1) as f64 / n as f64);
                    let (c, d) = (v0 + (v1 - v0) * j as f64 / n as f64, v0 + (v1 - v0) * (j + 1) as f64 / n as f64);
                    for &(x, wx) in &nodes {
                        for &(y, wy) in &nodes {
                            let u = (a + b) / 2. + (b - a) / 2. * x;
                            let v = (c + d) / 2. + (d - c) / 2. * y;
                            let h = 1e-6 * (u1 - u0).max(v1 - v0);
                            let p = e(u, v);
                            let su: Vec<f64> = (0..3).map(|k| (e(u + h, v)[k] - e(u - h, v)[k]) / (2. * h)).collect();
                            let sv: Vec<f64> = (0..3).map(|k| (e(u, v + h)[k] - e(u, v - h)[k]) / (2. * h)).collect();
                            let nrm = [su[1] * sv[2] - su[2] * sv[1], su[2] * sv[0] - su[0] * sv[2], su[0] * sv[1] - su[1] * sv[0]];
                            let f = p[0] * nrm[0] + p[1] * nrm[1] + p[2] * nrm[2];
                            flux += f * wx * wy * (b - a) * (d - c) / 4.;
                        }
                    }
                }
            }
            total += if use_.reversed { -flux } else { flux };
        }
    }
    total / 3.
}

/// Solid of revolution reference `∫ π r² dz` from the first wall's u=0 iso.
fn revolution_volume(model: &Model) -> f64 {
    let s = &model.faces[0].surface;
    let (v0, v1) = (s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]);
    let rz = |v: f64| {
        let p = s.evaluate(s.knots_u[s.degree_u], v).unwrap().point;
        (p[0].hypot(p[1]), p[2])
    };
    let n = 200_000;
    let mut total = 0.;
    let (mut ra, mut za) = rz(v0);
    for i in 1..=n {
        let (rb, zb) = rz(v0 + (v1 - v0) * i as f64 / n as f64);
        total += PI * (ra * ra + ra * rb + rb * rb) / 3. * (zb - za);
        (ra, za) = (rb, zb);
    }
    assert_eq!(rz(v0).1, 0.);
    assert!((rz(v1).1 - 10.).abs() < 1e-12, "first wall spans every station");
    total
}

fn assert_encloses(model: &Model, reference: f64, tolerance: f64) {
    let result = certified_freeform_volume(model, tolerance, 200_000).unwrap();
    assert_eq!(result.capability, CERTIFIED_FREEFORM_VOLUME_CAPABILITY);
    let v = result.volume_mm3;
    assert!(v.lower <= reference && reference <= v.upper, "{v:?} excludes {reference}");
    assert!(v.upper - v.lower <= 2.5 * tolerance, "{v:?} wider than requested");
    assert!(result.audit.ok);
}

#[test]
fn variable_weight_frustum_stack_volume_encloses_exact_value() {
    let model = frustum_stack();
    assert!(model.faces.iter().any(|f| f.surface.weights.iter().flatten().any(|w| *w != 1.)));
    // Previously refused by the analytic matrix.
    assert_eq!(
        brep_core::analysis::certified_mass_properties(&model).unwrap_err().code,
        "BREP_CERTIFIED_MASS_REFUSED"
    );
    let exact = PI * 5. / 3. * ((9. + 12. + 16.) + (16. + 8. + 4.));
    assert_encloses(&model, exact, 0.05);
    let field = certified_freeform_volume(&model, 0.05, 200_000).unwrap().field_axis.unwrap();
    assert!(field[2].abs() < 1e-12, "field axis lies in the cap planes");
}

#[test]
fn natural_loft_volume_encloses_dense_reference() {
    let model = natural_loft();
    assert_encloses(&model, revolution_volume(&model), 0.05);
}

#[test]
fn bent_sweep_volume_encloses_pappus_value() {
    let model = bent_sweep();
    // Square caps are full rectangular planar charts, so they are integrated.
    let result = certified_freeform_volume(&model, 5e-4, 200_000).unwrap();
    assert_eq!(result.orthogonal_planar_faces, 0);
    assert_eq!(result.integrated_faces, 6);
    // Pappus: area 4, centroid radius 5, quarter turn.
    assert_encloses(&model, 10. * PI, 5e-4);
}

#[test]
fn variable_weight_rational_patch_body_volume_encloses_dense_reference() {
    let model = weighted_bump();
    assert_eq!(
        brep_core::analysis::certified_freeform_mass_properties(&model).unwrap_err().code,
        "BREP_CERTIFIED_MASS_REFUSED"
    );
    let reference = dense_divergence_volume(&model);
    let result = certified_freeform_volume(&model, 1e-4, 200_000).unwrap();
    assert!(result.field_axis.is_none());
    assert_encloses(&model, reference, 1e-4);
}

#[test]
fn trimmed_curved_faces_and_bad_options_still_refuse() {
    let model = frustum_stack();
    assert_eq!(
        certified_freeform_volume(&model, 0., 1000).unwrap_err().code,
        "BREP_CERTIFIED_MASS_OPTIONS_INVALID"
    );
    assert_eq!(
        certified_freeform_volume(&model, 1e-12, 64).unwrap_err().code,
        "BREP_CERTIFIED_MASS_BUDGET_EXHAUSTED"
    );
    // A stretched sphere keeps quarter-disk charts: trimmed curved faces.
    let mut ellipsoid = brep_core::sphere(2.).unwrap();
    for vertex in &mut ellipsoid.vertices {
        vertex.point[0] *= 1.5;
    }
    for edge in &mut ellipsoid.edges {
        edge.curve.control_points.iter_mut().for_each(|p| p[0] *= 1.5);
    }
    for face in &mut ellipsoid.faces {
        face.surface.control_points.iter_mut().flatten().for_each(|p| p[0] *= 1.5);
    }
    let error = certified_freeform_volume(&ellipsoid, 1e-3, 200_000).unwrap_err();
    assert_eq!(error.code, "BREP_CERTIFIED_MASS_REFUSED");
    assert!(error.message.contains("trimmed"), "{error:?}");
}
