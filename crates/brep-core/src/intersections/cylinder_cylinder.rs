//! Analytic cylinder/cylinder intersection for canonical solids with
//! parallel axes (including the coaxial configuration).
//!
//! Both operands must be the exact canonical six-face cylinder of
//! `analytic::cylinder` (four rational circular side patches with weights
//! [1, cos(pi/4), 1] and unit-square trims, two bilinear caps with inscribed
//! quarter-circle trims), optionally carried through a rigid affine
//! placement, as certified by the shared strict recognizer in
//! `sphere_cylinder`. Anything else is an explicit `UnsupportedSurface`
//! region — this cell never falls back to numerical surface/surface
//! subdivision. Axis parallelism and coaxiality are certified, never forced:
//! pure-rounding direction or center-line offsets snap, recognition-scale
//! offsets report `NearCoincidence`, and a clearly non-parallel pair is
//! `UnsupportedSurface` (the general cylinder/cylinder pair is a quartic;
//! even the equal-radius perpendicular case — a pair of ellipses — is out of
//! scope). Classification uses outward binary64 bands widened by the observed
//! recognition deviation. A transverse parallel pair reduces to a planar
//! circle/circle section in the common perpendicular plane and yields two
//! exact straight rulings (degree-1 lines) clipped by both finite heights;
//! the lifts are per-patch iso-u segments on the side patches of both
//! cylinders, duplicated across the quadrant seam when a ruling sits on it.
//! Every tangency — external (d == r1 + r2), internal (d == |r1 - r2|), a
//! band-thin height clip, the coaxial equal-radius coincident side band, and
//! stacked equal-radius cap-plane coincidences — stays an explicit unresolved
//! region; tangent contacts are never reported as point or guessed-line
//! components, matching the house tangency discipline. Coaxial cylinders with
//! provably different radii and no coincident cap plane resolve empty (the
//! sides never meet and no rim lies on the other side, which would need equal
//! radii); a coincident cap plane of nested coaxial cylinders shares the
//! smaller cap disk and is an honest `CoincidentTrim` region. Nothing here
//! authorizes a topology change.
use super::sphere_cylinder::{CanonicalCylinder, CylinderPatchCurve, recognize_cylinder};
use super::sphere_sphere::RECOGNITION;
use super::*;
use crate::Model;

