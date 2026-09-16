//! Analytic cone/cone (frustum) intersection for canonical solids in the
//! coaxial configuration (axes parallel and coincident, either orientation).
//!
//! Both operands must be the exact canonical conical frustum of
//! `analytic::frustum` (four rational ruled side patches linearly
//! interpolated between the two radius rings, a bilinear cap per nonzero
//! ring, a true apex admitted when exactly one radius is zero; an
//! equal-radius frustum is a cylinder and is refused by the recognizer) —
//! recognized by `plane_cone::recognize_cone` under rigid affine placement.
//! Anything else is an explicit `UnsupportedSurface` region — this cell
//! never falls back to numerical surface/surface subdivision. Coaxiality is
//! certified, never forced: a pure-rounding axis tilt or perpendicular
//! offset snaps to the shared axis, a recognition-scale tilt or offset
//! reports a `NearCoincidence` region, and a clearly non-parallel or
//! off-axis pair is `UnsupportedSurface` (the general cone/cone pair is a
//! quartic, out of scope). Classification uses outward binary64 bands
//! widened by both recognition deviations.
//!
//! In the shared axial coordinate s measured from the first cone's bottom
//! ring center along its axis (the second cone oriented by the sign of the
//! axes' dot product, so apex-to-apex and base-to-base anti-axial pairs are
//! still coaxial), each side carries the linear radius profile rho_i(s)
//! between its rings. Side contacts solve the single linear equation
//! rho_1(s) == rho_2(s): with different tapers there is at most one height
//! s*, reported as the exact circle of radius rho(s*) when s* is certified
//! strictly inside both height ranges (a root clipped by either finite
//! height is honestly absent); a root within the band of any of the four
//! ring planes is a rim contact, and a root whose radius collapses into the
//! band is the apex-meeting contact — both stay `TangencyOrMultipleRoot`,
//! never a guessed circle. With equal tapers inside the band the profiles
//! are parallel: coincident over the overlapping height range reports
//! `CoincidentTrim` (never a surface component), clearly distinct profiles
//! resolve empty, and equality at a single ring plane only is the rim
//! tangency, unresolved. Any two ring planes (one per cone) coinciding
//! within the band carry a rim/rim, rim-on-cap or apex-on-cap boundary
//! contact and stay `TangencyOrMultipleRoot` independent of the side
//! classification. Resolved contacts are exact rational circles (four
//! 90-degree arcs, weights cos(pi/4)) with iso-v lifts on all four side
//! patches of both cones. Nothing here authorizes a topology change.
use super::plane_cone::{CanonicalCone, recognize_cone};
use super::sphere_cylinder::CylinderPatchCurve;
use super::sphere_sphere::{RECOGNITION, circle_curve};
use super::*;
use crate::Model;

#[derive(Clone, Debug)]
pub enum ConeConeComponent {
    /// Transverse side crossing: the exact circle plus both UV lifts.
    Circle {
        /// Exact rational full circle: degree 2, knots 0..=4, four 90-degree
        /// arcs with weights cos(pi/4).
        curve: Curve,
        center: [f64; 3],
        radius: f64,
        /// Unit shared axis (the first cone's axis, the circle plane normal).
        normal: [f64; 3],
        first_uv: Vec<CylinderPatchCurve>,
        second_uv: Vec<CylinderPatchCurve>,
        /// Worst residual over 16 circle samples against both cones' side
        /// profile equations.
        max_sample_residual: f64,
    },
}

/// The second cone's linear radius profile oriented into the shared axial
/// coordinate s (first cone's bottom ring center at s = 0, axis +axis).
struct OrientedProfile<'a> {
    cone: &'a CanonicalCone,
    /// Shared coordinate of this cone's own bottom ring center.
    origin: f64,
    /// +1 when its axis agrees with the shared axis, -1 when flipped.
    sign: f64,
}
impl OrientedProfile<'_> {
    fn lo(&self) -> f64 {
        if self.sign > 0. {
            self.origin
        } else {
            self.origin - self.cone.height
        }
    }
    fn hi(&self) -> f64 {
        if self.sign > 0. {
            self.origin + self.cone.height
        } else {
            self.origin
        }
    }
    /// Radius profile rho(s) in the shared coordinate; linear between rings.
    fn radius(&self, s: f64) -> f64 {
        self.cone.r_bottom + self.cone.slope * self.sign * (s - self.origin)
    }
    /// Side-patch v parameter of the shared height s (iso-v lift line).
    fn v(&self, s: f64) -> f64 {
        self.sign * (s - self.origin) / self.cone.height
    }
    /// Dimensionless taper of the radius profile in the shared coordinate.
    fn taper(&self) -> f64 {
        self.cone.slope * self.sign
    }
}

