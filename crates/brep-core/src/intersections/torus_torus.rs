//! Analytic torus/torus intersection for canonical solids in the coaxial
//! configuration (the two axes certified parallel and coincident; the two
//! center planes may differ by an axial offset h).
//!
//! Both operands must be the exact canonical ring-torus solid of
//! `analytic::torus` (sixteen rational biquadratic patches, strict ring tori
//! only — the constructor refuses horn and spindle tori), recognized by
//! `plane_torus::recognize_torus`. Rigid affine placements are admitted.
//! Anything else is an explicit `UnsupportedSurface` region — this cell
//! never falls back to numerical surface/surface subdivision. Coaxiality is
//! certified, never forced: pure-rounding axis tilts and center-line offsets
//! snap, recognition-scale tilts or offsets report `NearCoincidence`, and a
//! clearly tilted or off-axis pair is `UnsupportedSurface` (the general
//! torus/torus pair is a quartic, out of scope). Classification uses outward
//! binary64 bands widened by both recognition deviations.
//!
//! Both surfaces are surfaces of revolution about the shared axis, so the
//! section reduces to the meridian half-plane (rho >= 0, z measured along
//! the first torus's axis from its center plane): circle
//! (rho - R_1)^2 + z^2 = r_1^2 against circle
//! (rho - R_2)^2 + (z - h)^2 = r_2^2 with h the axial offset of the second
//! center plane. Circle/circle in the meridian plane (the same closed form
//! as sphere_torus: a = (d^2 + r_1^2 - r_2^2)/(2d), l^2 = r_1^2 - a^2 with d
//! the meridian center distance between (R_1, 0) and (R_2, h)) gives zero,
//! one (double) or two roots; each root (rho*, z*) revolves into the exact
//! 3D circle of radius rho* at height z*:
//! - two distinct roots with rho* provably beyond the band revolve into two
//!   exact circles; a one-circle resolved configuration is structurally
//!   unreachable for a strict ring-torus pair (every meridian root keeps
//!   rho* >= R - r >= 1e-5, far beyond the band) and is documented honestly
//!   rather than tested;
//! - a double root (the meridian circles tangent, d == r_1 + r_2 externally
//!   or d == |r_1 - r_2| internally) stays `TangencyOrMultipleRoot` — the
//!   tangent contact revolves into a circle but is never a guessed circle;
//! - a root whose radius rho* collapses into the band degenerates to a pole
//!   point on the axis — unresolved, never guessed (unreachable for a
//!   canonical strict ring torus, kept as an honest guard);
//! - equal meridian circles (R_1 == R_2, h == 0, r_1 == r_2 within the
//!   bands) are coincident surfaces and report `CoincidentTrim`, never a
//!   guessed curve;
//! - no roots (provably separate d > r_1 + r_2, or provably contained
//!   d + min(r_1, r_2) < max(r_1, r_2), beyond the band) resolves empty by
//!   the interval signs.
//! Resolved contacts are exact rational circles (four 90-degree arcs,
//! weights cos(pi/4)) with UV lifts on BOTH tori: iso-v parallels at the
//! exact profile angle phi = atan2(z*, rho* - R) mapped through the exact
//! rational 90-degree-arc parameter map t = s/(1+s),
//! s = sqrt(2) tan(phi/2)/(1 - tan(phi/2)), one degree-1 line per revolution
//! quadrant patch of the profile row, on both 4x4 tilings (the second
//! torus's profile angle is measured in its own frame — its axis may be
//! anti-parallel to the first). Nothing here authorizes a topology change.
use super::plane_torus::{
    CanonicalTorus, TorusPatchCurve, lift_parallel, recognize_torus, torus_residual,
};
use super::sphere_sphere::{ARC_WEIGHT, RECOGNITION, circle_curve};
use super::*;
use crate::Model;

const TAU: f64 = std::f64::consts::TAU;

#[derive(Clone, Debug)]
pub enum TorusTorusComponent {
    /// Transverse intersection: the exact circle plus both UV lifts.
    Circle {
        /// Exact rational full circle: degree 2, knots 0..=4, four 90-degree
        /// arcs with weights cos(pi/4).
        curve: Curve,
        center: [f64; 3],
        radius: f64,
        /// Unit shared axis (the first torus's axis; the circle plane normal).
        normal: [f64; 3],
        first_uv: Vec<TorusPatchCurve>,
        second_uv: Vec<TorusPatchCurve>,
        /// Worst residual over 16 circle samples against both scaled torus
        /// implicit equations.
        max_sample_residual: f64,
    },
}

