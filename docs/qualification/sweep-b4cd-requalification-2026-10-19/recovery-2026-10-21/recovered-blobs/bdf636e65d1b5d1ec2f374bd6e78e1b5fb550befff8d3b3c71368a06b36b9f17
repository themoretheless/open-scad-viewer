//! Coincidence without specifying a parameter map. A polynomial relation
//! H(s,t)=0 divides both homogeneous coordinate equations. Opposite boundary
//! signs prove a real correspondence; monotone projections prove uniqueness.
use super::*;
use num_bigint::BigInt;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NonlinearCurveAgreementCertificate {
    pub version: u32,
    pub source: String,
    pub axis: u8,
    /// Outer index is t power, inner index s power. Reduced exact ratios.
    pub relation: Vec<Vec<[String; 2]>>,
}
fn input(
    a: &Curve,
    b: &Curve,
    da: [f64; 2],
    db: [f64; 2],
) -> Result<(String, Vec<[Q; 3]>, Vec<[Q; 3]>)> {
    let bits = agreement::source(a, b, da, db)?;
    let q = |x| Q::from_float(x).unwrap();
    let x = extraction::interval(a, &q(da[0]), &q(da[1]))?;
    let y = extraction::interval(b, &q(db[0]), &q(db[1]))?;
    for (p, q) in [
        (x.first().unwrap(), y.first().unwrap()),
        (x.last().unwrap(), y.last().unwrap()),
    ] {
        if &p[0] * &q[2] != &q[0] * &p[2] || &p[1] * &q[2] != &q[1] * &p[2] {
            return Err(refused("Nonlinear agreement endpoint images differ"));
        }
    }

    Ok((
        format!("curve-nonlinear-v1/{bits}"),
        extraction::interval(a, &q(da[0]), &q(da[1]))?,
        extraction::interval(b, &q(db[0]), &q(db[1]))?,
    ))
}
pub(super) fn decode(v: &[Vec<[String; 2]>]) -> Result<Vec<Vec<Q>>> {
    if v.len() < 2 || v.len() > 18 || v.iter().map(Vec::len).sum::<usize>() > 4096 {
        return Err(refused("Nonlinear relation size budget exceeded"));
    }
    v.iter()
        .map(|r| {
            r.iter()
                .map(|s| {
                    if s.iter().any(|x| x.is_empty() || x.len() > 5000) {
                        return Err(refused("Nonlinear coefficient size budget exceeded"));
                    }
                    let n = s[0]
                        .parse::<BigInt>()
                        .map_err(|_| refused("Invalid nonlinear coefficient"))?;
                    let d = s[1]
                        .parse::<BigInt>()
                        .map_err(|_| refused("Invalid nonlinear denominator"))?;
                    if d <= BigInt::zero() || n.to_string() != s[0] || d.to_string() != s[1] {
                        return Err(refused("Noncanonical nonlinear coefficient"));
                    }
                    let q = Q::new(n, d);
                    if q.numer().to_string() != s[0] || q.denom().to_string() != s[1] {
                        return Err(refused("Nonlinear coefficient is not reduced"));
                    }
                    Ok(q)
                })
                .collect()
        })
        .collect()
}
pub fn certify_nonlinear_curve_agreement(
    a: &Curve,
    b: &Curve,
    da: [f64; 2],
    db: [f64; 2],
) -> Result<NonlinearCurveAgreementCertificate> {
    let (source, x, y) = input(a, b, da, db)?;
    for axis in 0..2 {
        if !arithmetic::monotone(&x, axis)? || !arithmetic::monotone(&y, axis)? {
            continue;
        }
        if let Ok(h) = arithmetic::nonlinear_relation(&x, &y, axis) {
            let relation = h
                .iter()
                .map(|r| {
                    r.iter()
                        .map(|q| [q.numer().to_string(), q.denom().to_string()])
                        .collect()
                })
                .collect();
            let c = NonlinearCurveAgreementCertificate {
                version: 1,
                source,
                axis: axis as u8,
                relation,
            };
            verify_nonlinear_curve_agreement(a, b, da, db, &c)?;
            return Ok(c);
        }
    }
    Err(refused("No complete nonlinear correspondence proof"))
}
pub fn verify_nonlinear_curve_agreement(
    a: &Curve,
    b: &Curve,
    da: [f64; 2],
    db: [f64; 2],
    c: &NonlinearCurveAgreementCertificate,
) -> Result<VerifiedCurveAgreement> {
    let (source, x, y) = input(a, b, da, db)?;
    if c.version != 1
        || c.source != source
        || !arithmetic::verify_relation(&x, &y, c.axis as usize, &decode(&c.relation)?)?
    {
        return Err(refused("Nonlinear curve correspondence claim is false"));
    }
    Ok(VerifiedCurveAgreement { common_spans: 1 })
}
#[cfg(feature = "codec")]
mod serialization {
    use super::*;
    use value_codec::{Deserialize, Serialize, Value, error, json};
    impl Serialize for NonlinearCurveAgreementCertificate {
        fn to_value(&self) -> Value {
            json!({"version":self.version,"source":self.source,"axis":self.axis,"relation":self.relation,"scope":"represented-curve-nonlinear-image-agreement"})
        }
    }
    impl<'de> Deserialize<'de> for NonlinearCurveAgreementCertificate {
        fn from_value(v: Value) -> value_codec::Result<Self> {
            let keys = ["version", "source", "axis", "relation", "scope"];
            if v.as_object()
                .is_none_or(|o| o.len() != keys.len() || keys.iter().any(|k| !o.contains_key(*k)))
                || v["scope"].as_str() != Some("represented-curve-nonlinear-image-agreement")
                || v["source"].as_str().is_none_or(|s| s.len() > 65800)
                || v["relation"].as_array().is_none_or(|r| {
                    r.len() > 18 || r.iter().any(|p| p.as_array().is_none_or(|p| p.len() > 257))
                })
            {
                return Err(error("Nonlinear agreement schema or scope mismatch"));
            }
            let c = Self {
                version: Deserialize::from_value(v["version"].clone())?,
                source: Deserialize::from_value(v["source"].clone())?,
                axis: Deserialize::from_value(v["axis"].clone())?,
                relation: Deserialize::from_value(v["relation"].clone())?,
            };
            decode(&c.relation).map_err(|e| error(e.message))?;
            Ok(c)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn curve(points: Vec<Vec<f64>>) -> Curve {
        let d = points.len() - 1;
        Curve {
            degree: d,
            knots: [vec![0.; d + 1], vec![1.; d + 1]].concat(),
            weights: vec![1.; d + 1],
            control_points: points,
            periodic: false,
        }
    }
    #[test]
    fn algebraic_cube_root_correspondence_and_false_relation() {
        // Same parabola, A(s)=(s²,s⁴), B(t)=(t³,t⁶): t=s^(2/3).
        // The correspondence is not a rational function or a Mobius map.
        let a = curve(vec![
            vec![0., 0.],
            vec![0., 0.],
            vec![1., 0.],
            vec![3., 0.],
            vec![6., 1.],
        ]);
        // Use exact integer-scaled Bernstein controls: x=60s², y=s⁴.
        let mut a = a;
        for (p, x) in a.control_points.iter_mut().zip([0., 0., 10., 30., 60.]) {
            p[0] = x;
        }
        let b = curve(vec![
            vec![0., 0.],
            vec![0., 0.],
            vec![0., 0.],
            vec![3., 0.],
            vec![12., 0.],
            vec![30., 0.],
            vec![60., 1.],
        ]);
        let mut c = certify_nonlinear_curve_agreement(&a, &b, [0., 1.], [0., 1.]).unwrap();
        verify_nonlinear_curve_agreement(&a, &b, [0., 1.], [0., 1.], &c).unwrap();
        c.relation[0][0] = ["1".into(), "1".into()];
        assert!(verify_nonlinear_curve_agreement(&a, &b, [0., 1.], [0., 1.], &c).is_err());
        let mut bad = b.clone();
        bad.control_points[3][1] = 1.;
        assert!(certify_nonlinear_curve_agreement(&a, &bad, [0., 1.], [0., 1.]).is_err());
    }
    #[test]
    fn rational_projective_reverse_and_partial_domains_need_no_map_hint() {
        let a = curve(vec![vec![0., 0.], vec![4., 0.], vec![8., 4.]]);
        let mut b = a.clone();
        b.weights = vec![1., 0.5, 0.25];
        let c = certify_nonlinear_curve_agreement(&a, &b, [0., 1.], [0., 1.]).unwrap();
        verify_nonlinear_curve_agreement(&a, &b, [0., 1.], [0., 1.], &c).unwrap();
        let reverse = b.reverse().unwrap();
        let c = certify_nonlinear_curve_agreement(&a, &reverse, [0., 1.], [1., 0.]).unwrap();
        verify_nonlinear_curve_agreement(&a, &reverse, [0., 1.], [1., 0.], &c).unwrap();
        let a = curve(vec![vec![0., 0.], vec![0., 0.], vec![8., 0.]]);
        let b = curve(vec![vec![0., 0.], vec![8., 0.]]);
        let c = certify_nonlinear_curve_agreement(&a, &b, [0.5, 1.], [0.25, 1.]).unwrap();
        verify_nonlinear_curve_agreement(&a, &b, [0.5, 1.], [0.25, 1.], &c).unwrap();
    }
    #[test]
    fn nonlinear_branch_coverage_rejects_wrong_arc() {
        let a = curve(vec![vec![0., 0.], vec![1., 1.], vec![2., 0.]]);
        let b = curve(vec![vec![0., 0.], vec![1., -1.], vec![2., 0.]]);
        assert!(certify_nonlinear_curve_agreement(&a, &b, [0., 1.], [0., 1.]).is_err());
    }
}
