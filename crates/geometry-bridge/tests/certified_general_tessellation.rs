//! Certified emitted-mesh deviation for variable-weight, lofted and bent-sweep solids.
use brep_core::Model;
use geometry_bridge::brep::{
    GENERAL_TESSELLATION_CAPABILITY, Tessellation, certified_display_levels,
    certified_general_tessellation_deviation, certified_nurbs,
};
use nurbs_core::{curve::Curve, surface::Surface};
use std::f64::consts::FRAC_1_SQRT_2;

type P = [f64; 3];

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

fn bent_sweep_with(v_knots: Vec<f64>, rows: impl Fn([f64; 2]) -> Vec<Vec<f64>>, weights: Vec<f64>) -> Model {
    let profile = [[4., 0.], [6., 0.], [6., 2.], [4., 2.]];
    let ring = |end: bool| -> Vec<Vec<Curve>> {
        let p = |q: [f64; 2]| if end { [0., q[0], q[1]] } else { [q[0], 0., q[1]] };
        vec![(0..4)
            .map(|i| nurbs_core::primitives::line(p(profile[i]), p(profile[(i + 1) % 4])).unwrap())
            .collect()]
    };
    let degree_v = v_knots.len() - weights.len() - 1;
    let sides = vec![(0..4)
        .map(|i| Surface {
            degree_u: 1,
            degree_v,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: v_knots.clone(),
            control_points: vec![rows(profile[i]), rows(profile[(i + 1) % 4])],
            weights: vec![weights.clone(); 2],
            periodic_u: false,
            periodic_v: false,
        })
        .collect::<Vec<_>>()];
    brep_core::capped_loft_surfaces(&ring(false), &ring(true), &sides).unwrap()
}

/// Square section swept a quarter turn about z: exact rational walls.
fn bent_sweep() -> Model {
    bent_sweep_with(
        vec![0., 0., 0., 1., 1., 1.],
        |q| vec![vec![q[0], 0., q[1]], vec![q[0], q[0], q[1]], vec![0., q[0], q[1]]],
        vec![1., FRAC_1_SQRT_2, 1.],
    )
}

/// Same quarter turn through a degree-1 path with a crease at v = 0.3.
fn creased_sweep() -> Model {
    bent_sweep_with(
        vec![0., 0., 0.3, 1., 1.],
        |q| vec![vec![q[0], 0., q[1]], vec![q[0], q[0], q[1]], vec![0., q[0], q[1]]],
        vec![1., 1., 1.],
    )
}

fn weighted_bump() -> Model {
    let mut model = brep_core::freeform_cuboid_with_bump_face([0., 0., 0.], [2., 2., 2.]).unwrap();
    model.faces[1].surface.weights[1][1] = 2.;
    model.faces[1].surface.weights[2][1] = 0.5;
    model
}

fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: P, b: P) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn norm(a: P) -> f64 {
    dot(a, a).sqrt()
}

