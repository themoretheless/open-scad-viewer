//! Analytic cylinder/torus intersection for canonical solids in the coaxial
//! configuration (the cylinder axis certified coincident with the torus
//! axis).
//!
//! The cylinder operand must be the exact canonical six-face cylinder of
//! `analytic::cylinder` (four rational circular side patches, two bilinear
//! caps with inscribed-circle trims), recognized by
//! `sphere_cylinder::recognize_cylinder`; the torus operand must be the exact
//! canonical ring-torus solid of `analytic::torus` (sixteen rational
//! biquadratic patches tiling four profile quadrants times four revolution
//! quadrants with exact tensor-product weights, strict ring tori only —
//! the constructor refuses horn and spindle tori), recognized by
//! `plane_torus::recognize_torus`. Rigid affine placements are admitted.
//! Anything else is an explicit `UnsupportedSurface` region — this cell
//! never falls back to numerical surface/surface subdivision. Coaxiality is
//! certified, never forced: pure-rounding axis tilts and center-line offsets
//! snap, recognition-scale tilts or offsets report `NearCoincidence`, and a
//! clearly tilted or off-axis pair is `UnsupportedSurface` (the general
//! cylinder/torus pair is a quartic, out of scope). Classification uses
//! outward binary64 bands widened by both recognition deviations.
//!
//! Both surfaces are surfaces of revolution about the shared axis, so the
//! section reduces to the meridian half-plane (rho >= 0, z): the cylinder
//! side is the vertical line rho = R_c and the torus side is the circle
//! (rho - R)^2 + z^2 = r^2.
//! - The line provably missing the circle (|R_c - R| > r beyond the band)
//! yields no side contact; the tangent line (|R_c - R| == r within the
//! band, contact at z == 0 — the one configuration where the two square
//! roots would coincide) stays `TangencyOrMultipleRoot`, never a guessed
//! circle; otherwise z = +-sqrt(r^2 - (R_c - R)^2) are two distinct exact
//! circles of radius R_c. A resolved branch therefore always yields two
//! distinct circles — a single-circle meridian resolution is structurally
//! unreachable (it would require the tangent double root), and this is
//! documented rather than tested. The circles are clipped by the cylinder's
//! finite height: a circle provably beyond a cap plane is absent, and a
//! circle landing on a cap plane within the band is the rim tangency —
//! unresolved, never guessed.
//! - Cap-plane contacts: the cap plane at cylinder axial position q
//! crosses the torus at torus-axial height h_c; when |h_c| < r is provable
//! the plane cuts the two exact circles of radii R +- sqrt(r^2 - h_c^2)
//! (the plane/torus perpendicular branch, reduced here to the cap disk):
//! a circle provably inside the cap disk (radius < R_c) is a cap circle
//! component, a radius equal to R_c within the band is the rim tangency
//! (unresolved), and a radius provably beyond R_c is absent. |h_c| == r
//! within the band is the plane/tube tangency — unresolved. An inner
//! radius R - sqrt(r^2 - h_c^2) collapsing into the band would degenerate
//! to a pole point; it is unreachable for a canonical strict ring torus
//! (R - r >= 1e-5) and kept as an honest guard, never guessed through.
//! A cylinder side can never coincide with a curved torus patch, so
//! `CoincidentTrim` is deliberately unused by this cell.
//!
//! Resolved contacts are exact rational circles (four 90-degree arcs,
//! weights cos(pi/4)) with UV lifts on both surfaces: on the cylinder side
//! the iso-v line at the exact height fraction on all four side patches, on
//! a cap disk the exact UV circle of radius rho/(2R_c) about [1/2, 1/2], and
//! on the torus the iso-v parallels at the exact profile angle
//! phi = atan2(z, rho - R) mapped through the exact rational 90-degree-arc
//! parameter map t = s/(1+s), s = sqrt(2) tan(phi/2)/(1 - tan(phi/2)), one
//! degree-1 line per revolution quadrant patch of the profile row. Nothing
//! here authorizes a topology change.
use super::plane_torus::{
    CanonicalTorus, TorusPatchCurve, lift_parallel, recognize_torus, torus_residual,
};
use super::sphere_cylinder::{CanonicalCylinder, CylinderPatchCurve, recognize_cylinder};
use super::sphere_sphere::{RECOGNITION, circle_arcs, circle_curve};
#[cfg(test)]
use super::sphere_sphere::ARC_WEIGHT;
use super::*;
use crate::Model;

const TAU: f64 = std::f64::consts::TAU;

