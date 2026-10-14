use super::*;

#[derive(Clone, Debug)]
pub enum CylinderCylinderComponent {
    /// Transverse side/side intersection: the exact straight ruling segment
    /// clipped by both finite heights, with UV lifts on both cylinders.
    Line {
        /// Exact degree-1 line, knots [0,0,1,1], unit weights, start -> end.
        curve: Curve,
        start: [f64; 3],
        end: [f64; 3],
        /// Unit common axis (the first cylinder's axis).
        direction: [f64; 3],
        /// Every resolved line is cut by cap planes on both ends, so the
        /// contact is on the boundary by construction; a band-thin clip
        /// degenerates and stays unresolved instead.
        contact: Contact,
        /// Iso-u segments on the first cylinder's side patches (two entries
        /// when the ruling sits on a quadrant seam).
        first_uv: Vec<CylinderPatchCurve>,
        second_uv: Vec<CylinderPatchCurve>,
        /// Worst radial residual over 9 line samples against both cylinder
        /// side equations.
        max_sample_residual: f64,
    },
}

/// Bezier parameter of the canonical rational quarter arc at angle `theta`
/// within its quadrant. For control points (1,0), (1,1), (0,1) with weights
/// [1, c, 1], c = cos(pi/4), and m = tan(theta), the angle equation is the
/// quadratic beta(m-1)u^2 + (2c + m(1-beta))u - m = 0 with beta = sqrt(2)-1;
/// the root in [0,1] is taken in the cancellation-free form.
pub(crate) fn arc_parameter(theta: f64) -> f64 {
    if !theta.is_finite() || theta <= 0. {
        return 0.;
    }
    let m = theta.tan();
    if !m.is_finite() {
        return 1.;
    }
    let beta = std::f64::consts::SQRT_2 - 1.;
    let b = std::f64::consts::SQRT_2 + m * (1. - beta);
    let disc = b * b + 4. * beta * m * (m - 1.);
    let root = if disc > 0. { disc.sqrt() } else { b };
    (2. * m / (b + root)).clamp(0., 1.)
}

/// Per-patch iso-u lift of a ruling: the line through `foot` along `axis`
/// over t in [lo, hi] sits at one fixed quadrant angle on the side wall;
/// the rational-arc parameter of that angle is exact via `arc_parameter`.
/// Seam rulings are emitted on both adjacent patches (u = 0 and u = 1).
pub(crate) fn lift_line(
    cylinder: &CanonicalCylinder,
    foot: [f64; 3],
    axis: [f64; 3],
    lo: f64,
    hi: f64,
) -> Vec<CylinderPatchCurve> {
    const SEAM: f64 = 1e-12;
    let rel = sub(foot, cylinder.center);
    let axial = dot(rel, cylinder.axis);
    let perp = sub(rel, cylinder.axis.map(|x| x * axial));
    let angle = dot(perp, cylinder.frame[1])
        .atan2(dot(perp, cylinder.frame[0]))
        .rem_euclid(TAU);
    let u_global = angle / QUARTER;
    let quadrant_f = u_global.floor();
    let u_linear = u_global - quadrant_f;
    let quadrant = quadrant_f as usize % 4;
    let u = arc_parameter(u_linear * QUARTER);
    let spin = dot(axis, cylinder.axis);
    let v_of = |t: f64| (axial + t * spin + cylinder.half_height) / (2. * cylinder.half_height);
    let (v_lo, v_hi) = (v_of(lo), v_of(hi));
    let segment = |u: f64| Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![[u, v_lo].to_vec(), [u, v_hi].to_vec()],
        weights: vec![1., 1.],
        periodic: false,
    };
    if u_linear <= SEAM {
        vec![
            CylinderPatchCurve {
                patch: cylinder.sides[quadrant],
                arcs: vec![segment(0.)],
            },
            CylinderPatchCurve {
                patch: cylinder.sides[(quadrant + 3) % 4],
                arcs: vec![segment(1.)],
            },
        ]
    } else if u_linear >= 1. - SEAM {
        vec![
            CylinderPatchCurve {
                patch: cylinder.sides[quadrant],
                arcs: vec![segment(1.)],
            },
            CylinderPatchCurve {
                patch: cylinder.sides[(quadrant + 1) % 4],
                arcs: vec![segment(0.)],
            },
        ]
    } else {
        vec![CylinderPatchCurve {
            patch: cylinder.sides[quadrant],
            arcs: vec![segment(u)],
        }]
    }
}

