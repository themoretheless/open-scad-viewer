//! Admit a uniform offset contact band on original UV trim regions.
//! Region roles, exact closure and simplicity are prerequisites. This does not
//! certify world coedge identity, create replacement trims or admit a solid.
use crate::{
    Result, check,
    curve::Curve,
    surface::Surface,
    surface_offset::{self, ContactBand},
    trim_domain::{Classification, Location, TrimDomain},
    trim_region_audit,
};
#[derive(Clone, Copy)]
pub struct Limits {
    pub max_pairs: usize,
    pub max_cells: usize,
    pub max_domain_cells: usize,
}
pub struct Report {
    pub contact: ContactBand,
    pub admitted: bool,
    pub reason: &'static str,
    pub regions: Vec<trim_region_audit::Report>,
    pub classifications: Vec<Classification>,
    pub pairs: usize,
    pub cells: usize,
    pub domain_cells: usize,
}
impl Report {
    pub fn to_value(&self) -> value_codec::Value {
        use value_codec::json;
        let (status, witness) = match &self.contact {
            ContactBand::Excluded => ("excluded", value_codec::Value::Null),
            ContactBand::Unresolved => ("unresolved", value_codec::Value::Null),
            ContactBand::ContinuousBranch(w) => (
                "continuous-branch",
                json!({"firstUV":w.first_uv,"secondUV":w.second_uv,"centerIntervalMm":w.point,"contractionUpper":w.contraction_upper}),
            ),
        };
        json!({"method":"interval-offset-band-trim-admission","scope":"audited-original-uv-regions",
            "contactStatus":status,"witness":witness,"continuousBranchProven":status=="continuous-branch",
            "trimMembershipProven":self.admitted,"reason":self.reason,"wholeCurveComplete":false,
            "worldCoedgeIdentityProven":false,"topologyAuthority":false,
            "pairs":self.pairs,"cells":self.cells,"domainCells":self.domain_cells,
            "regions":self.regions.iter().map(|r|json!({"valid":r.valid,"reason":r.reason,"pairs":r.pairs,"cells":r.cells,"domainCells":r.domain_cells,"problemLoops":r.problem_loops})).collect::<Vec<_>>(),
            "classifications":self.classifications.iter().map(Classification::to_value).collect::<Vec<_>>()})
    }
}
/// The complete certified source-parameter enclosures must be inside both
/// audited regions. Every audit/classification shares the declared budgets.
pub fn certify(
    surfaces: [&Surface; 2],
    loops: [&[Vec<Curve>]; 2],
    distances: [f64; 2],
    fixed_axis: usize,
    fixed_interval: [f64; 2],
    first_other: [f64; 2],
    second: [[f64; 2]; 2],
    max_spans: usize,
    tolerance_uv: f64,
    limits: Limits,
) -> Result<Report> {
    check(
        (1..=100000).contains(&limits.max_pairs)
            && (1..=100000).contains(&limits.max_cells)
            && (1..=1000000).contains(&limits.max_domain_cells),
        "Trim contact needs bounded positive work limits",
    )?;
    // Validate both inputs before an exclusion or early work stop can hide one.
    let domains = [
        TrimDomain::new(loops[0], tolerance_uv)?,
        TrimDomain::new(loops[1], tolerance_uv)?,
    ];
    let contact = surface_offset::certify_contact_band(
        surfaces,
        distances,
        fixed_axis,
        fixed_interval,
        first_other,
        second,
        max_spans,
    )?;
    let mut out = Report {
        contact,
        admitted: false,
        reason: "offset-contact-unresolved",
        regions: Vec::new(),
        classifications: Vec::new(),
        pairs: 0,
        cells: 0,
        domain_cells: 0,
    };
    let uv = match &out.contact {
        ContactBand::Excluded => {
            out.reason = "offset-carriers-excluded";
            return Ok(out);
        }
        ContactBand::Unresolved => return Ok(out),
        ContactBand::ContinuousBranch(w) => [w.first_uv, w.second_uv],
    };
    for side in 0..2 {
        if out.pairs == limits.max_pairs
            || out.cells == limits.max_cells
            || out.domain_cells == limits.max_domain_cells
        {
            out.reason = "trim-work-limit";
            return Ok(out);
        }
        let r = trim_region_audit::inspect(
            loops[side],
            tolerance_uv,
            limits.max_pairs - out.pairs,
            limits.max_cells - out.cells,
            limits.max_domain_cells - out.domain_cells,
        )?;
        out.pairs += r.pairs;
        out.cells += r.cells;
        out.domain_cells += r.domain_cells;
        let valid = r.valid;
        out.regions.push(r);
        if valid != Some(true) {
            out.reason = if valid == Some(false) {
                "invalid-trim-region"
            } else {
                "trim-region-unresolved"
            };
            return Ok(out);
        }
    }
    for side in 0..2 {
        if out.domain_cells == limits.max_domain_cells {
            out.reason = "trim-work-limit";
            return Ok(out);
        }
        let r = domains[side].classify(
            uv[side],
            (limits.max_domain_cells - out.domain_cells).min(100000),
        )?;
        out.domain_cells += r.cells;
        let location = r.location;
        out.classifications.push(r);
        match location {
            Location::Outside => {
                out.reason = "contact-outside-trim";
                return Ok(out);
            }
            Location::Unresolved => {
                out.reason = "contact-trim-unresolved";
                return Ok(out);
            }
            Location::Inside => {}
        }
    }
    out.admitted = true;
    out.reason = "contact-band-inside-trims";
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
    fn rectangle(lo: [f64; 2], hi: [f64; 2], reverse: bool) -> Vec<Curve> {
        let mut points = vec![lo, [hi[0], lo[1]], hi, [lo[0], hi[1]]];
        if reverse {
            points.reverse()
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
    fn query(first: Vec<Vec<Curve>>, limits: Limits) -> Report {
        let a = plane();
        let mut b = a.clone();
        for row in &mut b.control_points {
            for p in row {
                let z = p[1];
                p[1] = 0.5;
                p[2] = z
            }
        }
        let second = vec![rectangle([0., 0.], [1., 1.], false)];
        certify(
            [&a, &b],
            [&first, &second],
            [0.2, 0.2],
            0,
            [0.35, 0.39],
            [0.25, 0.35],
            [[0.30, 0.44], [0.15, 0.25]],
            2,
            1e-7,
            limits,
        )
        .unwrap()
    }
    fn limits() -> Limits {
        Limits {
            max_pairs: 10000,
            max_cells: 10000,
            max_domain_cells: 100000,
        }
    }
    #[test]
    fn whole_contact_band_must_be_inside_both_audited_original_regions() {
        let r = query(vec![rectangle([0., 0.], [1., 1.], false)], limits());
        assert!(r.admitted, "{}", r.reason);
        let json = r.to_value();
        assert_eq!(json["trimMembershipProven"], true);
        assert_eq!(json["worldCoedgeIdentityProven"], false);
        assert_eq!(json["topologyAuthority"], false);
        assert_eq!(r.regions.len(), 2);
        assert_eq!(r.classifications.len(), 2);
        let r = query(
            vec![
                rectangle([0., 0.], [1., 1.], false),
                rectangle([0.34, 0.29], [0.40, 0.31], true),
            ],
            limits(),
        );
        assert!(!r.admitted);
        assert_eq!(r.reason, "contact-outside-trim");
        // The middle contact lies inside this face, but a driving endpoint is
        // outside. Uniform rectangle classification must not admit the band.
        let r = query(vec![rectangle([0.36, 0.1], [0.9, 0.9], false)], limits());
        assert!(!r.admitted);
        assert_eq!(r.reason, "contact-trim-unresolved");
    }
    #[test]
    fn rational_outer_contours_and_disjoint_holes_preserve_admission() {
        let mut outer = rectangle([0., 0.], [1., 1.], false);
        outer[0] = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.5, 1., 1., 1.],
            control_points: vec![
                vec![0., 0.],
                vec![0.25, -0.1],
                vec![0.75, -0.1],
                vec![1., 0.],
            ],
            weights: vec![1., 0.8, 1.2, 1.],
            periodic: false,
        };
        let loops = vec![outer, rectangle([0.7, 0.7], [0.8, 0.8], true)];
        let before = format!("{loops:?}");
        let r = query(loops.clone(), limits());
        assert!(r.admitted, "{}", r.reason);
        assert_eq!(format!("{loops:?}"), before);
    }
    #[test]
    fn region_roles_and_shared_work_limits_cannot_be_bypassed() {
        let r = query(
            vec![
                rectangle([0., 0.], [1., 1.], false),
                rectangle([0.7, 0.7], [0.8, 0.8], false),
            ],
            limits(),
        );
        assert!(!r.admitted);
        assert_eq!(r.reason, "invalid-trim-region");
        let cap = Limits {
            max_pairs: 1,
            max_cells: 1,
            max_domain_cells: 1,
        };
        let r = query(vec![rectangle([0., 0.], [1., 1.], false)], cap);
        assert!(!r.admitted);
        assert!(
            r.pairs <= cap.max_pairs
                && r.cells <= cap.max_cells
                && r.domain_cells <= cap.max_domain_cells
        );
    }
}
