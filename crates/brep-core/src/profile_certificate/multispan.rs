//! Exact traversal of coincident multi-span images. Source knots need not
//! have rational preimages in the other curve: comparisons use image endpoints.
use super::*;
type Net = Vec<[Q; 3]>;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpanAgreementWitness {
    pub first_cell: usize,
    pub second_cell: usize,
    pub axis: u8,
    pub relation: Vec<Vec<[String; 2]>>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultispanAgreementCertificate {
    pub version: u32,
    pub source: String,
    pub subdivision_depth: u8,
    pub steps: Vec<SpanAgreementWitness>,
}
fn endpoint(net: &Net, end: bool) -> [Q; 2] {
    let p = if end { net.last().unwrap() } else { &net[0] };
    [&p[0] / &p[2], &p[1] / &p[2]]
}
pub(super) fn cells(c: &Curve, d: [f64; 2], depth: u8) -> Result<Vec<Net>> {
    if depth > 6 {
        return Err(refused("Multispan subdivision depth exceeded"));
    }
    let mut cuts = vec![d[0], d[1]];
    cuts.extend(
        c.knots
            .iter()
            .copied()
            .filter(|k| *k > d[0].min(d[1]) && *k < d[0].max(d[1])),
    );
    cuts.sort_by(f64::total_cmp);
    cuts.dedup();
    if d[0] > d[1] {
        cuts.reverse();
    }
    let mut out = Vec::new();
    for p in cuts.windows(2) {
        out.push(extraction::interval(
            c,
            &Q::from_float(p[0]).unwrap(),
            &Q::from_float(p[1]).unwrap(),
        )?);
    }
    for _ in 0..depth {
        out = out
            .iter()
            .flat_map(|net| {
                let (a, b) = split(net);
                [a, b]
            })
            .collect();
        if out.len() > 128 {
            return Err(refused("Multispan exact cell budget exceeded"));
        }
    }
    if out.is_empty() || out.len() > 128 {
        return Err(refused("Multispan exact cell count exceeded"));
    }
    for p in out.windows(2) {
        if endpoint(&p[0], true) != endpoint(&p[1], false) {
            return Err(refused("Multispan source seam is not exactly continuous"));
        }
    }
    Ok(out)
}
fn source(a: &Curve, b: &Curve, da: [f64; 2], db: [f64; 2]) -> Result<String> {
    Ok(format!(
        "multispan-image-v1/{}",
        agreement::source(a, b, da, db)?
    ))
}
fn advance(a: &Net, b: &Net, cursor: &[Q; 2], axis: usize) -> Result<(bool, bool, [Q; 2])> {
    if axis > 1 || !arithmetic::monotone(a, axis)? || !arithmetic::monotone(b, axis)? {
        return Err(refused(
            "Multispan witness has no common monotone coordinate",
        ));
    }
    let a0 = endpoint(a, false);
    let a1 = endpoint(a, true);
    let b0 = endpoint(b, false);
    let b1 = endpoint(b, true);
    let direction = if a1[axis] > a0[axis] {
        Q::one()
    } else {
        -Q::one()
    };
    if (&b1[axis] - &b0[axis]) * &direction <= Q::zero()
        || (&cursor[axis] - &a0[axis]) * &direction < Q::zero()
        || (&cursor[axis] - &b0[axis]) * &direction < Q::zero()
    {
        return Err(refused(
            "Multispan image traversal reverses or skips its cursor",
        ));
    }
    let end_a = (&a1[axis] - &cursor[axis]) * &direction;
    let end_b = (&b1[axis] - &cursor[axis]) * &direction;
    if end_a <= Q::zero() || end_b <= Q::zero() {
        return Err(refused("Multispan image overlap has no forward extent"));
    }
    if end_a == end_b {
        if a1 != b1 {
            return Err(refused("Multispan coincident endpoints disagree"));
        }
        Ok((true, true, a1))
    } else if end_a < end_b {
        Ok((true, false, a1))
    } else {
        Ok((false, true, b1))
    }
}
pub fn certify_multispan_agreement(
    a: &Curve,
    b: &Curve,
    da: [f64; 2],
    db: [f64; 2],
) -> Result<MultispanAgreementCertificate> {
    let source = source(a, b, da, db)?;
    // Exact affine identity needs no regularity or injective coordinate. This
    // also covers cusps and complete retracing, including closed paths.
    if agreement::check(a, b, da, db, &Q::one()).is_ok() {
        let c = MultispanAgreementCertificate {
            version: 2,
            source,
            subdivision_depth: 255,
            steps: Vec::new(),
        };
        verify_multispan_agreement(a, b, da, db, &c)?;
        return Ok(c);
    }
    for depth in 0..=6 {
        let attempt = (|| -> Result<MultispanAgreementCertificate> {
            let x = cells(a, da, depth)?;
            let y = cells(b, db, depth)?;
            let mut cursor = endpoint(&x[0], false);
            if cursor != endpoint(&y[0], false)
                || endpoint(x.last().unwrap(), true) != endpoint(y.last().unwrap(), true)
            {
                return Err(refused("Multispan whole-domain endpoints differ"));
            }
            let (mut i, mut j) = (0, 0);
            let mut steps = Vec::new();
            while i < x.len() && j < y.len() {
                let mut found = None;
                for axis in 0..2 {
                    if let Ok(next) = advance(&x[i], &y[j], &cursor, axis)
                        && let Ok(h) = arithmetic::bivariate::partial_relation(&x[i], &y[j], axis)
                    {
                        found = Some((axis, next, h));
                        break;
                    }
                }
                let (axis, (take_a, take_b, next), h) =
                    found.ok_or_else(|| refused("Multispan image overlap remains unproved"))?;
                steps.push(SpanAgreementWitness {
                    first_cell: i,
                    second_cell: j,
                    axis: axis as u8,
                    relation: h
                        .iter()
                        .map(|row| {
                            row.iter()
                                .map(|q| [q.numer().to_string(), q.denom().to_string()])
                                .collect()
                        })
                        .collect(),
                });
                cursor = next;
                i += usize::from(take_a);
                j += usize::from(take_b);
            }
            if i != x.len() || j != y.len() {
                return Err(refused(
                    "Multispan agreement leaves an uncovered source tail",
                ));
            }
            Ok(MultispanAgreementCertificate {
                version: 1,
                source: source.clone(),
                subdivision_depth: depth,
                steps,
            })
        })();
        if let Ok(c) = attempt {
            verify_multispan_agreement(a, b, da, db, &c)?;
            return Ok(c);
        }
    }
    Err(refused(
        "No complete multi-span coincidence proof within exact cell budgets",
    ))
}
pub fn verify_multispan_agreement(
    a: &Curve,
    b: &Curve,
    da: [f64; 2],
    db: [f64; 2],
    c: &MultispanAgreementCertificate,
) -> Result<VerifiedCurveAgreement> {
    if c.version == 2 {
        if c.source != source(a, b, da, db)? || c.subdivision_depth != 255 || !c.steps.is_empty() {
            return Err(refused(
                "Affine multi-span identity schema or source mismatch",
            ));
        }
        return Ok(VerifiedCurveAgreement {
            common_spans: agreement::check(a, b, da, db, &Q::one())?,
        });
    }
    if c.version != 1 || c.source != source(a, b, da, db)? || c.steps.len() > 256 {
        return Err(refused("Multispan agreement source or size mismatch"));
    }
    let x = cells(a, da, c.subdivision_depth)?;
    let y = cells(b, db, c.subdivision_depth)?;
    let mut cursor = endpoint(&x[0], false);
    if cursor != endpoint(&y[0], false) {
        return Err(refused("Multispan initial source points differ"));
    }
    let (mut i, mut j) = (0, 0);
    for step in &c.steps {
        if i >= x.len() || j >= y.len() || step.first_cell != i || step.second_cell != j {
            return Err(refused(
                "Multispan agreement omits or reorders an image cell",
            ));
        }
        let (take_a, take_b, next) = advance(&x[i], &y[j], &cursor, step.axis as usize)?;
        let h = nonlinear::decode(&step.relation)?;
        if !arithmetic::bivariate::verify_partial_relation(&x[i], &y[j], step.axis as usize, &h)? {
            return Err(refused(
                "Multispan partial image identity or coverage is false",
            ));
        }
        cursor = next;
        i += usize::from(take_a);
        j += usize::from(take_b);
    }
    if i != x.len()
        || j != y.len()
        || cursor != endpoint(x.last().unwrap(), true)
        || cursor != endpoint(y.last().unwrap(), true)
    {
        return Err(refused("Multispan coincidence coverage is incomplete"));
    }
    Ok(VerifiedCurveAgreement {
        common_spans: c.steps.len(),
    })
}
#[cfg(feature = "codec")]
mod serialization {
    use super::*;
    use value_codec::{Deserialize, Serialize, Value, error, json};
    impl Serialize for SpanAgreementWitness {
        fn to_value(&self) -> Value {
            json!({"firstCell":self.first_cell,"secondCell":self.second_cell,"axis":self.axis,"relation":self.relation})
        }
    }
    impl<'de> Deserialize<'de> for SpanAgreementWitness {
        fn from_value(v: Value) -> value_codec::Result<Self> {
            let keys = ["firstCell", "secondCell", "axis", "relation"];
            if v.as_object()
                .is_none_or(|o| o.len() != 4 || keys.iter().any(|k| !o.contains_key(*k)))
                || v["relation"].as_array().is_none_or(|a| {
                    a.len() > 18 || a.iter().any(|r| r.as_array().is_none_or(|r| r.len() > 257))
                })
            {
                return Err(error("Multispan relation schema or size mismatch"));
            }
            Ok(Self {
                first_cell: Deserialize::from_value(v["firstCell"].clone())?,
                second_cell: Deserialize::from_value(v["secondCell"].clone())?,
                axis: Deserialize::from_value(v["axis"].clone())?,
                relation: Deserialize::from_value(v["relation"].clone())?,
            })
        }
    }
    impl Serialize for MultispanAgreementCertificate {
        fn to_value(&self) -> Value {
            json!({"version":self.version,"source":self.source,"subdivisionDepth":self.subdivision_depth,"steps":self.steps,"scope":"represented-multispan-curve-image-agreement"})
        }
    }
    impl<'de> Deserialize<'de> for MultispanAgreementCertificate {
        fn from_value(v: Value) -> value_codec::Result<Self> {
            let keys = ["version", "source", "subdivisionDepth", "steps", "scope"];
            if v.as_object()
                .is_none_or(|o| o.len() != 5 || keys.iter().any(|k| !o.contains_key(*k)))
                || v["scope"].as_str() != Some("represented-multispan-curve-image-agreement")
                || v["source"].as_str().is_none_or(|s| s.len() > 66000)
                || v["steps"].as_array().is_none_or(|a| a.len() > 256)
            {
                return Err(error("Multispan certificate schema or size mismatch"));
            }
            Ok(Self {
                version: Deserialize::from_value(v["version"].clone())?,
                source: Deserialize::from_value(v["source"].clone())?,
                subdivision_depth: Deserialize::from_value(v["subdivisionDepth"].clone())?,
                steps: Deserialize::from_value(v["steps"].clone())?,
            })
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn polynomial(points: Vec<Vec<f64>>) -> Curve {
        let n = points.len();
        Curve {
            degree: n - 1,
            knots: [vec![0.; n], vec![1.; n]].concat(),
            weights: vec![1.; n],
            control_points: points,
            periodic: false,
        }
    }
    #[test]
    fn unequal_knot_partitions_and_nonlinear_speed_are_covered() {
        let a = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
            control_points: vec![
                vec![0., 0.],
                vec![2., 0.],
                vec![4., 1.],
                vec![6., 2.],
                vec![8., 4.],
            ],
            weights: vec![1.; 5],
            periodic: false,
        };
        let b = polynomial(vec![
            vec![0., 0.],
            vec![0., 0.],
            vec![8. / 6., 0.],
            vec![4., 0.],
            vec![8., 4.],
        ]);
        // Scale x to retain exact binary64 Bernstein coefficients for 48t².
        let mut a = a;
        for p in &mut a.control_points {
            p[0] *= 6.;
        }
        let mut b = b;
        b.control_points[2][0] = 8.;
        b.control_points[3][0] = 24.;
        b.control_points[4][0] = 48.;
        let c = certify_multispan_agreement(&a, &b, [0., 1.], [0., 1.]).unwrap();
        assert!(c.steps.len() >= 2);
        verify_multispan_agreement(&a, &b, [0., 1.], [0., 1.], &c).unwrap();
        let mut bad = c;
        bad.steps.pop();
        assert!(verify_multispan_agreement(&a, &b, [0., 1.], [0., 1.], &bad).is_err());
    }
    #[test]
    fn closed_multi_span_paths_and_reversed_traversal() {
        let a = Curve::from_polyline(vec![
            vec![0., 0.],
            vec![4., 0.],
            vec![4., 4.],
            vec![0., 4.],
            vec![0., 0.],
        ])
        .unwrap();
        let b = Curve::from_polyline(vec![
            vec![0., 0.],
            vec![2., 0.],
            vec![4., 0.],
            vec![4., 2.],
            vec![4., 4.],
            vec![0., 4.],
            vec![0., 0.],
        ])
        .unwrap();
        let da = a.domain();
        let db = b.domain();
        let c = certify_multispan_agreement(&a, &b, da, db).unwrap();
        verify_multispan_agreement(&a, &b, da, db, &c).unwrap();
        let reverse = b.reverse().unwrap();
        let d = reverse.domain();
        let c = certify_multispan_agreement(&a, &reverse, da, [d[1], d[0]]).unwrap();
        verify_multispan_agreement(&a, &reverse, da, [d[1], d[0]], &c).unwrap();
    }
    #[test]
    fn retracing_and_zero_jet_curves_use_exact_identity_without_monotonicity() {
        let a = polynomial(vec![
            vec![0., 0.],
            vec![4., 0.],
            vec![-4., 0.],
            vec![0., 0.],
        ]);
        let c = certify_multispan_agreement(&a, &a, [0., 1.], [0., 1.]).unwrap();
        assert_eq!(c.version, 2);
        verify_multispan_agreement(&a, &a, [0., 1.], [0., 1.], &c).unwrap();
        let mut b = a.clone();
        b.control_points[1][1] = 1.;
        assert!(verify_multispan_agreement(&a, &b, [0., 1.], [0., 1.], &c).is_err());
    }
}