fn point_of(jet_point: &[f64]) -> [f64; 3] {
    [jet_point[0], jet_point[1], jet_point[2]]
}

/// Scaled Euclidean norm that never overflows or flushes subnormals.
fn norm3(v: [f64; 3]) -> f64 {
    let scale = v.iter().map(|x| x.abs()).fold(0., f64::max);
    if scale == 0. {
        return 0.;
    }
    let scaled = v.map(|x| x / scale);
    scaled[0].hypot(scaled[1]).hypot(scaled[2]) * scale
}

/// Exact circle component at the shared axial position `s`, with iso-v
/// lifts on all four side patches of both cones and a 16-sample residual
/// bound against both side profile equations.
fn circle_component(
    first: &CanonicalCone,
    profile: &OrientedProfile,
    s: f64,
) -> Result<ConeConeComponent> {
    let center = std::array::from_fn(|k| first.bottom[k] + s * first.axis[k]);
    let radius = first.r_bottom + first.slope * s;
    let iso_v = |v: f64| {
        vec![Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![[0., v].to_vec(), [1., v].to_vec()],
            weights: vec![1., 1.],
            periodic: false,
        }]
    };
    let first_uv = first
        .sides
        .iter()
        .map(|&patch| CylinderPatchCurve {
            patch,
            arcs: iso_v(s / first.height),
        })
        .collect();
    let second_uv = profile
        .cone
        .sides
        .iter()
        .map(|&patch| CylinderPatchCurve {
            patch,
            arcs: iso_v(profile.v(s)),
        })
        .collect();
    let curve = circle_curve(center, radius, first.frame[0], first.frame[1]);
    let second = profile.cone;
    let mut max_sample_residual = 0_f64;
    for i in 0..16 {
        let point = point_of(&curve.evaluate(i as f64 / 4.)?.point);
        let side_residual = |cone: &CanonicalCone| {
            let rel = sub(point, cone.bottom);
            let a = dot(rel, cone.axis);
            let perp = sub(rel, cone.axis.map(|x| x * a));
            (norm3(perp) - (cone.r_bottom + cone.slope * a)).abs()
        };
        max_sample_residual = max_sample_residual
            .max(side_residual(first))
            .max(side_residual(second));
    }
    Ok(ConeConeComponent::Circle {
        curve,
        center,
        radius,
        normal: first.axis,
        first_uv,
        second_uv,
        max_sample_residual,
    })
}