/// Point-triangle distance (closest-point regions).
fn triangle_distance(p: P, a: P, b: P, c: P) -> f64 {
    let (ab, ac, ap) = (sub(b, a), sub(c, a), sub(p, a));
    let (d1, d2) = (dot(ab, ap), dot(ac, ap));
    if d1 <= 0. && d2 <= 0. {
        return norm(ap);
    }
    let bp = sub(p, b);
    let (d3, d4) = (dot(ab, bp), dot(ac, bp));
    if d3 >= 0. && d4 <= d3 {
        return norm(bp);
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0. && d1 >= 0. && d3 <= 0. {
        let t = d1 / (d1 - d3);
        return norm(sub(p, [a[0] + t * ab[0], a[1] + t * ab[1], a[2] + t * ab[2]]));
    }
    let cp = sub(p, c);
    let (d5, d6) = (dot(ab, cp), dot(ac, cp));
    if d6 >= 0. && d5 <= d6 {
        return norm(cp);
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0. && d2 >= 0. && d6 <= 0. {
        let t = d2 / (d2 - d6);
        return norm(sub(p, [a[0] + t * ac[0], a[1] + t * ac[1], a[2] + t * ac[2]]));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0. && d4 - d3 >= 0. && d5 - d6 >= 0. {
        let t = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        let bc = sub(c, b);
        return norm(sub(p, [b[0] + t * bc[0], b[1] + t * bc[1], b[2] + t * bc[2]]));
    }
    let denom = 1. / (va + vb + vc);
    let (v, w) = (vb * denom, vc * denom);
    norm(sub(p, std::array::from_fn(|k| a[k] + ab[k] * v + ac[k] * w)))
}

/// Dense lower estimate of the surface-to-mesh distance: samples every face
/// chart (curved faces) and every edge curve (planar cap boundaries) and
/// measures the distance to the triangles emitted for the incident faces.
fn dense_deviation(model: &Model, t: &Tessellation) -> f64 {
    let positions = &t.built.mesh.positions;
    let vertex = |i: usize| -> P { [positions[3 * i] as f64, positions[3 * i + 1] as f64, positions[3 * i + 2] as f64] };
    let triangles = |face: usize| -> Vec<[P; 3]> {
        t.face_ids
            .iter()
            .enumerate()
            .filter(|(_, f)| **f == face)
            .map(|(k, _)| std::array::from_fn(|j| vertex(t.built.mesh.indices[3 * k + j] as usize)))
            .collect()
    };
    let distance = |p: P, tris: &[[P; 3]]| tris.iter().map(|[a, b, c]| triangle_distance(p, *a, *b, *c)).fold(f64::INFINITY, f64::min);
    let mut worst = 0f64;
    for (index, face) in model.faces.iter().enumerate() {
        let tris = triangles(index);
        let s = &face.surface;
        if s.degree_u > 1 || s.degree_v > 1 || s.weights.iter().flatten().any(|w| *w != s.weights[0][0]) {
            let (u0, u1) = (s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]);
            let (v0, v1) = (s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]);
            let n = 40;
            for i in 0..=n {
                for j in 0..=n {
                    let p = s.evaluate(u0 + (u1 - u0) * i as f64 / n as f64, v0 + (v1 - v0) * j as f64 / n as f64).unwrap().point;
                    worst = worst.max(distance([p[0], p[1], p[2]], &tris));
                }
            }
        }
        for &wire in std::iter::once(&face.outer).chain(&face.holes) {
            for coedge in &model.loops[wire].coedges {
                let curve = &model.edges[coedge.edge].curve;
                let [a, b] = curve.domain();
                for k in 0..=200 {
                    let p = curve.evaluate(a + (b - a) * k as f64 / 200.).unwrap().point;
                    worst = worst.max(distance([p[0], p[1], p[2]], &tris));
                }
            }
        }
    }
    worst
}

fn check_certified(model: &Model, tolerance: f64) -> f64 {
    let certified = certified_nurbs(model, tolerance, 20_000).unwrap();
    assert_eq!(certified.capability, GENERAL_TESSELLATION_CAPABILITY);
    let bound = certified.surface_to_mesh_deviation_mm;
    assert!(bound > 0. && bound <= tolerance, "{bound}");
    assert_eq!(bound, certified.mesh_to_surface_deviation_mm);
    assert!(certified.tessellation.built.report.closed);
    let dense = dense_deviation(model, &certified.tessellation);
    assert!(dense <= bound, "dense {dense} exceeds certified {bound}");
    assert!(dense > 0.);
    // Coarser schedules too: every level stays an upper bound.
    for segments in [2, 4, 8] {
        let bound = certified_general_tessellation_deviation(model, segments).unwrap();
        let mesh = geometry_bridge::brep::certified_display_levels(model, &[segments, 16])
            .unwrap()
            .unwrap()
            .remove(0)
            .2;
        let dense = dense_deviation(model, &mesh);
        assert!(dense <= bound, "segments {segments}: dense {dense} exceeds {bound}");
    }
    bound
}

#[test]
fn variable_weight_frustum_stack_tessellation_is_certified() {
    let model = frustum_stack();
    assert_eq!(
        brep_core::analysis::certified_tessellation_deviation(&model, 4).unwrap_err().code,
        "BREP_CERTIFIED_TESSELLATION_REFUSED"
    );
    check_certified(&model, 0.05);
}

#[test]
fn natural_loft_tessellation_is_certified() {
    check_certified(&natural_loft(), 0.05);
}

#[test]
fn bent_sweep_tessellation_is_certified() {
    check_certified(&bent_sweep(), 0.02);
}

#[test]
fn variable_weight_rational_patch_body_tessellation_is_certified() {
    let model = weighted_bump();
    assert_eq!(
        brep_core::analysis::certified_freeform_tessellation_deviation(&model, 4).unwrap_err().code,
        "BREP_CERTIFIED_TESSELLATION_REFUSED"
    );
    check_certified(&model, 0.05);
}

#[test]
fn general_bodies_produce_certified_display_levels() {
    for model in [frustum_stack(), natural_loft(), bent_sweep(), weighted_bump()] {
        let levels = certified_display_levels(&model, &[2, 4, 8]).unwrap().expect("curved walls refine");
        for pair in levels.windows(2) {
            assert!(pair[0].1 > pair[1].1, "bounds strictly decrease");
            assert!(pair[0].2.built.mesh.indices.len() < pair[1].2.built.mesh.indices.len());
        }
        for (segments, bound, tessellation) in &levels {
            assert_eq!(*bound, certified_general_tessellation_deviation(&model, *segments).unwrap());
            assert!(tessellation.built.report.closed);
        }
    }
}

#[test]
fn analytic_shells_keep_closed_form_bounds() {
    let cylinder = brep_core::cylinder(3., 7.).unwrap();
    let certified = certified_nurbs(&cylinder, 0.02, 20_000).unwrap();
    assert_eq!(certified.capability, "certified-brep-tessellation/2");
    assert_eq!(
        certified.surface_to_mesh_deviation_mm,
        brep_core::analysis::certified_tessellation_deviation(&cylinder, certified.subdivisions_per_patch).unwrap()
    );
}

#[test]
fn creased_cells_and_trimmed_curved_faces_still_refuse() {
    let creased = creased_sweep();
    let error = certified_general_tessellation_deviation(&creased, 4).unwrap_err();
    assert_eq!(error.code, "BREP_CERTIFIED_TESSELLATION_REFUSED");
    assert!(error.message.contains("C1"), "{error:?}");
    assert!(certified_display_levels(&creased, &[2, 4]).is_err());
    // An ellipsoid (stretched sphere) leaves the analytic matrix; its
    // quarter-disk charts are trimmed curved faces.
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
    ellipsoid.validate().unwrap();
    let error = match certified_nurbs(&ellipsoid, 0.05, 20_000) {
        Ok(_) => panic!("trimmed ellipsoid charts unexpectedly certified"),
        Err(error) => error,
    };
    assert_eq!(error.code, "BREP_CERTIFIED_TESSELLATION_TRIMMED_REFUSED", "{error:?}");
    assert_eq!(
        certified_display_levels(&ellipsoid, &[2, 4]).err().unwrap().code,
        "BREP_CERTIFIED_TESSELLATION_TRIMMED_REFUSED"
    );
}