#[derive(Clone, Debug)]
pub enum CylinderTorusComponent {
    /// Transverse intersection: the exact circle plus both UV lifts.
    Circle {
        /// Exact rational full circle: degree 2, knots 0..=4, four 90-degree
        /// arcs with weights cos(pi/4).
        curve: Curve,
        center: [f64; 3],
        radius: f64,
        /// Unit shared axis (the circle plane normal).
        normal: [f64; 3],
        cylinder_uv: Vec<CylinderPatchCurve>,
        torus_uv: Vec<TorusPatchCurve>,
        /// Worst residual over 16 circle samples against the cylinder side
        /// or cap plane equation and the scaled torus implicit equation.
        max_sample_residual: f64,
    },
}

/// Where a resolved circle sits on the cylinder boundary.
#[derive(Clone, Copy)]
enum CircleSite {
    /// On the side wall: radius is the cylinder radius.
    Side,
    /// On a cap disk: cylinder axial position and circle radius.
    Cap(f64, f64),
}

/// Exact circle component at torus-axial height `z` above the torus center
/// plane, with UV lifts on both surfaces and a 16-sample residual bound.
fn circle_component(
    cylinder: &CanonicalCylinder,
    torus: &CanonicalTorus,
    z: f64,
    site: CircleSite,
) -> Result<CylinderTorusComponent> {
    let center = std::array::from_fn(|k| torus.center[k] + z * torus.axis[k]);
    let (radius, cylinder_uv) = match site {
        CircleSite::Side => {
            // Side patches: u sweeps the quadrant, v runs bottom to top, so
            // the circle is the iso-v line v = (axial + h/2) / h on each.
            let axial = dot(sub(center, cylinder.center), cylinder.axis);
            let v0 = (axial + cylinder.half_height) / (2. * cylinder.half_height);
            let arcs = vec![Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                control_points: vec![[0., v0].to_vec(), [1., v0].to_vec()],
                weights: vec![1., 1.],
                periodic: false,
            }];
            (
                cylinder.radius,
                cylinder
                    .sides
                    .iter()
                    .map(|&patch| CylinderPatchCurve {
                        patch,
                        arcs: arcs.clone(),
                    })
                    .collect::<Vec<_>>(),
            )
        }
        CircleSite::Cap(axial, rho) => {
            let face = if axial > 0. {
                cylinder.caps[1]
            } else {
                cylinder.caps[0]
            };
            // Cap UV maps the plane as [1/2 + x/(2R), 1/2 + y/(2R)]: the
            // circle is an exact UV circle of radius rho/(2R) about [1/2,1/2].
            (
                rho,
                vec![CylinderPatchCurve {
                    patch: face,
                    arcs: circle_arcs([0.5, 0.5], rho / (2. * cylinder.radius), 0., TAU),
                }],
            )
        }
    };
    let curve = circle_curve(center, radius, cylinder.frame[0], cylinder.frame[1]);
    // Torus lift: the iso-v parallel at the exact profile angle.
    let profile = z.atan2(radius - torus.major).rem_euclid(TAU);
    let torus_uv = lift_parallel(torus, profile, &[(0., TAU)]);
    let mut max_sample_residual = 0_f64;
    for i in 0..16 {
        let jet = curve.evaluate(i as f64 / 4.)?;
        let point = [jet.point[0], jet.point[1], jet.point[2]];
        let rel = sub(point, cylinder.center);
        let axial = dot(rel, cylinder.axis);
        let cylinder_residual = match site {
            CircleSite::Side => {
                let perp = sub(rel, cylinder.axis.map(|x| x * axial));
                (perp[0].hypot(perp[1]).hypot(perp[2]) - cylinder.radius).abs()
            }
            CircleSite::Cap(q, _) => (axial - q).abs(),
        };
        max_sample_residual = max_sample_residual
            .max(cylinder_residual)
            .max(torus_residual(torus, point));
    }
    Ok(CylinderTorusComponent::Circle {
        curve,
        center,
        radius,
        normal: cylinder.axis,
        cylinder_uv,
        torus_uv,
        max_sample_residual,
    })
}