const TAU: f64 = std::f64::consts::TAU;
const QUARTER: f64 = std::f64::consts::FRAC_PI_2;

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
fn coaxial(
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

impl value_codec::Serialize for CylinderCylinderComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Line {
                curve,
                start,
                end,
                direction,
                contact,
                first_uv,
                second_uv,
                max_sample_residual,
            } => {
                let contact = match contact {
                    Contact::Transverse => "transverse",
                    Contact::Boundary => "boundary",
                };
                value_codec::json!({"kind":"line","curve":curve,"start":start,"end":end,
                    "direction":direction,"contact":contact,"firstUv":first_uv,"secondUv":second_uv,
                    "maxSampleResidual":max_sample_residual})
            }
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
    fn only_lines(
        report: &Report<CylinderCylinderComponent>,
        count: usize,
    ) -> &Report<CylinderCylinderComponent> {
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        assert!(report.unresolved.is_empty(), "{report:?}");
        assert!(!report.permits_topology_change());
        assert_eq!(report.components.len(), count, "{report:?}");
        report
    }
    #[allow(clippy::type_complexity)]
    fn line_of(
        component: &CylinderCylinderComponent,
    ) -> (
        &Curve,
        [f64; 3],
        [f64; 3],
        [f64; 3],
        Contact,
        &[CylinderPatchCurve],
        &[CylinderPatchCurve],
        f64,
    ) {
        let CylinderCylinderComponent::Line {
            curve,
            start,
            end,
            direction,
            contact,
            first_uv,
            second_uv,
            max_sample_residual,
        } = component;
        (
            curve,
            *start,
            *end,
            *direction,
            *contact,
            first_uv,
            second_uv,
            *max_sample_residual,
        )
    }
    /// Radial residual against a cylinder side given center/axis/radius.
    fn side_residual(point: [f64; 3], center: [f64; 3], axis: [f64; 3], radius: f64) -> f64 {
        let rel = sub(point, center);
        let s = dot(rel, axis);
        let perp = sub(rel, axis.map(|x| x * s));
        (perp[0].hypot(perp[1]).hypot(perp[2]) - radius).abs()
    }
    /// Worst residual of lifted UV segments evaluated through their own side
    /// surfaces against both cylinder side equations.
    fn uv_samples(
        first_model: &Model,
        second_model: &Model,
        first_uv: &[CylinderPatchCurve],
        second_uv: &[CylinderPatchCurve],
        check: impl Fn([f64; 3]) -> f64,
    ) -> f64 {
        let mut worst = 0_f64;
        for (model, lifts) in [(first_model, first_uv), (second_model, second_uv)] {
            for lift in lifts {
                let surface = &model.faces[lift.patch].surface;
                for arc in &lift.arcs {
                    assert_eq!(arc.degree, 1);
                    for k in 0..=8 {
                        let uv = arc.evaluate(k as f64 / 8.).unwrap().point;
                        assert!((0. ..=1.).contains(&uv[0]) && (0. ..=1.).contains(&uv[1]));
                        let p = surface.evaluate(uv[0], uv[1]).unwrap().point;
                        worst = worst.max(check([p[0], p[1], p[2]]));
                    }
                }
            }
        }
        worst
    }

    #[test]
    fn two_parallel_cylinders_yield_two_exact_lines() {
        // r1=2 at the origin (z in 0..8), r2=3 offset by d=3.5 along x:
        // 1 = |r1-r2| < d < r1+r2 = 5, so the planar circle/circle section
        // gives two foot points at x = (d^2 + r1^2 - r2^2)/(2d), y = +-sqrt.
        let first = crate::analytic::cylinder(2., 8.).unwrap();
        let second = translated(&crate::analytic::cylinder(3., 8.).unwrap(), [3.5, 0., 0.]);
        let report = intersect_cylinder_cylinder(&first, &second, Options::default()).unwrap();
        let report = only_lines(&report, 2);
        let d = 3.5_f64;
        let x = (d * d + 4. - 9.) / (2. * d);
        let y = (4. - x * x).sqrt();
        for (component, sign) in report.components.iter().zip([-1., 1.]) {
            let (curve, start, end, direction, contact, first_uv, second_uv, sampled) =
                line_of(component);
            assert!(
                sub(start, [x, sign * y, 0.])
                    .iter()
                    .all(|v| v.abs() <= 1e-12),
                "{start:?}"
            );
            assert!(
                sub(end, [x, sign * y, 8.]).iter().all(|v| v.abs() <= 1e-12),
                "{end:?}"
            );
            assert!(direction[2].abs() >= 1. - 1e-12, "{direction:?}");
            assert_eq!(contact, Contact::Boundary);
            // Exact degree-1 line, unit knots and weights.
            assert_eq!(curve.degree, 1);
            assert_eq!(curve.knots, vec![0., 0., 1., 1.]);
            assert_eq!(curve.weights, vec![1., 1.]);
            assert_eq!(curve.control_points.len(), 2);
            // Nine samples satisfy both implicit cylinder equations.
            let mut worst = 0_f64;
            for k in 0..=8 {
                let p = curve.evaluate(k as f64 / 8.).unwrap().point;
                let p = [p[0], p[1], p[2]];
                worst = worst
                    .max((p[0].hypot(p[1]) - 2.).abs())
                    .max(((p[0] - 3.5).hypot(p[1]) - 3.).abs());
            }
            assert!(worst <= 1e-12, "{worst}");
            assert!(sampled <= 1e-12, "{sampled}");
            // The rulings sit strictly inside quadrants: one iso-u segment on
            // one side patch per cylinder, v sweeping the full height.
            assert_eq!(first_uv.len(), 1);
            assert_eq!(second_uv.len(), 1);
            for lift in first_uv.iter().chain(second_uv) {
                assert_eq!(lift.arcs.len(), 1);
                let arc = &lift.arcs[0];
                assert!((arc.control_points[0][1]).abs() <= 1e-12);
                assert!((arc.control_points[1][1] - 1.).abs() <= 1e-12);
                assert_eq!(arc.control_points[0][0], arc.control_points[1][0]);
            }
            let uv_worst = uv_samples(&first, &second, first_uv, second_uv, |p| {
                (p[0].hypot(p[1]) - 2.)
                    .abs()
                    .max(((p[0] - 3.5).hypot(p[1]) - 3.).abs())
            });
            assert!(uv_worst <= 1e-9, "{uv_worst}");
        }
    }

    #[test]
    fn seam_ruling_lifts_both_adjacent_side_patches() {
        // Place the second cylinder so that one ruling lands exactly on the
        // first cylinder's quadrant seam: offset along the direction at angle
        // -theta from the recognized frame x, where theta is the ruling angle.
        let first = crate::analytic::cylinder(2., 8.).unwrap();
        let rec = recognize_cylinder(&first).unwrap().unwrap();
        let d = 3.5_f64;
        let x = (d * d + 4. - 9.) / (2. * d);
        let y = (4. - x * x).sqrt();
        let (s, c) = (-y.atan2(x)).sin_cos();
        let u_world = [
            c * rec.frame[0][0] + s * rec.frame[1][0],
            c * rec.frame[0][1] + s * rec.frame[1][1],
            0.,
        ];
        let second = translated(
            &crate::analytic::cylinder(3., 8.).unwrap(),
            [d * u_world[0], d * u_world[1], 0.],
        );
        let report = intersect_cylinder_cylinder(&first, &second, Options::default()).unwrap();
        let report = only_lines(&report, 2);
        // The sign = +1 ruling sits on the first cylinder's seam at angle 0.
        let seam = line_of(&report.components[1]);
        assert_eq!(seam.5.len(), 2, "{:?}", seam.5);
        let us: Vec<f64> = seam
            .5
            .iter()
            .map(|lift| lift.arcs[0].control_points[0][0])
            .collect();
        assert!(us.contains(&0.) && us.contains(&1.), "{us:?}");
        assert_ne!(seam.5[0].patch, seam.5[1].patch);
        // The other ruling is strictly inside a quadrant: one patch.
        let plain = line_of(&report.components[0]);
        assert_eq!(plain.5.len(), 1);
        // Both seam lifts evaluate onto both cylinder side equations.
        let foot = [
            rec.center[0] + 2. * rec.frame[0][0],
            rec.center[1] + 2. * rec.frame[0][1],
            rec.center[2] + 2. * rec.frame[0][2],
        ];
        let second_center = [d * u_world[0], d * u_world[1], 4.];
        assert!(
            side_residual(foot, rec.center, rec.axis, 2.) <= 1e-12,
            "{foot:?}"
        );
        assert!(
            side_residual(foot, second_center, [0., 0., 1.], 3.) <= 1e-12,
            "{foot:?}"
        );
        for component in &report.components {
            let (_, _, _, _, _, first_uv, second_uv, _) = line_of(component);
            let uv_worst = uv_samples(&first, &second, first_uv, second_uv, |p| {
                side_residual(p, rec.center, rec.axis, 2.).max(side_residual(
                    p,
                    second_center,
                    [0., 0., 1.],
                    3.,
                ))
            });
            assert!(uv_worst <= 1e-9, "{uv_worst}");
        }
    }

    #[test]
    fn finite_heights_clip_or_eliminate_the_lines() {
        let first = crate::analytic::cylinder(2., 8.).unwrap();
        // Partial overlap: the second cylinder spans z in 5..9, so the clip
        // interval is [1, 4] about the first center plane — lines z in 5..8.
        let partial = translated(&crate::analytic::cylinder(3., 4.).unwrap(), [3.5, 0., 5.]);
        let report = intersect_cylinder_cylinder(&first, &partial, Options::default()).unwrap();
        let report = only_lines(&report, 2);
        let d = 3.5_f64;
        let x = (d * d + 4. - 9.) / (2. * d);
        let y = (4. - x * x).sqrt();
        for (component, sign) in report.components.iter().zip([-1., 1.]) {
            let (_, start, end, _, _, _, second_uv, _) = line_of(component);
            assert!(
                sub(start, [x, sign * y, 5.])
                    .iter()
                    .all(|v| v.abs() <= 1e-12),
                "{start:?}"
            );
            assert!(
                sub(end, [x, sign * y, 8.]).iter().all(|v| v.abs() <= 1e-12),
                "{end:?}"
            );
            // The second lift spans v in 0..3/4 of its own height.
            let arc = &second_uv[0].arcs[0];
            assert!((arc.control_points[0][1]).abs() <= 1e-12);
            assert!((arc.control_points[1][1] - 0.75).abs() <= 1e-12);
        }
        // Disjoint heights: the clip interval is empty beyond the band.
        let disjoint = translated(&crate::analytic::cylinder(3., 4.).unwrap(), [3.5, 0., 9.]);
        let report = intersect_cylinder_cylinder(&first, &disjoint, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
        // Heights touching at one cap plane: a band-thin clip degenerates.
        let touching = translated(&crate::analytic::cylinder(3., 4.).unwrap(), [3.5, 0., 8.]);
        let report = intersect_cylinder_cylinder(&first, &touching, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
    }

    #[test]
    fn external_and_internal_tangencies_stay_unresolved() {
        let first = crate::analytic::cylinder(2., 8.).unwrap();
        // External tangency: d == r1 + r2.
        let external = translated(&crate::analytic::cylinder(3., 8.).unwrap(), [5., 0., 0.]);
        // Internal tangency: d == |r1 - r2|.
        let internal = translated(&crate::analytic::cylinder(5., 8.).unwrap(), [3., 0., 0.]);
        for second in [&external, &internal] {
            let report = intersect_cylinder_cylinder(&first, second, Options::default()).unwrap();
            assert!(report.components.is_empty(), "{report:?}");
            assert_eq!(report.coverage, Coverage::Incomplete);
            assert_eq!(
                report.unresolved[0].reason,
                UnresolvedReason::TangencyOrMultipleRoot
            );
            assert!(!report.permits_topology_change());
        }
        // Just clear of the external band: separate, empty and resolved.
        let clear = translated(
            &crate::analytic::cylinder(3., 8.).unwrap(),
            [5. + 1e-9, 0., 0.],
        );
        let report = intersect_cylinder_cylinder(&first, &clear, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        // Just across: the transverse two-line case.
        let across = translated(
            &crate::analytic::cylinder(3., 8.).unwrap(),
            [5. - 1e-9, 0., 0.],
        );
        let report = intersect_cylinder_cylinder(&first, &across, Options::default()).unwrap();
        only_lines(&report, 2);
        // Radially nested without contact: d < |r1 - r2| provably.
        let nested = translated(&crate::analytic::cylinder(5., 8.).unwrap(), [2., 0., 0.]);
        let report = intersect_cylinder_cylinder(&first, &nested, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
    }

    #[test]
    fn coaxial_equal_radii_report_the_coincident_band() {
        let first = crate::analytic::cylinder(2., 8.).unwrap();
        // Exact equal radii, overlapping heights: coincident side surface.
        let same = crate::analytic::cylinder(2., 8.).unwrap();
        // Radii equal within the outward band: inseparable from coincidence.
        let near = crate::analytic::cylinder(2. + 2e-15, 8.).unwrap();
        for second in [&same, &near] {
            let report = intersect_cylinder_cylinder(&first, second, Options::default()).unwrap();
            assert!(report.components.is_empty(), "{report:?}");
            assert_eq!(report.coverage, Coverage::Incomplete);
            assert_eq!(
                report.unresolved[0].reason,
                UnresolvedReason::CoincidentTrim
            );
            assert!(!report.permits_topology_change());
        }
        // Stacked equal-radius cylinders sharing the cap plane z = 8: the
        // coincident rim circle is a tangency contact, never a guessed circle.
        let stacked = translated(&crate::analytic::cylinder(2., 4.).unwrap(), [0., 0., 8.]);
        let report = intersect_cylinder_cylinder(&first, &stacked, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
        // Equal radii, heights disjoint beyond the band: no contact.
        let apart = translated(&crate::analytic::cylinder(2., 4.).unwrap(), [0., 0., 9.]);
        let report = intersect_cylinder_cylinder(&first, &apart, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
    }

    #[test]
    fn coaxial_unequal_radii_resolve_empty_unless_cap_planes_coincide() {
        let outer = crate::analytic::cylinder(2., 8.).unwrap();
        // Nested coaxial cylinders with no shared cap plane (the inner spans
        // z in 2..6): the sides never meet and no rim lies on the other side.
        let inner = translated(&crate::analytic::cylinder(1., 4.).unwrap(), [0., 0., 2.]);
        let report = intersect_cylinder_cylinder(&outer, &inner, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
        // Same-height nested cylinders share both cap disks over the smaller
        // radius: an honest coincident-trim region, not resolved empty.
        let shared_caps = crate::analytic::cylinder(1., 8.).unwrap();
        let report = intersect_cylinder_cylinder(&outer, &shared_caps, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::CoincidentTrim
        );
        // Stacked cylinders with different radii share the smaller cap disk:
        // an honest coincident-trim region, not a resolved-empty report.
        let stacked = translated(&crate::analytic::cylinder(1., 4.).unwrap(), [0., 0., 8.]);
        let report = intersect_cylinder_cylinder(&outer, &stacked, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::CoincidentTrim
        );
    }

    #[test]
    fn near_parallel_within_the_band_stays_unresolved_not_forced() {
        let first = crate::analytic::cylinder(2., 8.).unwrap();
        let tilted = rotated_translated(
            &crate::analytic::cylinder(3., 8.).unwrap(),
            1e-10,
            [3.5, 0., 0.],
        );
        let report = intersect_cylinder_cylinder(&first, &tilted, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::NearCoincidence
        );
        // Clearly non-parallel: the general quartic is out of scope.
        let skew = rotated_translated(
            &crate::analytic::cylinder(3., 8.).unwrap(),
            0.3,
            [3.5, 0., 0.],
        );
        let report = intersect_cylinder_cylinder(&first, &skew, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.unresolved.len(), 1);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::UnsupportedSurface
        );
        assert_eq!(
            report.unresolved[0].parameter_box,
            vec![0., 1., 0., 1., 0., 1., 0., 1.]
        );
        // A near-coaxial offset inside the recognition band: near_coincidence.
        let near = translated(&crate::analytic::cylinder(2., 8.).unwrap(), [1e-10, 0., 0.]);
        let report = intersect_cylinder_cylinder(&first, &near, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::NearCoincidence
        );
    }

    #[test]
    fn rotated_translated_placement_keeps_the_exact_lines() {
        // Rigid placement of the whole parallel configuration about X by 0.5.
        let angle = 0.5;
        let offset = [0.3, -0.2, 1.1];
        let first = rotated_translated(&crate::analytic::cylinder(2., 8.).unwrap(), angle, offset);
        let second = rotated_translated(
            &translated(&crate::analytic::cylinder(3., 8.).unwrap(), [3.5, 0., 0.]),
            angle,
            offset,
        );
        let report = intersect_cylinder_cylinder(&first, &second, Options::default()).unwrap();
        let report = only_lines(&report, 2);
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
        let first_center = placed([0., 0., 4.]);
        let second_center = placed([3.5, 0., 4.]);
        let d = 3.5_f64;
        let x = (d * d + 4. - 9.) / (2. * d);
        let y = (4. - x * x).sqrt();
        for (component, sign) in report.components.iter().zip([-1., 1.]) {
            let (curve, start, end, direction, _, first_uv, second_uv, sampled) =
                line_of(component);
            assert!(
                sub(start, placed([x, sign * y, 0.]))
                    .iter()
                    .all(|v| v.abs() <= 1e-12),
                "{start:?}"
            );
            assert!(
                sub(end, placed([x, sign * y, 8.]))
                    .iter()
                    .all(|v| v.abs() <= 1e-12),
                "{end:?}"
            );
            assert!(
                sub(direction, axis).iter().all(|v| v.abs() <= 1e-12),
                "{direction:?}"
            );
            let mut worst = 0_f64;
            for k in 0..=8 {
                let p = curve.evaluate(k as f64 / 8.).unwrap().point;
                let p = [p[0], p[1], p[2]];
                worst = worst
                    .max(side_residual(p, first_center, axis, 2.))
                    .max(side_residual(p, second_center, axis, 3.));
            }
            assert!(worst <= 1e-12, "{worst}");
            assert!(sampled <= 1e-12);
            let uv_worst = uv_samples(&first, &second, first_uv, second_uv, |p| {
                side_residual(p, first_center, axis, 2.).max(side_residual(
                    p,
                    second_center,
                    axis,
                    3.,
                ))
            });
            assert!(uv_worst <= 1e-9, "{uv_worst}");
        }
    }

    #[test]
    fn noncanonical_operands_are_explicit_unsupported_regions() {
        let cylinder = crate::analytic::cylinder(2., 8.).unwrap();
        // Frusta, tubes, spheres and cuboids are not the canonical cylinder.
        for (a, b) in [
            (
                crate::analytic::frustum(1., 2., 3.).unwrap(),
                crate::analytic::cylinder(2., 8.).unwrap(),
            ),
            (
                crate::analytic::tube(2., 1., 3.).unwrap(),
                crate::analytic::cylinder(2., 8.).unwrap(),
            ),
            (
                crate::analytic::cylinder(2., 8.).unwrap(),
                crate::analytic::sphere(2.).unwrap(),
            ),
            (
                crate::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap(),
                crate::analytic::cylinder(2., 8.).unwrap(),
            ),
        ] {
            let report = intersect_cylinder_cylinder(&a, &b, Options::default()).unwrap();
            assert!(report.components.is_empty(), "{report:?}");
            assert_eq!(report.coverage, Coverage::Incomplete);
            assert_eq!(report.unresolved.len(), 1);
            assert_eq!(
                report.unresolved[0].reason,
                UnresolvedReason::UnsupportedSurface
            );
        }
        // A canonical pair still resolves.
        let second = translated(&crate::analytic::cylinder(3., 8.).unwrap(), [3.5, 0., 0.]);
        let report = intersect_cylinder_cylinder(&cylinder, &second, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
        assert_eq!(report.components.len(), 2);
        // A structurally perturbed cylinder fails validation as a hard error.
        let mut perturbed = crate::analytic::cylinder(1., 2.).unwrap();
        perturbed.faces[0].surface.weights[1][0] = 0.5;
        assert!(intersect_cylinder_cylinder(&cylinder, &perturbed, Options::default()).is_err());
    }
}