/// Exact circle component at height `z` (along the first torus's axis from
/// its center plane) with radius `rho`, with UV lifts on both tori and a
/// 16-sample residual bound against both implicit equations.
fn circle_component(
    first: &CanonicalTorus,
    second: &CanonicalTorus,
    z: f64,
    rho: f64,
) -> Result<TorusTorusComponent> {
    let center = std::array::from_fn(|k| first.center[k] + z * first.axis[k]);
    let curve = circle_curve(center, rho, first.frame[0], first.frame[1]);
    // First-torus lift: the iso-v parallel at the exact profile angle.
    let profile = z.atan2(rho - first.major).rem_euclid(TAU);
    let first_uv = lift_parallel(first, profile, &[(0., TAU)]);
    // Second-torus lift in its own frame: its axis may be anti-parallel to
    // the first, so the axial height is measured along its own axis.
    let z2 = dot(sub(center, second.center), second.axis);
    let profile2 = z2.atan2(rho - second.major).rem_euclid(TAU);
    let second_uv = lift_parallel(second, profile2, &[(0., TAU)]);
    let mut max_sample_residual = 0_f64;
    for i in 0..16 {
        let jet = curve.evaluate(i as f64 / 4.)?;
        let point = [jet.point[0], jet.point[1], jet.point[2]];
        max_sample_residual = max_sample_residual
            .max(torus_residual(first, point))
            .max(torus_residual(second, point));
    }
    Ok(TorusTorusComponent::Circle {
        curve,
        center,
        radius: rho,
        normal: first.axis,
        first_uv,
        second_uv,
        max_sample_residual,
    })
}

