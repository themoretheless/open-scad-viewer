//! Analytic sphere/cone (frustum) intersection for canonical solids in the
//! axial configuration (sphere center certified on the cone axis).
//!
//! The sphere operand must be the exact canonical stereographic sphere of
//! `analytic::sphere` (recognized by `sphere_sphere::recognize`); the cone
//! operand must be the exact canonical conical frustum of `analytic::frustum`
//! (four rational ruled side patches linearly interpolated between the two
//! radius rings, a bilinear cap per nonzero ring, a true apex admitted when
//! exactly one radius is zero; an equal-radius frustum is a cylinder and is
//! refused by the recognizer) — recognized by `plane_cone::recognize_cone`
//! under rigid affine placement. Anything else is an explicit
//! `UnsupportedSurface` region — this cell never falls back to numerical
//! surface/surface subdivision. Axiality is certified, never forced: a
//! perpendicular offset at pure rounding scale snaps to the axis, an offset
//! inside the recognition-scale band reports a `NearCoincidence` region, and
//! a clearly off-axis pair is `UnsupportedSurface` (the general sphere/cone
//! pair is a quartic, out of scope). Classification uses outward binary64
//! bands widened by both recognition deviations.
//!
//! In the axial coordinate t measured from the bottom ring center (sphere
//! center at t = s0), the cone side carries the linear radius profile
//! rho(t) = r_bottom + slope*t, so side contacts solve
//! (t - s0)^2 + rho(t)^2 = r^2 — one quadratic in the offset q = t - s0:
//! (1+m^2) q^2 + 2 rho_c m q + (rho_c^2 - r^2) = 0 with m = slope and
//! rho_c = rho(s0). Its discriminant is 4 (r^2 (1+m^2) - rho_c^2), i.e. the
//! side line passes the sphere center at distance |rho_c|/sqrt(1+m^2); a
//! sphere within the band of that distance is tangent to the side and stays
//! `TangencyOrMultipleRoot` (double root), never a guessed circle. Two
//! distinct roots clipped to the finite height (0, h) give the exact circles
//! of radius rho(t); a root within the band of a ring plane is the rim
//! contact and stays unresolved, and a root whose radius collapses into the
//! band is the apex contact (the sphere passes through the singular apex
//! point) and stays unresolved. Cap contacts mirror the sphere/cylinder
//! cell: a sphere crossing a ring cap plane inside the disk yields the exact
//! circle in that cap plane, a cap-plane touch or a cap circle whose radius
//! meets the ring radius (rim equality) stays `TangencyOrMultipleRoot`.
//! Provable misses (sphere axially beyond the solid, side line unreachable,
//! cap circles provably outside the disks) resolve empty, certified by the
//! interval signs. There is no sphere/cone surface coincidence, so
//! `CoincidentTrim` does not arise; every degeneracy reported here is an
//! explicit tangency region. Resolved contacts are exact rational circles
//! (four 90-degree arcs, weights cos(pi/4)) with UV lifts on both surfaces:
//! iso-v lines across all four cone side patches, an exact UV circle on a
//! cap face, and per-patch UV circles/lines on the sphere. Nothing here
//! authorizes a topology change.
use super::plane_cone::{CanonicalCone, recognize_cone};
use super::sphere_cylinder::CylinderPatchCurve;
#[cfg(test)]
use super::sphere_sphere::ARC_WEIGHT;
use super::sphere_sphere::{
    self, CanonicalSphere, RECOGNITION, SpherePatchCircle, circle_arcs, circle_curve, lift,
};
use super::*;
use crate::Model;

const TAU: f64 = std::f64::consts::TAU;

#[derive(Clone, Debug)]
pub enum SphereConeComponent {
    /// Transverse intersection: the exact circle plus both UV lifts.
    Circle {
        /// Exact rational full circle: degree 2, knots 0..=4, four 90-degree
        /// arcs with weights cos(pi/4).
        curve: Curve,
        center: [f64; 3],
        radius: f64,
        /// Unit cone axis (the circle plane normal).
        normal: [f64; 3],
        sphere_uv: Vec<SpherePatchCircle>,
        cone_uv: Vec<CylinderPatchCurve>,
        /// Worst residual over 16 circle samples against the sphere equation
        /// and the cone side profile or cap plane equation.
        max_sample_residual: f64,
    },
}

