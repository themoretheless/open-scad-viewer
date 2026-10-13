use super::*;

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