/// Analytic cone/cone intersection of two canonical conical frustum solids,
/// coaxial configuration only (either axis orientation). Non-canonical
/// operands and clearly non-parallel or off-axis pairs are explicit
/// unsupported regions, never a numerical fallback; recognition-scale tilts
/// and offsets report near_coincidence, ring-plane coincidences, rim
/// contacts, apex meetings, equal-taper profile coincidence and
/// ill-conditioned near-equal-taper crossings stay unresolved — tangent
/// contacts and coincident trims are never guessed into components.
pub fn intersect_cone_cone(
    first_model: &Model,
    second_model: &Model,
    options: Options,
) -> Result<Report<ConeConeComponent>> {
    let options = options.validate()?;
    let _ = options;
    let mut report = Report::default();
    let domain = vec![0., 1., 0., 1., 0., 1., 0., 1.];
    let (Some(first), Some(second)) = (recognize_cone(first_model)?, recognize_cone(second_model)?)
    else {
        report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    };
    report.boxes_visited = 1;
    let offset = sub(second.bottom, first.bottom);
    let s0 = dot(offset, first.axis);
    let terms = s0.abs()
        + first.height
        + second.height
        + first.r_bottom
        + first.r_top
        + second.r_bottom
        + second.r_top
        + 1.;
    // Outward binary64 classification band: both recognition deviations plus
    // a rounding allowance on the axial coordinates and radii.
    let band = first.error + second.error + 16. * f64::EPSILON * terms;
    let scale = first.height
        + second.height
        + first.r_bottom
        + first.r_top
        + second.r_bottom
        + second.r_top;
    // Parallelism is certified on the cross-product magnitude (~ sin of the
    // axis angle), never forced: pure-rounding tilts snap, recognition-scale
    // tilts report near_coincidence, larger tilts are unsupported.
    let cross = [
        first.axis[1] * second.axis[2] - first.axis[2] * second.axis[1],
        first.axis[2] * second.axis[0] - first.axis[0] * second.axis[2],
        first.axis[0] * second.axis[1] - first.axis[1] * second.axis[0],
    ];
    let tilt = norm3(cross);
    let angle_snap = 64. * f64::EPSILON * 8.;
    if tilt > angle_snap {
        let reason = if tilt <= RECOGNITION {
            UnresolvedReason::NearCoincidence
        } else {
            UnresolvedReason::UnsupportedSurface
        };
        report.unresolved(domain, reason);
        return Ok(report);
    }
    // Axiality of the two axis lines: perpendicular offset of the second
    // cone's bottom ring center from the first cone's axis line.
    let perp = sub(offset, first.axis.map(|x| x * s0));
    let d_perp = norm3(perp);
    let snap = 64. * f64::EPSILON * terms;
    if d_perp > snap {
        let reason = if d_perp <= RECOGNITION * scale {
            UnresolvedReason::NearCoincidence
        } else {
            UnresolvedReason::UnsupportedSurface
        };
        report.unresolved(domain, reason);
        return Ok(report);
    }
    let sign = if dot(first.axis, second.axis) >= 0. {
        1.
    } else {
        -1.
    };
    let profile = OrientedProfile {
        cone: &second,
        origin: s0,
        sign,
    };
    let lo = 0_f64.max(profile.lo());
    let hi = first.height.min(profile.hi());
    // Ring-plane coincidences (rim/rim, rim-on-cap, apex-on-cap): coaxial
    // disks at the same plane always share at least the smaller disk — a
    // tangent boundary contact, never a guessed circle. Independent of the
    // side classification below.
    let mut tangency = false;
    for p in [0., first.height] {
        for q in [profile.lo(), profile.hi()] {
            if (p - q).abs() <= band {
                tangency = true;
            }
        }
    }
    // Clearly separated axially: no shared point is possible.
    if hi < lo - band {
        if tangency {
            report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        }
        return Ok(report);
    }
    // Overlap collapsed into the band: the facing ring planes touch.
    if hi - lo <= band {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    let d = |s: f64| (first.r_bottom + first.slope * s) - profile.radius(s);
    let d_lo = d(lo);
    let d_hi = d(hi);
    // Equal-taper test: the taper difference is dimensionless, so the
    // rounding allowance scales with the tapers and the recognition
    // deviations enter through the heights they were measured over.
    let slope_band = 16. * f64::EPSILON * (first.slope.abs() + second.slope.abs() + 1.)
        + (first.error + second.error) / first.height.min(second.height);
    let dm = first.slope - profile.taper();
    if dm.abs() <= slope_band {
        // Parallel profiles: d is constant up to the band over the overlap.
        if d_lo.abs() <= band && d_hi.abs() <= band {
            // Coincident side profiles over the whole overlap: a coincident
            // trim, never a surface component.
            report.unresolved(domain, UnresolvedReason::CoincidentTrim);
        } else if d_lo.abs() <= band || d_hi.abs() <= band || d_lo.signum() != d_hi.signum() {
            // Equality at a single ring plane only (rim tangency), or a
            // crossing whose position is ill-conditioned at equal taper.
            report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        } else if tangency {
            // Strictly separated parallel profiles, but a ring-plane
            // coincidence carries its own boundary contact.
            report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        }
        // Otherwise strictly separated parallel profiles resolve empty.
        return Ok(report);
    }
    // Different tapers: at most one side root s* = lo + d_lo (hi-lo)/(d_lo-d_hi).
    if d_lo.signum() == d_hi.signum() && d_lo.abs() > band && d_hi.abs() > band {
        // Strict same sign at both overlap ends: the linear root is clipped
        // by a finite height — honestly absent.
        if tangency {
            report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        }
        return Ok(report);
    }
    if d_lo.abs() <= band || d_hi.abs() <= band {
        // The root sits within the band of an overlap boundary — every
        // overlap boundary is a ring plane: a rim contact, unresolved.
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    let s = lo + d_lo * (hi - lo) / (d_lo - d_hi);
    // A root within the band of any of the four ring planes is a rim
    // contact; a root whose radius collapses into the band is the
    // apex-meeting contact — both stay unresolved.
    if [0., first.height, profile.lo(), profile.hi()]
        .iter()
        .any(|&p| (s - p).abs() <= band)
    {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    let radius = first.r_bottom + first.slope * s;
    if radius <= band {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    report
        .components
        .push(circle_component(&first, &profile, s)?);
    if tangency {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
    }
    Ok(report)
}

impl value_codec::Serialize for ConeConeComponent {
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
        report: &Report<ConeConeComponent>,
        count: usize,
    ) -> &Report<ConeConeComponent> {
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        assert!(report.unresolved.is_empty(), "{report:?}");
        assert!(!report.permits_topology_change());
        assert_eq!(report.components.len(), count, "{report:?}");
        report
    }
    #[allow(clippy::type_complexity)]
    fn circle_of(
        component: &ConeConeComponent,
    ) -> (
        &Curve,
        [f64; 3],
        f64,
        [f64; 3],
        &[CylinderPatchCurve],
        &[CylinderPatchCurve],
        f64,
    ) {
        let ConeConeComponent::Circle {
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
    /// Cone side profile residual of a canonical z-up cone.
    fn side_residual_z(point: [f64; 3], r_bottom: f64, slope: f64) -> f64 {
        (point[0].hypot(point[1]) - (r_bottom + slope * point[2])).abs()
    }
    /// Worst residual of lifted UV arcs evaluated through their own surface
    /// against both cones' side predicates.
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
    fn different_tapers_cross_in_one_exact_circle() {
        // Cone1 r 1 -> 3 over z 0..6 (rho = 1 + s/3); cone2 r 4 -> 2 over
        // z 1..5 (rho = 4.5 - s/2). Root: (5/6) s = 3.5 — s* = 4.2 strictly
        // inside both ranges, radius 2.4, v = 0.7 on cone1 and 0.8 on cone2.
        let first = crate::analytic::frustum(1., 3., 6.).unwrap();
        let second = translated(&crate::analytic::frustum(4., 2., 4.).unwrap(), [0., 0., 1.]);
        let report = intersect_cone_cone(&first, &second, Options::default()).unwrap();
        let report = only_circles(&report, 1);
        let (curve, center, radius, normal, first_uv, second_uv, sampled) =
            circle_of(&report.components[0]);
        assert!((radius - 2.4).abs() <= 1e-12, "{radius}");
        assert!(
            sub(center, [0., 0., 4.2]).iter().all(|x| x.abs() <= 1e-12),
            "{center:?}"
        );
        assert!(normal[2] >= 1. - 1e-12, "{normal:?}");
        // Exact rational circle: four 90-degree arcs, weights cos(pi/4).
        assert_eq!(curve.degree, 2);
        assert_eq!(
            curve.knots,
            vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.]
        );
        assert_eq!(curve.control_points.len(), 9);
        // Sixteen samples satisfy both implicit side equations.
        let mut worst = 0_f64;
        for i in 0..16 {
            let p = point_of(&curve.evaluate(i as f64 / 4.).unwrap().point);
            worst = worst
                .max(side_residual_z(p, 1., 1. / 3.))
                .max((p[0].hypot(p[1]) - (4.5 - p[2] / 2.)).abs())
                .max((p[2] - 4.2).abs());
        }
        assert!(worst <= 1e-12, "{worst}");
        assert!(sampled <= 1e-12, "{sampled}");
        // Iso-v lifts on all four side patches of both cones.
        assert_eq!(first_uv.len(), 4);
        assert_eq!(second_uv.len(), 4);
        for lift in first_uv {
            assert_eq!(lift.arcs.len(), 1);
            assert!((lift.arcs[0].control_points[0][1] - 0.7).abs() <= 1e-12);
        }
        for lift in second_uv {
            assert_eq!(lift.arcs.len(), 1);
            assert!((lift.arcs[0].control_points[0][1] - 0.8).abs() <= 1e-12);
        }
        let uv_worst = uv_samples(&first, &second, first_uv, second_uv, |p| {
            side_residual_z(p, 1., 1. / 3.).max((p[0].hypot(p[1]) - (4.5 - p[2] / 2.)).abs())
        });
        assert!(uv_worst <= 1e-9, "{uv_worst}");
    }

    #[test]
    fn root_clipped_by_a_finite_height_resolves_empty() {
        // Cone1 r 3 -> 1 over z 0..6 (rho = 3 - s/3); cone2 r 4 -> 2 over
        // z 2..4 (rho = 6 - s). The linear root s* = 4.5 lies beyond cone2's
        // top ring: honestly absent, empty and resolved.
        let first = crate::analytic::frustum(3., 1., 6.).unwrap();
        let second = translated(&crate::analytic::frustum(4., 2., 2.).unwrap(), [0., 0., 2.]);
        let report = intersect_cone_cone(&first, &second, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
    }

    #[test]
    fn clearly_separated_axially_resolves_empty() {
        // Cone2 provably above cone1 with a 0.5 gap between the facing ring
        // planes: no shared point is possible in any radius relation.
        let first = crate::analytic::frustum(1., 3., 6.).unwrap();
        let second = translated(
            &crate::analytic::frustum(1., 2., 3.).unwrap(),
            [0., 0., 6.5],
        );
        let report = intersect_cone_cone(&first, &second, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
    }

    #[test]
    fn equal_taper_coincident_profiles_report_coincident_trim() {
        // Cone1 rho = 1 + s/3 over z 0..6; cone2 r 5/3 -> 11/3 over z 2..8
        // carries the same profile over the overlap z 2..6: coincident_trim,
        // never a surface component.
        let first = crate::analytic::frustum(1., 3., 6.).unwrap();
        let second = translated(
            &crate::analytic::frustum(5. / 3., 11. / 3., 6.).unwrap(),
            [0., 0., 2.],
        );
        let report = intersect_cone_cone(&first, &second, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(report.unresolved.len(), 1);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::CoincidentTrim
        );
        assert!(!report.permits_topology_change());
    }

    #[test]
    fn equal_taper_distinct_profiles_resolve_empty() {
        // Same taper 1/3, profiles 1/3 apart over the overlap z 2..6: one
        // side strictly inside the other, no contact, resolved empty.
        let first = crate::analytic::frustum(1., 3., 6.).unwrap();
        let second = translated(&crate::analytic::frustum(2., 4., 6.).unwrap(), [0., 0., 2.]);
        let report = intersect_cone_cone(&first, &second, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
    }

    #[test]
    fn stacked_rim_plane_contact_stays_unresolved() {
        // Cone2 stacked base-to-top on cone1 with equal ring radii 3 at the
        // shared plane z=6: the coincident ring planes carry a rim/rim
        // tangent boundary contact — tangency_or_multiple_root, never a
        // guessed circle.
        let first = crate::analytic::frustum(1., 3., 6.).unwrap();
        let second = translated(&crate::analytic::frustum(3., 5., 6.).unwrap(), [0., 0., 6.]);
        let report = intersect_cone_cone(&first, &second, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
        // Different ring radii at the shared plane (rim circle inside the
        // larger cap disk) are still a boundary contact: unresolved.
        let nested = translated(&crate::analytic::frustum(2., 4., 6.).unwrap(), [0., 0., 6.]);
        let report = intersect_cone_cone(&first, &nested, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
    }

    #[test]
    fn rim_on_side_contact_stays_unresolved() {
        // Cone2's bottom ring (radius 2 at z=3) lies exactly on cone1's side
        // (rho(3) = 2 for r 1 -> 3 over z 0..6): with the steeper taper 1/2
        // the side root sits on the overlap boundary — a rim contact, never
        // a guessed circle.
        let first = crate::analytic::frustum(1., 3., 6.).unwrap();
        let second = translated(&crate::analytic::frustum(2., 5., 6.).unwrap(), [0., 0., 3.]);
        let report = intersect_cone_cone(&first, &second, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
    }

    #[test]
    fn apex_meeting_stays_unresolved() {
        // Two true-apex cones apex-to-apex at the origin (cone2 flipped
        // down, anti-axial): the touching ring planes carry the apex contact
        // — tangency_or_multiple_root, never a point or circle.
        let first = crate::analytic::frustum(0., 3., 5.).unwrap();
        let second = flipped(&crate::analytic::frustum(0., 2., 4.).unwrap(), 0.);
        let report = intersect_cone_cone(&first, &second, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
    }

    #[test]
    fn anti_axial_pair_yields_the_exact_circle() {
        // Cone1 r 1 -> 3 over z 0..6 (rho = 1 + s/3); cone2 r 2 -> 4 over
        // its own height flipped down (rotation pi about X, then z -> 8 - z):
        // axis -z, own bottom ring (r=2) at z=8, rho = 2 + (8 - s)/3 over
        // s in [2,8]. Root: 2s/3 = 11/3 — s* = 5.5 strictly inside both
        // ranges, radius 17/6, v = 11/12 on cone1 and 5/12 on cone2.
        let first = crate::analytic::frustum(1., 3., 6.).unwrap();
        let second = flipped(&crate::analytic::frustum(2., 4., 6.).unwrap(), 8.);
        let report = intersect_cone_cone(&first, &second, Options::default()).unwrap();
        let report = only_circles(&report, 1);
        let (curve, center, radius, normal, first_uv, second_uv, sampled) =
            circle_of(&report.components[0]);
        assert!((radius - 17. / 6.).abs() <= 1e-12, "{radius}");
        assert!(
            sub(center, [0., 0., 5.5]).iter().all(|x| x.abs() <= 1e-12),
            "{center:?}"
        );
        assert!(normal[2] >= 1. - 1e-12, "{normal:?}");
        let mut worst = 0_f64;
        for i in 0..16 {
            let p = point_of(&curve.evaluate(i as f64 / 4.).unwrap().point);
            worst = worst
                .max(side_residual_z(p, 1., 1. / 3.))
                .max((p[0].hypot(p[1]) - (2. + (8. - p[2]) / 3.)).abs());
        }
        assert!(worst <= 1e-12, "{worst}");
        assert!(sampled <= 1e-12);
        for lift in first_uv {
            assert!((lift.arcs[0].control_points[0][1] - 11. / 12.).abs() <= 1e-12);
        }
        for lift in second_uv {
            assert!((lift.arcs[0].control_points[0][1] - 5. / 12.).abs() <= 1e-12);
        }
        let uv_worst = uv_samples(&first, &second, first_uv, second_uv, |p| {
            side_residual_z(p, 1., 1. / 3.).max((p[0].hypot(p[1]) - (2. + (8. - p[2]) / 3.)).abs())
        });
        assert!(uv_worst <= 1e-9, "{uv_worst}");
    }

    #[test]
    fn near_coaxial_bands_stay_unresolved() {
        let first = crate::analytic::frustum(1., 3., 6.).unwrap();
        // Recognition-scale perpendicular offset: near_coincidence, never
        // forced coaxial.
        let offset = translated(
            &crate::analytic::frustum(4., 2., 4.).unwrap(),
            [1e-10, 0., 1.],
        );
        let report = intersect_cone_cone(&first, &offset, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::NearCoincidence
        );
        // Recognition-scale tilt: near_coincidence as well.
        let tilted = rotated_translated(
            &crate::analytic::frustum(4., 2., 4.).unwrap(),
            1e-10,
            [0., 0., 1.],
        );
        let report = intersect_cone_cone(&first, &tilted, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::NearCoincidence
        );
        // Pure-rounding offsets snap: 1e-14 stays coaxial and resolves the
        // same circle as the exact pair.
        let snapped = translated(
            &crate::analytic::frustum(4., 2., 4.).unwrap(),
            [1e-14, 0., 1.],
        );
        let report = intersect_cone_cone(&first, &snapped, Options::default()).unwrap();
        only_circles(&report, 1);
    }

    #[test]
    fn off_axis_and_non_parallel_pairs_are_unsupported_regions() {
        let first = crate::analytic::frustum(1., 3., 6.).unwrap();
        // Parallel but 0.5 off the axis: coaxial quartic, out of scope.
        let off = translated(
            &crate::analytic::frustum(4., 2., 4.).unwrap(),
            [0.5, 0., 1.],
        );
        // Tilted 0.5 rad: the general quartic, out of scope.
        let tilted = rotated_translated(
            &crate::analytic::frustum(4., 2., 4.).unwrap(),
            0.5,
            [0., 0., 1.],
        );
        for second in [&off, &tilted] {
            let report = intersect_cone_cone(&first, second, Options::default()).unwrap();
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
        let cone = crate::analytic::frustum(1., 3., 6.).unwrap();
        // Equal-radius frusta are cylinders (refused as cone operands), and
        // spheres, tubes, cuboids and tori are not the canonical cone.
        for (a, b) in [
            (
                crate::analytic::frustum(1., 3., 6.).unwrap(),
                crate::analytic::cylinder(1., 3.).unwrap(),
            ),
            (
                crate::analytic::frustum(1., 3., 6.).unwrap(),
                crate::analytic::sphere(2.).unwrap(),
            ),
            (
                crate::analytic::tube(2., 1., 3.).unwrap(),
                crate::analytic::frustum(1., 3., 6.).unwrap(),
            ),
            (
                crate::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap(),
                crate::analytic::frustum(1., 3., 6.).unwrap(),
            ),
            (
                crate::analytic::torus(3., 1.).unwrap(),
                crate::analytic::frustum(1., 3., 6.).unwrap(),
            ),
        ] {
            let report = intersect_cone_cone(&a, &b, Options::default()).unwrap();
            assert!(report.components.is_empty(), "{report:?}");
            assert_eq!(report.coverage, Coverage::Incomplete);
            assert_eq!(report.unresolved.len(), 1);
            assert_eq!(
                report.unresolved[0].reason,
                UnresolvedReason::UnsupportedSurface
            );
        }
        // A structurally perturbed cone fails validation as a hard error.
        let mut perturbed = crate::analytic::frustum(1., 2., 3.).unwrap();
        perturbed.faces[0].surface.weights[1][0] = 0.5;
        assert!(intersect_cone_cone(&cone, &perturbed, Options::default()).is_err());
    }

    #[test]
    fn rotated_translated_placement_keeps_the_exact_circle() {
        // The crossing pair of the first test rigidly placed about X by 0.5.
        let angle = 0.5;
        let offset = [0.3, -0.2, 1.1];
        let first = rotated_translated(
            &crate::analytic::frustum(1., 3., 6.).unwrap(),
            angle,
            offset,
        );
        let second = rotated_translated(
            &translated(&crate::analytic::frustum(4., 2., 4.).unwrap(), [0., 0., 1.]),
            angle,
            offset,
        );
        let report = intersect_cone_cone(&first, &second, Options::default()).unwrap();
        let report = only_circles(&report, 1);
        let (curve, center, radius, normal, first_uv, second_uv, sampled) =
            circle_of(&report.components[0]);
        // Independent binary64 oracle in the placed frame.
        let (sin, cos) = angle.sin_cos();
        let placed = |p: [f64; 3]| {
            [
                p[0] + offset[0],
                cos * p[1] - sin * p[2] + offset[1],
                sin * p[1] + cos * p[2] + offset[2],
            ]
        };
        let expected = placed([0., 0., 4.2]);
        let axis = sub(placed([0., 0., 1.]), placed([0., 0., 0.]));
        assert!((radius - 2.4).abs() <= 1e-12, "{radius}");
        assert!(
            sub(center, expected).iter().all(|x| x.abs() <= 1e-12),
            "{center:?}"
        );
        assert!(
            sub(normal, axis).iter().all(|x| x.abs() <= 1e-12),
            "{normal:?}"
        );
        let side = |p: [f64; 3], origin: [f64; 3], r_bottom: f64, slope: f64| {
            let rel = sub(p, origin);
            let a = dot(rel, axis);
            let perp = sub(rel, axis.map(|x| x * a));
            (norm3(perp) - (r_bottom + slope * a)).abs()
        };
        let origin1 = placed([0., 0., 0.]);
        let origin2 = placed([0., 0., 1.]);
        let mut worst = 0_f64;
        for i in 0..16 {
            let p = point_of(&curve.evaluate(i as f64 / 4.).unwrap().point);
            worst = worst
                .max(side(p, origin1, 1., 1. / 3.))
                .max(side(p, origin2, 4., -0.5));
        }
        assert!(worst <= 1e-12, "{worst}");
        assert!(sampled <= 1e-12);
        assert_eq!(first_uv.len(), 4);
        assert_eq!(second_uv.len(), 4);
        let uv_worst = uv_samples(&first, &second, first_uv, second_uv, |p| {
            side(p, origin1, 1., 1. / 3.).max(side(p, origin2, 4., -0.5))
        });
        assert!(uv_worst <= 1e-9, "{uv_worst}");
    }
}
