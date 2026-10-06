//! Link admitted offset contacts to every original spatial coedge.
//! Exact identity and full-parameter tolerance agreement remain separate.
use crate::{
    Result, check, curve::Curve, curve_surface_agreement as agreement, surface::Surface,
    trimmed_offset_contact as trimmed,
};
pub struct Coedge {
    pub world: Curve,
    pub reversed: bool,
}
impl<'de> value_codec::Deserialize<'de> for Coedge {
    fn from_value(v: value_codec::Value) -> value_codec::Result<Self> {
        Ok(Self {
            world: value_codec::from_value(v["world"].clone())?,
            reversed: value_codec::from_value(v["reversed"].clone())?,
        })
    }
}
pub struct Audit {
    pub side: usize,
    pub loop_index: usize,
    pub curve_index: usize,
    pub exact_status: &'static str,
    pub agreement_status: &'static str,
    pub exact_work: u64,
    pub cells: usize,
    pub witness: Option<f64>,
    pub witness_distance: Option<[f64; 2]>,
}
pub struct Report {
    pub trim: trimmed::Report,
    pub exact_identity: bool,
    pub within_tolerance: bool,
    pub reason: &'static str,
    pub total_coedges: usize,
    pub checked_coedges: usize,
    pub exact_work: u64,
    pub agreement_cells: usize,
    pub tolerance_mm: f64,
    pub max_exact_work: u64,
    pub max_agreement_cells: usize,
    pub audits: Vec<Audit>,
}
impl Report {
    pub fn to_value(&self) -> value_codec::Value {
        use value_codec::json;
        json!({"method":"offset-source-boundary-admission","scope":"original-trims-and-spatial-coedges",
   "trim":self.trim.to_value(),"worldCoedgeIdentityProven":self.exact_identity,"worldBoundaryWithinToleranceProven":self.within_tolerance,
   "reason":self.reason,"totalCoedges":self.total_coedges,"checkedCoedges":self.checked_coedges,
   "exactWork":self.exact_work,"agreementCells":self.agreement_cells,"toleranceMm":self.tolerance_mm,"maxExactWork":self.max_exact_work,"maxAgreementCells":self.max_agreement_cells,"topologyAuthority":false,"wholeCurveComplete":false,
   "audits":self.audits.iter().map(|a|json!({"side":a.side,"loop":a.loop_index,"curve":a.curve_index,"exactStatus":a.exact_status,"agreementStatus":a.agreement_status,"exactWork":a.exact_work,"cells":a.cells,"witnessParameter":a.witness,"witnessDistanceMm":a.witness_distance})).collect::<Vec<_>>()})
    }
}
pub fn certify(
    surfaces: [&Surface; 2],
    loops: [&[Vec<Curve>]; 2],
    coedges: [&[Vec<Coedge>]; 2],
    distances: [f64; 2],
    fixed_axis: usize,
    fixed_interval: [f64; 2],
    first_other: [f64; 2],
    second: [[f64; 2]; 2],
    max_spans: usize,
    tolerance_uv: f64,
    trim_limits: trimmed::Limits,
    tolerance_mm: f64,
    max_exact_work: u64,
    max_agreement_cells: usize,
) -> Result<Report> {
    check(
        tolerance_mm.is_finite()
            && tolerance_mm > 0.
            && max_exact_work <= 10000000
            && (1..=100000).contains(&max_agreement_cells),
        "Source boundary needs a positive tolerance and bounded work",
    )?;
    let mut total = 0;
    for side in 0..2 {
        surfaces[side].validate()?;
        check(
            loops[side].len() == coedges[side].len(),
            "Each original trim loop needs its spatial coedges",
        )?;
        for (uv, world) in loops[side].iter().zip(coedges[side]) {
            check(
                uv.len() == world.len(),
                "Each original trim curve needs one spatial coedge",
            )?;
            for (p, c) in uv.iter().zip(world) {
                p.validate()?;
                c.world.validate()?;
                check(
                    p.control_points[0].len() == 2 && c.world.control_points[0].len() == 3,
                    "Source boundaries need 2D pcurves and 3D world curves",
                )?;
                total += 1;
            }
        }
    }
    let trim = trimmed::certify(
        surfaces,
        loops,
        distances,
        fixed_axis,
        fixed_interval,
        first_other,
        second,
        max_spans,
        tolerance_uv,
        trim_limits,
    )?;
    let mut out = Report {
        trim,
        exact_identity: false,
        within_tolerance: false,
        reason: "contact-not-admitted-on-trims",
        total_coedges: total,
        checked_coedges: 0,
        exact_work: 0,
        agreement_cells: 0,
        tolerance_mm,
        max_exact_work,
        max_agreement_cells,
        audits: Vec::new(),
    };
    if !out.trim.admitted {
        return Ok(out);
    }
    let mut all_exact = true;
    for side in 0..2 {
        for (li, (uv, world)) in loops[side].iter().zip(coedges[side]).enumerate() {
            for (ci, (p, c)) in uv.iter().zip(world).enumerate() {
                let decision = agreement::verify_exact(
                    &c.world,
                    p,
                    surfaces[side],
                    c.reversed,
                    max_exact_work - out.exact_work,
                )?;
                let (exact_status, work) = match decision {
                    None => ("unsupported", 0),
                    Some(d) => {
                        use cad_predicates::BezierIdentity;
                        let status = match d.outcome {
                            BezierIdentity::Equal => "equal",
                            BezierIdentity::Different => "different",
                            BezierIdentity::Indeterminate(_) => "unresolved",
                        };
                        (status, d.work_used)
                    }
                };
                check(
                    work <= max_exact_work - out.exact_work,
                    "Exact boundary work exceeded its shared limit",
                )?;
                out.exact_work += work;
                let mut audit = Audit {
                    side,
                    loop_index: li,
                    curve_index: ci,
                    exact_status,
                    agreement_status: "exact",
                    exact_work: work,
                    cells: 0,
                    witness: None,
                    witness_distance: None,
                };
                if exact_status != "equal" {
                    all_exact = false;
                    if out.agreement_cells == max_agreement_cells {
                        audit.agreement_status = "unresolved";
                        out.audits.push(audit);
                        out.reason = "boundary-work-limit";
                        return Ok(out);
                    }
                    let r = agreement::verify(
                        &c.world,
                        p,
                        surfaces[side],
                        c.reversed,
                        tolerance_mm,
                        max_agreement_cells - out.agreement_cells,
                    )?;
                    out.agreement_cells += r.cells;
                    audit.cells = r.cells;
                    audit.witness = r.witness;
                    audit.witness_distance = r.witness_distance;
                    audit.agreement_status = match r.status {
                        agreement::Status::WithinTolerance => "within-tolerance",
                        agreement::Status::Mismatch => "mismatch",
                        agreement::Status::Unresolved => "unresolved",
                    };
                }
                let accepted = audit.agreement_status == "exact"
                    || audit.agreement_status == "within-tolerance";
                out.checked_coedges += 1;
                let status = audit.agreement_status;
                out.audits.push(audit);
                if !accepted {
                    out.reason = if status == "mismatch" {
                        "spatial-boundary-mismatch"
                    } else {
                        "spatial-boundary-unresolved"
                    };
                    return Ok(out);
                }
            }
        }
    }
    out.exact_identity = all_exact;
    out.within_tolerance = true;
    out.reason = if all_exact {
        "exact-source-boundaries"
    } else {
        "source-boundaries-within-tolerance"
    };
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn plane() -> Surface {
        Surface {
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
        }
    }
    fn rectangle() -> Vec<Curve> {
        let p = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
        (0..4)
            .map(|i| Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                control_points: vec![p[i].to_vec(), p[(i + 1) % 4].to_vec()],
                weights: vec![1.; 2],
                periodic: false,
            })
            .collect()
    }
    fn query(damage: f64, multispan: bool, reverse: bool, cells: usize) -> Report {
        query_limited(damage, multispan, reverse, cells, 1000000)
    }
    fn query_limited(
        damage: f64,
        multispan: bool,
        reverse: bool,
        cells: usize,
        exact_work: u64,
    ) -> Report {
        let a = plane();
        let mut b = a.clone();
        for row in &mut b.control_points {
            for p in row {
                let z = p[1];
                p[1] = 0.5;
                p[2] = z
            }
        }
        let mut first = vec![rectangle()];
        let second = vec![rectangle()];
        if multispan {
            first[0][0] = Curve {
                degree: 2,
                knots: vec![0., 0., 0., 0.5, 1., 1., 1.],
                control_points: vec![vec![0., 0.], vec![0.25, 0.1], vec![0.75, 0.1], vec![1., 0.]],
                weights: vec![1., 0.8, 1.2, 1.],
                periodic: false,
            };
        }
        let world = |loops: &[Vec<Curve>], side: usize| {
            loops
                .iter()
                .map(|l| {
                    l.iter()
                        .map(|p| {
                            let mut c = p.clone();
                            c.control_points = p
                                .control_points
                                .iter()
                                .map(|uv| {
                                    if side == 0 {
                                        vec![uv[0], uv[1], 0.]
                                    } else {
                                        vec![uv[0], 0.5, uv[1]]
                                    }
                                })
                                .collect();
                            if reverse {
                                c = c.reverse().unwrap();
                            }
                            Coedge {
                                world: c,
                                reversed: reverse,
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        };
        let mut ca = world(&first, 0);
        let cb = world(&second, 1);
        ca[0][0].world.control_points[0][2] += damage;
        certify(
            [&a, &b],
            [&first, &second],
            [&ca, &cb],
            [0.2, 0.2],
            0,
            [0.35, 0.39],
            [0.25, 0.35],
            [[0.30, 0.44], [0.15, 0.25]],
            2,
            1e-7,
            trimmed::Limits {
                max_pairs: 10000,
                max_cells: 10000,
                max_domain_cells: 100000,
            },
            1e-5,
            exact_work,
            cells,
        )
        .unwrap()
    }
    #[test]
    fn all_original_coedges_require_full_parameter_agreement() {
        for reversed in [false, true] {
            let r = query(0., false, reversed, 10000);
            assert!(r.exact_identity && r.within_tolerance, "{}", r.reason);
            assert_eq!(r.checked_coedges, 8);
            assert_eq!(r.agreement_cells, 0);
        }
        let r = query(1e-3, false, false, 10000);
        assert!(!r.within_tolerance && !r.exact_identity);
        assert_eq!(r.reason, "spatial-boundary-mismatch");
        assert!(r.audits[0].witness_distance.unwrap()[0] > 1e-5);
        let r = query(1e-12, false, false, 10000);
        assert!(r.within_tolerance && !r.exact_identity, "{}", r.reason);
    }
    #[test]
    fn shared_boundary_limits_retain_unvisited_coedges_and_never_promote_approximation() {
        let r = query_limited(0., false, false, 1, 0);
        assert!(!r.within_tolerance && !r.exact_identity);
        assert_eq!(r.reason, "boundary-work-limit");
        assert_eq!(r.exact_work, 0);
        assert!(r.agreement_cells <= 1);
        assert!(r.checked_coedges < r.total_coedges);
        let r = query_limited(0., false, false, 10000, 0);
        assert!(r.within_tolerance && !r.exact_identity, "{}", r.reason);
        assert_eq!(r.checked_coedges, 8);
        assert_eq!(r.exact_work, 0);
        let json = r.to_value();
        assert_eq!(json["worldBoundaryWithinToleranceProven"], true);
        assert_eq!(json["worldCoedgeIdentityProven"], false);
        assert_eq!(json["topologyAuthority"], false);
    }
    #[test]
    fn out_of_domain_pcurves_never_gain_source_boundary_admission() {
        let s = plane();
        let p = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![0., -0.1], vec![1., -0.1]],
            weights: vec![1.; 2],
            periodic: false,
        };
        let c = Curve {
            control_points: vec![vec![0., -0.1, 0.], vec![1., -0.1, 0.]],
            ..p.clone()
        };
        assert!(
            agreement::verify_exact(&c, &p, &s, false, 1000)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            agreement::verify(&c, &p, &s, false, 1e-5, 1000)
                .unwrap()
                .status,
            agreement::Status::Unresolved
        );
    }
    #[test]
    fn multispan_boundaries_keep_tolerance_and_exact_identity_separate() {
        let r = query(0., true, false, 100000);
        assert!(r.within_tolerance && !r.exact_identity, "{}", r.reason);
        assert_eq!(r.checked_coedges, 8);
        assert_eq!(r.audits[0].exact_status, "unsupported");
        assert_eq!(r.audits[0].agreement_status, "within-tolerance");
        let r = query(1e-3, true, false, 1);
        assert!(!r.within_tolerance);
        assert!(r.agreement_cells <= 1);
    }
}
