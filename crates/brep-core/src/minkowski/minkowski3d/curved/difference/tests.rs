use super::super::super::tests::{cube, mesh};
use super::*;
fn source(m: &SourceMesh, k: &Q) -> Model {
    let points = m
        .positions
        .chunks_exact(3)
        .map(|p| std::array::from_fn(|i| Q::from_float(p[i]).unwrap()))
        .collect::<Vec<_>>();
    let triangles = m
        .indices
        .chunks_exact(3)
        .map(|t| [t[0], t[1], t[2]])
        .collect::<Vec<_>>();
    model::build(&points, &triangles, k, &mut Work::default()).unwrap()
}
fn qpoint(p: &ExactPoint) -> P {
    p.each_ref()
        .map(|q| Q::new(q[0].parse().unwrap(), q[1].parse().unwrap()))
}
fn reference_volume(c: &QuadraticDifferenceCertificate) -> Q {
    let p = c.reference_vertices.iter().map(qpoint).collect::<Vec<_>>();
    c.triangles
        .iter()
        .map(|t| {
            dot(
                &p[t[0] as usize],
                &cross(&p[t[1] as usize], &p[t[2] as usize]),
            ) / integer(6)
        })
        .sum()
}
#[test]
fn nonconvex_curved_cut_creates_editable_curved_caps() {
    let k = Q::new(1.into(), 64.into());
    let l = mesh(vec![
        bounding_box(&[[0, 0, 0].map(integer), [4, 1, 2].map(integer)]).unwrap(),
        bounding_box(&[[0, 1, 0].map(integer), [1, 4, 2].map(integer)]).unwrap(),
    ]);
    let a = source(&l, &k);
    let b = source(&cube([2, -1, -1], [3, 2, 1]), &k);
    let saved = [a.clone(), b.clone()];
    let r = certify_quadratic_difference(&a, &b, 1. / 64., 0).unwrap();
    assert_eq!(reference_volume(r.certificate()), integer(13));
    assert_eq!([a.clone(), b.clone()], saved);
    assert_eq!(r.model().bodies.len(), 1);
    assert!(r.model().edges.iter().all(|e| e.curve.degree == 2));
    r.model().validate().unwrap();
    verify_quadratic_difference(&a, &b, r.model(), r.certificate()).unwrap();
    let mut caps = 0;
    let mut curved_caps = 0;
    for face in &r.model().faces {
        let s = &face.surface;
        let u = [s.knots_u[0], *s.knots_u.last().unwrap()];
        let v = [s.knots_v[0], *s.knots_v.last().unwrap()];
        let at = |a: f64, b: f64| {
            s.evaluate(u[0] + a * (u[1] - u[0]), v[0] + b * (v[1] - v[0]))
                .unwrap()
                .point
        };
        if [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75)]
            .iter()
            .all(|(a, b)| {
                let p = at(*a, *b);
                (p[2] - 1. - p[0] * p[0] / 64.).abs() < 1e-12
            })
        {
            caps += 1;
            let a = at(0., 0.);
            let b = at(1., 1.);
            let m = at(0.5, 0.5);
            if (m[2] - (a[2] + b[2]) / 2.).abs() > 1e-6 {
                curved_caps += 1;
            }
        }
    }
    assert!(caps > 0 && curved_caps > 0);
    let mut edited = r.model().clone();
    edited.faces[0].surface.control_points[1][1][2] += 0.125;
    assert!(verify_quadratic_difference(&a, &b, &edited, r.certificate()).is_err());
    let mut forged = r.certificate().clone();
    forged.triangles.pop();
    assert!(verify_quadratic_difference(&a, &b, r.model(), &forged).is_err());
    assert!(certify_quadratic_difference(&a, &b, 1. / 32., 0).is_err());
}
#[test]
fn cavity_and_disconnected_curved_bodies_are_grouped() {
    let k = Q::new(1.into(), 64.into());
    let a = source(&cube([0, 0, 0], [4, 4, 4]), &k);
    let b = source(&cube([1, 1, 1], [3, 3, 3]), &k);
    let r = certify_quadratic_difference(&a, &b, 1. / 64., 0).unwrap();
    assert_eq!(r.model().bodies.len(), 1);
    assert_eq!(r.model().bodies[0].inner_shells.len(), 1);
    assert_eq!(reference_volume(r.certificate()), integer(56));
    let b = source(&cube([1, -1, -1], [3, 5, 5]), &k);
    let r = certify_quadratic_difference(&a, &b, 1. / 64., 0).unwrap();
    assert_eq!(r.model().bodies.len(), 2);
    assert_eq!(reference_volume(r.certificate()), integer(32));
}
#[test]
fn exact_nurbs_controls_refuse_rounding_and_empty_result_is_explicit() {
    let points = vec![[Q::new(1.into(), 3.into()), Q::zero(), Q::zero()]];
    assert!(model::build(&points, &[[0, 0, 0]], &Q::zero(), &mut Work::default()).is_err());
    let k = Q::new(1.into(), 64.into());
    let a = source(&cube([0, 0, 0], [1, 1, 1]), &k);
    let r = certify_quadratic_difference(&a, &a, 1. / 64., 0).unwrap();
    assert!(r.model().bodies.is_empty());
    assert!(r.certificate().result_atlas.is_none());
    verify_quadratic_difference(&a, &a, r.model(), r.certificate()).unwrap();
}

