use super::*;
use crate::{cuboid, cylinder, sphere, transform};

#[test]
fn dense_solver_handles_pivoting_and_singularity() {
    let m = vec![vec![0., 2., 1.], vec![1., 1., 1.], vec![2., 0., 3.]];
    let x = solve(m, vec![5., 6., 13.]).unwrap();
    for (got, want) in x.iter().zip([2., 1., 3.]) {
        assert!((got - want).abs() < 1e-12);
    }
    assert!(solve(vec![vec![1., 2.], vec![2., 4.]], vec![1., 2.]).is_none());
    assert!(solve(vec![vec![0., 0.], vec![0., 0.]], vec![0., 0.]).is_none());
}

#[test]
fn segment_intersection_and_distance() {
    let hit = segments_intersect([0., 0.], [2., 2.], [0., 2.], [2., 0.]).unwrap();
    assert!((hit.0 - 0.5).abs() < 1e-12 && (hit.1 - 0.5).abs() < 1e-12);
    assert!(segments_intersect([0., 0.], [1., 0.], [0., 1.], [1., 1.]).is_none());
    assert!(segments_intersect([0., 0.], [1., 1.], [2., 2.], [3., 3.]).is_none());
    assert!((segment_distance([0., 0.], [2., 0.], [1., 1.]) - 1.).abs() < 1e-12);
    assert!((segment_distance([0., 0.], [2., 0.], [3., 0.]) - 1.).abs() < 1e-12);
    assert!((segment_distance([1., 1.], [1., 1.], [1., 4.]) - 3.).abs() < 1e-12);
}

#[test]
fn chord_parameters_are_normalized_and_monotone() {
    let t = chord_parameters(&[[0., 0., 0.], [1., 0., 0.], [1., 2., 0.], [1., 2., 3.]]);
    assert_eq!(t[0], 0.);
    assert_eq!(*t.last().unwrap(), 1.);
    assert!(t.windows(2).all(|w| w[1] > w[0]));
    assert!((t[1] - 1. / 6.).abs() < 1e-12 && (t[2] - 0.5).abs() < 1e-12);
}

#[test]
fn interpolation_reproduces_samples_and_partitions_unity() {
    let pts: Vec<Vec<f64>> = (0..9)
        .map(|i| {
            let t = i as f64 / 8. * std::f64::consts::PI;
            vec![t.cos(), t.sin(), t * 0.1]
        })
        .collect();
    let params: Vec<f64> = (0..9).map(|i| i as f64 / 8.).collect();
    let curve = interpolate(&params, &pts).unwrap();
    assert_eq!(curve.degree, 3);
    curve.validate().unwrap();
    for (t, p) in params.iter().zip(&pts) {
        let q = curve.evaluate(*t).unwrap().point;
        for k in 0..3 {
            assert!((q[k] - p[k]).abs() < 1e-9, "sample {t} not reproduced");
        }
    }
    // Two points: a straight line; three: a quadratic through them.
    let line = interpolate(&[0., 1.], &[vec![0., 0.], vec![2., 4.]]).unwrap();
    assert_eq!(line.degree, 1);
    let mid = line.evaluate(0.5).unwrap().point;
    assert!((mid[0] - 1.).abs() < 1e-12 && (mid[1] - 2.).abs() < 1e-12);
    let quad =
        interpolate(&[0., 0.5, 1.], &[vec![0., 0.], vec![1., 1.], vec![2., 0.]]).unwrap();
    assert_eq!(quad.degree, 2);
    let p = quad.evaluate(0.5).unwrap().point;
    assert!((p[0] - 1.).abs() < 1e-12 && (p[1] - 1.).abs() < 1e-12);
    assert!(interpolate(&[0.], &[vec![0., 0.]]).is_err());
    let flat = vec![vec![0., 0.]; 3];
    assert!(
        interpolate(&[0., 0., 1.], &flat).is_err(),
        "coincident parameters are singular"
    );
}