/// Coaxial configuration: coincident side bands, stacked rim contacts and
/// coincident cap disks stay unresolved; provably separated or radially
/// nested pairs resolve empty.
pub(super) fn coaxial(
    a: &CanonicalCylinder,
    b: &CanonicalCylinder,
    along: f64,
    band: f64,
    mut report: Report<CylinderCylinderComponent>,
    domain: Vec<f64>,
) -> Result<Report<CylinderCylinderComponent>> {
    // Cap planes in the common axial coordinate (first cylinder's center).
    let a_caps = [-a.half_height, a.half_height];
    let b_caps = [along - b.half_height, along + b.half_height];
    let cap_coincidence = a_caps
        .iter()
        .any(|&x| b_caps.iter().any(|&y| (x - y).abs() <= band));
    let overlap =
        a.half_height.min(along + b.half_height) - (-a.half_height).max(along - b.half_height);
    if (a.radius - b.radius).abs() <= band {
        if overlap > band {
            // Equal radii within the band and a clear height overlap: the
            // side surfaces coincide — never geometry.
            report.unresolved(domain, UnresolvedReason::CoincidentTrim);
        } else if cap_coincidence {
            // Stacked equal-radius cylinders sharing a cap plane within the
            // band: the coincident rim circle is a tangency contact, never a
            // guessed circle.
            report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        }
        // Heights disjoint beyond the band: no contact, resolved empty.
        return Ok(report);
    }
    // Provably different radii: the coaxial sides never meet and no rim lies
    // on the other side (that needs equal radii), so nested or stacked pairs
    // have no side contact. A coincident cap plane of nested coaxial
    // cylinders still shares the smaller cap disk — an honest
    // coincident-trim region rather than a resolved-empty report.
    if cap_coincidence {
        report.unresolved(domain, UnresolvedReason::CoincidentTrim);
    }
    Ok(report)
}

