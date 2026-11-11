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
//! `NearCoincidence` region, and a clearly off-axis contact is traced and
//! certified as `Traced` pieces (the general sphere/torus pair is a quartic, out of
//! scope) unless it provably misses: an off-axis sphere entirely outside the
//! tube, entirely inside it, or enclosing the whole torus resolves empty. Classification uses outward binary64 bands widened by both
//! recognition deviations.
//!
//! Both surfaces are surfaces of revolution about the shared axis, so the
//! section is a union of exact circles. In the meridian half-plane
//! (rho >= 0, z) the torus side is the circle (rho - R)^2 + z^2 = r_t^2 and
//! the sphere is the circle rho^2 + (z - h)^2 = r_s^2 with h the sphere
//! center height above the torus center plane. Circle/circle in the
//! meridian plane gives zero, one (double) or two roots; each root (rho*,
//! z*) revolves into the exact 3D circle of radius rho* at height z*:
//! two distinct roots give two exact circles (for a strict ring torus
//!   every root sits on the torus meridian circle, whose rho >= R - r_t
//!   is clearly positive, so both roots always revolve — a one-circle
//!   configuration would require rho* <= 0, i.e. a horn/spindle torus,
//!   which the canonical constructor refuses);
//! a double root (the circles tangent in the meridian plane, d == r_t+r_s
//!   externally or d == |r_t - r_s| internally) stays
//!   `TangencyOrMultipleRoot` — the tangent contact revolves into a circle
//!   but is never a guessed circle;
//! a root whose radius rho* collapses into the band degenerates to a
//!   pole point on the axis — unresolved, never guessed (unreachable for a
//!   canonical strict ring torus since rho* >= R - r_t >= 1e-5, kept as an
//!   honest guard, never guessed through);
//! no roots (provably separate d > r_t + r_s, or provably contained
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
#[cfg(feature = "codec")]
mod serialization;
use super::plane_torus::{
    CanonicalTorus, TorusPatchCurve, lift_parallel, recognize_torus, torus_residual,
};
use super::*;
use brep_core::Model;
#[cfg(test)]
use brep_core::intersections::sphere_sphere::ARC_WEIGHT;
use brep_core::intersections::sphere_sphere::{
    self, CanonicalSphere, RECOGNITION, SpherePatchCircle, circle_curve, lift,
};

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
    /// Off-axis transverse intersection: a space quartic with no exact
    /// rational form, traced and represented by cubic pieces that close into
    /// one loop with exactly shared ends. Every point of every piece is
    /// certified within `distance_bound` of both surfaces (Bernstein bounds
    /// of the implicit equations in outward interval arithmetic). No UV
    /// lifts yet, and completeness of the component set is not certified.
    Traced {
        pieces: Vec<Curve>,
        /// Certified upper bound on the distance from either surface.
        distance_bound: f64,
        /// Number of certified Bezier spans.
        spans: usize,
    },
}

/// Certified distance of a traced curve from either surface.
pub const TRACE_TOLERANCE: f64 = 1e-7;

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
        sphere_sphere::recognize_any(sphere_model)?,
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
        // Off the axis the section is a quartic, but a miss is still
        // provable. The torus is the level set dist(p, core circle) = r_t
        // and that distance is 1-Lipschitz, so every torus point lies at a
        // distance from the sphere center between |dmin - r_t| and
        // dmax + r_t, with dmin and dmax the nearest and farthest distances
        // from the center to the core circle.
        let dmin = (d_perp - torus.major).hypot(h);
        let dmax = (d_perp + torus.major).hypot(h);
        let reach = band + 16. * f64::EPSILON * (d_perp + terms);
        let nearest = (dmin - torus.minor).abs();
        if sphere.radius + reach < nearest || sphere.radius > dmax + torus.minor + reach {
            return Ok(report);
        }
        let near = RECOGNITION * (sphere.radius + torus.major + torus.minor);
        if d_perp <= near {
            report.unresolved(domain, UnresolvedReason::NearCoincidence);
            return Ok(report);
        }
        // A genuine off-axis contact: trace the quartic and certify it.
        let frame = [torus.frame[0], torus.frame[1], torus.axis];
        let s_implicit = crate::implicit_trace::SphereImplicit {
            center: sphere.center,
            radius: sphere.radius,
        };
        let t_implicit = crate::implicit_trace::TorusImplicit {
            center: torus.center,
            frame,
            major: torus.major,
            minor: torus.minor,
        };
        let seed = |theta: f64, phi: f64| -> [f64; 3] {
            let (st, ct) = theta.sin_cos();
            let (sp, cp) = phi.sin_cos();
            std::array::from_fn(|k| {
                sphere.center[k]
                    + sphere.radius
                        * (st * cp * frame[0][k] + st * sp * frame[1][k] + ct * frame[2][k])
            })
        };
        let scale = terms + d_perp;
        let feature = sphere.radius.min(torus.minor);
        match crate::implicit_trace::trace_intersection(
            &s_implicit,
            &t_implicit,
            seed,
            scale,
            feature,
            TRACE_TOLERANCE,
        ) {
            Ok(loops) if !loops.is_empty() => {
                report
                    .components
                    .extend(loops.into_iter().map(|l| SphereTorusComponent::Traced {
                        pieces: l.pieces,
                        distance_bound: l.distance_bound,
                        spans: l.spans,
                    }));
            }
            Ok(_) => report.unresolved(domain, UnresolvedReason::UnsupportedSurface),
            Err(e) if e.message.contains("tangency") => {
                report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot)
            }
            Err(_) => report.unresolved(domain, UnresolvedReason::UnsupportedSurface),
        }
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

#[cfg(test)]
mod tests;