/// Where a resolved circle sits on the cone boundary.
#[derive(Clone, Copy)]
enum CircleSite {
    /// On the side wall: radius is the linearly interpolated rho(axial).
    Side,
    /// On a ring cap disk (slot 0 bottom, slot 1 top) with the circle radius.
    Cap(usize, f64),
}

fn point_of(jet_point: &[f64]) -> [f64; 3] {
    [jet_point[0], jet_point[1], jet_point[2]]
}

/// Exact circle component at axial position `axial` relative to the bottom
/// ring center, with UV lifts on both surfaces and a 16-sample residual
/// bound.
fn circle_component(
    sphere: &CanonicalSphere,
    cone: &CanonicalCone,
    axial: f64,
    site: CircleSite,
) -> Result<SphereConeComponent> {
    let center = std::array::from_fn(|k| cone.bottom[k] + axial * cone.axis[k]);
    let (radius, cone_uv) = match site {
        CircleSite::Side => {
            // Side patches: u sweeps the quadrant, v runs bottom ring to top
            // ring, so the circle is the iso-v line v = axial / h on each.
            let v0 = axial / cone.height;
            let radius = cone.r_bottom + cone.slope * axial;
            let arcs = vec![Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                control_points: vec![[0., v0].to_vec(), [1., v0].to_vec()],
                weights: vec![1., 1.],
                periodic: false,
            }];
            (
                radius,
                cone.sides
                    .iter()
                    .map(|&patch| CylinderPatchCurve {
                        patch,
                        arcs: arcs.clone(),
                    })
                    .collect::<Vec<_>>(),
            )
        }
        CircleSite::Cap(slot, rho) => {
            let face = cone.caps[slot].expect("cap circle needs a cap face");
            let ring_r = if slot == 0 { cone.r_bottom } else { cone.r_top };
            // Cap UV maps the plane as [1/2 + x/(2R), 1/2 + y/(2R)] at that
            // ring's radius: an exact UV circle of radius rho/(2R).
            (
                rho,
                vec![CylinderPatchCurve {
                    patch: face,
                    arcs: circle_arcs([0.5, 0.5], rho / (2. * ring_r), 0., TAU),
                }],
            )
        }
    };
    let curve = circle_curve(center, radius, cone.frame[0], cone.frame[1]);
    let sphere_uv = lift(sphere, cone.axis, center);
    let mut max_sample_residual = 0_f64;
    for i in 0..16 {
        let point = point_of(&curve.evaluate(i as f64 / 4.)?.point);
        let d = sub(point, sphere.center);
        let sphere_residual = (d[0].hypot(d[1]).hypot(d[2]) - sphere.radius).abs();
        let other = match site {
            CircleSite::Side => {
                let rel = sub(point, cone.bottom);
                let a = dot(rel, cone.axis);
                let perp = sub(rel, cone.axis.map(|x| x * a));
                (perp[0].hypot(perp[1]).hypot(perp[2]) - (cone.r_bottom + cone.slope * a)).abs()
            }
            CircleSite::Cap(..) => (dot(sub(point, cone.bottom), cone.axis) - axial).abs(),
        };
        max_sample_residual = max_sample_residual.max(sphere_residual).max(other);
    }
    Ok(SphereConeComponent::Circle {
        curve,
        center,
        radius,
        normal: cone.axis,
        sphere_uv,
        cone_uv,
        max_sample_residual,
    })
}

