//! Analytic sphere/torus intersection for canonical solids in the axial
//! configuration (sphere center certified on the torus axis).
//!
//! The sphere operand must be the exact canonical stereographic sphere of
//! `analytic::sphere` (recognized by `sphere_sphere::recognize`); the torus
//! operand must be the exact canonical ring-torus solid of `analytic::torus`
//! (sixteen rational biquadratic patches tiling four profile quadrants times
//! four revolution quadrants with exact tensor-product weights, rigid affine
//! placements admitted) — recognized by `plane_torus::recognize_torus`.
//! Anything else is an explicit `UnsupportedSurface` region — this cell
//! never falls back to numerical surface/surface subdivision. Axiality is
//! certified, never forced: a perpendicular offset at pure rounding scale
//! snaps to the axis, an offset inside the recognition-scale band reports a
//! `NearCoincidence` region, and a clearly off-axis pair is
//! `UnsupportedSurface` (the general sphere/torus pair is a quartic, out of
//! scope). Classification uses outward binary64 bands widened by both
//! recognition deviations.
//!
//! Both surfaces are surfaces of revolution about the shared axis, so the
//! section is a union of exact circles. In the meridian half-plane
//! (rho >= 0, z) the torus side is the circle (rho - R)^2 + z^2 = r_t^2 and
//! the sphere is the circle rho^2 + (z - h)^2 = r_s^2 with h the sphere
//! center height above the torus center plane. Circle/circle in the
//! meridian plane gives zero, one (double) or two roots; each root (rho*,
//! z*) revolves into the exact 3D circle of radius rho* at height z*:
//! - two distinct roots give two exact circles (for a strict ring torus
//!   every root sits on the torus meridian circle, whose rho >= R - r_t
//!   is clearly positive, so both roots always revolve — a one-circle
//!   configuration would require rho* <= 0, i.e. a horn/spindle torus,
//!   which the canonical constructor refuses);
//! - a double root (the circles tangent in the meridian plane, d == r_t+r_s
//!   externally or d == |r_t - r_s| internally) stays
//!   `TangencyOrMultipleRoot` — the tangent contact revolves into a circle
//!   but is never a guessed circle;
//! - a root whose radius rho* collapses into the band degenerates to a
//!   pole point on the axis — unresolved, never guessed (unreachable for a
//!   canonical strict ring torus since rho* >= R - r_t >= 1e-5, kept as an
//!   honest guard, never guessed through);
//! - no roots (provably separate d > r_t + r_s, or provably contained
//!   d < |r_t - r_s|, beyond the band) resolves empty by the interval
//!   signs.
//! Resolved contacts are exact rational circles (four 90-degree arcs,
//! weights cos(pi/4)) with UV lifts on both surfaces: on the torus, iso-v
//! parallels at the exact profile angle phi = atan2(z*, rho* - R) mapped
//! through the exact rational 90-degree-arc parameter map
//! t = s/(1+s), s = sqrt(2) tan(phi/2)/(1 - tan(phi/2)), one degree-1 line
//! per revolution quadrant patch of the profile row; on the sphere, the
//! per-patch stereographic UV circles/lines of `sphere_sphere::lift`.
//! There is no sphere/torus surface coincidence, so `CoincidentTrim` does
//! not arise; every degeneracy reported here is an explicit tangency
//! region. Nothing here authorizes a topology change.
use super::plane_torus::{
    CanonicalTorus, TorusPatchCurve, lift_parallel, recognize_torus, torus_residual,
};
use super::sphere_sphere::{
    self, ARC_WEIGHT, CanonicalSphere, RECOGNITION, SpherePatchCircle, circle_curve, lift,
};
use super::*;
use crate::Model;

const TAU: f64 = std::f64::consts::TAU;

