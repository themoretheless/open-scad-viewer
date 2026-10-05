//! Qualified contact curves for clipping original source faces against a patch.
//! UV branch correspondence, original trim membership and world agreement are
//! independent prerequisites. Complete face loops and topology are not created.
use crate::{
    Result, check,
    curve::Curve,
    curve_surface_agreement as agreement,
    distance_bounds::Interval,
    offset_contact_pcurve as uv, offset_envelope_fit as fit,
    surface::Surface,
    trim_domain::{Classification, Location, TrimDomain},
    trimmed_offset_contact as trimmed,
};
#[derive(Clone, Copy)]
pub struct Limits {
    pub fit_cells: usize,
    pub uv_cells: usize,
    pub agreement_cells: usize,
    pub root_refinements: usize,
    pub trim: trimmed::Limits,
}
pub struct Report {
    pub source_pcurves: [Curve; 2],
    pub fit: fit::Report,
    pub trim: trimmed::Report,
    pub world_curves: Option<[Curve; 2]>,
    pub uv_reports: Vec<uv::Report>,
    pub agreements: Vec<agreement::Report>,
    pub classifications: Vec<Classification>,
    pub uv_cells: usize,
    pub agreement_cells: usize,
    pub domain_cells: usize,
    pub patch_boundary_identity: bool,
    pub contact_trim_curves_proven: bool,
    pub reason: &'static str,
}
impl Report {
    pub fn to_value(&self) -> value_codec::Value {
        use value_codec::json;
        json!({"method":"interval-offset-contact-trim-curves","scope":"full-source-contact-branch",
            "fit":self.fit.to_value(),"trim":self.trim.to_value(),"sourcePcurves":self.source_pcurves,
            "worldCurves":self.world_curves,"patchBoundaryIdentityProven":self.patch_boundary_identity,
            "contactTrimCurvesProven":self.contact_trim_curves_proven,"reason":self.reason,
            "sourceParameterReports":self.uv_reports.iter().map(|r|r.to_value()).collect::<Vec<_>>(),
            "sourceMembershipReports":self.classifications.iter().map(|r|r.to_value()).collect::<Vec<_>>(),
            "sourceAgreementReports":self.agreements.iter().enumerate().map(|(side,r)|json!({"side":side,
                "status":match r.status {agreement::Status::WithinTolerance=>"within-tolerance",agreement::Status::Mismatch=>"mismatch",agreement::Status::Unresolved=>"unresolved"},
                "cells":r.cells,"witnessParameter":r.witness,"witnessDistanceMm":r.witness_distance})).collect::<Vec<_>>(),
            "uvCells":self.uv_cells,"agreementCells":self.agreement_cells,"domainCells":self.domain_cells,
            "originalWorldBoundaryIdentityProven":false,"replacementFaceTrimsProven":false,
            "stitchedTopologyProven":false,"wholeCurveComplete":false,"tangentToleranceProven":false,"embeddingProven":false,"topologyAuthority":false})
    }
}
/// Clamped V boundaries select original columns with no interpolation. Each
/// returned world curve has the original U knots, weights and traversal.
fn boundaries(candidate: &Surface) -> Result<Option<[Curve; 2]>> {
    candidate.validate()?;
    let Some(boundary) = crate::offset_patch_boundary::extract(candidate)? else {
        return Ok(None);
    };
    let n = candidate.control_points[0].len();
    if candidate.knots_v[candidate.degree_v] != 0. || candidate.knots_v[n] != 1. {
        return Ok(None);
    }
    Ok(Some([
        boundary.coedges[0].curve.clone(),
        boundary.coedges[2].curve.clone(),
    ]))
}
/// Fits and contact curves all refer to the same original driving parameter.
/// Reports retain incomplete/mismatching sides; no partial success is promoted.
pub fn certify(
    candidate: &Surface,
    pcurves: [&Curve; 2],
    surfaces: [&Surface; 2],
    loops: [&[Vec<Curve>]; 2],
    distances: [f64; 2],
    axis: usize,
    drive: [f64; 2],
    free: [f64; 2],
    second: [[f64; 2]; 2],
    spans: usize,
    tolerance_mm: f64,
    tolerance_uv: f64,
    limits: Limits,
) -> Result<Report> {
    check(
        limits.uv_cells > 0
            && limits.uv_cells <= 100000
            && limits.agreement_cells > 0
            && limits.agreement_cells <= 100000
            && limits.root_refinements <= 16,
        "Contact trims need bounded UV, agreement and refinement work",
    )?;
    // Validate all authored inputs before any earlier prerequisite can stop.
    for side in 0..2 {
        surfaces[side].validate()?;
        pcurves[side].validate()?;
        check(
            pcurves[side].control_points[0].len() == 2 && pcurves[side].domain() == drive,
            "Source contact curves need 2D original driving domains",
        )?;
        TrimDomain::new(loops[side], tolerance_uv)?;
    }
    let fit = fit::certify(
        candidate,
        surfaces,
        distances,
        axis,
        drive,
        free,
        second,
        spans,
        tolerance_mm,
        limits.fit_cells,
    )?;
    let trim = trimmed::certify(
        surfaces,
        loops,
        distances,
        axis,
        drive,
        free,
        second,
        spans,
        tolerance_uv,
        limits.trim,
    )?;
    let domain_cells = trim.domain_cells;
    let world_curves = boundaries(candidate)?;
    let mut out = Report {
        source_pcurves: [pcurves[0].clone(), pcurves[1].clone()],
        fit,
        trim,
        world_curves,
        uv_reports: vec![],
        agreements: vec![],
        classifications: vec![],
        uv_cells: 0,
        agreement_cells: 0,
        domain_cells,
        patch_boundary_identity: false,
        contact_trim_curves_proven: false,
        reason: "finite-patch-unqualified",
    };
    if !out.fit.approximation_proven {
        return Ok(out);
    }
    if !out.trim.admitted {
        out.reason = "original-contact-trim-unqualified";
        return Ok(out);
    }
    if out.world_curves.is_none() {
        out.reason = "patch-boundary-identity-unproven";
        return Ok(out);
    }
    out.patch_boundary_identity = true;
    for side in 0..2 {
        if out.uv_cells == limits.uv_cells {
            out.reason = "pcurve-work-limit";
            return Ok(out);
        }
        let r = uv::certify(
            pcurves[side],
            side,
            surfaces,
            distances,
            axis,
            drive,
            free,
            second,
            spans,
            tolerance_uv,
            limits.uv_cells - out.uv_cells,
            limits.root_refinements,
        )?;
        out.uv_cells += r.visited;
        let valid = r.correspondence_proven;
        out.uv_reports.push(r);
        if !valid {
            out.reason = "pcurve-contact-unqualified";
            return Ok(out);
        }
        if out.domain_cells == limits.trim.max_domain_cells {
            out.reason = "source-pcurve-domain-work-limit";
            return Ok(out);
        }
        let bounds = agreement::curve_bounds(pcurves[side], Interval::new(drive[0], drive[1])?)?;
        let domain = TrimDomain::new(loops[side], tolerance_uv)?;
        let r = domain.classify(
            [[bounds[0].lo, bounds[0].hi], [bounds[1].lo, bounds[1].hi]],
            (limits.trim.max_domain_cells - out.domain_cells).min(100000),
        )?;
        out.domain_cells += r.cells;
        let location = r.location;
        out.classifications.push(r);
        if location != Location::Inside {
            out.reason = if location == Location::Outside {
                "source-pcurve-outside-trim"
            } else {
                "source-pcurve-trim-unresolved"
            };
            return Ok(out);
        }
        if out.agreement_cells == limits.agreement_cells {
            out.reason = "source-pcurve-agreement-work-limit";
            return Ok(out);
        }
        let r = agreement::verify(
            &out.world_curves.as_ref().unwrap()[side],
            pcurves[side],
            surfaces[side],
            false,
            tolerance_mm,
            limits.agreement_cells - out.agreement_cells,
        )?;
        out.agreement_cells += r.cells;
        let status = r.status;
        out.agreements.push(r);
        if status != agreement::Status::WithinTolerance {
            out.reason = if status == agreement::Status::Mismatch {
                "source-pcurve-world-mismatch"
            } else {
                "source-pcurve-world-unresolved"
            };
            return Ok(out);
        }
    }
    out.contact_trim_curves_proven = true;
    out.reason = "contact-trim-curves-qualified";
    Ok(out)
}
/// Additional independent work per each of the two contact boundaries.
#[derive(Clone, Copy)]
pub struct TangentLimits {
    pub position_cells: usize,
    pub normal_cells: usize,
    pub normal_spans: usize,
}
pub struct TangentReport {
    pub contacts: Report,
    pub tangent_planes: Vec<crate::contact_normal_agreement::Report>,
    pub contact_curves_and_tangent_planes_proven: bool,
    pub reason: &'static str,
}
/// Fresh fit, original UV branch/trim and contact-angle checks on one snapshot.
/// This admits contact geometry only, never a replacement face or fillet solid.
pub fn certify_with_tangency(
    candidate: &Surface,
    pcurves: [&Curve; 2],
    surfaces: [&Surface; 2],
    loops: [&[Vec<Curve>]; 2],
    distances: [f64; 2],
    axis: usize,
    drive: [f64; 2],
    free: [f64; 2],
    second: [[f64; 2]; 2],
    spans: usize,
    tolerance_mm: f64,
    tolerance_uv: f64,
    limits: Limits,
    max_sine_squared: f64,
    tangent_limits: TangentLimits,
) -> Result<TangentReport> {
    check(
        max_sine_squared.is_finite()
            && (0. ..1.).contains(&max_sine_squared)
            && [
                tangent_limits.position_cells,
                tangent_limits.normal_cells,
                tangent_limits.normal_spans,
            ]
            .iter()
            .all(|n| (1..=100000).contains(n)),
        "Contact tangency needs squared-sine tolerance in [0,1) and bounded positive work",
    )?;
    // Normal qualification needs nonperiodic pcurves, checked before fit refusal.
    for pcurve in pcurves {
        pcurve.validate()?;
        check(
            !pcurve.periodic,
            "Contact tangency needs nonperiodic source pcurves",
        )?;
    }
    let contacts = certify(
        candidate,
        pcurves,
        surfaces,
        loops,
        distances,
        axis,
        drive,
        free,
        second,
        spans,
        tolerance_mm,
        tolerance_uv,
        limits,
    )?;
    let mut out = TangentReport {
        contacts,
        tangent_planes: vec![],
        contact_curves_and_tangent_planes_proven: false,
        reason: "contact-trim-curves-unqualified",
    };
    if !out.contacts.contact_trim_curves_proven {
        return Ok(out);
    }
    let boundary = crate::offset_patch_boundary::extract(candidate)?
        .expect("Qualified contact curves have clamped patch boundaries");
    for side in 0..2 {
        let edge = &boundary.coedges[if side == 0 { 0 } else { 2 }];
        // Keep canonical world traversal, independent of patch coedge reversal.
        let patch_pcurve = Curve {
            degree: 1,
            knots: vec![drive[0], drive[0], drive[1], drive[1]],
            control_points: vec![
                vec![drive[0], edge.fixed_parameter],
                vec![drive[1], edge.fixed_parameter],
            ],
            weights: vec![1.; 2],
            periodic: false,
        };
        let r = crate::contact_normal_agreement::certify(
            &edge.curve,
            [surfaces[side], candidate],
            [pcurves[side], &patch_pcurve],
            [false, false],
            tolerance_mm,
            max_sine_squared,
            tangent_limits.position_cells,
            tangent_limits.normal_cells,
            tangent_limits.normal_spans,
        )?;
        out.tangent_planes.push(r);
    }
    out.contact_curves_and_tangent_planes_proven = out
        .tangent_planes
        .iter()
        .all(|r| r.positions_proven && r.tangent_planes_proven);
    out.reason = if out.contact_curves_and_tangent_planes_proven {
        "contact-tangent-planes-qualified"
    } else {
        "contact-tangent-planes-unqualified"
    };
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn pair() -> [Surface; 2] {
        let a = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let mut b = a.clone();
        for row in &mut b.control_points {
            for p in row {
                let z = p[1];
                p[1] = 0.5;
                p[2] = z;
            }
        }
        [a, b]
    }
    fn rectangle(lo: [f64; 2], hi: [f64; 2], reverse: bool) -> Vec<Curve> {
        let mut points = vec![lo, [hi[0], lo[1]], hi, [lo[0], hi[1]]];
        if reverse {
            points.reverse();
        }
        (0..4)
            .map(|i| Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                control_points: vec![points[i].to_vec(), points[(i + 1) % 4].to_vec()],
                weights: vec![1.; 2],
                periodic: false,
            })
            .collect()
    }
    fn limits() -> Limits {
        Limits {
            fit_cells: 511,
            uv_cells: 511,
            agreement_cells: 511,
            root_refinements: 3,
            trim: trimmed::Limits {
                max_pairs: 1000,
                max_cells: 10000,
                max_domain_cells: 10000,
            },
        }
    }
    fn query(
        candidate: &Surface,
        pc: [&Curve; 2],
        s: [&Surface; 2],
        loops: [&[Vec<Curve>]; 2],
        limits: Limits,
    ) -> Report {
        certify(
            candidate,
            pc,
            s,
            loops,
            [0.2, 0.2],
            0,
            [0.35, 0.39],
            [0.25, 0.35],
            [[0.30, 0.44], [0.15, 0.25]],
            2,
            1e-3,
            1e-6,
            limits,
        )
        .unwrap()
    }
    #[test]
    fn both_contact_curves_follow_original_regions_and_patch_columns() {
        let [a, b] = pair();
        let c = fit::propose(
            [&a, &b],
            [0.2, 0.2],
            0,
            [0.35, 0.39],
            [0.25, 0.35],
            [[0.30, 0.44], [0.15, 0.25]],
            2,
        )
        .unwrap()
        .unwrap();
        let pc = uv::propose(
            [&a, &b],
            [0.2, 0.2],
            0,
            [0.35, 0.39],
            [0.25, 0.35],
            [[0.30, 0.44], [0.15, 0.25]],
            2,
            3,
        )
        .unwrap()
        .unwrap();
        let loops = vec![rectangle([0., 0.], [1., 1.], false)];
        let r = query(&c, [&pc[0], &pc[1]], [&a, &b], [&loops, &loops], limits());
        assert!(r.contact_trim_curves_proven, "{}", r.reason);
        assert!(r.patch_boundary_identity);
        let tangent = |normal_spans| {
            certify_with_tangency(
                &c,
                [&pc[0], &pc[1]],
                [&a, &b],
                [&loops, &loops],
                [0.2, 0.2],
                0,
                [0.35, 0.39],
                [0.25, 0.35],
                [[0.30, 0.44], [0.15, 0.25]],
                2,
                1e-3,
                1e-6,
                limits(),
                1e-10,
                TangentLimits {
                    position_cells: 10000,
                    normal_cells: 1000,
                    normal_spans,
                },
            )
            .unwrap()
        };
        let qualified = tangent(10000);
        assert!(
            qualified.contact_curves_and_tangent_planes_proven,
            "{}",
            qualified.reason
        );
        assert_eq!(qualified.tangent_planes.len(), 2);
        let stopped = tangent(1);
        assert!(stopped.contacts.contact_trim_curves_proven);
        assert!(!stopped.contact_curves_and_tangent_planes_proven);
        assert!(
            stopped
                .tangent_planes
                .iter()
                .all(|r| !r.tangent_planes_proven)
        );

        let value = r.to_value();
        for key in [
            "originalWorldBoundaryIdentityProven",
            "replacementFaceTrimsProven",
            "stitchedTopologyProven",
            "wholeCurveComplete",
            "tangentToleranceProven",
            "embeddingProven",
            "topologyAuthority",
        ] {
            assert_eq!(value[key], value_codec::json!(false));
        }
        assert_eq!(
            value["sourceParameterReports"][0]["side"],
            value_codec::json!(0)
        );
        assert_eq!(
            value["sourceParameterReports"][1]["side"],
            value_codec::json!(1)
        );
        assert_eq!(r.uv_reports.len(), 2);
        assert_eq!(r.agreements.len(), 2);
        assert_eq!(r.classifications.len(), 2);
        assert!(r.uv_cells <= 511 && r.agreement_cells <= 511 && r.domain_cells <= 10000);
    }
    #[test]
    fn curved_contact_trims_keep_uv_world_and_patch_correspondence_in_rotated_frames() {
        let a = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![[3., 0.], [3., 3.], [0., 3.]]
                .into_iter()
                .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 5.]])
                .collect(),
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.]
                .into_iter()
                .map(|w| vec![w; 2])
                .collect(),
            periodic_u: false,
            periodic_v: false,
        };
        let [_, mut b] = pair();
        for row in &mut b.control_points {
            for p in row {
                let y = 4. * p[0];
                let z = 5. * p[2];
                *p = vec![2. - z, y, z];
            }
        }
        let x = 2. + 0.2 * 2_f64.sqrt() - 0.5;
        let y = (3.2_f64.powi(2) - x * x).sqrt();
        let mut u = 0.5;
        for _ in 0..8 {
            let e = crate::surface_offset::evaluate(&a, [u, 0.1], 0.2).unwrap();
            u -= (e.point[0] - x) / e.du[0];
        }
        let bu = y / 4.;
        let bv = (0.5 - 0.2 / 2_f64.sqrt()) / 5.;
        let drive = [0.0999, 0.1001];
        let free = [u - 0.005, u + 0.005];
        let second = [[bu - 0.005, bu + 0.005], [bv - 0.005, bv + 0.005]];
        let loops = vec![rectangle([0., 0.], [1., 1.], false)];
        for rotated in [false, true] {
            let mut aa = a.clone();
            let mut bb = b.clone();
            if rotated {
                for s in [&mut aa, &mut bb] {
                    for row in &mut s.control_points {
                        for p in row {
                            *p = vec![p[2] + 17., p[0] - 9., p[1] + 23.];
                        }
                    }
                }
            }
            let c = fit::propose([&aa, &bb], [0.2, 0.2], 1, drive, free, second, 2)
                .unwrap()
                .unwrap();
            let pc = uv::propose([&aa, &bb], [0.2, 0.2], 1, drive, free, second, 2, 3)
                .unwrap()
                .unwrap();
            let r = certify(
                &c,
                [&pc[0], &pc[1]],
                [&aa, &bb],
                [&loops, &loops],
                [0.2, 0.2],
                1,
                drive,
                free,
                second,
                2,
                1e-3,
                1e-6,
                limits(),
            )
            .unwrap();
            assert!(r.contact_trim_curves_proven, "{rotated}: {}", r.reason);
            for report in &r.uv_reports {
                assert!(report.correspondence_proven);
                assert!(report.section_queries <= 4 * report.visited);
            }
            assert!(
                r.agreements
                    .iter()
                    .all(|r| r.status == agreement::Status::WithinTolerance)
            );
            assert!(
                r.classifications
                    .iter()
                    .all(|r| r.location == Location::Inside)
            );
        }
    }
    #[test]
    fn modified_pcurve_and_malformed_second_operand_cannot_be_hidden_by_earlier_stops() {
        let [a, b] = pair();
        let c = fit::propose(
            [&a, &b],
            [0.2, 0.2],
            0,
            [0.35, 0.39],
            [0.25, 0.35],
            [[0.30, 0.44], [0.15, 0.25]],
            2,
        )
        .unwrap()
        .unwrap();
        let mut pc = uv::propose(
            [&a, &b],
            [0.2, 0.2],
            0,
            [0.35, 0.39],
            [0.25, 0.35],
            [[0.30, 0.44], [0.15, 0.25]],
            2,
            3,
        )
        .unwrap()
        .unwrap();
        let loops = vec![rectangle([0., 0.], [1., 1.], false)];
        for p in &mut pc[1].control_points {
            p[1] += 0.01;
        }
        let r = query(&c, [&pc[0], &pc[1]], [&a, &b], [&loops, &loops], limits());
        assert!(!r.contact_trim_curves_proven);
        assert_eq!(r.reason, "pcurve-contact-unqualified");
        assert_eq!(r.uv_reports[1].reason, "pcurve-contact-mismatch");
        for p in &mut pc[1].control_points {
            p.push(0.);
        }
        let mut work = limits();
        work.fit_cells = 1;
        assert!(
            certify(
                &c,
                [&pc[0], &pc[1]],
                [&a, &b],
                [&loops, &loops],
                [0.2, 0.2],
                0,
                [0.35, 0.39],
                [0.25, 0.35],
                [[0.30, 0.44], [0.15, 0.25]],
                2,
                1e-3,
                1e-6,
                work
            )
            .is_err()
        );
    }
    #[test]
    fn original_holes_and_shared_work_stop_cannot_gain_contact_trims() {
        let [a, b] = pair();
        let c = fit::propose(
            [&a, &b],
            [0.2, 0.2],
            0,
            [0.35, 0.39],
            [0.25, 0.35],
            [[0.30, 0.44], [0.15, 0.25]],
            2,
        )
        .unwrap()
        .unwrap();
        let pc = uv::propose(
            [&a, &b],
            [0.2, 0.2],
            0,
            [0.35, 0.39],
            [0.25, 0.35],
            [[0.30, 0.44], [0.15, 0.25]],
            2,
            3,
        )
        .unwrap()
        .unwrap();
        let loops = vec![rectangle([0., 0.], [1., 1.], false)];
        let holes = vec![
            loops[0].clone(),
            rectangle([0.33, 0.27], [0.41, 0.33], true),
        ];
        let r = query(&c, [&pc[0], &pc[1]], [&a, &b], [&holes, &loops], limits());
        assert!(!r.contact_trim_curves_proven);
        assert_eq!(r.reason, "original-contact-trim-unqualified");
        let mut work = limits();
        work.uv_cells = 1;
        let r = query(&c, [&pc[0], &pc[1]], [&a, &b], [&loops, &loops], work);
        assert!(!r.contact_trim_curves_proven);
        assert_eq!(r.reason, "pcurve-work-limit");
        assert_eq!(r.uv_reports.len(), 1);
        assert_eq!(r.uv_cells, 1);
    }
}