/// Analytic torus/torus intersection of two canonical ring-torus solids,
/// coaxial configuration only (the two center planes may differ by the
/// axial offset h). Non-canonical operands and clearly tilted or off-axis
/// pairs are explicit unsupported regions, never a numerical fallback;
/// recognition-scale near-coaxial bands, the meridian circle/circle
/// tangencies (double roots), the pole-collapse guard and the coincident
/// pair stay unresolved — tangent contacts are never guessed. Provable
/// misses (separate or contained meridian circles) resolve empty.
pub fn intersect_torus_torus(
    first_model: &Model,
    second_model: &Model,
    options: Options,
) -> Result<Report<TorusTorusComponent>> {
    let options = options.validate()?;
    let _ = options;
    let mut report = Report::default();
    let domain = vec![0., 1., 0., 1., 0., 1., 0., 1.];
    let (Some(first), Some(second)) = (
        recognize_torus(first_model)?,
        recognize_torus(second_model)?,
    ) else {
        report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    };
    report.boxes_visited = 1;
    let delta = sub(second.center, first.center);
    let terms = first.major
        + first.minor
        + second.major
        + second.minor
        + delta.iter().map(|v| v.abs()).fold(0., f64::max)
        + 1.;
    // Outward binary64 classification band: both recognition deviations plus
    // a rounding allowance on the axial coordinates and radii.
    let band = first.error + second.error + 16. * f64::EPSILON * terms;
    // Axis parallelism is certified, never forced: pure-rounding direction
    // disagreement snaps to parallel, recognition-scale tilt reports
    // near_coincidence, and a clearly tilted pair is the unsupported general
    // (quartic) case. Either orientation is admitted: the torus surface is
    // symmetric about its center plane, so the axis sign is irrelevant.
    let direction = cross(first.axis, second.axis);
    let tilt = direction[0].hypot(direction[1]).hypot(direction[2]);
    let axis_snap =
        64. * f64::EPSILON + first.error / (2. * first.major) + second.error / (2. * second.major);
    if tilt > axis_snap {
        let reason = if tilt <= RECOGNITION {
            UnresolvedReason::NearCoincidence
        } else {
            UnresolvedReason::UnsupportedSurface
        };
        report.unresolved(domain, reason);
        return Ok(report);
    }
    // Coaxiality: a center-line distance at pure rounding scale snaps to
    // zero; a recognition-scale offset is near_coincidence, never forced;
    // a clear offset is the unsupported general (quartic) configuration.
    let along = dot(delta, first.axis);
    let perp = sub(delta, first.axis.map(|x| x * along));
    let pscale = perp.iter().map(|v| v.abs()).fold(0., f64::max);
    let d_perp = if pscale == 0. {
        0.
    } else {
        let scaled = perp.map(|x| x / pscale);
        scaled[0].hypot(scaled[1]).hypot(scaled[2]) * pscale
    };
    let dist_snap = 64. * f64::EPSILON * terms + first.error + second.error;
    if d_perp > dist_snap {
        let reason = if d_perp <= RECOGNITION * terms {
            UnresolvedReason::NearCoincidence
        } else {
            UnresolvedReason::UnsupportedSurface
        };
        report.unresolved(domain, reason);
        return Ok(report);
    }
    let (r1, big_r1) = (first.minor, first.major);
    let (r2, big_r2) = (second.minor, second.major);
    // Meridian centers (R_1, 0) and (R_2, h) in the half-plane, h the axial
    // offset of the second center plane along the first torus's axis; the
    // meridian circle is symmetric in z, so the sign of h is irrelevant.
    let h = along;
    let dr = big_r2 - big_r1;
    let d = dr.hypot(h);
    if !d.is_finite() {
        report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    }
    if d <= band {
        if (r1 - r2).abs() <= band {
            // Equal meridian circles within the band: the surfaces coincide.
            report.unresolved(domain, UnresolvedReason::CoincidentTrim);
        }
        // Concentric within the band with provably different radii:
        // d + min(r_1, r_2) <= band + min < max — certified containment.
        return Ok(report);
    }
    let sum = r1 + r2;
    let diff = (r1 - r2).abs();
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
    // Two distinct roots: with e = (C_2 - C_1)/d = (dr/d, h/d) and the
    // normal n = (-h/d, dr/d), the roots are C_1 + a e +- l n where
    // a = (d^2 + r_1^2 - r_2^2)/(2d) and l^2 = r_1^2 - a^2.
    let a = (d * d + r1 * r1 - r2 * r2) / (2. * d);
    let l2 = r1 * r1 - a * a;
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
        let rho = big_r1 + (a * dr - sign * l * h) / d;
        let z = (a * h + sign * l * dr) / d;
        if rho.abs() <= band {
            // The revolved circle degenerates to a pole point on the axis
            // (unreachable for a canonical strict ring torus, whose roots
            // keep rho >= R - r >= 1e-5): unresolved, never guessed.
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
    // Deterministic order: by height along the first axis, then by radius.
    found.sort_by(|x, y| x.0.total_cmp(&y.0).then(x.1.total_cmp(&y.1)));
    for (z, rho) in found {
        report
            .components
            .push(circle_component(&first, &second, z, rho)?);
    }
    Ok(report)
}

impl value_codec::Serialize for TorusTorusComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Circle {
                curve,
                center,
                radius,
                normal,
                first_uv,
                second_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"circle","curve":curve,"center":center,"radius":radius,
                "normal":normal,"firstUv":first_uv,"secondUv":second_uv,
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
    /// Rotate pi about X then lift along Z: y -> -y, z -> z0 - z (axis flips).
    fn flipped(model: &Model, z0: f64) -> Model {
        crate::transform::affine(
            model,
            [
                [1., 0., 0., 0.],
                [0., -1., 0., 0.],
                [0., 0., -1., z0],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap()
    }
    fn only_circles(
        report: &Report<TorusTorusComponent>,
        count: usize,
    ) -> &Report<TorusTorusComponent> {
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        assert!(report.unresolved.is_empty(), "{report:?}");
        assert!(!report.permits_topology_change());
        assert_eq!(report.components.len(), count, "{report:?}");
        report
    }
    #[allow(clippy::type_complexity)]
    fn circle_of(
        component: &TorusTorusComponent,
    ) -> (
        &Curve,
        [f64; 3],
        f64,
        [f64; 3],
        &[TorusPatchCurve],
        &[TorusPatchCurve],
        f64,
    ) {
        let TorusTorusComponent::Circle {
            curve,
            center,
            radius,
            normal,
            first_uv,
            second_uv,
            max_sample_residual,
        } = component;
        (
            curve,
            *center,
            *radius,
            *normal,
            first_uv,
            second_uv,
            *max_sample_residual,
        )
    }
    /// Torus implicit residual (unscaled quartic) in the canonical z-up frame.
    fn implicit(point: [f64; 3], major: f64, minor: f64) -> f64 {
        let total = dot(point, point);
        let rho2 = point[0] * point[0] + point[1] * point[1];
        let base = total + major * major - minor * minor;
        (base * base - 4. * major * major * rho2).abs()
    }
    /// Exact rational 90-degree-arc parameter (the plane_torus map).
    fn arc_parameter(local: f64) -> f64 {
        let t = (local / 2.).tan();
        let s = std::f64::consts::SQRT_2 * t / (1. - t);
        s / (1. + s)
    }
    /// Meridian circle/circle oracle: roots of (rho-R1)^2 + z^2 = r1^2
    /// against (rho-R2)^2 + (z-h)^2 = r2^2 as (rho, z) pairs.
    fn meridian_roots(r1: f64, big_r1: f64, r2: f64, big_r2: f64, h: f64) -> [(f64, f64); 2] {
        let dr = big_r2 - big_r1;
        let d = dr.hypot(h);
        let a = (d * d + r1 * r1 - r2 * r2) / (2. * d);
        let l = (r1 * r1 - a * a).sqrt();
        let root = |sign: f64| {
            (
                big_r1 + (a * dr - sign * l * h) / d,
                (a * h + sign * l * dr) / d,
            )
        };
        [root(1.), root(-1.)]
    }
    /// Worst residual of lifted UV arcs evaluated through their own surface,
    /// against both caller-supplied torus predicates.
    fn uv_samples(
        first_model: &Model,
        second_model: &Model,
        first_uv: &[TorusPatchCurve],
        second_uv: &[TorusPatchCurve],
        first_check: impl Fn([f64; 3]) -> f64,
        second_check: impl Fn([f64; 3]) -> f64,
    ) -> f64 {
        let mut worst = 0_f64;
        for (model, lifts) in [(first_model, first_uv), (second_model, second_uv)] {
            for lift in lifts {
                let surface = &model.faces[lift.patch].surface;
                for arc in &lift.arcs {
                    for k in 0..=8 {
                        let uv = arc.evaluate(k as f64 / 8.).unwrap().point;
                        assert!((0. ..=1.).contains(&uv[0]) && (0. ..=1.).contains(&uv[1]));
                        let p = surface.evaluate(uv[0], uv[1]).unwrap().point;
                        let p = [p[0], p[1], p[2]];
                        worst = worst.max(first_check(p)).max(second_check(p));
                    }
                }
            }
        }
        worst
    }

    #[test]
    fn two_circle_pair_matches_the_circle_circle_oracle() {
        // Torus R=3, r=1 at the origin against torus R=3, r=sqrt(5.2) lifted
        // to z=3: the meridian centers (3,0) and (3,3) sit at d=3, a=0.8,
        // l=0.6 — the exact circle pair of radii 2.4 and 3.6, both at
        // z=0.8, sorted by height then radius.
        let first = crate::analytic::torus(3., 1.).unwrap();
        let second = translated(
            &crate::analytic::torus(3., 5.2_f64.sqrt()).unwrap(),
            [0., 0., 3.],
        );
        let report = intersect_torus_torus(&first, &second, Options::default()).unwrap();
        let report = only_circles(&report, 2);
        for (component, rho) in report.components.iter().zip([2.4, 3.6]) {
            let (curve, center, radius, normal, first_uv, second_uv, sampled) =
                circle_of(component);
            assert!((radius - rho).abs() <= 1e-12, "{radius}");
            assert!(
                sub(center, [0., 0., 0.8]).iter().all(|x| x.abs() <= 1e-12),
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
            // Sixteen samples satisfy both torus implicit equations.
            let mut worst = 0_f64;
            for i in 0..16 {
                let p = curve.evaluate(i as f64 / 4.).unwrap().point;
                let p = [p[0], p[1], p[2]];
                worst = worst.max(implicit(p, 3., 1.)).max(implicit(
                    sub(p, [0., 0., 3.]),
                    3.,
                    5.2_f64.sqrt(),
                ));
            }
            assert!(worst <= 1e-10, "{worst}");
            assert!(sampled <= 1e-10, "{sampled}");
            // First-torus lift: one iso-v line per revolution quadrant patch
            // of the profile row, at the exact rational-arc v parameter.
            assert_eq!(first_uv.len(), 4, "{first_uv:?}");
            let phi = 0.8_f64.atan2(rho - 3.).rem_euclid(TAU);
            let quadrant = (phi / std::f64::consts::FRAC_PI_2).floor() as usize;
            let v0 = arc_parameter(phi - quadrant as f64 * std::f64::consts::FRAC_PI_2);
            for lift in first_uv {
                assert_eq!(lift.arcs.len(), 1);
                let arc = &lift.arcs[0];
                assert_eq!(arc.degree, 1);
                assert!((arc.control_points[0][1] - v0).abs() <= 1e-12, "{arc:?}");
                assert!((arc.control_points[1][1] - v0).abs() <= 1e-12, "{arc:?}");
                assert_eq!(arc.control_points[0][0], 0.);
                assert_eq!(arc.control_points[1][0], 1.);
            }
            // Second-torus lift at the exact profile angle atan2(z-3, rho-3).
            assert_eq!(second_uv.len(), 4, "{second_uv:?}");
            let phi2 = (0.8_f64 - 3.).atan2(rho - 3.).rem_euclid(TAU);
            let quadrant2 = (phi2 / std::f64::consts::FRAC_PI_2).floor() as usize;
            let v2 = arc_parameter(phi2 - quadrant2 as f64 * std::f64::consts::FRAC_PI_2);
            for lift in second_uv {
                assert_eq!(lift.arcs.len(), 1);
                let arc = &lift.arcs[0];
                assert_eq!(arc.degree, 1);
                assert!((arc.control_points[0][1] - v2).abs() <= 1e-12, "{arc:?}");
                assert!((arc.control_points[1][1] - v2).abs() <= 1e-12, "{arc:?}");
            }
            let uv_worst = uv_samples(
                &first,
                &second,
                first_uv,
                second_uv,
                |p| implicit(p, 3., 1.),
                |p| implicit(sub(p, [0., 0., 3.]), 3., 5.2_f64.sqrt()),
            );
            assert!(uv_worst <= 1e-9, "{uv_worst}");
        }
    }

    #[test]
    fn equal_minor_radii_pair_matches_the_oracle() {
        // Torus R=3, r=1 at the origin against torus R=3, r=1 lifted to
        // z=1: d=1, a=1/2, l=sqrt(3)/2 — the exact circles of radii
        // 3 -+ sqrt(3)/2, both at z=1/2.
        let first = crate::analytic::torus(3., 1.).unwrap();
        let second = translated(&crate::analytic::torus(3., 1.).unwrap(), [0., 0., 1.]);
        let report = intersect_torus_torus(&first, &second, Options::default()).unwrap();
        let report = only_circles(&report, 2);
        let root = 0.75_f64.sqrt();
        for (component, rho) in report.components.iter().zip([3. - root, 3. + root]) {
            let (curve, center, radius, _, first_uv, second_uv, sampled) = circle_of(component);
            assert!((radius - rho).abs() <= 1e-12, "{radius} vs {rho}");
            assert!(
                sub(center, [0., 0., 0.5]).iter().all(|x| x.abs() <= 1e-12),
                "{center:?}"
            );
            let mut worst = 0_f64;
            for i in 0..16 {
                let p = curve.evaluate(i as f64 / 4.).unwrap().point;
                let p = [p[0], p[1], p[2]];
                worst = worst
                    .max(implicit(p, 3., 1.))
                    .max(implicit(sub(p, [0., 0., 1.]), 3., 1.));
            }
            assert!(worst <= 1e-10, "{worst}");
            assert!(sampled <= 1e-10, "{sampled}");
            assert_eq!(first_uv.len(), 4);
            assert_eq!(second_uv.len(), 4);
            let uv_worst = uv_samples(
                &first,
                &second,
                first_uv,
                second_uv,
                |p| implicit(p, 3., 1.),
                |p| implicit(sub(p, [0., 0., 1.]), 3., 1.),
            );
            assert!(uv_worst <= 1e-9, "{uv_worst}");
        }
    }

    #[test]
    fn generic_configuration_matches_the_meridian_oracle() {
        // Torus R=3, r=1 at the origin against torus R=2.5, r=1.2 lifted to
        // z=0.7: both roots land strictly inside both meridian circles and
        // revolve into two circles.
        let first = crate::analytic::torus(3., 1.).unwrap();
        let second = translated(&crate::analytic::torus(2.5, 1.2).unwrap(), [0., 0., 0.7]);
        let report = intersect_torus_torus(&first, &second, Options::default()).unwrap();
        let report = only_circles(&report, 2);
        let mut roots = meridian_roots(1., 3., 1.2, 2.5, 0.7);
        roots.sort_by(|x, y| x.1.total_cmp(&y.1).then(x.0.total_cmp(&y.0)));
        for (component, (rho, z)) in report.components.iter().zip(roots) {
            let (curve, center, radius, _, first_uv, second_uv, sampled) = circle_of(component);
            assert!((radius - rho).abs() <= 1e-12, "{radius} vs {rho}");
            assert!(
                sub(center, [0., 0., z]).iter().all(|x| x.abs() <= 1e-12),
                "{center:?} vs {z}"
            );
            assert!(sampled <= 1e-10, "{sampled}");
            assert_eq!(first_uv.len(), 4);
            assert_eq!(second_uv.len(), 4);
            let uv_worst = uv_samples(
                &first,
                &second,
                first_uv,
                second_uv,
                |p| implicit(p, 3., 1.),
                |p| implicit(sub(p, [0., 0., 0.7]), 2.5, 1.2),
            );
            assert!(uv_worst <= 1e-9, "{uv_worst}");
        }
    }

    #[test]
    fn anti_axial_second_torus_keeps_the_exact_circles() {
        // The two-circle configuration with the second torus flipped
        // (rotation pi about X, then z -> 6 - z): its center plane stays at
        // z=3 but its axis is -z. The surface is symmetric, so the exact
        // circles are unchanged and the second lift follows its own frame.
        let first = crate::analytic::torus(3., 1.).unwrap();
        let second = flipped(
            &translated(
                &crate::analytic::torus(3., 5.2_f64.sqrt()).unwrap(),
                [0., 0., 3.],
            ),
            6.,
        );
        let report = intersect_torus_torus(&first, &second, Options::default()).unwrap();
        let report = only_circles(&report, 2);
        for (component, rho) in report.components.iter().zip([2.4, 3.6]) {
            let (_, center, radius, _, first_uv, second_uv, sampled) = circle_of(component);
            assert!((radius - rho).abs() <= 1e-12, "{radius}");
            assert!(
                sub(center, [0., 0., 0.8]).iter().all(|x| x.abs() <= 1e-12),
                "{center:?}"
            );
            assert!(sampled <= 1e-10, "{sampled}");
            assert_eq!(first_uv.len(), 4);
            // Flipped lift: profile angle atan2(-(z-3), rho-3) in its frame.
            assert_eq!(second_uv.len(), 4);
            let phi2 = (3. - 0.8_f64).atan2(rho - 3.).rem_euclid(TAU);
            let quadrant2 = (phi2 / std::f64::consts::FRAC_PI_2).floor() as usize;
            let v2 = arc_parameter(phi2 - quadrant2 as f64 * std::f64::consts::FRAC_PI_2);
            for lift in second_uv {
                let arc = &lift.arcs[0];
                assert!((arc.control_points[0][1] - v2).abs() <= 1e-12, "{arc:?}");
            }
            let uv_worst = uv_samples(
                &first,
                &second,
                first_uv,
                second_uv,
                |p| implicit(p, 3., 1.),
                |p| implicit(sub(p, [0., 0., 3.]), 3., 5.2_f64.sqrt()),
            );
            assert!(uv_worst <= 1e-9, "{uv_worst}");
        }
    }

    #[test]
    fn external_meridian_tangency_stays_a_tangency_region() {
        // Torus R=3, r=1 at the origin against torus R=3, r=1 at z=2: the
        // meridian circles touch externally (d = 2 = r_1 + r_2) at rho=3,
        // z=1 — the contact revolves into a circle but is never guessed.
        let first = crate::analytic::torus(3., 1.).unwrap();
        for dz in [0., 2e-15, -2e-15] {
            let second = translated(&crate::analytic::torus(3., 1.).unwrap(), [0., 0., 2. + dz]);
            let report = intersect_torus_torus(&first, &second, Options::default()).unwrap();
            assert!(report.components.is_empty(), "{report:?}");
            assert_eq!(report.coverage, Coverage::Incomplete);
            assert_eq!(
                report.unresolved[0].reason,
                UnresolvedReason::TangencyOrMultipleRoot
            );
            assert!(!report.permits_topology_change());
        }
        // Just clear of the band: provable miss, resolved.
        let clear = translated(
            &crate::analytic::torus(3., 1.).unwrap(),
            [0., 0., 2. + 1e-9],
        );
        let report = intersect_torus_torus(&first, &clear, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        // Just across: two small transverse circles around the touch point.
        let across = translated(
            &crate::analytic::torus(3., 1.).unwrap(),
            [0., 0., 2. - 1e-9],
        );
        let report = intersect_torus_torus(&first, &across, Options::default()).unwrap();
        only_circles(&report, 2);
    }

    #[test]
    fn internal_meridian_tangency_stays_a_tangency_region() {
        // Torus R=3, r=1 at the origin against torus R=3, r=2 at z=1: the
        // meridian circles touch internally (d = 1 = |r_1 - r_2|) at rho=3,
        // z=-1 — never a guessed circle.
        let first = crate::analytic::torus(3., 1.).unwrap();
        for dr in [0., 2e-15, -2e-15] {
            let second = translated(&crate::analytic::torus(3., 2. + dr).unwrap(), [0., 0., 1.]);
            let report = intersect_torus_torus(&first, &second, Options::default()).unwrap();
            assert!(report.components.is_empty(), "{report:?}");
            assert_eq!(report.coverage, Coverage::Incomplete);
            assert_eq!(
                report.unresolved[0].reason,
                UnresolvedReason::TangencyOrMultipleRoot
            );
        }
        // Just larger: the second meridian circle swallows the first,
        // provably contained, resolved empty.
        let swallow = translated(
            &crate::analytic::torus(3., 2. + 1e-9).unwrap(),
            [0., 0., 1.],
        );
        let report = intersect_torus_torus(&first, &swallow, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        // Just smaller: two transverse circles around the touch point.
        let across = translated(
            &crate::analytic::torus(3., 2. - 1e-9).unwrap(),
            [0., 0., 1.],
        );
        let report = intersect_torus_torus(&first, &across, Options::default()).unwrap();
        only_circles(&report, 2);
    }

    #[test]
    fn coincident_tori_report_coincident_trim() {
        // The identical torus twice: equal meridian circles within the band
        // (R_1 == R_2, h == 0, r_1 == r_2) — coincident surfaces, never a
        // guessed curve.
        let first = crate::analytic::torus(3., 1.).unwrap();
        let second = crate::analytic::torus(3., 1.).unwrap();
        let report = intersect_torus_torus(&first, &second, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(report.unresolved.len(), 1);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::CoincidentTrim
        );
        assert_eq!(
            report.unresolved[0].parameter_box,
            vec![0., 1., 0., 1., 0., 1., 0., 1.]
        );
        assert!(!report.permits_topology_change());
    }

    #[test]
    fn nested_and_separate_pairs_resolve_empty() {
        let first = crate::analytic::torus(3., 1.).unwrap();
        // Concentric within the band with a provably different minor radius:
        // certified containment through the d <= band branch.
        let concentric = crate::analytic::torus(3., 0.5).unwrap();
        // Nested without contact: R=3.2, r=2.5 contains the first tube
        // (d = 0.2, d + r_1 = 1.2 < r_2 = 2.5).
        let nested = crate::analytic::torus(3.2, 2.5).unwrap();
        // The first torus nested inside the second's tube without contact:
        // R=2.7, r=0.4 (d = 0.3, d + r_2 = 0.7 < r_1 = 1).
        let inner = crate::analytic::torus(2.7, 0.4).unwrap();
        // Far along the axis, no reach back.
        let far = translated(&crate::analytic::torus(3., 1.).unwrap(), [0., 0., 10.]);
        for second in [&concentric, &nested, &inner, &far] {
            let report = intersect_torus_torus(&first, second, Options::default()).unwrap();
            assert!(
                report.components.is_empty() && report.unresolved.is_empty(),
                "{report:?}"
            );
            assert_eq!(report.coverage, Coverage::NumericallyResolved);
        }
    }

    #[test]
    fn near_coaxial_bands_stay_unresolved_not_forced() {
        let first = crate::analytic::torus(3., 1.).unwrap();
        let base = translated(
            &crate::analytic::torus(3., 5.2_f64.sqrt()).unwrap(),
            [0., 0., 3.],
        );
        // Center-line offset inside the recognition band: near_coincidence.
        let near = translated(&base, [1e-10, 0., 0.]);
        let report = intersect_torus_torus(&first, &near, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::NearCoincidence
        );
        // Recognition-scale tilt of the second axis: near_coincidence.
        let (sin, cos) = 1e-10_f64.sin_cos();
        let tilted = crate::transform::affine(
            &base,
            [
                [cos, 0., sin, 0.],
                [0., 1., 0., 0.],
                [-sin, 0., cos, 3. - 3. * cos],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let report = intersect_torus_torus(&first, &tilted, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::NearCoincidence
        );
    }

    #[test]
    fn clearly_off_axis_or_tilted_pairs_are_unsupported_regions() {
        let first = crate::analytic::torus(3., 1.).unwrap();
        let base = translated(
            &crate::analytic::torus(3., 5.2_f64.sqrt()).unwrap(),
            [0., 0., 3.],
        );
        let off = translated(&base, [0.5, 0., 0.]);
        let (sin, cos) = 0.3_f64.sin_cos();
        let tilted = crate::transform::affine(
            &base,
            [
                [cos, 0., sin, -3. * sin],
                [0., 1., 0., 0.],
                [-sin, 0., cos, 3. - 3. * cos],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        for second in [&off, &tilted] {
            let report = intersect_torus_torus(&first, second, Options::default()).unwrap();
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
    }

    #[test]
    fn noncanonical_operands_are_explicit_unsupported_regions() {
        let torus = crate::analytic::torus(3., 1.).unwrap();
        let other = translated(
            &crate::analytic::torus(3., 5.2_f64.sqrt()).unwrap(),
            [0., 0., 3.],
        );
        // Spheres, cuboids, cylinders and frusta are not the canonical
        // operand on either side.
        for (a, b) in [
            (
                crate::analytic::sphere(2.).unwrap(),
                crate::analytic::torus(3., 1.).unwrap(),
            ),
            (
                crate::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap(),
                crate::analytic::torus(3., 1.).unwrap(),
            ),
            (
                crate::analytic::torus(3., 1.).unwrap(),
                crate::analytic::cylinder(2.2, 8.).unwrap(),
            ),
            (
                crate::analytic::torus(3., 1.).unwrap(),
                crate::analytic::frustum(1., 3., 6.).unwrap(),
            ),
            (
                crate::analytic::frustum(1., 3., 6.).unwrap(),
                crate::analytic::frustum(1., 3., 6.).unwrap(),
            ),
        ] {
            let report = intersect_torus_torus(&a, &b, Options::default()).unwrap();
            assert!(report.components.is_empty(), "{report:?}");
            assert_eq!(report.coverage, Coverage::Incomplete);
            assert_eq!(report.unresolved.len(), 1);
            assert_eq!(
                report.unresolved[0].reason,
                UnresolvedReason::UnsupportedSurface
            );
        }
        // A canonical pair still resolves.
        let report = intersect_torus_torus(&torus, &other, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
        assert_eq!(report.components.len(), 2);
        // A structurally perturbed torus (one moved control point) fails the
        // exact certification and is an unsupported region, never a fallback.
        let mut perturbed = other.clone();
        perturbed.faces[0].surface.control_points[1][1][0] += 1e-6;
        perturbed.rebuild_topology_ids();
        let report = intersect_torus_torus(&torus, &perturbed, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::UnsupportedSurface
        );
    }

    #[test]
    fn rotated_translated_placement_keeps_the_exact_circles() {
        // Rigid placement of the whole coaxial two-circle configuration
        // about X by 0.5.
        let angle = 0.5;
        let offset = [0.3, -0.2, 1.1];
        let first = rotated_translated(&crate::analytic::torus(3., 1.).unwrap(), angle, offset);
        let second = rotated_translated(
            &translated(
                &crate::analytic::torus(3., 5.2_f64.sqrt()).unwrap(),
                [0., 0., 3.],
            ),
            angle,
            offset,
        );
        let report = intersect_torus_torus(&first, &second, Options::default()).unwrap();
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
        let axis = sub(placed([0., 0., 1.]), placed([0., 0., 0.]));
        let placed_implicit = |p: [f64; 3], plane_z: f64| {
            // First torus (plane_z = 0): R=3, r^2=1; second (plane_z = 3): r^2=5.2.
            let minor2 = if plane_z == 0. { 1. } else { 5.2 };
            let rel = sub(p, placed([0., 0., plane_z]));
            let a = dot(rel, axis);
            let perp = sub(rel, axis.map(|x| x * a));
            let rho_p = perp[0].hypot(perp[1]).hypot(perp[2]);
            let base = rho_p * rho_p + a * a + 9. - minor2;
            (base * base - 36. * rho_p * rho_p).abs()
        };
        for expected_rho in [2.4, 3.6] {
            // Placement rounding may swap the equal-height circles at the
            // binary64 level; match by radius instead of position.
            let component = report
                .components
                .iter()
                .find(|c| {
                    let TorusTorusComponent::Circle { radius, .. } = c;
                    (radius - expected_rho).abs() <= 1e-12
                })
                .unwrap_or_else(|| panic!("no circle of radius {expected_rho}"));
            let (curve, center, radius, normal, first_uv, second_uv, sampled) =
                circle_of(component);
            let expected = placed([0., 0., 0.8]);
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
                worst = worst
                    .max(placed_implicit(p, 0.))
                    .max(placed_implicit(p, 3.));
            }
            assert!(worst <= 1e-9, "{worst}");
            assert!(sampled <= 1e-10, "{sampled}");
            assert_eq!(first_uv.len(), 4);
            assert_eq!(second_uv.len(), 4);
            let uv_worst = uv_samples(
                &first,
                &second,
                first_uv,
                second_uv,
                |p| placed_implicit(p, 0.),
                |p| placed_implicit(p, 3.),
            );
            assert!(uv_worst <= 1e-9, "{uv_worst}");
        }
    }
}