#[derive(Clone, Debug)]
pub enum SphereTorusComponent {
    /// Transverse intersection: the exact circle plus both UV lifts.
    Circle {
        /// Exact rational full circle: degree 2, knots 0..=4, four 90-degree
        /// arcs with weights cos(pi/4).
        curve: Curve,
        center: [f64; 3],
        radius: f64,
        /// Unit torus axis (the circle plane normal).
        normal: [f64; 3],
        sphere_uv: Vec<SpherePatchCircle>,
        torus_uv: Vec<TorusPatchCurve>,
        /// Worst residual over 16 circle samples against the sphere equation
        /// and the scaled torus implicit equation.
        max_sample_residual: f64,
    },
}

/// Exact circle component at axial height `z` above the torus center plane
/// with radius `rho`, with UV lifts on both surfaces and a 16-sample
/// residual bound.
fn circle_component(
    sphere: &CanonicalSphere,
    torus: &CanonicalTorus,
    z: f64,
    rho: f64,
) -> Result<SphereTorusComponent> {
    let center = std::array::from_fn(|k| torus.center[k] + z * torus.axis[k]);
    let curve = circle_curve(center, rho, torus.frame[0], torus.frame[1]);
    // Torus lift: the iso-v parallel at the exact profile angle.
    let profile = z.atan2(rho - torus.major).rem_euclid(TAU);
    let torus_uv = lift_parallel(torus, profile, &[(0., TAU)]);
    let sphere_uv = lift(sphere, torus.axis, center);
    let mut max_sample_residual = 0_f64;
    for i in 0..16 {
        let jet = curve.evaluate(i as f64 / 4.)?;
        let point = [jet.point[0], jet.point[1], jet.point[2]];
        let d = sub(point, sphere.center);
        let sphere_residual = (d[0].hypot(d[1]).hypot(d[2]) - sphere.radius).abs();
        max_sample_residual = max_sample_residual
            .max(sphere_residual)
            .max(torus_residual(torus, point));
    }
    Ok(SphereTorusComponent::Circle {
        curve,
        center,
        radius: rho,
        normal: torus.axis,
        sphere_uv,
        torus_uv,
        max_sample_residual,
    })
}