/// Analytic cylinder/torus intersection of a canonical cylinder solid and a
/// canonical ring-torus solid, coaxial configuration only. Non-canonical
/// operands and clearly tilted or off-axis pairs are explicit unsupported
/// regions, never a numerical fallback; recognition-scale near-coaxial
/// bands, the meridian line/circle tangency, rim contacts and cap-plane
/// touches stay unresolved — tangent contacts are never guessed. Provable
/// misses (the line missing the meridian circle, circles beyond the finite
/// height, cap circles outside the cap disks) resolve empty.
pub fn intersect_cylinder_torus(
    cylinder_model: &Model,
    torus_model: &Model,
    options: Options,
) -> Result<Report<CylinderTorusComponent>> {
    let options = options.validate()?;
    let _ = options;
    let mut report = Report::default();
    let domain = vec![0., 1., 0., 1., 0., 1., 0., 1.];
    let (Some(cylinder), Some(torus)) = (
        recognize_cylinder(cylinder_model)?,
        recognize_torus(torus_model)?,
    ) else {
        report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    };
    report.boxes_visited = 1;
    let delta = sub(torus.center, cylinder.center);
    let terms = cylinder.half_height
        + cylinder.radius
        + torus.major
        + torus.minor
        + delta.iter().map(|v| v.abs()).fold(0., f64::max)
        + 1.;
    // Outward binary64 classification band: both recognition deviations plus
    // a rounding allowance on the axial coordinates and radii.
    let band = cylinder.error + torus.error + 16. * f64::EPSILON * terms;
    // Axis parallelism is certified, never forced: pure-rounding direction
    // disagreement snaps to parallel, recognition-scale tilt reports
    // near_coincidence, and a clearly tilted pair is the unsupported general
    // (quartic) case.
    let direction = cross(cylinder.axis, torus.axis);
    let tilt = direction[0].hypot(direction[1]).hypot(direction[2]);
    let axis_snap = 64. * f64::EPSILON
        + cylinder.error / (2. * cylinder.half_height)
        + torus.error / (2. * torus.major);
    if tilt > axis_snap {
        let reason = if tilt <= RECOGNITION {
            UnresolvedReason::NearCoincidence
        } else {
            UnresolvedReason::UnsupportedSurface
        };
        report.unresolved(domain, reason);
        return Ok(report);
    }
    let axis = cylinder.axis;
    let along = dot(delta, axis);
    let perp = sub(delta, axis.map(|x| x * along));
    let pscale = perp.iter().map(|v| v.abs()).fold(0., f64::max);
    let d_perp = if pscale == 0. {
        0.
    } else {
        let scaled = perp.map(|x| x / pscale);
        scaled[0].hypot(scaled[1]).hypot(scaled[2]) * pscale
    };
    // Coaxiality: a center-line distance at pure rounding scale snaps to
    // zero; a recognition-scale offset is near_coincidence, never forced;
    // a clear offset is the unsupported general (quartic) configuration.
    let dist_snap = 64. * f64::EPSILON * terms + cylinder.error + torus.error;
    if d_perp > dist_snap {
        let reason = if d_perp <= RECOGNITION * terms {
            UnresolvedReason::NearCoincidence
        } else {
            UnresolvedReason::UnsupportedSurface
        };
        report.unresolved(domain, reason);
        return Ok(report);
    }
    let (r, big_r, r_c, half) = (
        torus.minor,
        torus.major,
        cylinder.radius,
        cylinder.half_height,
    );
    let mut tangency = false;
    let mut found: Vec<(f64, CircleSite)> = Vec::new();
    // Meridian line rho = R_c against the circle (rho - R)^2 + z^2 = r^2.
    let a = r_c - big_r;
    let aa = a.abs();
    if aa > r + band {
        // Provable miss: the line clears the meridian circle.
    } else if (aa - r).abs() <= band {
        // Tangent line: the double root at z == 0 revolves into a circle but
        // stays unresolved — never a guessed circle.
        tangency = true;
    } else {
        // |a| < r provably: two distinct roots z = +-sqrt(r^2 - a^2). The
        // roots coincide only at the tangent, so a resolved branch always
        // yields two distinct circles (no single-circle configuration).
        let z2 = r * r - a * a;
        if !z2.is_finite() || z2 <= 0. {
            // Defensive: the band comparisons above should have caught every
            // tangency; a non-positive discriminant here is rounding spill
            // and stays unresolved rather than guessed.
            tangency = true;
        } else {
            let z0 = z2.sqrt();
            for sign in [1., -1.] {
                let z = sign * z0;
                let center = std::array::from_fn(|k| torus.center[k] + z * torus.axis[k]);
                let axial = dot(sub(center, cylinder.center), axis);
                if axial.abs() > half + band {
                    // Provably beyond a cap plane: honestly absent.
                    continue;
                }
                if (axial.abs() - half).abs() <= band {
                    // Rim tangency: the circle lands on the cap plane.
                    tangency = true;
                    continue;
                }
                found.push((z, CircleSite::Side));
            }
        }
    }
    // Cap-plane contacts: the torus tube pokes through the cap plane at
    // torus-axial height h_c, cutting the circles of radii R +- sqrt(r^2 -
    // h_c^2) — the plane/torus perpendicular branch, reduced to the disk.
    for &q in &[half, -half] {
        let cap = std::array::from_fn(|k| cylinder.center[k] + q * axis[k]);
        let h_c = dot(sub(cap, torus.center), torus.axis);
        let ah = h_c.abs();
        if ah > r + band {
            // Provable miss: the whole band clears the minor radius.
            continue;
        }
        if ah >= (r - band).max(0.) {
            // Tangent plane, or a band straddling it: never a guessed circle.
            tangency = true;
            continue;
        }
        let s2 = r * r - h_c * h_c;
        if !s2.is_finite() || s2 <= 0. {
            tangency = true;
            continue;
        }
        let s = s2.sqrt();
        if big_r - s <= band {
            // Inner-radius collapse (a pole degeneracy): unreachable for a
            // canonical strict ring torus (R - r >= 1e-5), kept unresolved,
            // never guessed.
            tangency = true;
            continue;
        }
        for side in [1., -1.] {
            let rho = big_r + side * s;
            if rho > r_c + band {
                // Provably outside the cap disk: honestly absent.
                continue;
            }
            if (rho - r_c).abs() <= band {
                // Rim tangency: the cap circle meets the side wall.
                tangency = true;
                continue;
            }
            found.push((h_c, CircleSite::Cap(q, rho)));
        }
    }
    if tangency {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
    }
    // Deterministic order: by torus-axial height, then by radius.
    let radius_of = |site: &CircleSite| match site {
        CircleSite::Side => r_c,
        CircleSite::Cap(_, rho) => *rho,
    };
    found.sort_by(|x, y| {
        x.0.total_cmp(&y.0)
            .then(radius_of(&x.1).total_cmp(&radius_of(&y.1)))
    });
    for (z, site) in found {
        report
            .components
            .push(circle_component(&cylinder, &torus, z, site)?);
    }
    Ok(report)
}