/// Analytic sphere/cone intersection of a canonical sphere solid and a
/// canonical conical frustum solid, axial configuration only. Non-canonical
/// operands and clearly off-axis pairs are explicit unsupported regions,
/// never a numerical fallback; near-axial offsets, the side tangency
/// (double-root discriminant), rim contacts, apex contacts and cap-plane
/// tangencies stay unresolved — tangent contacts are never guessed.
pub fn intersect_sphere_cone(
    sphere_model: &Model,
    cone_model: &Model,
    options: Options,
) -> Result<Report<SphereConeComponent>> {
    let options = options.validate()?;
    let _ = options;
    let mut report = Report::default();
    let domain = vec![0., 1., 0., 1., 0., 1., 0., 1.];
    let (Some(sphere), Some(cone)) = (
        sphere_sphere::recognize(sphere_model)?,
        recognize_cone(cone_model)?,
    ) else {
        report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    };
    report.boxes_visited = 1;
    let offset = sub(sphere.center, cone.bottom);
    let s0 = dot(offset, cone.axis);
    let perp = sub(offset, cone.axis.map(|x| x * s0));
    let pscale = perp.iter().map(|v| v.abs()).fold(0., f64::max);
    let d_perp = if pscale == 0. {
        0.
    } else {
        let scaled = perp.map(|x| x / pscale);
        scaled[0].hypot(scaled[1]).hypot(scaled[2]) * pscale
    };
    let terms = s0.abs() + cone.height + sphere.radius + cone.r_bottom + cone.r_top + 1.;
    // Outward binary64 classification band: both recognition deviations plus
    // a rounding allowance on the axial coordinates and radii.
    let band = sphere.error + cone.error + 16. * f64::EPSILON * terms;
    // Axiality is certified, never forced: pure-rounding offsets snap to the
    // axis, recognition-scale offsets report near_coincidence, and anything
    // larger is the unsupported general (quartic) configuration.
    let snap = 64. * f64::EPSILON * terms;
    if d_perp > snap {
        let near = RECOGNITION * (sphere.radius + cone.r_bottom + cone.r_top + cone.height);
        let reason = if d_perp <= near {
            UnresolvedReason::NearCoincidence
        } else {
            UnresolvedReason::UnsupportedSurface
        };
        report.unresolved(domain, reason);
        return Ok(report);
    }
    let r = sphere.radius;
    let h = cone.height;
    let m = cone.slope;
    // Sphere provably beyond the whole axial extent: no contact in any
    // radius relation (every boundary point sits in [0, h]).
    if s0 - r > h + band || s0 + r < -band {
        return Ok(report);
    }
    let mut tangency = false;
    let mut found: Vec<(f64, CircleSite)> = Vec::new();
    // Side contacts: rho(t) = r_bottom + m t; with q = t - s0 the sphere
    // equation is (1+m^2) q^2 + 2 rho_c m q + (rho_c^2 - r^2) = 0, whose
    // discriminant 4 (r^2 (1+m^2) - rho_c^2) compares r against the distance
    // |rho_c| / sqrt(1+m^2) from the sphere center to the side line.
    let rho_c = cone.r_bottom + m * s0;
    let k2 = 1. + m * m;
    let dist = rho_c.abs() / k2.sqrt();
    if (r - dist).abs() <= band {
        // Tangent to the side line: a double root. Only a tangency whose
        // foot lands on the finite side segment belongs to this surface;
        // beyond it the rim bands below own the contact.
        let foot = s0 - rho_c * m / k2;
        if (-band..=h + band).contains(&foot) {
            tangency = true;
        }
    } else if r > dist {
        let disc = (r * r * k2 - rho_c * rho_c).max(0.).sqrt();
        for sign in [1., -1.] {
            let q = (-rho_c * m + sign * disc) / k2;
            let t = s0 + q;
            if !(-band..=h + band).contains(&t) {
                continue;
            }
            // A root within the band of a ring plane is the rim contact.
            if t.abs() <= band || (t - h).abs() <= band {
                tangency = true;
                continue;
            }
            // A root whose radius collapses into the band is the apex
            // contact: the sphere passes through the singular apex point.
            let rho = cone.r_bottom + m * t;
            if rho <= band {
                tangency = true;
                continue;
            }
            found.push((t, CircleSite::Side));
        }
    }
    // Rim bands: the whole ring circle lies on the sphere — a tangent
    // boundary contact, never a guessed circle (independent of which branch
    // above ran).
    for &(t_ring, r_ring) in &[(0., cone.r_bottom), (h, cone.r_top)] {
        let d = t_ring - s0;
        let rim = (d * d + r_ring * r_ring).sqrt();
        if (rim - r).abs() <= band {
            tangency = true;
        }
    }
    // Cap-plane contacts: a circle strictly inside the disk is exact; a
    // plane touch or a rim equality stays a tangency region.
    for slot in 0..2 {
        if cone.caps[slot].is_none() {
            continue;
        }
        let t_cap = if slot == 0 { 0. } else { h };
        let r_ring = if slot == 0 { cone.r_bottom } else { cone.r_top };
        let d = (t_cap - s0).abs();
        if d > r + band {
            continue;
        }
        if (d - r).abs() <= band {
            tangency = true;
            continue;
        }
        let rho2 = r * r - d * d;
        if !rho2.is_finite() || rho2 <= 0. {
            tangency = true;
            continue;
        }
        let rho = rho2.sqrt();
        if rho > r_ring + band {
            continue;
        }
        if (rho - r_ring).abs() <= band {
            tangency = true;
            continue;
        }
        found.push((t_cap, CircleSite::Cap(slot, rho)));
    }
    if tangency {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
    }
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (axial, site) in found {
        report
            .components
            .push(circle_component(&sphere, &cone, axial, site)?);
    }
    Ok(report)
}

