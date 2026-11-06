//! A new three-part planar corner proposal, not a fillet-body certificate.
//! Endpoint offsets and all actual coefficient fields require fresh admission.
use crate::{Result, check, curve::Curve};

pub struct Proposal {
    pub parts: [Curve; 3],
    pub nominal_radius: f64,
    pub endpoint_offset: f64,
}
impl Proposal {
    /// A single rational quartic proposal, with no internal micro-clock knots.
    /// The first/last three poles lie on their support coordinate lines. Tiny
    /// positive endpoint weights regularize the circular interior. Deriving
    /// the weight from the ACTUAL stored ordinate increment controls endpoint
    /// tangential acceleration without rounding a small coordinate offset
    /// into a large divided second derivative. Fresh cap/G2 proofs are needed.
    pub fn quartic_g2(&self) -> Result<Curve> {
        let margin = self.endpoint_offset - self.nominal_radius;
        check(
            self.nominal_radius.is_finite()
                && self.nominal_radius > 0.
                && margin.is_finite()
                && margin > 0.,
            "Choose a positive original corner margin",
        )?;
        let radius = self.nominal_radius + margin / 4.;
        let ordinate = radius + margin / 16.;
        let delta = ordinate - radius;
        let middle = std::f64::consts::FRAC_1_SQRT_2 / 3.;
        let endpoint = delta / (delta + 6. * middle * ordinate);
        check(
            delta > 0. && endpoint.is_finite() && endpoint > 0.,
            "Quartic endpoint weight is not representable",
        )?;
        let c = curve(
            vec![
                vec![0., ordinate, 0.],
                vec![0., radius, 0.],
                vec![0.; 3],
                vec![radius, 0., 0.],
                vec![ordinate, 0., 0.],
            ],
            vec![endpoint, 0.25, middle, 0.25, endpoint],
        );
        c.validate()?;
        Ok(c)
    }
    /// New degree-five proposal with matched homogeneous endpoint jets and a
    /// small bending relief. G2 is subject to the requested tensor tolerance.
    /// Tiny runouts receive their own clocks; the middle arc keeps its clock.
    /// Stored rounding is NOT an image/curvature certificate. All original
    /// spans, joins and contact correspondences must be admitted afresh.
    pub fn composite_matched_g2(&self) -> Result<Curve> {
        for c in &self.parts {
            c.validate()?;
        }
        check(
            self.parts[0].degree == 3
                && self.parts[0].control_points.len() == 4
                && self.parts.iter().all(|c| c.control_points[0].len() == 3),
            "Matched corner needs the authored XYZ cubic runout",
        )?;
        let mut base = self.parts[1].clone();
        base.validate()?;
        check(
            base.degree == 2
                && base.control_points.len() == 3
                && base.control_points[0][0] == base.control_points[2][1]
                && base.control_points[0][1] == base.control_points[2][0]
                && base.control_points[1][0] == base.control_points[1][1]
                && base
                    .control_points
                    .iter()
                    .all(|p| p.len() == 3 && p[2] == 0.),
            "C2 corner proposal needs the symmetric planar middle arc",
        )?;
        // A symmetric projective clock avoids incompatible endpoint gauges.
        // The rounded square root authors a NEW arc: fresh cap/G2 checks apply.
        let middle_weight = base.weights[1] / (base.weights[0].sqrt() * base.weights[2].sqrt());
        base.weights = vec![1., middle_weight, 1.];
        let mut arc = base.elevate(5)?;
        for i in 3..6 {
            let p = &arc.control_points[5 - i];
            arc.control_points[i] = vec![p[1], p[0], 0.];
            arc.weights[i] = arc.weights[5 - i];
        }
        let homogeneous = |c: &Curve, i: usize| -> [f64; 4] {
            let w = c.weights[i];
            [
                c.control_points[i][0] * w,
                c.control_points[i][1] * w,
                c.control_points[i][2] * w,
                w,
            ]
        };
        let jets = |end: usize| {
            let (a, b, c) = if end == 0 { (0, 1, 2) } else { (5, 4, 3) };
            let h = homogeneous(&arc, a);
            let n = homogeneous(&arc, b);
            let m = homogeneous(&arc, c);
            let sign = if end == 0 { 1. } else { -1. };
            (
                h,
                std::array::from_fn::<_, 4, _>(|k| sign * 5. * (n[k] - h[k])),
                std::array::from_fn::<_, 4, _>(|k| 20. * ((m[k] - n[k]) - (n[k] - h[k]))),
            )
        };
        let (a, da, dda) = jets(0);
        let speed = |h: [f64; 4], d: [f64; 4]| {
            ((d[0] - h[0] / h[3] * d[3]) / h[3]).hypot((d[1] - h[1] / h[3] * d[3]) / h[3])
        };
        let left = &self.parts[0];
        let chord = |c: &Curve, i: usize, j: usize| {
            (c.control_points[i][0] - c.control_points[j][0])
                .hypot(c.control_points[i][1] - c.control_points[j][1])
        };
        let ideal = 3. * chord(left, 0, 1) / speed(a, da);
        let h = (ideal / f64::EPSILON).round() * f64::EPSILON;
        let breaks = [0., h, h + 1., 1. + 2. * h];
        check(
            breaks.windows(2).all(|w| w[0].is_finite() && w[0] < w[1]),
            "C2 runout clocks are not representable",
        )?;
        let wa = breaks[2] - breaks[1];
        let ratio = (breaks[1] - breaks[0]) / wa;
        let mut lh = [[0.; 4]; 6];
        let mut rh = [[0.; 4]; 6];
        for k in 0..4 {
            let lp = if k == 3 {
                a[3]
            } else {
                left.control_points[0][k] * a[3]
            };
            let lv = if k == 3 {
                0.
            } else {
                3. * (left.control_points[1][k] - left.control_points[0][k]) * a[3]
            };
            lh[0][k] = lp;
            lh[1][k] = lp + lv / 5.;
            lh[2][k] = lp + 2. * lv / 5.;
            lh[5][k] = a[k];
            lh[4][k] = a[k] - ratio * da[k] / 5.;
            lh[3][k] = a[k] - 2. * ratio * da[k] / 5. + ratio * ratio * dda[k] / 20.;
        }
        // Re-author the end normal acceleration slightly below that of the
        // arc. Otherwise dehomogenization at the micro-runout can round the
        // actual curvature above 10/mm. This is geometric relief, NOT proof
        // or a tolerance change; original cap and G2 checks remain mandatory.
        lh[3][0] -= 1e-6 * ((lh[5][0] - lh[4][0]) - (lh[4][0] - lh[3][0]));
        for i in 0..6 {
            let p = lh[5 - i];
            rh[i] = [p[1], p[0], p[2], p[3]];
        }
        let mut controls = lh.to_vec();
        controls.extend((1..6).map(|i| homogeneous(&arc, i)));
        controls.extend_from_slice(&rh[1..]);
        check(
            controls.iter().all(|p| p[3].is_finite() && p[3] > 0.),
            "C2 runout weights must be positive",
        )?;
        let out = Curve {
            degree: 5,
            knots: [
                vec![breaks[0]; 6],
                vec![breaks[1]; 5],
                vec![breaks[2]; 5],
                vec![breaks[3]; 6],
            ]
            .concat(),
            weights: controls.iter().map(|p| p[3]).collect(),
            control_points: controls
                .iter()
                .map(|p| (0..3).map(|k| p[k] / p[3]).collect())
                .collect(),
            periodic: false,
        };
        out.validate()?;
        Ok(out)
    }
    /// Author a C0 cubic composite for surface construction. The rounded
    /// homogeneous elevation changes stored coefficients: all three resulting
    /// original cells and both G2 joins must be qualified afresh.
    pub fn composite_cubic(&self) -> Result<Curve> {
        let middle = &self.parts[1];
        let mut arc = middle.clone();
        arc.degree = 3;
        arc.knots = [vec![0.; 4], vec![1.; 4]].concat();
        arc.control_points = vec![middle.control_points[0].clone()];
        arc.weights = vec![middle.weights[0]];
        for i in 1..=2 {
            let alpha = i as f64 / 3.;
            let weight = alpha * middle.weights[i - 1] + (1. - alpha) * middle.weights[i];
            let point = (0..3)
                .map(|k| {
                    (alpha * middle.weights[i - 1] * middle.control_points[i - 1][k]
                        + (1. - alpha) * middle.weights[i] * middle.control_points[i][k])
                        / weight
                })
                .collect();
            arc.control_points.push(point);
            arc.weights.push(weight);
        }
        arc.control_points.push(middle.control_points[2].clone());
        arc.weights.push(middle.weights[2]);
        let mut out = self.parts[0].clone();
        out.weights.fill(arc.weights[0]);
        out.control_points
            .extend_from_slice(&arc.control_points[1..]);
        out.weights.extend_from_slice(&arc.weights[1..]);
        out.control_points
            .extend_from_slice(&self.parts[2].control_points[1..]);
        out.weights.extend(vec![*arc.weights.last().unwrap(); 3]);
        out.knots = [vec![0.; 4], vec![1.; 3], vec![2.; 3], vec![3.; 4]].concat();
        out.validate()?;
        Ok(out)
    }
}
fn curve(points: Vec<Vec<f64>>, weights: Vec<f64>) -> Curve {
    let p = points.len() - 1;
    Curve {
        degree: p,
        knots: [vec![0.; p + 1], vec![1.; p + 1]].concat(),
        control_points: points,
        weights,
        periodic: false,
    }
}
/// Short polynomial runouts with zero endpoint curvature and a rational middle
/// arc. Arithmetic here authors a proposal; it is never proof of its shape.
pub fn propose(radius: f64, maximum: f64, contact_tolerance: f64) -> Result<Proposal> {
    check(
        radius.is_finite()
            && radius > 0.
            && maximum.is_finite()
            && maximum > 0.
            && contact_tolerance.is_finite()
            && contact_tolerance > 0.
            && contact_tolerance < radius / 100.,
        "Choose positive radius, cap and a small contact tolerance",
    )?;
    let r = (radius + contact_tolerance / 32.).max(1. / maximum + contact_tolerance / 64.);
    let wanted = (contact_tolerance / (32. * radius)).min(1. / 16.);
    check(
        wanted.is_finite() && wanted > 0.,
        "Runout is not representable",
    )?;
    let m0 = 2f64.powf(wanted.log2().floor());
    let footprint = radius + contact_tolerance;
    let quantum = 5. * (footprint.next_up() - footprint);
    check(
        quantum.is_finite() && quantum > 0.,
        "Runout grid is not representable",
    )?;
    // Each stored linear ordinate must share the endpoint's binary grid.
    // Unquantized subtraction at a micro-runout can turn rounding into a
    // genuine second derivative and violate the physical curvature cap.
    // This authors new coefficients; the fresh curvature predicate decides.
    let d = ((2. * r * m0 / 3.) / quantum).ceil() * quantum;
    let a = 1.5 * d * d / r;
    let m = a / d;
    let factor = 1.5 * d * (1. + m * m) / m;
    let y = 3. * d + a + factor * (1. - m);
    check(
        y.is_finite() && a > 0. && d > 0. && (y - radius).abs() < contact_tolerance / 8.,
        "The proposed contact shift exceeds the unchanged tolerance",
    )?;
    let b = y - 3. * d;
    let k = m * (b + d) / (1. + m);
    let left = curve(
        vec![
            vec![0., y, 0.],
            vec![0., y - d, 0.],
            vec![0., y - 2. * d, 0.],
            vec![a, b, 0.],
        ],
        vec![1.; 4],
    );
    let middle = curve(
        vec![vec![a, b, 0.], vec![k, k, 0.], vec![b, a, 0.]],
        vec![1. + m * m, 1. + m, 2.],
    );
    let right = curve(
        left.control_points
            .iter()
            .rev()
            .map(|p| vec![p[1], p[0], p[2]])
            .collect(),
        vec![1.; 4],
    );
    for c in [&left, &middle, &right] {
        c.validate()?;
    }
    Ok(Proposal {
        parts: [left, middle, right],
        nominal_radius: radius,
        endpoint_offset: y,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::bezier_curvature_bound::inspect;
    #[test]
    fn actual_three_part_corner_fields_respect_ten_without_sampling() {
        for radius in [0.1, 0.2, 0.35] {
            let p = propose(radius, 10., 1e-7).unwrap();
            for (i, c) in p.parts.iter().enumerate() {
                let r = inspect(c, 10., 4096, 1000000).unwrap();
                println!(
                    "original radius={radius} part={i} work={} cells={} outcome={:?}",
                    r.decision.work_used, r.decision.cells, r.decision.outcome
                );
                assert!(
                    r.certificate.is_some(),
                    "radius={radius} part={i} {:?}",
                    r.decision
                );
                assert!(std::ptr::eq(r.certificate.unwrap().source(), c));
            }
            assert_eq!(
                p.parts[0].control_points.last(),
                p.parts[1].control_points.first()
            );
            assert_eq!(
                p.parts[1].control_points.last(),
                p.parts[2].control_points.first()
            );
        }
    }
    #[test]
    fn cannot_silently_enlarge_an_infeasible_radius_or_ignore_work() {
        assert!(propose(0.09, 10., 1e-7).is_err());
        let p = propose(0.1, 10., 1e-7).unwrap();
        assert!(
            inspect(&p.parts[0], 10., 4096, 0)
                .unwrap()
                .certificate
                .is_none()
        );
        let mut changed = p.parts[0].clone();
        changed.control_points[1][0] = 0.1;
        assert!(
            inspect(&changed, 10., 4096, 1000000)
                .unwrap()
                .certificate
                .is_none()
        );
    }
    #[test]
    fn production_tolerance_does_not_round_the_runout_above_the_cap() {
        let p = propose(0.1, 10., 2.5e-8).unwrap();
        let c = p.composite_cubic().unwrap();
        let mut sources = p.parts.to_vec();
        for start in [0, 3, 6] {
            sources.push(curve(
                c.control_points[start..start + 4].to_vec(),
                c.weights[start..start + 4].to_vec(),
            ));
        }
        for (i, source) in sources.iter().enumerate() {
            let r = inspect(source, 10., 4096, 1000000).unwrap();
            assert!(
                r.certificate.is_some(),
                "production cell {i}: {:?}",
                r.decision
            );
        }
        assert!((p.endpoint_offset - 0.1).abs() < 2.5e-8 / 8.);
    }
    #[test]
    fn original_composite_cells_and_complete_extruded_joins_are_rechecked() {
        use crate::{surface::Surface, surface_g2_pcurves as g2, surface_join::Decision};
        for radius in [0.1, 0.2, 0.35] {
            let p = propose(radius, 10., 1e-7).unwrap();
            let c = p.composite_cubic().unwrap();
            for start in [0, 3, 6] {
                let cell = curve(
                    c.control_points[start..start + 4].to_vec(),
                    c.weights[start..start + 4].to_vec(),
                );
                let r = inspect(&cell, 10., 4096, 1000000).unwrap();
                println!(
                    "composite radius={radius} cell={start} work={} cells={} outcome={:?}",
                    r.decision.work_used, r.decision.cells, r.decision.outcome
                );
                assert!(
                    r.certificate.is_some(),
                    "radius={radius} cell={start} {:?}",
                    r.decision
                );
            }
            let s = Surface {
                degree_u: 3,
                degree_v: 1,
                knots_u: c.knots.clone(),
                knots_v: vec![0., 0., 1., 1.],
                control_points: c
                    .control_points
                    .iter()
                    .map(|p| vec![p.clone(), vec![p[0], p[1], 1.]])
                    .collect(),
                weights: c.weights.iter().map(|w| vec![*w; 2]).collect(),
                periodic_u: false,
                periodic_v: false,
            };
            let before = s.clone();
            let mut pc = Curve::from_polyline(vec![vec![0., 0.], vec![3., 0.]]).unwrap();
            pc.knots = vec![0., 0., 3., 3.];
            let position =
                crate::curve_surface_agreement::verify(&c, &pc, &s, false, 1e-7, 32).unwrap();
            assert_eq!(
                position.status,
                crate::curve_surface_agreement::Status::WithinTolerance,
                "complete literal original clocks at radius={radius}"
            );
            assert!(position.cells <= 32);
            let mut wrong = s.clone();
            wrong.control_points[4][0][0] += 0.001;
            let rejected =
                crate::curve_surface_agreement::verify(&c, &pc, &wrong, false, 1e-7, 32).unwrap();
            assert_ne!(
                rejected.status,
                crate::curve_surface_agreement::Status::WithinTolerance
            );
            for knot in [1., 2.] {
                let r =
                    g2::inspect_original_geometric_span_join(&s, knot, 1e-5, 1e-4, 512).unwrap();
                assert_eq!(
                    r.decision,
                    Decision::WithinTolerance,
                    "radius={radius} knot={knot} normal={:?} tensor={:?} scales={:?}",
                    r.normal,
                    r.tensor,
                    r.cross_scales
                );
                assert!(r.regularity_proven && r.pending == 0);
            }
            assert_eq!(s, before);
        }
    }
}
