//! Analytic plane/cylinder intersection for a canonical planar patch operand
//! and a canonical cylinder solid.
//!
//! The plane operand is the canonical finite rectangular planar patch of
//! `plane_sphere::recognize_plane` (one-face open model, exact bilinear
//! affine surface over the unit square, orthogonal edge vectors); the
//! cylinder operand is the exact canonical six-face cylinder of
//! `analytic::cylinder` (recognized by `sphere_cylinder::recognize_cylinder`).
//! Anything else is an explicit `UnsupportedSurface` region — this cell
//! never falls back to numerical surface/surface subdivision. The axis/plane
//! angle is certified, never forced: pure-rounding tilts snap to
//! perpendicular or parallel, recognition-scale tilts report
//! `NearCoincidence`, and every clearly oblique configuration is handled
//! exactly. Classification uses outward binary64 bands widened by both
//! recognition deviations.
//!
//! Plane perpendicular to the axis: the section is a circle of the
//!   cylinder radius at the section height. Provably beyond both cap planes
//!   resolves empty; a plane within the band of a cap plane is a coincident
//!   cap/rim region (`CoincidentTrim`) — never a guessed circle; a clear
//!   section yields the exact circle, clipped by the patch's finite
//!   rectangular domain with exact trim parameters (domain-edge tangencies
//!   stay unresolved). Lifts: exact ellipse arcs in the plane UV and iso-v
//!   circle arcs on the side patches (full circle: all four patches; clipped
//!   arcs: per-patch sub-u ranges with the exact rational-arc angle
//!   parameter of `cylinder_cylinder::arc_parameter`).
//! Plane parallel to the axis: distance d from the axis. d > R resolves
//!   empty, d == R within the band stays `TangencyOrMultipleRoot` (a tangent
//!   plane is never a guessed line), d < R yields two exact straight lines
//!   (degree-1 NURBS) clipped by the finite height and by the patch
//!   rectangle (band-thin clips and patch-corner touches stay unresolved),
//!   lifted as iso-u rulings on the side patches (seam rulings duplicated on
//!   both adjacent patches) and as degree-1 segments in the plane UV.
//! Oblique plane: the section is the exact ellipse with semi-minor R and
//!   semi-major R / |axis.n| — a rational quadratic NURBS with exact weights
//!   — clipped by the finite height and the patch rectangle in one exact
//!   eccentric-angle clip (all six constraints are linear in the plane UV);
//!   cap-plane tangencies and boundary tangencies stay unresolved. The lift
//!   into the plane UV is the exact rotated ellipse. The lift onto the
//!   cylinder side is NOT shipped in this slice: on the rational side
//!   patches the ellipse unrolls to a cosine curve with no low-degree exact
//!   rational UV representation, so `cylinder_uv` is null rather than a
//!   numerical fit — the 3D ellipse and the plane lift stay exact.
//!
//! Scope note (shared with the cylinder/cylinder cell): only side-surface
//! contacts are reported; where the plane patch would cross a cap disk
//! (parallel/oblique cases) the cap chord is not a component — coverage is
//! NumericallyResolved, never certified complete, and nothing here
//! authorizes a topology change.
use super::cylinder_cylinder::{arc_parameter, lift_line};
use super::plane_sphere::{
    CanonicalPlane, EllipseClip, Halfplane2, PlanePatchCurve, clip_ellipse, conic_arcs_2d,
    conic_sweep, recognize_plane,
};
use super::sphere_cylinder::{CanonicalCylinder, CylinderPatchCurve, recognize_cylinder};
use super::sphere_sphere::{RECOGNITION, ccw_intersect};
use super::*;
use crate::Model;

const TAU: f64 = std::f64::consts::TAU;
const QUARTER: f64 = std::f64::consts::FRAC_PI_2;

#[derive(Clone, Debug)]
pub enum PlaneCylinderComponent {
    /// Plane perpendicular to the axis: the exact circle (full, or clipped
    /// to the finite plane domain) with lifts on both surfaces.
    Circle {
        /// Exact rational quadratic sweep (full circle: four 90-degree arcs,
        /// weights cos(pi/4), knots 0..=4).
        curve: Curve,
        center: [f64; 3],
        radius: f64,
        /// Unit plane normal.
        normal: [f64; 3],
        /// True when the circle lies fully inside the plane patch domain.
        full: bool,
        plane_uv: Vec<PlanePatchCurve>,
        /// Iso-v circle arcs on the side patches.
        cylinder_uv: Vec<CylinderPatchCurve>,
        max_sample_residual: f64,
    },
    /// Plane parallel to the axis: one exact straight ruling clipped by the
    /// finite height and the patch rectangle.
    Line {
        /// Exact degree-1 line, knots [0,0,1,1], unit weights, start -> end.
        curve: Curve,
        start: [f64; 3],
        end: [f64; 3],
        /// Unit cylinder axis (the line direction).
        direction: [f64; 3],
        /// Clip endpoints sit on cap planes or the patch boundary.
        contact: Contact,
        plane_uv: Vec<PlanePatchCurve>,
        /// Iso-u segments on the cylinder side patches (two entries when the
        /// ruling sits on a quadrant seam).
        cylinder_uv: Vec<CylinderPatchCurve>,
        max_sample_residual: f64,
    },
    /// Oblique plane: the exact ellipse (full, or clipped to the finite
    /// height and patch domain) with the exact plane-UV lift. The cylinder
    /// side lift is null: the unrolled ellipse is a cosine curve with no
    /// exact low-degree rational UV representation (documented in the module
    /// header); the 3D ellipse is exact regardless.
    Ellipse {
        /// Exact rational quadratic sweep over the eccentric-angle interval.
        curve: Curve,
        center: [f64; 3],
        /// Semi-major R / |axis.n| along `major`.
        semi_major: f64,
        /// Semi-minor R along `minor`.
        semi_minor: f64,
        /// Unit in-plane direction of the axis projection (semi-major axis).
        major: [f64; 3],
        /// Unit in-plane direction perpendicular to it (semi-minor axis).
        minor: [f64; 3],
        /// Unit plane normal.
        normal: [f64; 3],
        /// True when the full ellipse survives every clip.
        full: bool,
        plane_uv: Vec<PlanePatchCurve>,
        cylinder_uv: Option<Vec<CylinderPatchCurve>>,
        max_sample_residual: f64,
    },
}

fn point3_of(jet_point: &[f64]) -> [f64; 3] {
    [jet_point[0], jet_point[1], jet_point[2]]
}

/// Plane UV coordinates of a 3D point under the rectangular patch frame.
fn plane_uv(plane: &CanonicalPlane, point: [f64; 3]) -> [f64; 2] {
    let rel = sub(point, plane.origin);
    [
        dot(rel, plane.u) / (plane.u_len * plane.u_len),
        dot(rel, plane.v) / (plane.v_len * plane.v_len),
    ]
}

