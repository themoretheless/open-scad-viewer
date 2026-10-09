use super::*;
use crate::{primitives, surface};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    fn f64(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.f64()
    }
}

/// Regular spherical patch: a meridian arc at +/-45 deg revolved 60 deg.
fn spherical_patch(radius: f64) -> Surface {
    let arc = primitives::circle_arc(
        [0.; 3],
        [0., 1., 0.],
        radius,
        -45.,
        90.,
    )
    .unwrap();
    surface::revolve(&arc, [0.; 3], [0., 0., 1.], 60.).unwrap()
}

fn sample_normals(s: &Surface, n: usize) -> Vec<[f64; 3]> {
    let (du, dv) = (
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    );
    let mut out = Vec::new();
    for i in 0..n {
        let u = du[0] + (du[1] - du[0]) * i as f64 / (n - 1) as f64;
        for j in 0..n {
            let v = dv[0] + (dv[1] - dv[0]) * j as f64 / (n - 1) as f64;
            if let Some(nrm) = s.evaluate(u, v).unwrap().unit_normal() {
                out.push(nrm);
            }
        }
    }
    out
}

#[test]
fn cone_contains_sampled_normals_property() {
    let surfaces = [
        spherical_patch(2.),
        primitives::quadratic_patch([0.5, 1., 0.5, 1.], [1., 0., 1., 0., 0., 0.]).unwrap(),
        primitives::quadratic_patch([-1., 1., -1., 1.], [1., 0., -1., 0., 0., 0.]).unwrap(),
    ];
    let mut rng = Rng(41);
    for s in &surfaces {
        let cone = normal_cone(s).unwrap();
        assert!(cone.half_angle <= std::f64::consts::PI);
        assert!(norm(cone.axis) > 0.);
        for n in sample_normals(s, 17) {
            assert!(
                cone.contains(n),
                "sampled normal escapes the cone (half_angle {})",
                cone.half_angle
            );
        }
        // Random interior samples too.
        let (du, dv) = (
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        );
        for _ in 0..256 {
            let u = rng.range(du[0], du[1]);
            let v = rng.range(dv[0], dv[1]);
            if let Some(n) = s.evaluate(u, v).unwrap().unit_normal() {
                assert!(cone.contains(n));
            }
        }
    }
}

#[test]
fn silhouette_rejection_on_convex_and_saddle_patches() {
    // Elliptic paraboloid z = x^2 + y^2 on [0.5, 1]^2: every normal has a
    // strictly negative x-component, so no silhouette from +x.
    let hill = primitives::quadratic_patch([0.5, 1., 0.5, 1.], [1., 0., 1., 0., 0., 0.]).unwrap();
    let cone = normal_cone(&hill).unwrap();
    assert!(
        cone.silhouette_impossible([1., 0., 0.]).unwrap(),
        "convex patch viewed along +x must reject the silhouette (cone {:?})",
        cone
    );
    // Saddle z = x^2 - y^2 on [-1,1]^2 viewed along +y: normals flip the
    // y sign across v, so a perpendicular direction exists.
    let saddle = primitives::quadratic_patch([-1., 1., -1., 1.], [1., 0., -1., 0., 0., 0.]).unwrap();
    let cone = normal_cone(&saddle).unwrap();
    assert!(
        !cone.silhouette_impossible([0., 1., 0.]).unwrap(),
        "saddle patch viewed along +y must not reject the silhouette (cone {:?})",
        cone
    );
}

#[test]
fn offset_certificate_on_spherical_patch() {
    let r = 2.;
    let s = spherical_patch(r);
    // Exact principal curvature magnitude is 1/r = 0.5.
    match offset_globally_smooth(&s, 0.4).unwrap() {
        OffsetCertificate::Valid { margin } => {
            assert!(margin > 0. && margin <= 1. - 0.4 / r + 1e-12);
        }
        other => panic!("d = 0.4 << r must be certified valid, got {other:?}"),
    }
    // Zero distance is trivially smooth.
    assert!(matches!(
        offset_globally_smooth(&s, 0.).unwrap(),
        OffsetCertificate::Valid { .. }
    ));
    // d = 8 > r: focal crossing proven by sampled lower bounds.
    match offset_globally_smooth(&s, 8.).unwrap() {
        OffsetCertificate::Invalid { max_d_kappa } => assert!(max_d_kappa >= 1.),
        other => panic!("d = 8 >> r must be certified invalid, got {other:?}"),
    }
    // Plane: zero curvature, any distance is valid.
    let plane = primitives::quadratic_patch([0., 1., 0., 1.], [0.; 6]).unwrap();
    assert!(matches!(
        offset_globally_smooth(&plane, 1e6).unwrap(),
        OffsetCertificate::Valid { .. }
    ));
}

#[test]
fn curvature_enclosure_brackets_exact_sphere_curvature() {
    let r = 2.;
    let s = spherical_patch(r);
    let enc = curvature_enclosure(&s).unwrap();
    assert!(enc.regular);
    // |K| = 1/r^2 = 0.25, |H| = 1/r = 0.5 (sign depends on orientation).
    assert!(enc.gaussian.contains(0.25) || enc.gaussian.contains(-0.25));
    assert!(enc.mean.contains(0.5) || enc.mean.contains(-0.5));
    assert!(enc.max_abs_principal >= 0.5);
    assert!(
        enc.max_abs_principal < 2.0,
        "sphere bound is absurdly loose: {}",
        enc.max_abs_principal
    );
}

#[test]
fn draft_enclosure_on_cylinder_and_cone() {
    // Cylindrical patch (arc of ~115 deg extruded along z): every normal
    // is exactly perpendicular to the pull direction.
    let arc = primitives::circle_arc([0.; 3], [0., 0., 1.], 1., 0., 115.).unwrap();
    let cyl = surface::extrude(&arc, [0., 0., 2.]).unwrap();
    let draft = draft_angle_enclosure(&cyl, [0., 0., 1.]).unwrap();
    assert!(
        draft.contains(std::f64::consts::FRAC_PI_2),
        "cylinder draft must enclose pi/2: [{}, {}]",
        draft.lo,
        draft.hi
    );
    assert!(
        draft.lo > 0.2 && draft.hi < std::f64::consts::PI - 0.2,
        "cylinder draft should be far from 0 and pi: [{}, {}]",
        draft.lo,
        draft.hi
    );
    // Cone frustum: constant draft angle everywhere; enclosure contains
    // every sampled angle.
    let frustum = primitives::cone_frustum([0.; 3], 2., 1., 3.).unwrap();
    let draft = draft_angle_enclosure(&frustum, [0., 0., 1.]).unwrap();
    for n in sample_normals(&frustum, 9) {
        let angle = angle_between(n, [0., 0., 1.]);
        assert!(
            draft.contains(angle),
            "frustum draft [{}, {}] misses {angle}",
            draft.lo,
            draft.hi
        );
    }
}