impl value_codec::Serialize for CylinderTorusComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Circle {
                curve,
                center,
                radius,
                normal,
                cylinder_uv,
                torus_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"circle","curve":curve,"center":center,"radius":radius,
                "normal":normal,"cylinderUv":cylinder_uv,"torusUv":torus_uv,
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
        report: &Report<CylinderTorusComponent>,
        count: usize,
    ) -> &Report<CylinderTorusComponent> {
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        assert!(report.unresolved.is_empty(), "{report:?}");
        assert!(!report.permits_topology_change());
        assert_eq!(report.components.len(), count, "{report:?}");
        report
    }
    #[allow(clippy::type_complexity)]
    fn circle_of(
        component: &CylinderTorusComponent,
    ) -> (
        &Curve,
        [f64; 3],
        f64,
        [f64; 3],
        &[CylinderPatchCurve],
        &[TorusPatchCurve],
        f64,
    ) {
        let CylinderTorusComponent::Circle {
            curve,
            center,
            radius,
            normal,
            cylinder_uv,
            torus_uv,
            max_sample_residual,
        } = component;
        (
            curve,
            *center,
            *radius,
            *normal,
            cylinder_uv,
            torus_uv,
            *max_sample_residual,
        )
    }
    /// Radial distance from the z axis (canonical frames in these tests).
    fn radial_z(point: [f64; 3]) -> f64 {
        point[0].hypot(point[1])
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
    /// Worst residual of lifted UV arcs evaluated through their own surface,
    /// against a caller-supplied cylinder predicate and the torus equation.
    fn uv_samples(
        cylinder_model: &Model,
        torus_model: &Model,
        cylinder_uv: &[CylinderPatchCurve],
        torus_uv: &[TorusPatchCurve],
        cylinder_check: impl Fn([f64; 3]) -> f64,
        torus_check: impl Fn([f64; 3]) -> f64,
    ) -> f64 {
        let mut worst = 0_f64;
        for lift in cylinder_uv {
            let surface = &cylinder_model.faces[lift.patch].surface;
            for arc in &lift.arcs {
                for k in 0..=8 {
                    let uv = arc.evaluate(k as f64 / 8.).unwrap().point;
                    assert!((0. ..=1.).contains(&uv[0]) && (0. ..=1.).contains(&uv[1]));
                    let p = surface.evaluate(uv[0], uv[1]).unwrap().point;
                    let p = [p[0], p[1], p[2]];
                    worst = worst.max(cylinder_check(p)).max(torus_check(p));
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
                    worst = worst.max(cylinder_check(p)).max(torus_check(p));
                }
            }
        }
        worst
    }

    #[test]
    fn two_side_circles_match_the_sqrt_oracle() {
        // Torus R=3, r=1; cylinder R_c=2.2 spanning z in -4..4: a = -0.8,
        // side circles of radius 2.2 at z = +-sqrt(1 - 0.64) = +-0.6.
        let torus = crate::analytic::torus(3., 1.).unwrap();
        let cylinder = translated(&crate::analytic::cylinder(2.2, 8.).unwrap(), [0., 0., -4.]);
        let report = intersect_cylinder_torus(&cylinder, &torus, Options::default()).unwrap();
        let report = only_circles(&report, 2);
        for (component, z) in report.components.iter().zip([-0.6, 0.6]) {
            let (curve, center, radius, normal, cylinder_uv, torus_uv, sampled) =
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
                    .max((radial_z(p) - 2.2).abs())
                    .max(implicit(p, 3., 1.));
            }
            assert!(worst <= 1e-10, "{worst}");
            assert!(sampled <= 1e-10, "{sampled}");
            // Cylinder lift: the iso-v line at the exact height fraction on
            // all four side patches.
            assert_eq!(cylinder_uv.len(), 4, "{cylinder_uv:?}");
            let v0 = (z + 4.) / 8.;
            for lift in cylinder_uv {
                assert_eq!(lift.arcs.len(), 1);
                let arc = &lift.arcs[0];
                assert_eq!(arc.degree, 1);
                assert!((arc.control_points[0][1] - v0).abs() <= 1e-12, "{arc:?}");
                assert!((arc.control_points[1][1] - v0).abs() <= 1e-12, "{arc:?}");
                assert_eq!(arc.control_points[0][0], 0.);
                assert_eq!(arc.control_points[1][0], 1.);
            }
            // Torus lift: one iso-v line per revolution quadrant patch of the
            // profile row, at the exact rational-arc v parameter.
            assert_eq!(torus_uv.len(), 4, "{torus_uv:?}");
            let phi = z.atan2(2.2 - 3.).rem_euclid(TAU);
            let quadrant = (phi / std::f64::consts::FRAC_PI_2).floor() as usize;
            let vv = arc_parameter(phi - quadrant as f64 * std::f64::consts::FRAC_PI_2);
            for lift in torus_uv {
                assert_eq!(lift.arcs.len(), 1);
                let arc = &lift.arcs[0];
                assert_eq!(arc.degree, 1);
                assert!((arc.control_points[0][1] - vv).abs() <= 1e-12, "{arc:?}");
                assert!((arc.control_points[1][1] - vv).abs() <= 1e-12, "{arc:?}");
            }
            let uv_worst = uv_samples(
                &cylinder,
                &torus,
                cylinder_uv,
                torus_uv,
                |p| (radial_z(p) - 2.2).abs(),
                |p| implicit(p, 3., 1.),
            );
            assert!(uv_worst <= 1e-9, "{uv_worst}");
        }
    }

    #[test]
    fn height_clipping_drops_the_lower_side_circle() {
        // Torus R=3, r=1 at the origin; cylinder R_c=2.2 spanning z in
        // 0.3..8.3: the z=-0.6 side circle is provably beyond the bottom
        // cap, the z=+0.6 circle survives, and the bottom cap plane at
        // h_c=0.3 cuts the torus at radii 3 +- sqrt(1 - 0.09) — the inner
        // circle (2.0460608...) sits inside the cap disk, the outer one is
        // beyond it.
        let torus = crate::analytic::torus(3., 1.).unwrap();
        let cylinder = translated(&crate::analytic::cylinder(2.2, 8.).unwrap(), [0., 0., 0.3]);
        let report = intersect_cylinder_torus(&cylinder, &torus, Options::default()).unwrap();
        let report = only_circles(&report, 2);
        // Sorted by torus-axial height: the cap circle at z=0.3, then the
        // side circle at z=0.6. No component at z=-0.6.
        let s = (1_f64 - 0.09).sqrt();
        let (curve, center, radius, _, cylinder_uv, torus_uv, sampled) =
            circle_of(&report.components[0]);
        assert!((radius - (3. - s)).abs() <= 1e-12, "{radius}");
        assert!(
            sub(center, [0., 0., 0.3]).iter().all(|x| x.abs() <= 1e-12),
            "{center:?}"
        );
        let mut worst = 0_f64;
        for i in 0..16 {
            let p = curve.evaluate(i as f64 / 4.).unwrap().point;
            let p = [p[0], p[1], p[2]];
            worst = worst.max((p[2] - 0.3).abs()).max(implicit(p, 3., 1.));
        }
        assert!(worst <= 1e-10, "{worst}");
        assert!(sampled <= 1e-10, "{sampled}");
        // Cap lift: one face, four exact 90-degree arcs of the UV circle of
        // radius rho/(2R_c) about [1/2, 1/2].
        assert_eq!(cylinder_uv.len(), 1);
        let lift = &cylinder_uv[0];
        assert_eq!(lift.arcs.len(), 4);
        for arc in &lift.arcs {
            assert_eq!(arc.degree, 2);
            assert_eq!(arc.weights, vec![1., ARC_WEIGHT, 1.]);
            let uvr = (arc.control_points[0][0] - 0.5).hypot(arc.control_points[0][1] - 0.5);
            assert!((uvr - (3. - s) / 4.4).abs() <= 1e-12, "{uvr}");
        }
        assert_eq!(torus_uv.len(), 4);
        let uv_worst = uv_samples(
            &cylinder,
            &torus,
            cylinder_uv,
            torus_uv,
            |p| (p[2] - 0.3).abs(),
            |p| implicit(p, 3., 1.),
        );
        assert!(uv_worst <= 1e-9, "{uv_worst}");
        let (_, center, radius, _, cylinder_uv, _, sampled) = circle_of(&report.components[1]);
        assert!((radius - 2.2).abs() <= 1e-12, "{radius}");
        assert!(
            sub(center, [0., 0., 0.6]).iter().all(|x| x.abs() <= 1e-12),
            "{center:?}"
        );
        assert_eq!(cylinder_uv.len(), 4);
        assert!(sampled <= 1e-10, "{sampled}");
    }

    #[test]
    fn torus_poking_through_both_caps_yields_cap_circle_pairs() {
        // Torus R=3, r=1; short wide cylinder R_c=4.5 spanning -0.25..0.25:
        // |a| = 1.5 > 1 — no side contact; each cap plane at h_c = +-0.25
        // cuts the torus at 3 +- sqrt(1 - 0.0625), both inside the disk:
        // four cap circles.
        let torus = crate::analytic::torus(3., 1.).unwrap();
        let cylinder = translated(
            &crate::analytic::cylinder(4.5, 0.5).unwrap(),
            [0., 0., -0.25],
        );
        let report = intersect_cylinder_torus(&cylinder, &torus, Options::default()).unwrap();
        let report = only_circles(&report, 4);
        let s = (1_f64 - 0.0625).sqrt();
        let expected = [
            (-0.25, 3. - s),
            (-0.25, 3. + s),
            (0.25, 3. - s),
            (0.25, 3. + s),
        ];
        for (component, &(z, rho)) in report.components.iter().zip(&expected) {
            let (curve, center, radius, _, cylinder_uv, torus_uv, sampled) = circle_of(component);
            assert!((radius - rho).abs() <= 1e-12, "{radius} vs {rho}");
            assert!(
                sub(center, [0., 0., z]).iter().all(|x| x.abs() <= 1e-12),
                "{center:?}"
            );
            let mut worst = 0_f64;
            for i in 0..16 {
                let p = curve.evaluate(i as f64 / 4.).unwrap().point;
                let p = [p[0], p[1], p[2]];
                worst = worst.max((p[2] - z).abs()).max(implicit(p, 3., 1.));
            }
            assert!(worst <= 1e-10, "{worst}");
            assert!(sampled <= 1e-10, "{sampled}");
            assert_eq!(cylinder_uv.len(), 1);
            assert_eq!(cylinder_uv[0].arcs.len(), 4);
            // Torus lift at the exact profile angle atan2(z, rho - R).
            assert_eq!(torus_uv.len(), 4);
            let phi = z.atan2(rho - 3.).rem_euclid(TAU);
            let quadrant = (phi / std::f64::consts::FRAC_PI_2).floor() as usize;
            let vv = arc_parameter(phi - quadrant as f64 * std::f64::consts::FRAC_PI_2);
            for lift in torus_uv {
                let arc = &lift.arcs[0];
                assert!((arc.control_points[0][1] - vv).abs() <= 1e-12, "{arc:?}");
            }
            let uv_worst = uv_samples(
                &cylinder,
                &torus,
                cylinder_uv,
                torus_uv,
                |p| (p[2] - z).abs(),
                |p| implicit(p, 3., 1.),
            );
            assert!(uv_worst <= 1e-9, "{uv_worst}");
        }
    }

    #[test]
    fn meridian_tangency_stays_a_tangency_region() {
        // R_c == R - r = 2 (inner equator) and R_c == R + r = 4 (outer
        // equator): the line rho = R_c is tangent to the meridian circle at
        // z == 0 — the one configuration where the roots would coincide.
        // Never a guessed circle.
        let torus = crate::analytic::torus(3., 1.).unwrap();
        for r_c in [2., 2. + 2e-15, 2. - 2e-15, 4., 4. + 2e-15, 4. - 2e-15] {
            let cylinder = translated(&crate::analytic::cylinder(r_c, 8.).unwrap(), [0., 0., -4.]);
            let report = intersect_cylinder_torus(&cylinder, &torus, Options::default()).unwrap();
            assert!(report.components.is_empty(), "{report:?}");
            assert_eq!(report.coverage, Coverage::Incomplete);
            assert_eq!(
                report.unresolved[0].reason,
                UnresolvedReason::TangencyOrMultipleRoot
            );
            assert!(!report.permits_topology_change());
        }
        // Just clear of the band on the outside: provable miss, resolved.
        let clear = translated(
            &crate::analytic::cylinder(2. - 1e-9, 8.).unwrap(),
            [0., 0., -4.],
        );
        let report = intersect_cylinder_torus(&clear, &torus, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        // Just across: two small transverse circles around the inner equator.
        let across = translated(
            &crate::analytic::cylinder(2. + 1e-9, 8.).unwrap(),
            [0., 0., -4.],
        );
        let report = intersect_cylinder_torus(&across, &torus, Options::default()).unwrap();
        only_circles(&report, 2);
    }

    #[test]
    fn zero_circle_configurations_resolve_empty() {
        let torus = crate::analytic::torus(3., 1.).unwrap();
        // Thin cylinder in the hole: the line rho = 1 misses the meridian
        // circle, caps far above the tube.
        let thin = translated(&crate::analytic::cylinder(1., 8.).unwrap(), [0., 0., -4.]);
        // Short narrow cylinder around the center plane: no side contact,
        // and both cap circles (radii 3 +- sqrt(0.75)) lie outside the
        // R_c = 1.5 disks.
        let narrow = translated(&crate::analytic::cylinder(1.5, 1.).unwrap(), [0., 0., -0.5]);
        // Huge cylinder swallowing the whole torus.
        let huge = translated(
            &crate::analytic::cylinder(20., 40.).unwrap(),
            [0., 0., -20.],
        );
        for cylinder in [&thin, &narrow, &huge] {
            let report = intersect_cylinder_torus(cylinder, &torus, Options::default()).unwrap();
            assert!(
                report.components.is_empty() && report.unresolved.is_empty(),
                "{report:?}"
            );
            assert_eq!(report.coverage, Coverage::NumericallyResolved);
        }
    }

    #[test]
    fn rim_tangency_stays_unresolved() {
        // R_c = 2.5, half height sqrt(0.75): the side circles at
        // z = +-sqrt(1 - 0.25) land exactly on the cap planes, and the cap
        // circle of radius R + |a| = 2.5 equals R_c — rim tangencies, never
        // guessed circles. The outer cap circles (radius 3.5) are absent.
        let torus = crate::analytic::torus(3., 1.).unwrap();
        let half = 0.75_f64.sqrt();
        let cylinder = translated(
            &crate::analytic::cylinder(2.5, 2. * half).unwrap(),
            [0., 0., -half],
        );
        let report = intersect_cylinder_torus(&cylinder, &torus, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
    }

    #[test]
    fn cap_plane_tube_tangency_stays_unresolved() {
        // R_c = 5 (no side contact: |a| = 2 > 1), half height 1: both cap
        // planes sit at |h_c| == r — tangent to the tube, never guessed.
        let torus = crate::analytic::torus(3., 1.).unwrap();
        let cylinder = translated(&crate::analytic::cylinder(5., 2.).unwrap(), [0., 0., -1.]);
        let report = intersect_cylinder_torus(&cylinder, &torus, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
        // Just clear of the band on the outside: both caps beyond the tube,
        // resolved empty.
        let clear = translated(
            &crate::analytic::cylinder(5., 2.).unwrap(),
            [0., 0., 2. + 1e-9],
        );
        let report = intersect_cylinder_torus(&clear, &torus, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        // Just across: the top cap clears the tube (h_c = 1 + 1e-9) and the
        // bottom cap at h_c = -(1 - 1e-9) cuts two small cap circles.
        let across = translated(&crate::analytic::cylinder(5., 2.).unwrap(), [0., 0., 1e-9]);
        let report = intersect_cylinder_torus(&across, &torus, Options::default()).unwrap();
        only_circles(&report, 2);
    }

    #[test]
    fn near_coaxial_bands_stay_unresolved_not_forced() {
        let torus = crate::analytic::torus(3., 1.).unwrap();
        let base = translated(&crate::analytic::cylinder(2.2, 8.).unwrap(), [0., 0., -4.]);
        // Center-line offset inside the recognition band: near_coincidence.
        let near = translated(&base, [1e-10, 0., 0.]);
        let report = intersect_cylinder_torus(&near, &torus, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::NearCoincidence
        );
        // Recognition-scale tilt of the cylinder axis: near_coincidence.
        let (sin, cos) = 1e-10_f64.sin_cos();
        let tilted = crate::transform::affine(
            &base,
            [
                [cos, 0., sin, 0.],
                [0., 1., 0., 0.],
                [-sin, 0., cos, 0.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let report = intersect_cylinder_torus(&tilted, &torus, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::NearCoincidence
        );
    }

    #[test]
    fn clearly_off_axis_or_tilted_pairs_are_unsupported_regions() {
        let torus = crate::analytic::torus(3., 1.).unwrap();
        let base = translated(&crate::analytic::cylinder(2.2, 8.).unwrap(), [0., 0., -4.]);
        let off = translated(&base, [0.5, 0., 0.]);
        let (sin, cos) = 0.3_f64.sin_cos();
        let tilted = crate::transform::affine(
            &base,
            [
                [cos, 0., sin, 0.],
                [0., 1., 0., 0.],
                [-sin, 0., cos, 0.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        for cylinder in [&off, &tilted] {
            let report = intersect_cylinder_torus(cylinder, &torus, Options::default()).unwrap();
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
        let cylinder = translated(&crate::analytic::cylinder(2.2, 8.).unwrap(), [0., 0., -4.]);
        // Spheres, cuboids, tori-as-first-operand and swapped operand order
        // are not the canonical pair.
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
                crate::analytic::torus(2., 1.).unwrap(),
                crate::analytic::torus(3., 1.).unwrap(),
            ),
            (
                crate::analytic::torus(3., 1.).unwrap(),
                crate::analytic::cylinder(2.2, 8.).unwrap(),
            ),
            (
                crate::analytic::cylinder(2.2, 8.).unwrap(),
                crate::analytic::sphere(2.).unwrap(),
            ),
        ] {
            let report = intersect_cylinder_torus(&a, &b, Options::default()).unwrap();
            assert!(report.components.is_empty(), "{report:?}");
            assert_eq!(report.coverage, Coverage::Incomplete);
            assert_eq!(report.unresolved.len(), 1);
            assert_eq!(
                report.unresolved[0].reason,
                UnresolvedReason::UnsupportedSurface
            );
        }
        // A canonical pair still resolves.
        let report = intersect_cylinder_torus(&cylinder, &torus, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
        assert_eq!(report.components.len(), 2);
        // A structurally perturbed torus (one moved control point) fails the
        // exact certification and is an unsupported region, never a fallback.
        let mut perturbed = torus.clone();
        perturbed.faces[0].surface.control_points[1][1][0] += 1e-6;
        perturbed.rebuild_topology_ids();
        let report = intersect_cylinder_torus(&cylinder, &perturbed, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::UnsupportedSurface
        );
    }

    #[test]
    fn rotated_translated_placement_keeps_the_exact_circles() {
        // Rigid placement of the whole coaxial configuration about X by 0.5.
        let angle = 0.5;
        let offset = [0.3, -0.2, 1.1];
        let torus = rotated_translated(&crate::analytic::torus(3., 1.).unwrap(), angle, offset);
        let cylinder = rotated_translated(
            &translated(&crate::analytic::cylinder(2.2, 8.).unwrap(), [0., 0., -4.]),
            angle,
            offset,
        );
        let report = intersect_cylinder_torus(&cylinder, &torus, Options::default()).unwrap();
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
        for (component, z) in report.components.iter().zip([-0.6, 0.6]) {
            let (curve, center, radius, normal, cylinder_uv, torus_uv, sampled) =
                circle_of(component);
            let expected = placed([0., 0., z]);
            assert!((radius - 2.2).abs() <= 1e-12, "{radius}");
            assert!(
                sub(center, expected).iter().all(|x| x.abs() <= 1e-12),
                "{center:?}"
            );
            assert!(
                sub(normal, axis).iter().all(|x| x.abs() <= 1e-12),
                "{normal:?}"
            );
            let placed_check = |p: [f64; 3]| {
                let rel = sub(p, placed([0., 0., 0.]));
                let a = dot(rel, axis);
                let perp = sub(rel, axis.map(|x| x * a));
                let rho_p = perp[0].hypot(perp[1]).hypot(perp[2]);
                let base = rho_p * rho_p + a * a + 9. - 1.;
                (
                    (rho_p - 2.2).abs(),
                    (base * base - 36. * rho_p * rho_p).abs(),
                )
            };
            let mut worst = 0_f64;
            for i in 0..16 {
                let p = curve.evaluate(i as f64 / 4.).unwrap().point;
                let p = [p[0], p[1], p[2]];
                let (c_r, t_r) = placed_check(p);
                worst = worst.max(c_r).max(t_r);
            }
            assert!(worst <= 1e-9, "{worst}");
            assert!(sampled <= 1e-10, "{sampled}");
            assert_eq!(cylinder_uv.len(), 4);
            assert_eq!(torus_uv.len(), 4);
            let uv_worst = uv_samples(
                &cylinder,
                &torus,
                cylinder_uv,
                torus_uv,
                |p| placed_check(p).0,
                |p| placed_check(p).1,
            );
            assert!(uv_worst <= 1e-9, "{uv_worst}");
        }
    }
}