#[test]
fn chart_membership_on_a_box_face_and_a_sphere_patch() {
    let b = cuboid([0., 0., 0.], [2., 3., 4.]).unwrap();
    let chart = Chart::new(&b, 0).unwrap();
    assert!(chart.inside([0.5, 0.5]));
    assert!(!chart.inside([1.5, 0.5]));
    assert!(!chart.inside([0.5, -0.1]));
    assert!((chart.boundary_distance([0.5, 0.25]) - 0.25).abs() < 1e-9);
    assert!(chart.crossing([0.5, 0.5], [0.5, 1.5]).is_some());
    assert!(chart.crossing([0.2, 0.2], [0.4, 0.4]).is_none());
    let s = sphere(2.).unwrap();
    let patch = Chart::new(&s, 0).unwrap();
    assert!(patch.inside([0.3, 0.3]));
    assert!(!patch.inside([0.8, 0.8]), "outside the quarter disk");
    assert!(patch.boundary_distance([0.1, 0.1]) <= 0.1 + 1e-9);
    let (li, ci, local) = patch.crossing([0.3, 0.3], [0.9, 0.9]).unwrap();
    assert_eq!(li, 0);
    assert!(ci < 3 && (0.0..=1.0).contains(&local));
    let jet = patch.jet([0.3, 0.3]).unwrap();
    assert!(
        (norm(jet.p) - 2.).abs() < 1e-12,
        "jet point lies on the sphere"
    );
    assert!(norm(cross(jet.du, jet.dv)) > 0.);
}

#[test]
fn projection_and_direction_on_a_known_intersection() {
    // Cylinder wall (face 0) against an off-axis sphere: every converged
    // point lies on both surfaces and the march direction is orthogonal
    // to both normals.
    let c = cylinder(3., 10.).unwrap();
    let s = placed(sphere(2.).unwrap(), [2.5, 0.4, 5.]);
    let tracer = Tracer {
        a: Chart::new(&c, 0).unwrap(),
        b: Chart::new(&s, 0).unwrap(),
        scale: 10.,
        step: 0.2,
    };
    let pt = tracer
        .project([0.5, 0.5], [0.3, 0.3])
        .unwrap()
        .expect("converges");
    let pa = tracer.a.point(pt.a).unwrap();
    let pb = tracer.b.point(pt.b).unwrap();
    assert!(dist(pa, pb) < 1e-9);
    assert!((pa[0].hypot(pa[1]) - 3.).abs() < 1e-9, "on the wall");
    assert!(
        (dist(pa, [2.5, 0.4, 5.]) - 2.).abs() < 1e-9,
        "on the sphere"
    );
    let dir = tracer.direction(&pt).unwrap().expect("transversal");
    let ja = tracer.a.jet(pt.a).unwrap();
    let jb = tracer.b.jet(pt.b).unwrap();
    assert!(dot(dir, cross(ja.du, ja.dv)).abs() < 1e-9);
    assert!(dot(dir, cross(jb.du, jb.dv)).abs() < 1e-9);
    assert!((norm(dir) - 1.).abs() < 1e-12);
    // The corrector lands on the plane through the target.
    let target = add(pt.p, scale(dir, 0.1));
    let next = tracer
        .correct(pt.a, pt.b, target, dir)
        .unwrap()
        .expect("corrects");
    assert!(dot(sub(next.p, target), dir).abs() < 1e-9);
    assert!((dist(next.p, [2.5, 0.4, 5.]) - 2.).abs() < 1e-9);
    assert!((next.p[0].hypot(next.p[1]) - 3.).abs() < 1e-9);
}