/// Rectangle half-planes of the patch domain with per-axis bands.
fn rect_halfplanes(plane: &CanonicalPlane, band: f64) -> [Halfplane2; 4] {
    [
        Halfplane2 {
            normal: [-1., 0.],
            offset: 0.,
            band: band / plane.u_len,
        },
        Halfplane2 {
            normal: [1., 0.],
            offset: 1.,
            band: band / plane.u_len,
        },
        Halfplane2 {
            normal: [0., -1.],
            offset: 0.,
            band: band / plane.v_len,
        },
        Halfplane2 {
            normal: [0., 1.],
            offset: 1.,
            band: band / plane.v_len,
        },
    ]
}

/// Worst residual over `count` curve samples against the cylinder side
/// equation and the plane equation.
fn side_plane_residuals(
    curve: &Curve,
    cylinder: &CanonicalCylinder,
    plane: &CanonicalPlane,
    count: usize,
) -> Result<f64> {
    let [lo, hi] = curve.domain();
    let mut worst = 0_f64;
    for i in 0..count {
        let t = lo + (hi - lo) * i as f64 / (count - 1) as f64;
        let p = point3_of(&curve.evaluate(t)?.point);
        let rel = sub(p, cylinder.center);
        let a = dot(rel, cylinder.axis);
        let perp = sub(rel, cylinder.axis.map(|x| x * a));
        let side = (perp[0].hypot(perp[1]).hypot(perp[2]) - cylinder.radius).abs();
        let planar = dot(plane.normal, sub(p, plane.origin)).abs();
        worst = worst.max(side).max(planar);
    }
    Ok(worst)
}

/// Clipped iso-v circle lifts on the side patches for a section circle whose
/// swept parameter intervals are `arcs`. `point_at(phi)` evaluates the exact
/// 3D circle point; the angular direction per arc is fixed by the exact
/// midpoint image, and the per-quadrant trims use the exact rational-arc
/// parameter (a quadratic solve in tan(theta), never a linear fraction).
fn lift_circle_side(
    cylinder: &CanonicalCylinder,
    center: [f64; 3],
    v0: f64,
    arcs: &[(f64, f64)],
    point_at: impl Fn(f64) -> [f64; 3],
) -> Vec<CylinderPatchCurve> {
    let mut lifted = Vec::new();
    for &(a, b) in arcs {
        let sweep = b - a;
        if !sweep.is_finite() || sweep <= 0. {
            continue;
        }
        let theta = |phi: f64| {
            let rel = sub(point_at(phi), center);
            dot(rel, cylinder.frame[1])
                .atan2(dot(rel, cylinder.frame[0]))
                .rem_euclid(TAU)
        };
        let (ta, tb, tm) = (theta(a), theta(b), theta(a + sweep / 2.));
        let forward = (tb - ta).rem_euclid(TAU);
        let image = if (tm - ta).rem_euclid(TAU) <= forward {
            (ta, forward)
        } else {
            (tb, (ta - tb).rem_euclid(TAU))
        };
        for quadrant in 0..4 {
            let span = (quadrant as f64 * QUARTER, QUARTER);
            for (s0, sw) in ccw_intersect(span, image) {
                let rel0 = s0 - quadrant as f64 * QUARTER;
                let u0 = arc_parameter(rel0);
                let u1 = arc_parameter(rel0 + sw);
                if u1 - u0 <= 1e-12 {
                    continue;
                }
                lifted.push(CylinderPatchCurve {
                    patch: cylinder.sides[quadrant],
                    arcs: vec![Curve {
                        degree: 1,
                        knots: vec![0., 0., 1., 1.],
                        control_points: vec![[u0, v0].to_vec(), [u1, v0].to_vec()],
                        weights: vec![1., 1.],
                        periodic: false,
                    }],
                });
            }
        }
    }
    lifted
}

/// Iso-v lines on all four side patches (full circle at height v0).
fn lift_circle_side_full(cylinder: &CanonicalCylinder, v0: f64) -> Vec<CylinderPatchCurve> {
    cylinder
        .sides
        .iter()
        .map(|&patch| CylinderPatchCurve {
            patch,
            arcs: vec![Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                control_points: vec![[0., v0].to_vec(), [1., v0].to_vec()],
                weights: vec![1., 1.],
                periodic: false,
            }],
        })
        .collect()
}

/// Clip the line `foot_uv + t * dir_uv` (t in model units along the axis) to
/// `[lo0, hi0]` and the unit square. Returns the surviving interval, or the
/// reason it fails: misses are silent, boundary-coincident lines, band-thin
/// clips and patch-corner touches are tangencies.
enum LineClip {
    Miss,
    Tangent,
    Span(f64, f64),
}

fn clip_line_rect(
    foot_uv: [f64; 2],
    dir_uv: [f64; 2],
    lo0: f64,
    hi0: f64,
    band: f64,
    band_uv: [f64; 2],
) -> LineClip {
    let mut lo = lo0;
    let mut hi = hi0;
    for k in 0..2 {
        let f = foot_uv[k];
        let d = dir_uv[k];
        if d.abs() <= 1e-30 {
            // Line parallel to this axis' domain edges.
            if f < -band_uv[k] || f > 1. + band_uv[k] {
                return LineClip::Miss;
            }
            if f.abs() <= band_uv[k] || (f - 1.).abs() <= band_uv[k] {
                // The line runs along the patch boundary within the band.
                return LineClip::Tangent;
            }
            continue;
        }
        let t0 = -f / d;
        let t1 = (1. - f) / d;
        lo = lo.max(t0.min(t1));
        hi = hi.min(t0.max(t1));
    }
    if hi < lo - band {
        return LineClip::Miss;
    }
    if hi - lo <= band {
        // Band-thin clip: a boundary touch, never a degenerate segment.
        return LineClip::Tangent;
    }
    // Patch-corner touch: a clip endpoint within the band of an edge in both
    // UV axes is a tangency, never a guessed corner segment endpoint.
    for t in [lo, hi] {
        let q = [foot_uv[0] + t * dir_uv[0], foot_uv[1] + t * dir_uv[1]];
        let near_u = q[0].abs() <= band_uv[0] || (q[0] - 1.).abs() <= band_uv[0];
        let near_v = q[1].abs() <= band_uv[1] || (q[1] - 1.).abs() <= band_uv[1];
        if near_u && near_v {
            return LineClip::Tangent;
        }
    }
    LineClip::Span(lo, hi)
}