#[test]
fn crossing_hole_is_a_trimmed_curved_cap_not_a_filled_disk() {
    let k = Q::new(1.into(), 64.into());
    let a = source(&cube([0, 0, 0], [4, 4, 4]), &k);
    let b = source(&cube([1, 1, -1], [3, 3, 5]), &k);
    let r = certify_quadratic_difference(&a, &b, 1. / 64., 0).unwrap();
    assert_eq!(reference_volume(r.certificate()), integer(48));
    assert_eq!(r.model().bodies.len(), 1);
    assert!(r.model().bodies[0].inner_shells.is_empty());
    assert_eq!(
        r.model()
            .faces
            .iter()
            .filter(|f| !f.holes.is_empty())
            .count(),
        2
    );
    verify_quadratic_difference(&a, &b, r.model(), r.certificate()).unwrap();
}

#[test]
fn independent_curved_models_support_union_intersection_difference_and_xor() {
    let k = Q::new(1.into(), 64.into());
    let a = source(&cube([0, 0, 0], [2, 2, 2]), &k);
    let b = source(&cube([1, 1, 1], [3, 3, 3]), &k);
    for (op, volume) in [("union", 15), ("intersection", 1), ("difference", 7)] {
        let r = certify_quadratic_boolean(&a, &b, op, 1. / 64., 0).unwrap();
        assert_eq!(reference_volume(&r.certificate().geometry), integer(volume));
        r.model().validate_continuous(32768).unwrap();
        verify_quadratic_boolean(&a, &b, r.model(), r.certificate()).unwrap();
        let mut forged = r.certificate().clone();
        forged.operation = if op == "union" {
            "intersection"
        } else {
            "union"
        }
        .into();
        assert!(verify_quadratic_boolean(&a, &b, r.model(), &forged).is_err());
    }
    let b = source(&cube([3, 0, 0], [4, 1, 1]), &k);
    let r = certify_quadratic_boolean(&a, &b, "xor", 1. / 64., 0).unwrap();
    assert_eq!(reference_volume(&r.certificate().geometry), integer(9));
    assert_eq!(r.model().bodies.len(), 2);
    verify_quadratic_boolean(&a, &b, r.model(), r.certificate()).unwrap();
    let b = source(&cube([1, 1, 1], [3, 3, 3]), &k);
    let direct = certify_quadratic_boolean(&a, &b, "union", 1. / 64., 1).unwrap();
    let dispatched = crate::operations::boolean(&a, &b, "union").unwrap();
    assert_eq!(direct.model(), &dispatched);
}
#[test]
fn independent_curved_boolean_crosses_seams_and_can_be_edited_again() {
    let k = Q::new(1.into(), 64.into());
    let a = source(&cube([0, 0, 0], [4, 4, 4]), &k);
    // This tool crosses the top, bottom and side supports of the source.
    let b = source(&cube([1, 1, -1], [3, 5, 5]), &k);
    let r = certify_quadratic_boolean(&a, &b, "difference", 1. / 64., 0).unwrap();
    assert_eq!(reference_volume(&r.certificate().geometry), integer(40));
    r.model().validate_continuous(32768).unwrap();
    let c = source(&cube([-1, 1, -1], [1, 3, 5]), &k);
    let next = certify_quadratic_boolean(r.model(), &c, "difference", 1. / 64., 0).unwrap();
    assert_eq!(reference_volume(&next.certificate().geometry), integer(32));
    verify_quadratic_boolean(r.model(), &c, next.model(), next.certificate()).unwrap();
}