#[test]
fn marching_closes_a_loop_that_stays_inside_both_faces() {
    // A box face whose normal is the octant diagonal cuts a unit sphere
    // 0.9 mm from its centre: the cap is a ring inside one sphere patch
    // and inside the face, so the pair yields exactly one closed chain.
    let n = [1. / 3f64.sqrt(); 3];
    let u = [1. / 2f64.sqrt(), -1. / 2f64.sqrt(), 0.];
    let v = [
        n[1] * u[2] - n[2] * u[1],
        n[2] * u[0] - n[0] * u[2],
        n[0] * u[1] - n[1] * u[0],
    ];
    let t = 0.9 - 50.;
    let b = transform::affine(
        &cuboid([-50., -50., -50.], [50., 50., 50.]).unwrap(),
        [
            [u[0], v[0], n[0], t * n[0]],
            [u[1], v[1], n[1], t * n[1]],
            [u[2], v[2], n[2], t * n[2]],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let s = sphere(1.).unwrap();
    // The face at the top of the tilted box: its centre is nearest the
    // sphere.
    let face = (0..6)
        .min_by(|&i, &j| {
            let c = |f: usize| {
                let p = b.faces[f].surface.evaluate(0.5, 0.5).unwrap().point;
                norm([p[0], p[1], p[2]])
            };
            c(i).total_cmp(&c(j))
        })
        .unwrap();
    let tracer = Tracer {
        a: Chart::new(&b, face).unwrap(),
        b: Chart::new(&s, 0).unwrap(),
        scale: 100.,
        step: 0.05,
    };
    let chains = tracer.trace_all(12).unwrap();
    assert_eq!(chains.len(), 1, "one ring expected");
    assert!(chains[0].closed);
    assert!(chains[0].start.is_none() && chains[0].end.is_none());
    let rho = (1f64 - 0.81).sqrt();
    for p in &chains[0].points {
        assert!((norm(p.p) - 1.).abs() < 1e-9, "on the sphere");
        assert!((dot(p.p, n) - 0.9).abs() < 1e-9, "on the face plane");
        let radial = sub(p.p, scale(n, 0.9));
        assert!((norm(radial) - rho).abs() < 1e-9, "on the section circle");
    }
    let mut pts = chains[0].points.clone();
    pts.push(pts[0]);
    let f = fit(&pts, &tracer.a, &tracer.b).unwrap();
    assert!(f.deviation < 1e-4, "deviation {:e}", f.deviation);
}

#[test]
fn marching_exits_through_trim_edges_with_shared_hits() {
    let c = cylinder(3., 10.).unwrap();
    let s = placed(sphere(2.).unwrap(), [2.5, 0.4, 5.]);
    let tracer = Tracer {
        a: Chart::new(&c, 0).unwrap(),
        b: Chart::new(&s, 0).unwrap(),
        scale: 10.,
        step: 0.2,
    };
    let chains = tracer.trace_all(12).unwrap();
    assert!(!chains.is_empty());
    for chain in &chains {
        assert!(!chain.closed);
        let (start, end) = (chain.start.unwrap(), chain.end.unwrap());
        for hit in [start, end] {
            let model = if hit.operand == 0 { &c } else { &s };
            let on_edge = model.edges[hit.edge].curve.evaluate(hit.t).unwrap().point;
            assert!(dist([on_edge[0], on_edge[1], on_edge[2]], hit.point) < 1e-9);
            assert!((hit.point[0].hypot(hit.point[1]) - 3.).abs() < 1e-8);
            assert!((dist(hit.point, [2.5, 0.4, 5.]) - 2.).abs() < 1e-8);
        }
    }
}

#[test]
fn chart_samples_only_authored_nonunit_trim_intervals() {
    fn rescale(curve: &mut crate::Curve, domain: [f64; 2]) {
        let old = curve.domain();
        for knot in &mut curve.knots {
            *knot = domain[0]
                + (*knot - old[0]) / (old[1] - old[0]) * (domain[1] - domain[0]);
        }
    }
    let mut model = cuboid([0., 0., 0.], [2., 2., 2.]).unwrap();
    let wire = model.faces[0].outer;
    for i in 0..model.loops[wire].coedges.len() {
        let edge = model.loops[wire].coedges[i].edge;
        rescale(&mut model.edges[edge].curve, [2., 6.]);
        rescale(&mut model.loops[wire].coedges[i].pcurve, [10., 14.]);
        let coedge = model.loops[wire].coedges[i].clone();
        model.loops[wire].coedges[i] = coedge
            .with_trim(brep_topology::CoedgeTrim {
                curve_parameter: [3., 5.],
                pcurve_parameter: [11., 13.],
                periodic_lift: [[0; 2]; 2],
            })
            .unwrap();
    }
    let chart = Chart::new(&model, 0).unwrap();
    for (ci, sampled) in chart.coedges[0].iter().enumerate() {
        let source = &model.loops[wire].coedges[ci].pcurve;
        assert_eq!(sampled.curve_parameter, [3., 5.]);
        assert_eq!(sampled.pcurve_parameter, [11., 13.]);
        assert_eq!(sampled.samples[0], pcurve_point(source, 11.).unwrap());
        assert_eq!(*sampled.samples.last().unwrap(), pcurve_point(source, 13.).unwrap());
    }
    assert!(chart.inside([0.5, 0.5]));
}

#[test]
fn ray_parity_classifies_points_against_exact_solids() {
    let sphere_model = sphere(2.).unwrap();
    let s = Solid::new(&sphere_model).unwrap();
    assert!(s.contains([0.3, -0.2, 0.1]).unwrap());
    assert!(!s.contains([2.5, 0., 0.]).unwrap());
    assert!(!s.contains([0., 0., -7.]).unwrap());
    let cylinder_model = cylinder(3., 10.).unwrap();
    let c = Solid::new(&cylinder_model).unwrap();
    assert!(c.contains([1., 1., 5.]).unwrap());
    assert!(!c.contains([1., 1., 11.]).unwrap());
    assert!(!c.contains([3.5, 0., 5.]).unwrap());
    let box_model = cuboid([0., 0., 0.], [1., 2., 3.]).unwrap();
    let b = Solid::new(&box_model).unwrap();
    assert!(b.contains([0.5, 1., 1.5]).unwrap());
    assert!(!b.contains([-0.5, 1., 1.5]).unwrap());
    // A point on the boundary is inconsistent across rays and refuses
    // rather than guessing.
    assert!(b.contains([0., 1., 1.5]).is_err());
}

#[test]
fn hull_overlap_prefilter() {
    let a = ([0., 0., 0.], [1., 1., 1.]);
    let b = ([1.5, 0., 0.], [2., 1., 1.]);
    assert!(!hulls_overlap(a, b, 0.));
    assert!(hulls_overlap(a, b, 0.6));
    assert!(hulls_overlap(a, ([0.5, 0.5, 0.5], [3., 3., 3.]), 0.));
}

fn placed(model: Model, at: [f64; 3]) -> Model {
    transform::affine(
        &model,
        [
            [1., 0., 0., at[0]],
            [0., 1., 0., at[1]],
            [0., 0., 1., at[2]],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap()
}
fn rotated_x(model: Model, degrees: f64) -> Model {
    let (s, c) = degrees.to_radians().sin_cos();
    transform::affine(
        &model,
        [
            [1., 0., 0., 0.],
            [0., c, -s, 0.],
            [0., s, c, 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap()
}

#[test]
fn off_axis_sphere_through_a_cylinder_wall() {
    let c = cylinder(3., 10.).unwrap();
    let s = placed(sphere(2.).unwrap(), [2.5, 0.4, 5.]);
    for op in ["union", "difference", "intersection"] {
        let model = boolean(&c, &s, op).unwrap();
        model.validate().unwrap();
        assert!(model.tolerance_mm <= CEILING, "{op}");
        assert!(model.tolerance_mm > 1e-7, "{op} carries its tolerance");
        assert_eq!(model.bodies.len(), 1, "{op}");
    }
}

#[test]
fn crossing_cylinders() {
    let a = cylinder(2., 12.).unwrap();
    let b = placed(rotated_x(cylinder(1.2, 12.).unwrap(), 90.), [0.3, 6., 6.]);
    for op in ["union", "difference", "intersection"] {
        let model = boolean(&a, &b, op).unwrap();
        model.validate().unwrap();
        assert_eq!(model.bodies.len(), 1, "{op}");
    }
}

#[test]
fn box_corner_into_a_sphere_off_axis_is_not_needed_but_works() {
    let b = cuboid([-4., -4., -4.], [4., 4., 4.]).unwrap();
    let s = placed(sphere(3.).unwrap(), [3.1, 2.7, 2.9]);
    let model = boolean(&b, &s, "difference").unwrap();
    model.validate().unwrap();
}

#[test]
fn the_public_boolean_falls_back_after_an_exact_refusal() {
    let c = cylinder(3., 10.).unwrap();
    let s = placed(sphere(2.).unwrap(), [2.5, 0.4, 5.]);
    let model = crate::boolean(&c, &s, "difference").unwrap();
    assert!(model.tolerance_mm > 1e-7 && model.tolerance_mm <= CEILING);
    // Exact pairs stay exact: the axial sphere keeps the kernel tolerance.
    let axial = placed(sphere(4.).unwrap(), [0., 0., 5.]);
    let exact = crate::boolean(&c, &axial, "difference").unwrap();
    assert!(exact.tolerance_mm <= 1e-6);
    // Hard errors are not masked by the fallback.
    let err = crate::boolean(&c, &s, "xor").unwrap_err();
    assert!(err.message.contains("xor") || err.code != "BREP_INVALID_OPERATION");
}

#[test]
fn torus_through_a_box_corner_region() {
    let b = cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
    let t = placed(crate::torus(4., 1.).unwrap(), [0.3, 0.7, 0.4]);
    for op in ["union", "difference", "intersection"] {
        let model = crate::boolean(&b, &t, op).unwrap();
        model.validate().unwrap();
        assert!(model.tolerance_mm > 1e-6, "{op}");
    }
    // The torus at the origin meets the box exactly along its own seam
    // circles (the planes x = 0, y = 0, z = 0 are torus patch seams):
    // coincident boundaries refuse instead of guessing a containment.
    let seam = crate::torus(4., 1.).unwrap();
    let err = crate::boolean(&b, &seam, "union").unwrap_err();
    assert!(err.message.contains("coincident"), "{}", err.message);
}

#[test]
fn disjoint_pairs_resolve_by_empty_algebra() {
    let c = cylinder(3., 10.).unwrap();
    let s = placed(sphere(2.).unwrap(), [20., 0., 5.]);
    assert_eq!(boolean(&c, &s, "union").unwrap().bodies.len(), 2);
    assert!(boolean(&c, &s, "intersection").unwrap().is_empty());
}