/// Analytic plane/cylinder intersection of a canonical planar patch and a
/// canonical cylinder solid. Non-canonical operands are explicit
/// `UnsupportedSurface` regions, never a numerical fallback; tangencies
/// (d == R, cap-plane coincidence, domain-edge and cap-plane touches) stay
/// unresolved — tangent contacts are never guessed.
pub fn intersect_plane_cylinder(
    plane_model: &Model,
    cylinder_model: &Model,
    options: Options,
) -> Result<Report<PlaneCylinderComponent>> {
    let options = options.validate()?;
    let _ = options;
    let mut report = Report::default();
    let domain = vec![0., 1., 0., 1., 0., 1., 0., 1.];
    let (Some(plane), Some(cylinder)) = (
        recognize_plane(plane_model)?,
        recognize_cylinder(cylinder_model)?,
    ) else {
        report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    };
    report.boxes_visited = 1;
    let g = dot(cylinder.axis, plane.normal);
    let terms = cylinder.radius
        + cylinder.half_height
        + plane.u_len
        + plane.v_len
        + sub(cylinder.center, plane.origin)
            .iter()
            .map(|v| v.abs())
            .fold(0., f64::max)
        + 1.;
    // Outward binary64 classification band: both recognition deviations plus
    // a rounding allowance on the section arithmetic and the dimensions.
    let band = plane.error + cylinder.error + 16. * f64::EPSILON * terms;
    // Angular certification: pure-rounding tilts snap, recognition-scale
    // tilts report near_coincidence, clearly oblique angles are exact.
    let angular_snap = 64. * f64::EPSILON
        + plane.error / plane.u_len.min(plane.v_len)
        + cylinder.error / (2. * cylinder.half_height);
    let ga = g.abs();
    let rect = rect_halfplanes(&plane, band);
    if ga >= 1. - angular_snap {
        // Plane perpendicular to the axis: exact circle at the section
        // height, clipped by the patch domain.
        let axial = dot(sub(plane.origin, cylinder.center), plane.normal) / g;
        let half = cylinder.half_height;
        if axial.abs() > half + band {
            return Ok(report);
        }
        if (axial.abs() - half).abs() <= band {
            // The plane coincides with a cap plane within the band: the
            // coincident cap/rim region is never a guessed circle.
            report.unresolved(domain, UnresolvedReason::CoincidentTrim);
            return Ok(report);
        }
        let center = std::array::from_fn(|k| cylinder.center[k] + axial * cylinder.axis[k]);
        let r = cylinder.radius;
        let e1 = plane.u.map(|x| x / plane.u_len);
        let e2 = plane.v.map(|x| x / plane.v_len);
        let [uc, vc] = plane_uv(&plane, center);
        let w1 = [r / plane.u_len, 0.];
        let w2 = [0., r / plane.v_len];
        let (clip, tangent) = clip_ellipse([uc, vc], w1, w2, &rect);
        if tangent {
            report.unresolved(domain.clone(), UnresolvedReason::TangencyOrMultipleRoot);
        }
        let v0 = (axial + half) / (2. * half);
        let point_at = |phi: f64| {
            std::array::from_fn::<f64, 3, _>(|k| {
                center[k] + r * (phi.cos() * e1[k] + phi.sin() * e2[k])
            })
        };
        let we1 = e1.map(|x| x * r);
        let we2 = e2.map(|x| x * r);
        match clip {
            EllipseClip::Empty => {}
            EllipseClip::Full if !tangent => {
                let curve = conic_sweep(center, we1, we2, 0., TAU);
                let max_sample_residual = side_plane_residuals(&curve, &cylinder, &plane, 16)?;
                report.components.push(PlaneCylinderComponent::Circle {
                    curve,
                    center,
                    radius: r,
                    normal: plane.normal,
                    full: true,
                    plane_uv: vec![PlanePatchCurve {
                        patch: 0,
                        arcs: conic_arcs_2d([uc, vc], w1, w2, 0., TAU),
                    }],
                    cylinder_uv: lift_circle_side_full(&cylinder, v0),
                    max_sample_residual,
                });
            }
            EllipseClip::Arcs(intervals) => {
                for (a, b) in intervals {
                    let curve = conic_sweep(center, we1, we2, a, b);
                    let max_sample_residual = side_plane_residuals(&curve, &cylinder, &plane, 9)?;
                    report.components.push(PlaneCylinderComponent::Circle {
                        curve,
                        center,
                        radius: r,
                        normal: plane.normal,
                        full: false,
                        plane_uv: vec![PlanePatchCurve {
                            patch: 0,
                            arcs: conic_arcs_2d([uc, vc], w1, w2, a, b),
                        }],
                        cylinder_uv: lift_circle_side(&cylinder, center, v0, &[(a, b)], point_at),
                        max_sample_residual,
                    });
                }
            }
            // Tangency within the band: the full circle is withheld; the
            // unresolved region above carries the contact.
            EllipseClip::Full => {}
        }
        return Ok(report);
    }
    if ga <= angular_snap {
        // Plane parallel to the axis: two exact rulings at chord distance.
        let s0 = dot(plane.normal, sub(cylinder.center, plane.origin));
        let d = s0.abs();
        let r = cylinder.radius;
        if d > r + band {
            return Ok(report);
        }
        if (d - r).abs() <= band {
            report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
            return Ok(report);
        }
        let h2 = r * r - d * d;
        if !h2.is_finite() || h2 <= 0. {
            report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
            return Ok(report);
        }
        let h = h2.sqrt();
        let e = cross(cylinder.axis, plane.normal);
        let e_len = e[0].hypot(e[1]).hypot(e[2]);
        if !e_len.is_finite() || e_len <= 0. {
            report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
            return Ok(report);
        }
        let e = e.map(|x| x / e_len);
        let half = cylinder.half_height;
        let band_uv = [band / plane.u_len, band / plane.v_len];
        let du = dot(cylinder.axis, plane.u) / (plane.u_len * plane.u_len);
        let dv = dot(cylinder.axis, plane.v) / (plane.v_len * plane.v_len);
        for sign in [-1., 1.] {
            let foot: [f64; 3] = std::array::from_fn(|k| {
                cylinder.center[k] - s0 * plane.normal[k] + sign * h * e[k]
            });
            let foot_uv = plane_uv(&plane, foot);
            match clip_line_rect(foot_uv, [du, dv], -half, half, band, band_uv) {
                LineClip::Miss => {}
                LineClip::Tangent => {
                    report.unresolved(domain.clone(), UnresolvedReason::TangencyOrMultipleRoot);
                }
                LineClip::Span(lo, hi) => {
                    let start: [f64; 3] = std::array::from_fn(|k| foot[k] + lo * cylinder.axis[k]);
                    let end: [f64; 3] = std::array::from_fn(|k| foot[k] + hi * cylinder.axis[k]);
                    let curve = Curve {
                        degree: 1,
                        knots: vec![0., 0., 1., 1.],
                        control_points: vec![start.to_vec(), end.to_vec()],
                        weights: vec![1., 1.],
                        periodic: false,
                    };
                    let cylinder_uv = lift_line(&cylinder, foot, cylinder.axis, lo, hi);
                    let plane_uv = vec![PlanePatchCurve {
                        patch: 0,
                        arcs: vec![Curve {
                            degree: 1,
                            knots: vec![0., 0., 1., 1.],
                            control_points: vec![
                                [foot_uv[0] + lo * du, foot_uv[1] + lo * dv].to_vec(),
                                [foot_uv[0] + hi * du, foot_uv[1] + hi * dv].to_vec(),
                            ],
                            weights: vec![1., 1.],
                            periodic: false,
                        }],
                    }];
                    let max_sample_residual = side_plane_residuals(&curve, &cylinder, &plane, 9)?;
                    report.components.push(PlaneCylinderComponent::Line {
                        curve,
                        start,
                        end,
                        direction: cylinder.axis,
                        contact: Contact::Boundary,
                        plane_uv,
                        cylinder_uv,
                        max_sample_residual,
                    });
                }
            }
        }
        return Ok(report);
    }
    if ga >= 1. - RECOGNITION || ga <= RECOGNITION {
        // Recognition-scale tilt: parallelism/perpendicularity cannot be
        // certified against the recognition deviations — never forced.
        report.unresolved(domain, UnresolvedReason::NearCoincidence);
        return Ok(report);
    }
    // Clearly oblique: the exact ellipse with semi-minor R and semi-major
    // R / |axis.n|, clipped by the finite height and the patch domain in one
    // exact eccentric-angle clip (every constraint is linear in plane UV).
    let m = sub(cylinder.axis, plane.normal.map(|x| x * g));
    let s = m[0].hypot(m[1]).hypot(m[2]);
    if !s.is_finite() || s <= 0. {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    let major = m.map(|x| x / s);
    let minor = cross(plane.normal, major);
    let semi_major = cylinder.radius / ga;
    let semi_minor = cylinder.radius;
    let t = dot(sub(plane.origin, cylinder.center), plane.normal) / g;
    let center = std::array::from_fn(|k| cylinder.center[k] + t * cylinder.axis[k]);
    let uh = plane.u.map(|x| x / plane.u_len);
    let vh = plane.v.map(|x| x / plane.v_len);
    let w1 = [
        semi_major * dot(major, uh) / plane.u_len,
        semi_major * dot(major, vh) / plane.v_len,
    ];
    let w2 = [
        semi_minor * dot(minor, uh) / plane.u_len,
        semi_minor * dot(minor, vh) / plane.v_len,
    ];
    let c2 = plane_uv(&plane, center);
    // The finite height as two half-planes in patch UV: the axial coordinate
    // of origin + u*U + v*V is a0 + u*au + v*av (model units).
    let a0 = dot(sub(plane.origin, cylinder.center), cylinder.axis);
    let au = dot(plane.u, cylinder.axis);
    let av = dot(plane.v, cylinder.axis);
    let half = cylinder.half_height;
    let mut halfplanes = rect.to_vec();
    halfplanes.push(Halfplane2 {
        normal: [au, av],
        offset: half - a0,
        band,
    });
    halfplanes.push(Halfplane2 {
        normal: [-au, -av],
        offset: half + a0,
        band,
    });
    let (clip, tangent) = clip_ellipse(c2, w1, w2, &halfplanes);
    if tangent {
        report.unresolved(domain.clone(), UnresolvedReason::TangencyOrMultipleRoot);
    }
    let we1 = major.map(|x| x * semi_major);
    let we2 = minor.map(|x| x * semi_minor);
    let push_ellipse = |report: &mut Report<PlaneCylinderComponent>,
                        start: f64,
                        end: f64,
                        full: bool|
     -> Result<()> {
        let curve = conic_sweep(center, we1, we2, start, end);
        let max_sample_residual = side_plane_residuals(&curve, &cylinder, &plane, 9)?;
        report.components.push(PlaneCylinderComponent::Ellipse {
            curve,
            center,
            semi_major,
            semi_minor,
            major,
            minor,
            normal: plane.normal,
            full,
            plane_uv: vec![PlanePatchCurve {
                patch: 0,
                arcs: conic_arcs_2d(c2, w1, w2, start, end),
            }],
            cylinder_uv: None,
            max_sample_residual,
        });
        Ok(())
    };
    match clip {
        EllipseClip::Empty => {}
        EllipseClip::Full if !tangent => push_ellipse(&mut report, 0., TAU, true)?,
        EllipseClip::Arcs(intervals) => {
            for (a, b) in intervals {
                push_ellipse(&mut report, a, b, false)?;
            }
        }
        // Tangency within the band: the full ellipse is withheld; the
        // unresolved region above carries the contact.
        EllipseClip::Full => {}
    }
    Ok(report)
}

impl value_codec::Serialize for PlaneCylinderComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Circle {
                curve,
                center,
                radius,
                normal,
                full,
                plane_uv,
                cylinder_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"circle","curve":curve,"center":center,"radius":radius,
                "normal":normal,"full":full,"planeUv":plane_uv,"cylinderUv":cylinder_uv,
                "maxSampleResidual":max_sample_residual}),
            Self::Line {
                curve,
                start,
                end,
                direction,
                contact,
                plane_uv,
                cylinder_uv,
                max_sample_residual,
            } => {
                let contact = match contact {
                    Contact::Transverse => "transverse",
                    Contact::Boundary => "boundary",
                };
                value_codec::json!({"kind":"line","curve":curve,"start":start,"end":end,
                    "direction":direction,"contact":contact,"planeUv":plane_uv,
                    "cylinderUv":cylinder_uv,"maxSampleResidual":max_sample_residual})
            }
            Self::Ellipse {
                curve,
                center,
                semi_major,
                semi_minor,
                major,
                minor,
                normal,
                full,
                plane_uv,
                cylinder_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"ellipse","curve":curve,"center":center,
                "semiMajor":semi_major,"semiMinor":semi_minor,"major":major,"minor":minor,
                "normal":normal,"full":full,"planeUv":plane_uv,"cylinderUv":cylinder_uv,
                "maxSampleResidual":max_sample_residual}),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::plane_sphere::plane_patch;
    use super::*;

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

    fn point_of(jet: &[f64]) -> [f64; 3] {
        [jet[0], jet[1], jet[2]]
    }

    /// Worst residual of the UV lifts evaluated through their own surfaces
    /// against the cylinder side and plane equations.
    fn lift_worst(
        plane_model: &Model,
        cylinder_model: &Model,
        plane_uv: &[PlanePatchCurve],
        cylinder_uv: &[CylinderPatchCurve],
        cylinder_def: ([f64; 3], [f64; 3], f64),
        plane_def: ([f64; 3], [f64; 3]),
    ) -> f64 {
        let check = |p: [f64; 3]| {
            let rel = sub(p, cylinder_def.0);
            let a = dot(rel, cylinder_def.1);
            let perp = sub(rel, cylinder_def.1.map(|x| x * a));
            (perp[0].hypot(perp[1]).hypot(perp[2]) - cylinder_def.2)
                .abs()
                .max(dot(plane_def.0, sub(p, plane_def.1)).abs())
        };
        let mut worst = 0_f64;
        for lift in plane_uv {
            let surface = &plane_model.faces[lift.patch].surface;
            for arc in &lift.arcs {
                for k in 0..=8 {
                    let uv = arc.evaluate(k as f64 / 8.).unwrap().point;
                    assert!((-1e-9..=1. + 1e-9).contains(&uv[0]), "{uv:?}");
                    assert!((-1e-9..=1. + 1e-9).contains(&uv[1]), "{uv:?}");
                    let p = surface
                        .evaluate(uv[0].clamp(0., 1.), uv[1].clamp(0., 1.))
                        .unwrap()
                        .point;
                    worst = worst.max(check([p[0], p[1], p[2]]));
                }
            }
        }
        for lift in cylinder_uv {
            let surface = &cylinder_model.faces[lift.patch].surface;
            for arc in &lift.arcs {
                for k in 0..=8 {
                    let uv = arc.evaluate(k as f64 / 8.).unwrap().point;
                    assert!((-1e-9..=1. + 1e-9).contains(&uv[0]), "{uv:?}");
                    assert!((-1e-9..=1. + 1e-9).contains(&uv[1]), "{uv:?}");
                    let p = surface
                        .evaluate(uv[0].clamp(0., 1.), uv[1].clamp(0., 1.))
                        .unwrap()
                        .point;
                    worst = worst.max(check([p[0], p[1], p[2]]));
                }
            }
        }
        worst
    }

    #[test]
    fn perpendicular_full_circle_has_exact_lifts() {
        // Plane z = 4, patch [-3,3]^2; cylinder r = 2, z in [0, 8]: the exact
        // circle of radius 2 at mid-height, fully inside the patch.
        let plane = plane_patch([-3., -3., 4.], [6., 0., 0.], [0., 6., 0.]);
        let cylinder = crate::analytic::cylinder(2., 8.).unwrap();
        let report = intersect_plane_cylinder(&plane, &cylinder, Options::default()).unwrap();
        assert!(report.unresolved.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
        assert!(!report.permits_topology_change());
        let [
            PlaneCylinderComponent::Circle {
                curve,
                center,
                radius,
                normal,
                full,
                plane_uv,
                cylinder_uv,
                max_sample_residual,
            },
        ] = &report.components[..]
        else {
            panic!("expected one circle component: {report:?}")
        };
        assert!(*full);
        assert!((*radius - 2.).abs() <= 1e-12, "{radius}");
        assert!(
            sub(*center, [0., 0., 4.]).iter().all(|x| x.abs() <= 1e-12),
            "{center:?}"
        );
        assert_eq!(*normal, [0., 0., 1.]);
        // Four exact 90-degree arcs with the unit-circle weights.
        assert_eq!(
            curve.knots,
            vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.]
        );
        assert_eq!(curve.weights.len(), 9);
        assert!((curve.weights[1] - std::f64::consts::FRAC_1_SQRT_2).abs() <= 1e-15);
        assert!(*max_sample_residual <= 1e-12, "{max_sample_residual}");
        // Plane UV: the exact ellipse (a circle in an isotropic patch frame).
        assert_eq!(plane_uv[0].arcs.len(), 4);
        // Cylinder UV: iso-v rulings at mid-height on all four side patches.
        assert_eq!(cylinder_uv.len(), 4);
        for lift in cylinder_uv {
            assert_eq!(lift.arcs.len(), 1);
            let arc = &lift.arcs[0];
            assert_eq!(arc.degree, 1);
            assert_eq!(arc.control_points[0], vec![0., 0.5]);
            assert_eq!(arc.control_points[1], vec![1., 0.5]);
        }
        let worst = lift_worst(
            &plane,
            &cylinder,
            plane_uv,
            cylinder_uv,
            ([0., 0., 4.], [0., 0., 1.], 2.),
            ([0., 0., 1.], [-3., -3., 4.]),
        );
        assert!(worst <= 1e-9, "{worst}");
    }

    #[test]
    fn perpendicular_clipped_circle_has_exact_trims() {
        // Plane z = 4, patch x in [0, 3], y in [-3, 3]: the circle of radius 2
        // clipped to its x >= 0 half, swept [3pi/2, 5pi/2], endpoints
        // (0, -+2, 4).
        let plane = plane_patch([0., -3., 4.], [3., 0., 0.], [0., 6., 0.]);
        let cylinder = crate::analytic::cylinder(2., 8.).unwrap();
        let report = intersect_plane_cylinder(&plane, &cylinder, Options::default()).unwrap();
        assert!(report.unresolved.is_empty(), "{report:?}");
        let [
            PlaneCylinderComponent::Circle {
                curve,
                center,
                full,
                plane_uv,
                cylinder_uv,
                max_sample_residual,
                ..
            },
        ] = &report.components[..]
        else {
            panic!("expected one clipped circle: {report:?}")
        };
        assert!(!*full);
        assert!(
            sub(*center, [0., 0., 4.]).iter().all(|x| x.abs() <= 1e-12),
            "{center:?}"
        );
        assert_eq!(curve.knots, vec![0., 0., 0., 1., 1., 2., 2., 2.]);
        let start = point_of(&curve.evaluate(0.).unwrap().point);
        let end = point_of(&curve.evaluate(2.).unwrap().point);
        assert!(
            start[0].abs() <= 1e-12 && (start[1] + 2.).abs() <= 1e-12,
            "{start:?}"
        );
        assert!((start[2] - 4.).abs() <= 1e-12);
        assert!(
            end[0].abs() <= 1e-12 && (end[1] - 2.).abs() <= 1e-12,
            "{end:?}"
        );
        assert!((end[2] - 4.).abs() <= 1e-12);
        assert!(*max_sample_residual <= 1e-12, "{max_sample_residual}");
        // Plane UV: two arcs meeting at the phi = 0 seam (2/3, 1/2); the free
        // endpoints sit exactly on u = 0 at v = 1/6 and v = 5/6.
        assert_eq!(plane_uv[0].arcs.len(), 2);
        let mut edge_v: Vec<f64> = Vec::new();
        let mut seam: Vec<[f64; 2]> = Vec::new();
        for arc in &plane_uv[0].arcs {
            for endpoint in [&arc.control_points[0], &arc.control_points[2]] {
                if endpoint[0].abs() <= 1e-12 {
                    edge_v.push(endpoint[1]);
                } else {
                    seam.push([endpoint[0], endpoint[1]]);
                }
            }
        }
        edge_v.sort_by(f64::total_cmp);
        assert_eq!(edge_v.len(), 2, "{edge_v:?}");
        assert!((edge_v[0] - 1. / 6.).abs() <= 1e-12, "{edge_v:?}");
        assert!((edge_v[1] - 5. / 6.).abs() <= 1e-12, "{edge_v:?}");
        assert_eq!(seam.len(), 2, "{seam:?}");
        for p in &seam {
            assert!((p[0] - 2. / 3.).abs() <= 1e-12, "{p:?}");
            assert!((p[1] - 0.5).abs() <= 1e-12, "{p:?}");
        }
        // Cylinder UV: iso-v sub-u arcs at mid-height on the two side patches
        // the half circle sweeps (quadrants 3 and 0).
        assert_eq!(cylinder_uv.len(), 2, "{cylinder_uv:?}");
        for lift in cylinder_uv {
            for arc in &lift.arcs {
                assert_eq!(arc.degree, 1);
                assert!((arc.control_points[0][1] - 0.5).abs() <= 1e-12);
                assert!((arc.control_points[1][1] - 0.5).abs() <= 1e-12);
            }
        }
        let worst = lift_worst(
            &plane,
            &cylinder,
            plane_uv,
            cylinder_uv,
            ([0., 0., 4.], [0., 0., 1.], 2.),
            ([0., 0., 1.], [0., -3., 4.]),
        );
        assert!(worst <= 1e-9, "{worst}");
    }

    #[test]
    fn cap_plane_coincidence_and_beyond_classification() {
        let cylinder = crate::analytic::cylinder(2., 8.).unwrap();
        let at = |z: f64| plane_patch([-3., -3., z], [6., 0., 0.], [0., 6., 0.]);
        // Plane coincident with the top cap plane: a coincident cap/rim
        // region, never a guessed circle.
        let report = intersect_plane_cylinder(&at(8.), &cylinder, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::CoincidentTrim
        );
        // Provably beyond the cap: empty and resolved.
        for z in [9., 8. + 1e-9] {
            let report = intersect_plane_cylinder(&at(z), &cylinder, Options::default()).unwrap();
            assert!(
                report.components.is_empty() && report.unresolved.is_empty(),
                "{z} {report:?}"
            );
            assert_eq!(report.coverage, Coverage::NumericallyResolved);
        }
        // Just clear of the cap band below: a full circle near the rim.
        let report =
            intersect_plane_cylinder(&at(8. - 1e-9), &cylinder, Options::default()).unwrap();
        let [PlaneCylinderComponent::Circle { center, full, .. }] = &report.components[..] else {
            panic!("expected one circle: {report:?}")
        };
        assert!(*full);
        assert!((center[2] - (8. - 1e-9)).abs() <= 1e-12, "{center:?}");
    }

    #[test]
    fn parallel_plane_yields_two_exact_rulings() {
        // Plane x = 1, patch y in [-3, 3], z in [0, 10]: the two exact
        // rulings at (1, +-sqrt(3), z), z in [0, 8] (clipped by the caps).
        let plane = plane_patch([1., -3., 0.], [0., 6., 0.], [0., 0., 10.]);
        let cylinder = crate::analytic::cylinder(2., 8.).unwrap();
        let report = intersect_plane_cylinder(&plane, &cylinder, Options::default()).unwrap();
        assert!(report.unresolved.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
        assert_eq!(report.components.len(), 2, "{report:?}");
        let h = 3_f64.sqrt();
        let mut seen: Vec<f64> = Vec::new();
        for component in &report.components {
            let PlaneCylinderComponent::Line {
                curve,
                start,
                end,
                direction,
                contact,
                plane_uv,
                cylinder_uv,
                max_sample_residual,
            } = component
            else {
                panic!("expected line components: {report:?}")
            };
            assert_eq!(*direction, [0., 0., 1.]);
            assert_eq!(*contact, Contact::Boundary);
            assert!(
                (start[0] - 1.).abs() <= 1e-12 && start[2].abs() <= 1e-12,
                "{start:?}"
            );
            assert!(
                (end[0] - 1.).abs() <= 1e-12 && (end[2] - 8.).abs() <= 1e-12,
                "{end:?}"
            );
            assert!((start[1].abs() - h).abs() <= 1e-12, "{start:?}");
            assert_eq!(start[1], end[1]);
            seen.push(start[1]);
            assert_eq!(curve.degree, 1);
            assert_eq!(curve.knots, vec![0., 0., 1., 1.]);
            assert!(*max_sample_residual <= 1e-12, "{max_sample_residual}");
            // Plane UV: a degree-1 segment at constant u, v from 0 to 0.8.
            assert_eq!(plane_uv[0].arcs.len(), 1);
            let arc = &plane_uv[0].arcs[0];
            assert_eq!(arc.degree, 1);
            let u_const = (3. + start[1]) / 6.;
            assert!(
                (arc.control_points[0][0] - u_const).abs() <= 1e-12,
                "{arc:?}"
            );
            assert!(
                (arc.control_points[1][0] - u_const).abs() <= 1e-12,
                "{arc:?}"
            );
            assert!(arc.control_points[0][1].abs() <= 1e-12, "{arc:?}");
            assert!((arc.control_points[1][1] - 0.8).abs() <= 1e-12, "{arc:?}");
            assert!(!cylinder_uv.is_empty());
            let worst = lift_worst(
                &plane,
                &cylinder,
                plane_uv,
                cylinder_uv,
                ([0., 0., 4.], [0., 0., 1.], 2.),
                ([1., 0., 0.], [1., -3., 0.]),
            );
            assert!(worst <= 1e-9, "{worst}");
        }
        seen.sort_by(f64::total_cmp);
        assert!(seen[0] < 0. && seen[1] > 0., "{seen:?}");
    }

    #[test]
    fn parallel_miss_tangency_and_band_classification() {
        let cylinder = crate::analytic::cylinder(2., 8.).unwrap();
        let at = |x: f64| plane_patch([x, -3., 0.], [0., 6., 0.], [0., 0., 10.]);
        // Miss: d = 3 > r, empty and resolved.
        let report = intersect_plane_cylinder(&at(3.), &cylinder, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
        // Tangency and within the band around it: never a guessed line.
        for x in [2., 2. - 3e-14] {
            let report = intersect_plane_cylinder(&at(x), &cylinder, Options::default()).unwrap();
            assert!(report.components.is_empty(), "{x} {report:?}");
            assert_eq!(report.coverage, Coverage::Incomplete);
            assert_eq!(
                report.unresolved[0].reason,
                UnresolvedReason::TangencyOrMultipleRoot
            );
        }
        // Just clear of the band outside: empty resolved.
        let report =
            intersect_plane_cylinder(&at(2. + 1e-9), &cylinder, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        // Just clear inside: two rulings near the tangent line.
        let report =
            intersect_plane_cylinder(&at(2. - 1e-9), &cylinder, Options::default()).unwrap();
        assert_eq!(report.components.len(), 2, "{report:?}");
        let h = (4_f64 - (2. - 1e-9) * (2. - 1e-9)).sqrt();
        for component in &report.components {
            let PlaneCylinderComponent::Line { start, .. } = component else {
                panic!("expected lines: {report:?}")
            };
            // h^2 = r^2 - d^2 loses digits to cancellation: compare relative.
            assert!((start[1].abs() - h).abs() <= 1e-6 * h, "{start:?} vs {h}");
        }
    }

    #[test]
    fn parallel_boundary_line_is_a_tangency_region() {
        // Plane x = 1 with the patch edge exactly on y = sqrt(3): one ruling
        // runs along the patch boundary within the band (tangency region),
        // the other is resolved with exact trims.
        let plane = plane_patch([1., -3., 0.], [0., 3. + 3_f64.sqrt(), 0.], [0., 0., 10.]);
        let cylinder = crate::analytic::cylinder(2., 8.).unwrap();
        let report = intersect_plane_cylinder(&plane, &cylinder, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(report.unresolved.len(), 1, "{report:?}");
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
        let [PlaneCylinderComponent::Line { start, end, .. }] = &report.components[..] else {
            panic!("expected one resolved line: {report:?}")
        };
        let h = 3_f64.sqrt();
        assert!(
            (start[0] - 1.).abs() <= 1e-12 && (start[1] + h).abs() <= 1e-12,
            "{start:?}"
        );
        assert!(
            start[2].abs() <= 1e-12 && (end[2] - 8.).abs() <= 1e-12,
            "{end:?}"
        );
    }

    /// Oblique fixture: plane through the cylinder center (0, 0, 4) with
    /// normal (0, -sin(beta), cos(beta)), u along x, v in the yz-plane.
    fn oblique_plane(beta: f64, half_u: f64, half_v: f64) -> Model {
        let (s, c) = beta.sin_cos();
        plane_patch(
            [-half_u, -half_v * c, 4. - half_v * s],
            [2. * half_u, 0., 0.],
            [0., 2. * half_v * c, 2. * half_v * s],
        )
    }

    #[test]
    fn oblique_full_ellipse_matches_the_exact_oracle() {
        // beta = 30 degrees: semi-major 4/sqrt(3) along (0, cos, sin),
        // semi-minor 2 along (-1, 0, 0); the ellipse clears both caps and
        // the patch rectangle.
        let beta = std::f64::consts::PI / 6.;
        let (s, c) = beta.sin_cos();
        let plane = oblique_plane(beta, 6., 8.);
        let cylinder = crate::analytic::cylinder(2., 8.).unwrap();
        let report = intersect_plane_cylinder(&plane, &cylinder, Options::default()).unwrap();
        assert!(report.unresolved.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
        let [
            PlaneCylinderComponent::Ellipse {
                curve,
                center,
                semi_major,
                semi_minor,
                major,
                minor,
                normal,
                full,
                plane_uv,
                cylinder_uv,
                max_sample_residual,
            },
        ] = &report.components[..]
        else {
            panic!("expected one ellipse: {report:?}")
        };
        assert!(*full);
        assert!(cylinder_uv.is_none(), "cylinder side lift is out of scope");
        assert!((semi_major - 2. / c).abs() <= 1e-12, "{semi_major}");
        assert!((semi_minor - 2.).abs() <= 1e-12, "{semi_minor}");
        assert!(
            sub(*center, [0., 0., 4.]).iter().all(|x| x.abs() <= 1e-12),
            "{center:?}"
        );
        assert!(
            sub(*major, [0., c, s]).iter().all(|x| x.abs() <= 1e-12),
            "{major:?}"
        );
        assert!(
            sub(*minor, [-1., 0., 0.]).iter().all(|x| x.abs() <= 1e-12),
            "{minor:?}"
        );
        assert!(
            sub(*normal, [0., -s, c]).iter().all(|x| x.abs() <= 1e-12),
            "{normal:?}"
        );
        // phi = 0 is the major-axis endpoint (0, 2, 4 + 2/sqrt(3)).
        let p0 = point_of(&curve.evaluate(0.).unwrap().point);
        let oracle = [0., 2., 4. + 2. / 3_f64.sqrt()];
        assert!(sub(p0, oracle).iter().all(|x| x.abs() <= 1e-12), "{p0:?}");
        assert!(*max_sample_residual <= 1e-12, "{max_sample_residual}");
        let worst = lift_worst(
            &plane,
            &cylinder,
            plane_uv,
            &[],
            ([0., 0., 4.], [0., 0., 1.], 2.),
            ([0., -s, c], [-6., -8. * c, 4. - 8. * s]),
        );
        assert!(worst <= 1e-9, "{worst}");
    }

    #[test]
    fn oblique_clipped_ellipse_has_exact_cap_trims() {
        // beta = 75 degrees: the axial semi-extent 2*tan(beta) exceeds the
        // half height, so the ellipse is clipped by both cap planes into two
        // arcs whose endpoints sit exactly on z = 0 and z = 8.
        let beta = 5. * std::f64::consts::PI / 12.;
        let plane = oblique_plane(beta, 6., 8.);
        let cylinder = crate::analytic::cylinder(2., 8.).unwrap();
        let report = intersect_plane_cylinder(&plane, &cylinder, Options::default()).unwrap();
        assert!(report.unresolved.is_empty(), "{report:?}");
        assert_eq!(report.components.len(), 2, "{report:?}");
        let mut z_levels: Vec<f64> = Vec::new();
        for component in &report.components {
            let PlaneCylinderComponent::Ellipse {
                curve,
                full,
                cylinder_uv,
                max_sample_residual,
                ..
            } = component
            else {
                panic!("expected ellipse arcs: {report:?}")
            };
            assert!(!*full);
            assert!(cylinder_uv.is_none());
            assert!(*max_sample_residual <= 1e-12, "{max_sample_residual}");
            let [lo, hi] = curve.domain();
            let start = point_of(&curve.evaluate(lo).unwrap().point);
            let end = point_of(&curve.evaluate(hi).unwrap().point);
            for p in [start, end] {
                // Every trim endpoint lies on a cap plane.
                assert!(p[2].abs() <= 1e-12 || (p[2] - 8.).abs() <= 1e-12, "{p:?}");
                z_levels.push(p[2]);
                // ... and on the cylinder side.
                assert!((p[0].hypot(p[1]) - 2.).abs() <= 1e-12, "{p:?}");
            }
        }
        // One arc touches the top cap at both ends, the other the bottom.
        let tops = z_levels.iter().filter(|&&z| z > 4.).count();
        assert_eq!(tops, 2, "{z_levels:?}");
    }

    #[test]
    fn oblique_cap_tangency_stays_unresolved() {
        // tan(beta) = 2: the axial semi-extent equals the half height, so the
        // ellipse touches both cap planes exactly — a tangency region, never
        // a guessed ellipse.
        let beta = 2_f64.atan();
        let plane = oblique_plane(beta, 6., 8.);
        let cylinder = crate::analytic::cylinder(2., 8.).unwrap();
        let report = intersect_plane_cylinder(&plane, &cylinder, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
    }

    #[test]
    fn near_coincident_tilts_are_never_forced() {
        let cylinder = crate::analytic::cylinder(2., 8.).unwrap();
        // Recognition-scale tilt off perpendicular (about X by 1e-6).
        let a = 1e-6_f64;
        let (sa, ca) = a.sin_cos();
        let near_perp = plane_patch(
            [-3., -3. * ca, 4. - 3. * sa],
            [6., 0., 0.],
            [0., 6. * ca, 6. * sa],
        );
        let report = intersect_plane_cylinder(&near_perp, &cylinder, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::NearCoincidence
        );
        // Recognition-scale tilt off parallel: normal (cos b, 0, sin b) with
        // sin(b) between the rounding snap and the recognition scale.
        let b = 5e-10_f64;
        let (sb, cb) = b.sin_cos();
        let near_par = plane_patch(
            [cb, -3., sb - 5. * sb],
            [0., 6., 0.],
            [-10. * sb, 0., 10. * cb],
        );
        let report = intersect_plane_cylinder(&near_par, &cylinder, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::NearCoincidence
        );
    }

    #[test]
    fn noncanonical_operands_are_explicit_unsupported_regions() {
        let plane = plane_patch([-3., -3., 4.], [6., 0., 0.], [0., 6., 0.]);
        let cylinder = crate::analytic::cylinder(2., 8.).unwrap();
        for (a, b) in [
            (
                plane.clone(),
                crate::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap(),
            ),
            (plane.clone(), crate::analytic::sphere(2.).unwrap()),
            // Operand order is fixed: the cylinder as plane operand refuses.
            (cylinder.clone(), plane.clone()),
            (
                crate::cuboid([0., 0., 0.], [4., 4., 1.]).unwrap(),
                cylinder.clone(),
            ),
        ] {
            let report = intersect_plane_cylinder(&a, &b, Options::default()).unwrap();
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
            assert!(!report.permits_topology_change());
        }
        // A skewed (non-rectangular) affine patch is not canonical.
        let skewed = plane_patch([-3., -3., 4.], [6., 0., 0.], [3., 6., 0.]);
        let report = intersect_plane_cylinder(&skewed, &cylinder, Options::default()).unwrap();
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::UnsupportedSurface
        );
        // A structurally perturbed patch (non-unit weight) fails validation.
        let mut perturbed = plane.clone();
        perturbed.faces[0].surface.weights[0][0] = 2.;
        assert!(intersect_plane_cylinder(&perturbed, &cylinder, Options::default()).is_err());
        // A valid canonical pair still resolves.
        let report = intersect_plane_cylinder(&plane, &cylinder, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
        assert!(!report.components.is_empty(), "{report:?}");
    }

    #[test]
    fn rigid_placement_keeps_the_exact_ellipse() {
        let angle = 0.5;
        let offset = [0.3, -0.2, 1.1];
        let beta = std::f64::consts::PI / 6.;
        let (s, c) = beta.sin_cos();
        let plane = rotated_translated(&oblique_plane(beta, 6., 8.), angle, offset);
        let cylinder =
            rotated_translated(&crate::analytic::cylinder(2., 8.).unwrap(), angle, offset);
        let report = intersect_plane_cylinder(&plane, &cylinder, Options::default()).unwrap();
        assert!(report.unresolved.is_empty(), "{report:?}");
        let [
            PlaneCylinderComponent::Ellipse {
                curve,
                center,
                semi_major,
                semi_minor,
                major,
                minor,
                normal,
                full,
                plane_uv,
                cylinder_uv,
                max_sample_residual,
            },
        ] = &report.components[..]
        else {
            panic!("expected one ellipse: {report:?}")
        };
        assert!(*full);
        assert!(cylinder_uv.is_none());
        let (sa, ca) = angle.sin_cos();
        let placed = |p: [f64; 3]| {
            [
                p[0] + offset[0],
                ca * p[1] - sa * p[2] + offset[1],
                sa * p[1] + ca * p[2] + offset[2],
            ]
        };
        let placed_dir = |p: [f64; 3]| sub(placed(p), placed([0., 0., 0.]));
        assert!((semi_major - 2. / c).abs() <= 1e-12, "{semi_major}");
        assert!((semi_minor - 2.).abs() <= 1e-12, "{semi_minor}");
        assert!(
            sub(*center, placed([0., 0., 4.]))
                .iter()
                .all(|x| x.abs() <= 1e-12),
            "{center:?}"
        );
        assert!(
            sub(*major, placed_dir([0., c, s]))
                .iter()
                .all(|x| x.abs() <= 1e-12),
            "{major:?}"
        );
        assert!(
            sub(*minor, placed_dir([-1., 0., 0.]))
                .iter()
                .all(|x| x.abs() <= 1e-12),
            "{minor:?}"
        );
        assert!(
            sub(*normal, placed_dir([0., -s, c]))
                .iter()
                .all(|x| x.abs() <= 1e-12),
            "{normal:?}"
        );
        let p0 = point_of(&curve.evaluate(0.).unwrap().point);
        let oracle = placed([0., 2., 4. + 2. / 3_f64.sqrt()]);
        assert!(sub(p0, oracle).iter().all(|x| x.abs() <= 1e-12), "{p0:?}");
        assert!(*max_sample_residual <= 1e-12, "{max_sample_residual}");
        let worst = lift_worst(
            &plane,
            &cylinder,
            plane_uv,
            &[],
            (placed([0., 0., 4.]), placed_dir([0., 0., 1.]), 2.),
            (placed_dir([0., -s, c]), placed([-6., -8. * c, 4. - 8. * s])),
        );
        assert!(worst <= 1e-9, "{worst}");
    }
}