/// Analytic cylinder/cylinder intersection of two canonical cylinder solids,
/// parallel axes only. Non-canonical operands and clearly non-parallel pairs
/// are explicit unsupported regions, never a numerical fallback; near-
/// parallel and near-coaxial offsets inside the recognition band stay
/// unresolved — parallelism and coaxiality are certified, never forced.
/// Tangencies, band-thin height clips and coincident configurations stay
/// unresolved — tangent contacts are never guessed.
pub fn intersect_cylinder_cylinder(
    first: &Model,
    second: &Model,
    options: Options,
) -> Result<Report<CylinderCylinderComponent>> {
    let options = options.validate()?;
    let _ = options;
    let mut report = Report::default();
    let domain = vec![0., 1., 0., 1., 0., 1., 0., 1.];
    let (Some(a), Some(b)) = (recognize_cylinder(first)?, recognize_cylinder(second)?) else {
        report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    };
    report.boxes_visited = 1;
    let delta = sub(b.center, a.center);
    let terms = a.radius
        + b.radius
        + a.half_height
        + b.half_height
        + delta.iter().map(|v| v.abs()).fold(0., f64::max)
        + 1.;
    // Outward binary64 classification band: both recognition deviations plus
    // a rounding allowance on the axis/offset arithmetic and the radii.
    let band = a.error + b.error + 16. * f64::EPSILON * terms;
    // Axis parallelism is certified, never forced: pure-rounding direction
    // disagreement snaps to parallel, recognition-scale tilt reports
    // near_coincidence, and a clearly non-parallel pair is the unsupported
    // general (quartic) case.
    let direction = cross(a.axis, b.axis);
    let tilt = direction[0].hypot(direction[1]).hypot(direction[2]);
    let axis_snap =
        64. * f64::EPSILON + a.error / (2. * a.half_height) + b.error / (2. * b.half_height);
    if tilt > axis_snap {
        let reason = if tilt <= RECOGNITION {
            UnresolvedReason::NearCoincidence
        } else {
            UnresolvedReason::UnsupportedSurface
        };
        report.unresolved(domain, reason);
        return Ok(report);
    }
    let axis = a.axis;
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
    // zero; a recognition-scale offset is near_coincidence, never forced.
    let dist_snap = 64. * f64::EPSILON * terms + a.error + b.error;
    if d_perp <= dist_snap {
        return coaxial(&a, &b, along, band, report, domain);
    }
    if d_perp <= RECOGNITION * terms {
        report.unresolved(domain, UnresolvedReason::NearCoincidence);
        return Ok(report);
    }
    let r1 = a.radius;
    let r2 = b.radius;
    let sum = r1 + r2;
    let diff = (r1 - r2).abs();
    // Separate or radially nested sides: provably no contact.
    if d_perp > sum + band || d_perp + band < diff {
        return Ok(report);
    }
    // External or internal tangency, or a classification band straddling
    // one: tangent contacts are never reported as lines (house rule).
    if (d_perp - sum).abs() <= band || (d_perp - diff).abs() <= band {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    // Transverse pair: the planar circle/circle section in the common
    // perpendicular plane gives two exact foot points on both side walls.
    let u_dir = perp.map(|x| x / d_perp);
    let v_dir = cross(axis, u_dir);
    let x = (d_perp * d_perp + r1 * r1 - r2 * r2) / (2. * d_perp);
    let y2 = r1 * r1 - x * x;
    if !y2.is_finite() || y2 <= 0. {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    let y = y2.sqrt();
    // Clip interval in the common axial coordinate measured at the first
    // cylinder's center plane: the first cylinder owns |t| <= h1, the second
    // |t - along| <= h2; the same interval serves both rulings.
    let lo = (-a.half_height).max(along - b.half_height);
    let hi = a.half_height.min(along + b.half_height);
    if hi < lo - band {
        // The finite heights never overlap: both rulings are clipped away.
        return Ok(report);
    }
    if hi - lo <= band {
        // A band-thin clip degenerates to a boundary touch: unresolved.
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    for sign in [-1., 1.] {
        let foot: [f64; 3] =
            std::array::from_fn(|k| a.center[k] + x * u_dir[k] + sign * y * v_dir[k]);
        let start: [f64; 3] = std::array::from_fn(|k| foot[k] + lo * axis[k]);
        let end: [f64; 3] = std::array::from_fn(|k| foot[k] + hi * axis[k]);
        let curve = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![start.to_vec(), end.to_vec()],
            weights: vec![1., 1.],
            periodic: false,
        };
        let first_uv = lift_line(&a, foot, axis, lo, hi);
        let second_uv = lift_line(&b, foot, axis, lo, hi);
        let mut max_sample_residual = 0_f64;
        for k in 0..=8 {
            let t = lo + (hi - lo) * k as f64 / 8.;
            let point: [f64; 3] = std::array::from_fn(|i| foot[i] + t * axis[i]);
            for cylinder in [&a, &b] {
                let rel = sub(point, cylinder.center);
                let s = dot(rel, cylinder.axis);
                let radial = sub(rel, cylinder.axis.map(|x| x * s));
                let residual =
                    (radial[0].hypot(radial[1]).hypot(radial[2]) - cylinder.radius).abs();
                max_sample_residual = max_sample_residual.max(residual);
            }
        }
        report.components.push(CylinderCylinderComponent::Line {
            curve,
            start,
            end,
            direction: axis,
            contact: Contact::Boundary,
            first_uv,
            second_uv,
            max_sample_residual,
        });
    }
    Ok(report)
}