impl value_codec::Serialize for SphereConeComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Circle {
                curve,
                center,
                radius,
                normal,
                sphere_uv,
                cone_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"circle","curve":curve,"center":center,"radius":radius,
                "normal":normal,"sphereUv":sphere_uv,"coneUv":cone_uv,
                "maxSampleResidual":max_sample_residual}),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_utils::{rotated_translated, translated};
    use super::*;

    fn only_circles(
        report: &Report<SphereConeComponent>,
        count: usize,
    ) -> &Report<SphereConeComponent> {
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        assert!(report.unresolved.is_empty(), "{report:?}");
        assert!(!report.permits_topology_change());
        assert_eq!(report.components.len(), count, "{report:?}");
        report
    }
    #[allow(clippy::type_complexity)]
    fn circle_of(
        component: &SphereConeComponent,
    ) -> (
        &Curve,
        [f64; 3],
        f64,
        [f64; 3],
        &[SpherePatchCircle],
        &[CylinderPatchCurve],
        f64,
    ) {
        let SphereConeComponent::Circle {
            curve,
            center,
            radius,
            normal,
            sphere_uv,
            cone_uv,
            max_sample_residual,
        } = component;
        (
            curve,
            *center,
            *radius,
            *normal,
            sphere_uv,
            cone_uv,
            *max_sample_residual,
        )
    }
    fn sphere_residual(point: [f64; 3], center: [f64; 3], radius: f64) -> f64 {
        let d = sub(point, center);
        (d[0].hypot(d[1]).hypot(d[2]) - radius).abs()
    }
    /// Cone side profile residual of the canonical z-up frames used here.
    fn side_residual_z(point: [f64; 3], r_bottom: f64, slope: f64) -> f64 {
        (point[0].hypot(point[1]) - (r_bottom + slope * point[2])).abs()
    }
    /// Worst residual of lifted UV arcs evaluated through their own surface,
    /// against the sphere equation and a per-circle cone predicate.
    fn uv_samples(
        sphere_model: &Model,
        cone_model: &Model,
        sphere_center: [f64; 3],
        sphere_radius: f64,
        sphere_uv: &[SpherePatchCircle],
        cone_uv: &[CylinderPatchCurve],
        cone_check: impl Fn([f64; 3]) -> f64,
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
                        .max(cone_check(p));
                }
            }
        }
        for lift in cone_uv {
            let surface = &cone_model.faces[lift.patch].surface;
            for arc in &lift.arcs {
                for k in 0..=8 {
                    let uv = arc.evaluate(k as f64 / 8.).unwrap().point;
                    assert!((0. ..=1.).contains(&uv[0]) && (0. ..=1.).contains(&uv[1]));
                    let p = surface.evaluate(uv[0], uv[1]).unwrap().point;
                    let p = [p[0], p[1], p[2]];
                    worst = worst
                        .max(sphere_residual(p, sphere_center, sphere_radius))
                        .max(cone_check(p));
                }
            }
        }
        worst
    }

    #[test]
    fn two_side_circles_match_the_quadratic_oracle() {
        // Frustum r 1 -> 3 over z 0..6 (slope 1/3); sphere r=2 centered on
        // the axis at z=3: rho_c = 2, discriminant 4(4 * 10/9 - 4) = 16/9,
        // roots q = (-2/3 +- 2/3) * 9/10 = 0 and -1.2 — two exact side
        // circles at z = 3 (radius 2) and z = 1.8 (radius 1.6), the sphere
        // clear of both cap planes and both rims.
        let sphere = translated(&crate::analytic::sphere(2.).unwrap(), [0., 0., 3.]);
        let cone = crate::analytic::frustum(1., 3., 6.).unwrap();
        let report = intersect_sphere_cone(&sphere, &cone, Options::default()).unwrap();
        let report = only_circles(&report, 2);
        let q = |sign: f64| (-2. / 3. + sign * 2. / 3.) * 0.9;
        for (component, qi) in report.components.iter().zip([q(-1.), q(1.)]) {
            let z = 3. + qi;
            let rho = 1. + z / 3.;
            let (curve, center, radius, normal, sphere_uv, cone_uv, sampled) = circle_of(component);
            assert!((radius - rho).abs() <= 1e-12, "{radius}");
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
                    .max(sphere_residual(p, [0., 0., 3.], 2.))
                    .max(side_residual_z(p, 1., 1. / 3.));
            }
            assert!(worst <= 1e-12, "{worst}");
            assert!(sampled <= 1e-12, "{sampled}");
            // Side circles lift to iso-v lines on all four side patches.
            assert_eq!(cone_uv.len(), 4);
            for lift in cone_uv {
                assert_eq!(lift.arcs.len(), 1);
                let arc = &lift.arcs[0];
                assert_eq!(arc.degree, 1);
                assert!((arc.control_points[0][1] - z / 6.).abs() <= 1e-12);
                assert!((arc.control_points[1][1] - z / 6.).abs() <= 1e-12);
            }
            assert!(!sphere_uv.is_empty());
            let uv_worst = uv_samples(&sphere, &cone, [0., 0., 3.], 2., sphere_uv, cone_uv, |p| {
                side_residual_z(p, 1., 1. / 3.)
            });
            assert!(uv_worst <= 1e-9, "{uv_worst}");
        }
    }

    #[test]
    fn one_circle_survives_clipping_by_the_finite_height() {
        // Frustum r 2 -> 4 over z 0..6 (slope 1/3); sphere r=2.5 at z=0.5:
        // rho_c = 13/6, discriminant 4(6.25 * 10/9 - 169/36) = 9, roots
        // q = (-13/18 +- 3/2) * 9/10 = 0.7 and -2 — only t = 1.2 survives,
        // radius 2.4. The bottom cap circle would have radius sqrt(6) > 2,
        // outside the disk.
        let sphere = translated(&crate::analytic::sphere(2.5).unwrap(), [0., 0., 0.5]);
        let cone = crate::analytic::frustum(2., 4., 6.).unwrap();
        let report = intersect_sphere_cone(&sphere, &cone, Options::default()).unwrap();
        let report = only_circles(&report, 1);
        let (curve, center, radius, _, _, cone_uv, sampled) = circle_of(&report.components[0]);
        assert!((radius - 2.4).abs() <= 1e-12, "{radius}");
        assert!(
            sub(center, [0., 0., 1.2]).iter().all(|x| x.abs() <= 1e-12),
            "{center:?}"
        );
        let mut worst = 0_f64;
        for i in 0..16 {
            let p = curve.evaluate(i as f64 / 4.).unwrap().point;
            let p = [p[0], p[1], p[2]];
            worst = worst
                .max(sphere_residual(p, [0., 0., 0.5], 2.5))
                .max(side_residual_z(p, 2., 1. / 3.));
        }
        assert!(worst <= 1e-12, "{worst}");
        assert!(sampled <= 1e-12);
        assert_eq!(cone_uv.len(), 4);
    }

    #[test]
    fn small_sphere_poking_through_a_cap_yields_a_cap_circle() {
        // Frustum r 2 -> 4 over z 0..6; sphere r=1.5 centered 0.5 below the
        // top ring plane z=6: circle of radius sqrt(2) in the cap plane,
        // inside the top disk (ring radius 4); the side line stays
        // unreachable (distance 3.64 > 1.5).
        let sphere = translated(&crate::analytic::sphere(1.5).unwrap(), [0., 0., 5.5]);
        let cone = crate::analytic::frustum(2., 4., 6.).unwrap();
        let report = intersect_sphere_cone(&sphere, &cone, Options::default()).unwrap();
        let report = only_circles(&report, 1);
        let (curve, center, radius, _, sphere_uv, cone_uv, sampled) =
            circle_of(&report.components[0]);
        let oracle = 2_f64.sqrt();
        assert!((radius - oracle).abs() <= 1e-12, "{radius}");
        assert!(
            sub(center, [0., 0., 6.]).iter().all(|x| x.abs() <= 1e-12),
            "{center:?}"
        );
        let mut worst = 0_f64;
        for i in 0..16 {
            let p = curve.evaluate(i as f64 / 4.).unwrap().point;
            let p = [p[0], p[1], p[2]];
            worst = worst
                .max(sphere_residual(p, [0., 0., 5.5], 1.5))
                .max((p[2] - 6.).abs());
        }
        assert!(worst <= 1e-12, "{worst}");
        assert!(sampled <= 1e-12);
        // Cap lift: one face, four exact 90-degree arcs of the UV circle of
        // radius sqrt(2)/(2*4) about [1/2, 1/2].
        assert_eq!(cone_uv.len(), 1);
        let lift = &cone_uv[0];
        assert_eq!(lift.arcs.len(), 4);
        for arc in &lift.arcs {
            assert_eq!(arc.degree, 2);
            assert_eq!(arc.weights, vec![1., ARC_WEIGHT, 1.]);
            let endpoint = &arc.control_points[0];
            let uvr = (endpoint[0] - 0.5).hypot(endpoint[1] - 0.5);
            assert!((uvr - oracle / 8.).abs() <= 1e-12, "{uvr}");
        }
        let uv_worst = uv_samples(
            &sphere,
            &cone,
            [0., 0., 5.5],
            1.5,
            sphere_uv,
            cone_uv,
            |p| (p[2] - 6.).abs(),
        );
        assert!(uv_worst <= 1e-9, "{uv_worst}");
    }

    #[test]
    fn bottom_cap_circle_uses_the_bottom_cap_face() {
        // Sphere r=1.5 centered 0.5 above the bottom ring plane z=0 of
        // frustum r 2 -> 4 over z 0..6: circle of radius sqrt(2) inside the
        // bottom disk (ring radius 2), on the bottom cap face.
        let sphere = translated(&crate::analytic::sphere(1.5).unwrap(), [0., 0., 0.5]);
        let cone = crate::analytic::frustum(2., 4., 6.).unwrap();
        let report = intersect_sphere_cone(&sphere, &cone, Options::default()).unwrap();
        let report = only_circles(&report, 1);
        let (_, center, radius, _, _, cone_uv, _) = circle_of(&report.components[0]);
        assert!((radius - 2_f64.sqrt()).abs() <= 1e-12);
        assert!(sub(center, [0., 0., 0.]).iter().all(|x| x.abs() <= 1e-12));
        assert_eq!(cone_uv.len(), 1);
        // The bottom cap is a different face than the top cap; its lift
        // still evaluates onto the sphere and the cap plane.
        let lift = &cone_uv[0];
        let surface = &cone.faces[lift.patch].surface;
        for arc in &lift.arcs {
            for k in 0..=8 {
                let uv = arc.evaluate(k as f64 / 8.).unwrap().point;
                let p = surface.evaluate(uv[0], uv[1]).unwrap().point;
                let p = [p[0], p[1], p[2]];
                assert!(sphere_residual(p, [0., 0., 0.5], 1.5) <= 1e-9);
                assert!(p[2].abs() <= 1e-9);
            }
        }
    }

    #[test]
    fn zero_circle_configurations_resolve_empty() {
        let cone = crate::analytic::frustum(2., 4., 6.).unwrap();
        // Small sphere strictly inside the side wall, clear of both caps.
        let inside = translated(&crate::analytic::sphere(0.5).unwrap(), [0., 0., 3.]);
        // Sphere beyond the top ring plane, no reach back.
        let beyond = translated(&crate::analytic::sphere(1.).unwrap(), [0., 0., 8.]);
        // Large sphere swallowing the whole frustum (side roots beyond the
        // height, cap circles outside the disks).
        let swallow = translated(&crate::analytic::sphere(20.).unwrap(), [0., 0., 3.]);
        for sphere in [&inside, &beyond, &swallow] {
            let report = intersect_sphere_cone(sphere, &cone, Options::default()).unwrap();
            assert!(
                report.components.is_empty() && report.unresolved.is_empty(),
                "{report:?}"
            );
            assert_eq!(report.coverage, Coverage::NumericallyResolved);
        }
    }

    #[test]
    fn side_tangency_stays_a_tangency_region() {
        let cone = crate::analytic::frustum(1., 3., 6.).unwrap();
        // Tangent to the side line: r == rho_c / sqrt(1+m^2) = 6/sqrt(10)
        // for the sphere centered at z=3 — a double root, never a circle.
        let tangent = 6. / 10_f64.sqrt();
        for r in [tangent, tangent + 2e-15, tangent - 2e-15] {
            let sphere = translated(&crate::analytic::sphere(r).unwrap(), [0., 0., 3.]);
            let report = intersect_sphere_cone(&sphere, &cone, Options::default()).unwrap();
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
            &crate::analytic::sphere(tangent - 1e-9).unwrap(),
            [0., 0., 3.],
        );
        let report = intersect_sphere_cone(&clear, &cone, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        // Just across: two small transverse side circles around the foot
        // t = 3 - 0.6 = 2.4.
        let across = translated(
            &crate::analytic::sphere(tangent + 1e-9).unwrap(),
            [0., 0., 3.],
        );
        let report = intersect_sphere_cone(&across, &cone, Options::default()).unwrap();
        only_circles(&report, 2);
    }

    #[test]
    fn rim_contact_stays_unresolved() {
        // Sphere centered on the bottom ring plane with r == r_bottom: the
        // whole bottom rim circle lies on the sphere — a tangent boundary
        // contact, never a guessed circle.
        let cone = crate::analytic::frustum(2., 4., 6.).unwrap();
        let sphere = crate::analytic::sphere(2.).unwrap();
        let report = intersect_sphere_cone(&sphere, &cone, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
    }

    #[test]
    fn apex_contact_stays_unresolved() {
        // True-apex cone r 0 -> 3 over z 0..5 (apex at the bottom ring
        // plane). Sphere r=2 centered at z=2 passes through the apex (rim
        // distance sqrt(4+0) == r): the apex root stays a tangency region
        // while the other root — a clean transverse side circle — still
        // reports as a component.
        let cone = crate::analytic::frustum(0., 3., 5.).unwrap();
        let sphere = translated(&crate::analytic::sphere(2.).unwrap(), [0., 0., 2.]);
        let report = intersect_sphere_cone(&sphere, &cone, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
        assert_eq!(report.components.len(), 1, "{report:?}");
        let (_, center, radius, _, _, cone_uv, sampled) = circle_of(&report.components[0]);
        // Oracle: rho_c = 6/5, roots q = (-18/25 +- 2) * 25/34; the in-range
        // root is q = 32/34 = 16/17, t = 50/17, rho = 3 * 10/17 = 30/17.
        let t = 50. / 17.;
        assert!((radius - 30. / 17.).abs() <= 1e-12, "{radius}");
        assert!(
            sub(center, [0., 0., t]).iter().all(|x| x.abs() <= 1e-12),
            "{center:?}"
        );
        assert!(sampled <= 1e-12);
        assert_eq!(cone_uv.len(), 4);
        // Sphere centered at the apex height cutting the side transversally
        // (apex strictly inside the sphere) stays resolved.
        let clear = translated(&crate::analytic::sphere(2.).unwrap(), [0., 0., 1e-6]);
        let report = intersect_sphere_cone(&clear, &cone, Options::default()).unwrap();
        only_circles(&report, 1);
    }

    #[test]
    fn cap_plane_touch_stays_a_tangency_region() {
        let cone = crate::analytic::frustum(2., 4., 6.).unwrap();
        // r=1.5 sphere tangent to the top cap plane z=6 from inside.
        let sphere = translated(&crate::analytic::sphere(1.5).unwrap(), [0., 0., 4.5]);
        let report = intersect_sphere_cone(&sphere, &cone, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
        // Just clear of the band: strictly inside, empty and resolved.
        let clear = translated(&crate::analytic::sphere(1.5).unwrap(), [0., 0., 4.5 - 1e-9]);
        let report = intersect_sphere_cone(&clear, &cone, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        // Just across: a small transverse cap circle.
        let across = translated(&crate::analytic::sphere(1.5).unwrap(), [0., 0., 4.5 + 1e-9]);
        let report = intersect_sphere_cone(&across, &cone, Options::default()).unwrap();
        only_circles(&report, 1);
    }

    #[test]
    fn near_axial_offset_within_the_band_stays_unresolved() {
        let cone = crate::analytic::frustum(2., 4., 6.).unwrap();
        let sphere = translated(&crate::analytic::sphere(2.5).unwrap(), [1e-10, 0., 0.5]);
        let report = intersect_sphere_cone(&sphere, &cone, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::NearCoincidence
        );
    }

    #[test]
    fn clearly_off_axis_pairs_are_unsupported_regions() {
        let cone = crate::analytic::frustum(2., 4., 6.).unwrap();
        let sphere = translated(&crate::analytic::sphere(2.5).unwrap(), [0.5, 0., 0.5]);
        let report = intersect_sphere_cone(&sphere, &cone, Options::default()).unwrap();
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
        let sphere = crate::analytic::sphere(2.).unwrap();
        let cone = crate::analytic::frustum(1., 2., 3.).unwrap();
        // An equal-radius frustum is a cylinder (refused as a cone operand),
        // and tubes, cuboids and tori are neither canonical operand; swapped
        // operand order is refused by the fixed order.
        for (a, b) in [
            (
                crate::analytic::sphere(2.).unwrap(),
                crate::analytic::cylinder(1., 3.).unwrap(),
            ),
            (
                crate::analytic::sphere(2.).unwrap(),
                crate::analytic::tube(2., 1., 3.).unwrap(),
            ),
            (
                crate::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap(),
                crate::analytic::frustum(1., 2., 3.).unwrap(),
            ),
            (
                crate::analytic::torus(3., 1.).unwrap(),
                crate::analytic::frustum(1., 2., 3.).unwrap(),
            ),
            (
                crate::analytic::frustum(1., 2., 3.).unwrap(),
                crate::analytic::sphere(2.).unwrap(),
            ),
        ] {
            let report = intersect_sphere_cone(&a, &b, Options::default()).unwrap();
            assert!(report.components.is_empty(), "{report:?}");
            assert_eq!(report.coverage, Coverage::Incomplete);
            assert_eq!(report.unresolved.len(), 1);
            assert_eq!(
                report.unresolved[0].reason,
                UnresolvedReason::UnsupportedSurface
            );
        }
        // A canonical pair still resolves: sphere r=1.5 at z=1 against the
        // frustum r 1 -> 2 over z 0..3 crosses the side once (the other root
        // is below the bottom ring; the bottom cap circle radius sqrt(1.25)
        // lies outside the unit disk).
        let s15 = translated(&crate::analytic::sphere(1.5).unwrap(), [0., 0., 1.]);
        let report = intersect_sphere_cone(&s15, &cone, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
        assert_eq!(report.components.len(), 1);
        // A structurally perturbed cone fails validation as a hard error.
        let mut perturbed = crate::analytic::frustum(1., 2., 3.).unwrap();
        perturbed.faces[0].surface.weights[1][0] = 0.5;
        assert!(intersect_sphere_cone(&sphere, &perturbed, Options::default()).is_err());
    }

    #[test]
    fn rotated_translated_placement_keeps_the_exact_circles() {
        // Rigid placement of the whole axial configuration about X by 0.5.
        let angle = 0.5;
        let offset = [0.3, -0.2, 1.1];
        let sphere = rotated_translated(
            &translated(&crate::analytic::sphere(2.).unwrap(), [0., 0., 3.]),
            angle,
            offset,
        );
        let cone = rotated_translated(
            &crate::analytic::frustum(1., 3., 6.).unwrap(),
            angle,
            offset,
        );
        let report = intersect_sphere_cone(&sphere, &cone, Options::default()).unwrap();
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
        let sphere_center = placed([0., 0., 3.]);
        let axis = {
            let a = placed([0., 0., 1.]);
            let b = placed([0., 0., 0.]);
            sub(a, b)
        };
        let q = |sign: f64| (-2. / 3. + sign * 2. / 3.) * 0.9;
        for (component, qi) in report.components.iter().zip([q(-1.), q(1.)]) {
            let z = 3. + qi;
            let rho = 1. + z / 3.;
            let (curve, center, radius, normal, sphere_uv, cone_uv, sampled) = circle_of(component);
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
                worst = worst.max(sphere_residual(p, sphere_center, 2.)).max({
                    let rel = sub(p, expected);
                    let a = dot(rel, axis);
                    let perp = sub(rel, axis.map(|x| x * a));
                    (perp[0].hypot(perp[1]).hypot(perp[2]) - rho).abs()
                });
            }
            assert!(worst <= 1e-12, "{worst}");
            assert!(sampled <= 1e-12);
            assert_eq!(cone_uv.len(), 4);
            assert!(!sphere_uv.is_empty());
            let axis = normal;
            let uv_worst = uv_samples(&sphere, &cone, sphere_center, 2., sphere_uv, cone_uv, |p| {
                let rel = sub(p, placed([0., 0., 0.]));
                let a = dot(rel, axis);
                let perp = sub(rel, axis.map(|x| x * a));
                (perp[0].hypot(perp[1]).hypot(perp[2]) - (1. + a / 3.)).abs()
            });
            assert!(uv_worst <= 1e-9, "{uv_worst}");
        }
    }
}
