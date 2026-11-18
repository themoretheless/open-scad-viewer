//! Exact, source-bound agreement of rational curves on partial domains under a
//! specified affine parameter map. Common knot refinement and homogeneous
//! polynomial identities prove every point, including stationary endpoints.
//! This does not assert equivalence under an arbitrary nonlinear map.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurveAgreementCertificate {
    pub version: u32,
    pub source: String,
    pub common_spans: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedCurveAgreement {
    pub(super) common_spans: usize,
}
impl VerifiedCurveAgreement {
    pub fn common_spans(&self) -> usize {
        self.common_spans
    }
}

pub(super) fn source(a: &Curve, b: &Curve, da: [f64; 2], db: [f64; 2]) -> Result<String> {
    let bits = source_bits(&[vec![a.clone()], vec![b.clone()]], 1e-7)?;
    for (curve, domain) in [(a, da), (b, db)] {
        let active = curve.domain();
        if domain
            .iter()
            .any(|t| !t.is_finite() || *t < active[0] || *t > active[1])
            || domain[0] == domain[1]
        {
            return Err(refused(
                "Curve agreement needs nonempty intervals inside the active domains",
            ));
        }
    }
    Ok(format!(
        "curve-agreement-affine-v1/{:016x}/{:016x}/{:016x}/{:016x}/{bits}",
        da[0].to_bits(),
        da[1].to_bits(),
        db[0].to_bits(),
        db[1].to_bits()
    ))
}