/// Analytic sphere/torus intersection of a canonical sphere solid and a
/// canonical ring-torus solid, axial configuration only. Non-canonical
/// operands and clearly off-axis pairs are explicit unsupported regions,
/// never a numerical fallback; near-axial offsets, the meridian
/// circle/circle tangencies (double roots) and the pole-collapse guard stay
/// unresolved — tangent contacts are never guessed. Provable misses
/// (separate or contained meridian circles) resolve empty.
pub fn intersect_sphere_torus(
    sphere_model: &Model,
    torus_model: &Model,
    options: Options,
) -> Result<Report<SphereTorusComponent>> {
    let options = options.validate()?;
    let _ = options;
    let mut report = Report::default();
    let domain = vec![0., 1., 0., 1., 0., 1., 0., 1.];
    let (Some(sphere), Some(torus)) = (
        sphere_sphere::recognize(sphere_model)?,
        recognize_torus(torus_model)?,
    ) else {
        report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    };
    report.boxes_visited = 1;
    let offset = sub(sphere.center, torus.center);
    let h = dot(offset, torus.axis);
    let perp = sub(offset, torus.axis.map(|x| x * h));
    let pscale = perp.iter().map(|v| v.abs()).fold(0., f64::max);
    let d_perp = if pscale == 0. {
        0.
    } else {
        let scaled = perp.map(|x| x / pscale);
        scaled[0].hypot(scaled[1]).hypot(scaled[2]) * pscale
    };
    let terms = h.abs() + torus.major + torus.minor + sphere.radius + 1.;
    // Outward binary64 classification band: both recognition deviations plus
    // a rounding allowance on the axial coordinates and radii.
    let band = sphere.error + torus.error + 16. * f64::EPSILON * terms;
    // Axiality is certified, never forced: pure-rounding offsets snap to the
    // axis, recognition-scale offsets report near_coincidence, and anything
    // larger is the unsupported general (quartic) configuration.
    let snap = 64. * f64::EPSILON * terms;
    if d_perp > snap {
        let near = RECOGNITION * (sphere.radius + torus.major + torus.minor);
        let reason = if d_perp <= near {
            UnresolvedReason::NearCoincidence
        } else {
            UnresolvedReason::UnsupportedSurface
        };
        report.unresolved(domain, reason);
        return Ok(report);
    }
    let (r_t, r_s, big_r) = (torus.minor, sphere.radius, torus.major);
    // Meridian circle/circle: centers (R, 0) and (0, h), distance d > 0
    // (R >= 1e-5 for a canonical strict ring torus).
    let d = big_r.hypot(h);
    if !d.is_finite() {
        report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    }
    let sum = r_t + r_s;
    let diff = (r_t - r_s).abs();
    // Provable miss: the meridian circles are separate or one contains the
    // other, certified beyond the outward band.
    if d > sum + band || d + band < diff {
        return Ok(report);
    }
    // Tangent meridian circles: a double root. The tangent contact revolves
    // into a circle but stays unresolved — never a guessed circle.
    if (d - sum).abs() <= band || (d - diff).abs() <= band {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    // Two distinct roots: with e = (S - T)/d = (-R/d, h/d) and the normal
    // n = (-h/d, -R/d), the roots are T + a e +- l n where
    // a = (d^2 + r_t^2 - r_s^2)/(2d) and l^2 = r_t^2 - a^2.
    let a = (d * d + r_t * r_t - r_s * r_s) / (2. * d);
    let l2 = r_t * r_t - a * a;
    if !l2.is_finite() || l2 <= 0. {
        // Defensive: the band comparisons above should have caught every
        // tangency; a non-positive discriminant here is rounding spill and
        // stays unresolved rather than guessed.
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    let l = l2.sqrt();
    let mut tangency = false;
    let mut found: Vec<(f64, f64)> = Vec::new();
    for sign in [1., -1.] {
        let rho = big_r - (a * big_r + sign * l * h) / d;
        let z = (a * h - sign * l * big_r) / d;
        if rho.abs() <= band {
            // The revolved circle degenerates to a pole point on the axis
            // (unreachable for a canonical strict ring torus, whose roots
            // keep rho >= R - r_t >= 1e-5): unresolved, never guessed.
            tangency = true;
            continue;
        }
        if rho < 0. {
            // Mirror root outside the meridian half-plane: not on the
            // surface (defensive; unreachable for a strict ring torus).
            continue;
        }
        found.push((z, rho));
    }
    if tangency {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
    }
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (z, rho) in found {
        report
            .components
            .push(circle_component(&sphere, &torus, z, rho)?);
    }
    Ok(report)
}

impl value_codec::Serialize for SphereTorusComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Circle {
                curve,
                center,
                radius,
                normal,
                sphere_uv,
                torus_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"circle","curve":curve,"center":center,"radius":radius,
                "normal":normal,"sphereUv":sphere_uv,"torusUv":torus_uv,
                "maxSampleResidual":max_sample_residual}),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn translated(model: &Model, offset: [f64; 3]) -> Model {
        crate::transform::affine(
            model,
            [
                [1., 0., 0., offset[0]],
                [0., 1., 0., offset[1]],
                [0., 0., 1., offset[2]],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap()
    }
    fn rotated_translated(model: &Model, angle: f64, offset: [f64; 3]) -> Model {
        let (sin, cos) = angle.sin_cos();
        crate::transform::affine(
            model,
            [
                [1., 0., 0., offset[0]],
                [0., cos, -sin, offset[1]],
                [0., sin, cos, offset[2]],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap()
    }
    fn only_circles(
        report: &Report<SphereTorusComponent>,
        count: usize,
    ) -> &Report<SphereTorusComponent> {
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        assert!(report.unresolved.is_empty(), "{report:?}");
        assert!(!report.permits_topology_change());
        assert_eq!(report.components.len(), count, "{report:?}");
        report
    }
    #[allow(clippy::type_complexity)]
    fn circle_of(
        component: &SphereTorusComponent,
    ) -> (
        &Curve,
        [f64; 3],
        f64,
        [f64; 3],
        &[SpherePatchCircle],
        &[TorusPatchCurve],
        f64,
    ) {
        let SphereTorusComponent::Circle {
            curve,
            center,
            radius,
            normal,
            sphere_uv,
            torus_uv,
            max_sample_residual,
        } = component;
        (
            curve,
            *center,
            *radius,
            *normal,
            sphere_uv,
            torus_uv,
            *max_sample_residual,
        )
    }
    fn sphere_residual(point: [f64; 3], center: [f64; 3], radius: f64) -> f64 {
        let d = sub(point, center);
        (d[0].hypot(d[1]).hypot(d[2]) - radius).abs()
    }
    /// Torus implicit residual (unscaled quartic) in the canonical z-up frame.
    fn implicit(point: [f64; 3], major: f64, minor: f64) -> f64 {
        let total = dot(point, point);
        let rho2 = point[0] * point[0] + point[1] * point[1];
        let base = total + major * major - minor * minor;
        (base * base - 4. * major * major * rho2).abs()
    }
    /// Worst residual of lifted UV arcs evaluated through their own surface,
    /// against the sphere equation and a caller-supplied torus predicate.
    fn uv_samples(
        sphere_model: &Model,
        torus_model: &Model,
        sphere_center: [f64; 3],
        sphere_radius: f64,
        sphere_uv: &[SpherePatchCircle],
        torus_uv: &[TorusPatchCurve],
        torus_check: impl Fn([f64; 3]) -> f64,
    ) -> f64 {
        let mut worst = 0_f64;
        for lift in sphere_uv {
            let surface = &sphere_model.faces[lift.patch].surface;
            for arc in &lift.arcs {
                for k in 0..=8 {
                    let uv = arc.evaluate(k as f64 / 8.).unwrap().point;
                    let p = surface
                        .evaluate(uv[0].clamp(0., 1.), uv[1].clamp(0., 1.))
                        .unwrap()
                        .point;
                    let p = [p[0], p[1], p[2]];
                    worst = worst
                        .max(sphere_residual(p, sphere_center, sphere_radius))
                        .max(torus_check(p));
                }
            }
        }
        for lift in torus_uv {
            let surface = &torus_model.faces[lift.patch].surface;
            for arc in &lift.arcs {
                for k in 0..=8 {
                    let uv = arc.evaluate(k as f64 / 8.).unwrap().point;
                    assert!((0. ..=1.).contains(&uv[0]) && (0. ..=1.).contains(&uv[1]));
                    let p = surface.evaluate(uv[0], uv[1]).unwrap().point;
                    let p = [p[0], p[1], p[2]];
                    worst = worst
                        .max(sphere_residual(p, sphere_center, sphere_radius))
                        .max(torus_check(p));
                }
            }
        }
        worst
    }
    /// Exact rational 90-degree-arc parameter (the plane_torus map).
    fn arc_parameter(local: f64) -> f64 {
        let t = (local / 2.).tan();
        let s = std::f64::consts::SQRT_2 * t / (1. - t);
        s / (1. + s)
    }
    /// Meridian circle/circle oracle: roots of (rho-R)^2 + z^2 = r_t^2
    /// against rho^2 + (z-h)^2 = r_s^2 as (rho, z) pairs.
    fn meridian_roots(major: f64, minor: f64, h: f64, r_s: f64) -> [(f64, f64); 2] {
        let d = major.hypot(h);
        let a = (d * d + minor * minor - r_s * r_s) / (2. * d);
        let l = (minor * minor - a * a).sqrt();
        let root = |sign: f64| {
            (
                major - (a * major + sign * l * h) / d,
                (a * h - sign * l * major) / d,
            )
        };
        [root(1.), root(-1.)]
    }

    #[test]
    fn equator_symmetric_pair_matches_the_circle_circle_oracle() {
        // Torus R=3, r=1; sphere r=sqrt(5.2) centered at the torus center:
        // d=3, a=0.8, l=0.6 — the exact circle pair of radius 2.2 at
        // z = +-0.6.
        let sphere = crate::analytic::sphere(5.2_f64.sqrt()).unwrap();
        let torus = crate::analytic::torus(3., 1.).unwrap();
        let report = intersect_sphere_torus(&sphere, &torus, Options::default()).unwrap();
        let report = only_circles(&report, 2);
        for (component, z) in report.components.iter().zip([-0.6, 0.6]) {
            let (curve, center, radius, normal, sphere_uv, torus_uv, sampled) =
                circle_of(component);
            assert!((radius - 2.2).abs() <= 1e-12, "{radius}");
            assert!(
                sub(center, [0., 0., z]).iter().all(|x| x.abs() <= 1e-12),
                "{center:?}"
            );
            assert!(normal[2].abs() >= 1. - 1e-12, "{normal:?}");
            // Exact rational circle: four 90-degree arcs, weights cos(pi/4).
            assert_eq!(curve.degree, 2);
            assert_eq!(
                curve.knots,
                vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.]
            );
            assert_eq!(curve.control_points.len(), 9);
            assert_eq!(
                curve.weights,
                vec![
                    1., ARC_WEIGHT, 1., ARC_WEIGHT, 1., ARC_WEIGHT, 1., ARC_WEIGHT, 1.
                ]
            );
            // Sixteen samples satisfy both implicit equations.
            let mut worst = 0_f64;
            for i in 0..16 {
                let p = curve.evaluate(i as f64 / 4.).unwrap().point;
                let p = [p[0], p[1], p[2]];
                worst = worst
                    .max(sphere_residual(p, [0., 0., 0.], 5.2_f64.sqrt()))
                    .max(implicit(p, 3., 1.));
            }
            assert!(worst <= 1e-10, "{worst}");
            assert!(sampled <= 1e-10, "{sampled}");
            // Torus lift: one iso-v line per revolution quadrant patch of the
            // profile row, at the exact rational-arc v parameter.
            assert_eq!(torus_uv.len(), 4, "{torus_uv:?}");
            let phi = z.atan2(2.2 - 3.).rem_euclid(TAU);
            let quadrant = (phi / std::f64::consts::FRAC_PI_2).floor() as usize;
            let v0 = arc_parameter(phi - quadrant as f64 * std::f64::consts::FRAC_PI_2);
            for lift in torus_uv {
                assert_eq!(lift.arcs.len(), 1);
                let arc = &lift.arcs[0];
                assert_eq!(arc.degree, 1);
                assert!((arc.control_points[0][1] - v0).abs() <= 1e-12, "{arc:?}");
                assert!((arc.control_points[1][1] - v0).abs() <= 1e-12, "{arc:?}");
                assert_eq!(arc.control_points[0][0], 0.);
                assert_eq!(arc.control_points[1][0], 1.);
            }
            assert!(!sphere_uv.is_empty());
            let uv_worst = uv_samples(
                &sphere,
                &torus,
                [0., 0., 0.],
                5.2_f64.sqrt(),
                sphere_uv,
                torus_uv,
                |p| implicit(p, 3., 1.),
            );
            assert!(uv_worst <= 1e-9, "{uv_worst}");
        }
    }

    #[test]
    fn off_center_sphere_yields_two_distinct_circles() {
        // Torus R=3, r=1; sphere r=sqrt(26) centered at z=-4: d=5, a=0,
        // l=1 — the exact circles of radii 3.8 at z=-0.6 and 2.2 at z=0.6.
        let sphere = translated(
            &crate::analytic::sphere(26_f64.sqrt()).unwrap(),
            [0., 0., -4.],
        );
        let torus = crate::analytic::torus(3., 1.).unwrap();
        let report = intersect_sphere_torus(&sphere, &torus, Options::default()).unwrap();
        let report = only_circles(&report, 2);
        for (component, (rho, z)) in report.components.iter().zip([(3.8, -0.6), (2.2, 0.6)]) {
            let (curve, center, radius, _, sphere_uv, torus_uv, sampled) = circle_of(component);
            assert!((radius - rho).abs() <= 1e-12, "{radius}");
            assert!(
                sub(center, [0., 0., z]).iter().all(|x| x.abs() <= 1e-12),
                "{center:?}"
            );
            let mut worst = 0_f64;
            for i in 0..16 {
                let p = curve.evaluate(i as f64 / 4.).unwrap().point;
                let p = [p[0], p[1], p[2]];
                worst = worst
                    .max(sphere_residual(p, [0., 0., -4.], 26_f64.sqrt()))
                    .max(implicit(p, 3., 1.));
            }
            assert!(worst <= 1e-10, "{worst}");
            assert!(sampled <= 1e-10, "{sampled}");
            assert_eq!(torus_uv.len(), 4);
            let uv_worst = uv_samples(
                &sphere,
                &torus,
                [0., 0., -4.],
                26_f64.sqrt(),
                sphere_uv,
                torus_uv,
                |p| implicit(p, 3., 1.),
            );
            assert!(uv_worst <= 1e-9, "{uv_worst}");
        }
    }

    #[test]
    fn generic_height_pair_matches_the_meridian_oracle() {
        // Torus R=3, r=1; sphere r=2.2 centered at z=0.5: both roots land
        // strictly inside the meridian circle and revolve into two circles.
        let sphere = translated(&crate::analytic::sphere(2.2).unwrap(), [0., 0., 0.5]);
        let torus = crate::analytic::torus(3., 1.).unwrap();
        let report = intersect_sphere_torus(&sphere, &torus, Options::default()).unwrap();
        let report = only_circles(&report, 2);
        let mut roots = meridian_roots(3., 1., 0.5, 2.2);
        roots.sort_by(|a, b| a.1.total_cmp(&b.1));
        for (component, (rho, z)) in report.components.iter().zip(roots) {
            let (_, center, radius, _, _, torus_uv, sampled) = circle_of(component);
            assert!((radius - rho).abs() <= 1e-12, "{radius} vs {rho}");
            assert!(
                sub(center, [0., 0., z]).iter().all(|x| x.abs() <= 1e-12),
                "{center:?} vs {z}"
            );
            assert!(sampled <= 1e-10, "{sampled}");
            assert_eq!(torus_uv.len(), 4);
        }
    }

    #[test]
    fn inner_equator_tangency_stays_a_tangency_region() {
        // Sphere centered at the torus center with r == R - r_t = 2: the
        // meridian circles touch externally (d = 3 = r_t + r_s), the contact
        // revolves into the inner equator circle — never a guessed circle.
        let torus = crate::analytic::torus(3., 1.).unwrap();
        for r in [2., 2. + 2e-15, 2. - 2e-15] {
            let sphere = crate::analytic::sphere(r).unwrap();
            let report = intersect_sphere_torus(&sphere, &torus, Options::default()).unwrap();
            assert!(report.components.is_empty(), "{report:?}");
            assert_eq!(report.coverage, Coverage::Incomplete);
            assert_eq!(
                report.unresolved[0].reason,
                UnresolvedReason::TangencyOrMultipleRoot
            );
            assert!(!report.permits_topology_change());
        }
        // Just clear of the band on the outside: provable miss, resolved.
        let clear = crate::analytic::sphere(2. - 1e-9).unwrap();
        let report = intersect_sphere_torus(&clear, &torus, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        // Just across: two small transverse circles around the inner equator.
        let across = crate::analytic::sphere(2. + 1e-9).unwrap();
        let report = intersect_sphere_torus(&across, &torus, Options::default()).unwrap();
        only_circles(&report, 2);
    }

    #[test]
    fn outer_equator_internal_tangency_stays_a_tangency_region() {
        // Sphere centered at the torus center with r == R + r_t = 4: the
        // meridian circles touch internally (d = 3 = |r_t - r_s|) at the
        // outer equator — never a guessed circle.
        let torus = crate::analytic::torus(3., 1.).unwrap();
        for r in [4., 4. + 2e-15, 4. - 2e-15] {
            let sphere = crate::analytic::sphere(r).unwrap();
            let report = intersect_sphere_torus(&sphere, &torus, Options::default()).unwrap();
            assert!(report.components.is_empty(), "{report:?}");
            assert_eq!(report.coverage, Coverage::Incomplete);
            assert_eq!(
                report.unresolved[0].reason,
                UnresolvedReason::TangencyOrMultipleRoot
            );
        }
        // Just larger: the sphere swallows the tube, provably contained.
        let swallow = crate::analytic::sphere(4. + 1e-9).unwrap();
        let report = intersect_sphere_torus(&swallow, &torus, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        // Just smaller: two transverse circles around the outer equator.
        let across = crate::analytic::sphere(4. - 1e-9).unwrap();
        let report = intersect_sphere_torus(&across, &torus, Options::default()).unwrap();
        only_circles(&report, 2);
        // Off-center external tangency: r_s = hypot(R, h) - r_t at h = 0.5.
        let tangent = 3_f64.hypot(0.5) - 1.;
        let sphere = translated(&crate::analytic::sphere(tangent).unwrap(), [0., 0., 0.5]);
        let report = intersect_sphere_torus(&sphere, &torus, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
    }

    #[test]
    fn zero_circle_configurations_resolve_empty() {
        let torus = crate::analytic::torus(3., 1.).unwrap();
        // Small sphere floating in the hole, clear of the tube.
        let hole = translated(&crate::analytic::sphere(1.).unwrap(), [0., 0., 0.5]);
        // Sphere far along the axis, no reach back.
        let far = translated(&crate::analytic::sphere(1.).unwrap(), [0., 0., 10.]);
        // Huge sphere swallowing the whole torus (contained meridian circle).
        let swallow = crate::analytic::sphere(10.).unwrap();
        for sphere in [&hole, &far, &swallow] {
            let report = intersect_sphere_torus(sphere, &torus, Options::default()).unwrap();
            assert!(
                report.components.is_empty() && report.unresolved.is_empty(),
                "{report:?}"
            );
            assert_eq!(report.coverage, Coverage::NumericallyResolved);
        }
    }

    #[test]
    fn near_axial_offset_within_the_band_stays_unresolved() {
        let torus = crate::analytic::torus(3., 1.).unwrap();
        let sphere = translated(
            &crate::analytic::sphere(5.2_f64.sqrt()).unwrap(),
            [1e-10, 0., 0.],
        );
        let report = intersect_sphere_torus(&sphere, &torus, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::NearCoincidence
        );
    }

    #[test]
    fn clearly_off_axis_pairs_are_unsupported_regions() {
        let torus = crate::analytic::torus(3., 1.).unwrap();
        let sphere = translated(
            &crate::analytic::sphere(5.2_f64.sqrt()).unwrap(),
            [0.5, 0., 0.],
        );
        let report = intersect_sphere_torus(&sphere, &torus, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(report.unresolved.len(), 1);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::UnsupportedSurface
        );
        assert_eq!(
            report.unresolved[0].parameter_box,
            vec![0., 1., 0., 1., 0., 1., 0., 1.]
        );
    }

    #[test]
    fn noncanonical_operands_are_explicit_unsupported_regions() {
        let sphere = crate::analytic::sphere(5.2_f64.sqrt()).unwrap();
        let torus = crate::analytic::torus(3., 1.).unwrap();
        // Cylinders, tori, cuboids and tori-as-first-operand are neither
        // canonical operand; swapped operand order is refused by the fixed
        // order.
        for (a, b) in [
            (
                crate::analytic::cylinder(1., 3.).unwrap(),
                crate::analytic::torus(3., 1.).unwrap(),
            ),
            (
                crate::analytic::sphere(2.).unwrap(),
                crate::analytic::sphere(2.).unwrap(),
            ),
            (
                crate::analytic::torus(3., 1.).unwrap(),
                crate::analytic::torus(2., 1.).unwrap(),
            ),
            (
                crate::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap(),
                crate::analytic::torus(3., 1.).unwrap(),
            ),
            (
                crate::analytic::torus(3., 1.).unwrap(),
                crate::analytic::sphere(2.).unwrap(),
            ),
        ] {
            let report = intersect_sphere_torus(&a, &b, Options::default()).unwrap();
            assert!(report.components.is_empty(), "{report:?}");
            assert_eq!(report.coverage, Coverage::Incomplete);
            assert_eq!(report.unresolved.len(), 1);
            assert_eq!(
                report.unresolved[0].reason,
                UnresolvedReason::UnsupportedSurface
            );
        }
        // A canonical pair still resolves.
        let report = intersect_sphere_torus(&sphere, &torus, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
        assert_eq!(report.components.len(), 2);
        // A structurally perturbed torus (one moved control point) fails the
        // exact certification and is an unsupported region, never a fallback.
        let mut perturbed = torus.clone();
        perturbed.faces[0].surface.control_points[1][1][0] += 1e-6;
        perturbed.rebuild_topology_ids();
        let report = intersect_sphere_torus(&sphere, &perturbed, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::UnsupportedSurface
        );
    }

    #[test]
    fn rotated_translated_placement_keeps_the_exact_circles() {
        // Rigid placement of the whole axial configuration about X by 0.5.
        let angle = 0.5;
        let offset = [0.3, -0.2, 1.1];
        let sphere = rotated_translated(
            &translated(
                &crate::analytic::sphere(26_f64.sqrt()).unwrap(),
                [0., 0., -4.],
            ),
            angle,
            offset,
        );
        let torus = rotated_translated(&crate::analytic::torus(3., 1.).unwrap(), angle, offset);
        let report = intersect_sphere_torus(&sphere, &torus, Options::default()).unwrap();
        let report = only_circles(&report, 2);
        // Independent binary64 oracle in the placed frame.
        let (sin, cos) = angle.sin_cos();
        let placed = |p: [f64; 3]| {
            [
                p[0] + offset[0],
                cos * p[1] - sin * p[2] + offset[1],
                sin * p[1] + cos * p[2] + offset[2],
            ]
        };
        let sphere_center = placed([0., 0., -4.]);
        let axis = sub(placed([0., 0., 1.]), placed([0., 0., 0.]));
        for (component, (rho, z)) in report.components.iter().zip([(3.8, -0.6), (2.2, 0.6)]) {
            let (curve, center, radius, normal, sphere_uv, torus_uv, sampled) =
                circle_of(component);
            let expected = placed([0., 0., z]);
            assert!((radius - rho).abs() <= 1e-12, "{radius}");
            assert!(
                sub(center, expected).iter().all(|x| x.abs() <= 1e-12),
                "{center:?}"
            );
            assert!(
                sub(normal, axis).iter().all(|x| x.abs() <= 1e-12),
                "{normal:?}"
            );
            let mut worst = 0_f64;
            for i in 0..16 {
                let p = curve.evaluate(i as f64 / 4.).unwrap().point;
                let p = [p[0], p[1], p[2]];
                worst = worst.max(sphere_residual(p, sphere_center, 26_f64.sqrt()));
                // Placed torus implicit via the axial/radial decomposition.
                let rel = sub(p, placed([0., 0., 0.]));
                let a = dot(rel, axis);
                let perp = sub(rel, axis.map(|x| x * a));
                let rho_p = perp[0].hypot(perp[1]).hypot(perp[2]);
                let base = rho_p * rho_p + a * a + 9. - 1.;
                worst = worst.max((base * base - 36. * rho_p * rho_p).abs());
            }
            assert!(worst <= 1e-9, "{worst}");
            assert!(sampled <= 1e-10);
            assert_eq!(torus_uv.len(), 4);
            assert!(!sphere_uv.is_empty());
            let axis = normal;
            let uv_worst = uv_samples(
                &sphere,
                &torus,
                sphere_center,
                26_f64.sqrt(),
                sphere_uv,
                torus_uv,
                |p| {
                    let rel = sub(p, placed([0., 0., 0.]));
                    let a = dot(rel, axis);
                    let perp = sub(rel, axis.map(|x| x * a));
                    let rho_p = perp[0].hypot(perp[1]).hypot(perp[2]);
                    let base = rho_p * rho_p + a * a + 9. - 1.;
                    (base * base - 36. * rho_p * rho_p).abs()
                },
            );
            assert!(uv_worst <= 1e-9, "{uv_worst}");
        }
    }
}