pub(super) fn check(a: &Curve, b: &Curve, da: [f64; 2], db: [f64; 2], scale: &Q) -> Result<usize> {
    let domains = [da, db].map(|d| d.map(|t| Q::from_float(t).unwrap()));
    let mut cuts = vec![Q::zero(), Q::one()];
    for (index, (curve, d)) in [a, b].into_iter().zip(&domains).enumerate() {
        for knot in &curve.knots {
            let u = (Q::from_float(*knot).unwrap() - &d[0]) / (&d[1] - &d[0]);
            if u > Q::zero() && u < Q::one() {
                cuts.push(if index == 1 {
                    &u / (scale + (&Q::one() - scale) * &u)
                } else {
                    u
                });
            }
        }
    }
    cuts.sort();
    cuts.dedup();
    if cuts.len() > MAX_PIECES + 1 {
        return Err(refused("Exact curve agreement common-span budget exceeded"));
    }
    for pair in cuts.windows(2) {
        let bounds = domains
            .clone()
            .into_iter()
            .enumerate()
            .map(|(index, d)| {
                pair.iter()
                    .map(|u| {
                        let v = if index == 1 {
                            scale * u / (&Q::one() + (scale - &Q::one()) * u)
                        } else {
                            u.clone()
                        };
                        &d[0] + (&d[1] - &d[0]) * v
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let x = extraction::interval(a, &bounds[0][0], &bounds[0][1])?;
        let mut y = extraction::interval(b, &bounds[1][0], &bounds[1][1])?;
        // Pull the restricted second Bernstein net back through the positive
        // Mobius map. On this subinterval its scale is D(end)/D(start).
        let local = (&Q::one() + (scale - &Q::one()) * &pair[1])
            / (&Q::one() + (scale - &Q::one()) * &pair[0]);
        for (i, h) in y.iter_mut().enumerate() {
            let factor = local.pow(i as i32);
            for x in h {
                *x *= &factor;
            }
        }
        if !arithmetic::projectively_equal(&x, &y)? {
            return Err(refused(
                "Exact homogeneous curve agreement identity is false",
            ));
        }
    }
    Ok(cuts.len() - 1)
}

pub fn certify_curve_agreement(
    a: &Curve,
    b: &Curve,
    first_domain: [f64; 2],
    second_domain: [f64; 2],
) -> Result<CurveAgreementCertificate> {
    let source = source(a, b, first_domain, second_domain)?;
    let common_spans = check(a, b, first_domain, second_domain, &Q::one())?;
    let c = CurveAgreementCertificate {
        version: 1,
        source,
        common_spans,
    };
    verify_curve_agreement(a, b, first_domain, second_domain, &c)?;
    Ok(c)
}

pub fn verify_curve_agreement(
    a: &Curve,
    b: &Curve,
    first_domain: [f64; 2],
    second_domain: [f64; 2],
    c: &CurveAgreementCertificate,
) -> Result<VerifiedCurveAgreement> {
    if c.version != 1 || c.source != source(a, b, first_domain, second_domain)? {
        return Err(refused("Curve agreement version or source does not match"));
    }
    let common_spans = check(a, b, first_domain, second_domain, &Q::one())?;
    if common_spans != c.common_spans {
        return Err(refused("Curve agreement omits or adds common spans"));
    }
    Ok(VerifiedCurveAgreement { common_spans })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectiveCurveAgreementCertificate {
    pub version: u32,
    pub source: String,
    pub common_spans: usize,
}
fn projective_source(
    a: &Curve,
    b: &Curve,
    da: [f64; 2],
    db: [f64; 2],
    scale: f64,
) -> Result<String> {
    if !scale.is_finite() || !(1e-12..=1e12).contains(&scale) {
        return Err(refused(
            "Projective parameter scale must be positive and bounded",
        ));
    }
    Ok(format!(
        "curve-agreement-projective-v1/{:016x}/{}",
        scale.to_bits(),
        source(a, b, da, db)?
    ))
}
pub fn certify_projective_curve_agreement(
    a: &Curve,
    b: &Curve,
    da: [f64; 2],
    db: [f64; 2],
    scale: f64,
) -> Result<ProjectiveCurveAgreementCertificate> {
    let source = projective_source(a, b, da, db, scale)?;
    let common_spans = check(a, b, da, db, &Q::from_float(scale).unwrap())?;
    let c = ProjectiveCurveAgreementCertificate {
        version: 1,
        source,
        common_spans,
    };
    verify_projective_curve_agreement(a, b, da, db, scale, &c)?;
    Ok(c)
}
pub fn verify_projective_curve_agreement(
    a: &Curve,
    b: &Curve,
    da: [f64; 2],
    db: [f64; 2],
    scale: f64,
    c: &ProjectiveCurveAgreementCertificate,
) -> Result<VerifiedCurveAgreement> {
    if c.version != 1 || c.source != projective_source(a, b, da, db, scale)? {
        return Err(refused(
            "Projective agreement source or version does not match",
        ));
    }
    let common_spans = check(a, b, da, db, &Q::from_float(scale).unwrap())?;
    if common_spans != c.common_spans {
        return Err(refused("Projective agreement omits or adds common spans"));
    }
    Ok(VerifiedCurveAgreement { common_spans })
}

#[cfg(feature = "codec")]
mod serialization {
    use super::*;
    use value_codec::{Deserialize, Serialize, Value, error, json};
    impl Serialize for ProjectiveCurveAgreementCertificate {
        fn to_value(&self) -> Value {
            json!({"version":self.version,"source":self.source,"commonSpans":self.common_spans,"scope":"represented-curve-projective-parameter-agreement"})
        }
    }
    impl<'de> Deserialize<'de> for ProjectiveCurveAgreementCertificate {
        fn from_value(v: Value) -> value_codec::Result<Self> {
            let keys = ["version", "source", "commonSpans", "scope"];
            let o = v
                .as_object()
                .ok_or_else(|| error("Expected projective agreement certificate"))?;
            if o.len() != 4
                || keys.iter().any(|k| !o.contains_key(*k))
                || v["scope"].as_str() != Some("represented-curve-projective-parameter-agreement")
                || v["source"].as_str().is_none_or(|s| s.len() > 65800)
            {
                return Err(error("Projective agreement schema or scope does not match"));
            }
            Ok(Self {
                version: Deserialize::from_value(v["version"].clone())?,
                source: Deserialize::from_value(v["source"].clone())?,
                common_spans: Deserialize::from_value(v["commonSpans"].clone())?,
            })
        }
    }
    impl Serialize for CurveAgreementCertificate {
        fn to_value(&self) -> Value {
            json!({"version":self.version,"source":self.source,
            "commonSpans":self.common_spans,"scope":"represented-curve-affine-parameter-agreement"})
        }
    }
    impl<'de> Deserialize<'de> for CurveAgreementCertificate {
        fn from_value(v: Value) -> value_codec::Result<Self> {
            let keys = ["version", "source", "commonSpans", "scope"];
            let o = v
                .as_object()
                .ok_or_else(|| error("Expected curve agreement certificate"))?;
            if o.len() != keys.len()
                || keys.iter().any(|k| !o.contains_key(*k))
                || v["scope"].as_str() != Some("represented-curve-affine-parameter-agreement")
                || v["source"].as_str().is_none_or(|s| s.len() > 65700)
            {
                return Err(error(
                    "Curve agreement schema, scope or size does not match",
                ));
            }
            Ok(Self {
                version: Deserialize::from_value(v["version"].clone())?,
                source: Deserialize::from_value(v["source"].clone())?,
                common_spans: Deserialize::from_value(v["commonSpans"].clone())?,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nonlinear_projective_agreement_binds_map_and_rejects_a_pole() {
        let a = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![5., 0.], vec![10., 5.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let mut b = a.clone();
        b.weights = vec![1., 0.5, 0.25];
        let c = certify_projective_curve_agreement(&a, &b, [0., 1.], [0., 1.], 2.).unwrap();
        verify_projective_curve_agreement(&a, &b, [0., 1.], [0., 1.], 2., &c).unwrap();
        assert!(certify_curve_agreement(&a, &b, [0., 1.], [0., 1.]).is_err());
        assert!(verify_projective_curve_agreement(&a, &b, [0., 1.], [0., 1.], 1., &c).is_err());
        assert!(certify_projective_curve_agreement(&a, &b, [0., 1.], [0., 1.], -1.).is_err());
        let mut forged = c.clone();
        forged.source = projective_source(&a, &b, [0., 1.], [0., 1.], 3.).unwrap();
        assert!(
            verify_projective_curve_agreement(&a, &b, [0., 1.], [0., 1.], 3., &forged).is_err()
        );
    }
    #[test]
    fn partial_agreement_cancels_common_factors_across_different_degrees() {
        let a = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![2.5, 0.], vec![6., 1.], vec![10., 5.]],
            weights: vec![3., 4., 5., 6.],
            periodic: false,
        };
        let b = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![5., 0.], vec![10., 5.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let c = certify_curve_agreement(&a, &b, [0.25, 0.75], [0.25, 0.75]).unwrap();
        verify_curve_agreement(&a, &b, [0.25, 0.75], [0.25, 0.75], &c).unwrap();
        let reversed = b.reverse().unwrap();
        let r = certify_curve_agreement(&a, &reversed, [0.25, 0.75], [0.75, 0.25]).unwrap();
        verify_curve_agreement(&a, &reversed, [0.25, 0.75], [0.75, 0.25], &r).unwrap();
        let mut changed = b.clone();
        changed.control_points[1][1] = f64::from_bits(1);
        let mut forged = c.clone();
        forged.source = source(&a, &changed, [0.25, 0.75], [0.25, 0.75]).unwrap();
        assert!(verify_curve_agreement(&a, &changed, [0.25, 0.75], [0.25, 0.75], &forged).is_err());
        let mut forged = c.clone();
        forged.common_spans = 0;
        assert!(verify_curve_agreement(&a, &b, [0.25, 0.75], [0.25, 0.75], &forged).is_err());
        assert!(verify_curve_agreement(&a, &b, [0., 1.], [0., 1.], &c).is_err());
        #[cfg(feature = "codec")]
        {
            let s = value_codec::to_string(&c).unwrap();
            let decoded: CurveAgreementCertificate = value_codec::from_str(&s).unwrap();
            assert_eq!(c, decoded);
            let mut v = value_codec::to_value(&c).unwrap();
            v["solidValid"] = value_codec::Value::Bool(true);
            assert!(value_codec::from_value::<CurveAgreementCertificate>(v).is_err());
        }
    }
    #[test]
    fn common_refinement_checks_every_partial_bspline_span() {
        let a = Curve::from_polyline(vec![vec![0., 0.], vec![4., 0.], vec![4., 4.], vec![0., 4.]])
            .unwrap();
        let mut b = a.clone();
        for k in &mut b.knots {
            *k = 2. * *k + 10.;
        }
        let c = certify_curve_agreement(&a, &b, [0.5, 2.5], [11., 15.]).unwrap();
        assert_eq!(c.common_spans, 3);
        verify_curve_agreement(&a, &b, [0.5, 2.5], [11., 15.], &c).unwrap();
        assert!(certify_curve_agreement(&a, &b, [0., 3.], [10., 15.]).is_err());
        assert!(certify_curve_agreement(&a, &b, [-1., 2.], [10., 14.]).is_err());
    }
}
