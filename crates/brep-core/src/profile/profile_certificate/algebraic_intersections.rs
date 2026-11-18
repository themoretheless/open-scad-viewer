//! Portable exact isolating intervals: irrational and non-dyadic parameters
//! are roots of checked polynomial factors, never rounded into exact witnesses.
use super::*;
use arithmetic::algebraic_events as engine;
use num_bigint::BigInt;
use num_traits::ToPrimitive;
pub type ExactRatio = [String; 2];
pub type ExactPolynomial = Vec<ExactRatio>;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlgebraicRootWitness {
    pub interval: [ExactRatio; 2],
    pub kind: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlgebraicFactorWitness {
    pub polynomial: ExactPolynomial,
    pub parameter_map: Option<ExactPolynomial>,
    pub roots: Vec<AlgebraicRootWitness>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct AlgebraicIntersectionCertificate {
    pub version: u32,
    pub source: String,
    pub self_pair: bool,
    pub maximum_parameter_width: f64,
    pub injective_axis: Option<u8>,
    pub projection_shear: u8,
    pub factors: Vec<AlgebraicFactorWitness>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct VerifiedAlgebraicIntersections {
    events: Vec<[[f64; 2]; 2]>,
    singular_parameters: Vec<[f64; 2]>,
    stationary_census_complete: bool,
}
impl VerifiedAlgebraicIntersections {
    pub fn stationary_census_complete(&self) -> bool {
        self.stationary_census_complete
    }
    pub fn events(&self) -> &[[[f64; 2]; 2]] {
        &self.events
    }
    pub fn singular_parameters(&self) -> &[[f64; 2]] {
        &self.singular_parameters
    }
}
pub(super) fn ratio(q: &Q) -> ExactRatio {
    [q.numer().to_string(), q.denom().to_string()]
}
pub(super) fn decode_ratio(s: &ExactRatio) -> Result<Q> {
    if s.iter().any(|s| s.is_empty() || s.len() > 5000) {
        return Err(refused("Exact algebraic ratio size exceeded"));
    }
    let n = s[0]
        .parse::<BigInt>()
        .map_err(|_| refused("Invalid exact numerator"))?;
    let d = s[1]
        .parse::<BigInt>()
        .map_err(|_| refused("Invalid exact denominator"))?;
    if d <= BigInt::zero() {
        return Err(refused("Exact denominator must be positive"));
    }
    let q = Q::new(n, d);
    if ratio(&q) != *s {
        return Err(refused("Exact algebraic ratio is not canonical"));
    }
    Ok(q)
}
pub(super) fn polynomial(p: &[Q]) -> ExactPolynomial {
    p.iter().map(ratio).collect()
}
pub(super) fn decode_polynomial(p: &ExactPolynomial) -> Result<Vec<Q>> {
    if p.len() > 257 {
        return Err(refused("Exact algebraic polynomial degree exceeded"));
    }
    let p = p.iter().map(decode_ratio).collect::<Result<Vec<_>>>()?;
    if p.last().is_some_and(Zero::is_zero) {
        return Err(refused("Exact polynomial has trailing zero coefficients"));
    }
    Ok(p)
}
fn input(
    a: &Curve,
    b: &Curve,
    self_pair: bool,
    width: f64,
) -> Result<(String, Vec<[Q; 3]>, Vec<[Q; 3]>)> {
    if !width.is_finite() || !(1e-18..=1e-3).contains(&width) {
        return Err(refused(
            "Algebraic parameter width must be positive and bounded",
        ));
    }
    let (s, x, y) = intersections::input(a, b, self_pair)?;
    Ok((
        format!("algebraic-intersection-v2/{:016x}/{s}", width.to_bits()),
        x,
        y,
    ))
}
fn local_width(a: &Curve, b: &Curve, width: f64) -> Q {
    let extent = |c: &Curve| {
        let [lo, hi] = c.domain().map(|v| Q::from_float(v).unwrap());
        hi - lo
    };
    Q::from_float(width).unwrap() / extent(a).max(extent(b)).max(Q::one())
}
pub fn certify_algebraic_intersections(
    a: &Curve,
    b: &Curve,
    self_pair: bool,
    width: f64,
) -> Result<AlgebraicIntersectionCertificate> {
    let (source, x, y) = input(a, b, self_pair, width)?;
    let mut c = AlgebraicIntersectionCertificate {
        version: 3,
        source,
        self_pair,
        maximum_parameter_width: width,
        injective_axis: None,
        projection_shear: 0,
        factors: Vec::new(),
    };
    if self_pair {
        for axis in 0..2 {
            if arithmetic::monotone(&x, axis)? {
                c.injective_axis = Some(axis as u8);
                c.factors = encode_factors(&engine::stationary(&x, &local_width(a, b, width))?);
                verify_algebraic_intersections(a, b, &c)?;
                return Ok(c);
            }
        }
    }
    let (k, factors) = engine::produce(&x, &y, self_pair, &local_width(a, b, width))?;
    c.projection_shear = k;
    c.factors = encode_factors(&factors);
    verify_algebraic_intersections(a, b, &c)?;
    Ok(c)
}
pub(super) fn encode_factors(factors: &[engine::Factor]) -> Vec<AlgebraicFactorWitness> {
    factors
        .iter()
        .map(|f| AlgebraicFactorWitness {
            polynomial: polynomial(&f.polynomial),
            parameter_map: f.map.as_ref().map(|p| polynomial(p)),
            roots: f
                .roots
                .iter()
                .map(|r| AlgebraicRootWitness {
                    interval: r.interval.clone().map(|q| ratio(&q)),
                    kind: r.kind.clone(),
                })
                .collect(),
        })
        .collect()
}
pub(super) fn decode_factors(factors: &[AlgebraicFactorWitness]) -> Result<Vec<engine::Factor>> {
    if factors.len() > 256 {
        return Err(refused("Algebraic factor count exceeded"));
    }
    factors
        .iter()
        .map(|f| {
            if f.roots.len() > 256 {
                return Err(refused("Algebraic root count exceeded"));
            }
            Ok(engine::Factor {
                polynomial: decode_polynomial(&f.polynomial)?,
                map: f
                    .parameter_map
                    .as_ref()
                    .map(decode_polynomial)
                    .transpose()?,
                roots: f
                    .roots
                    .iter()
                    .map(|r| {
                        Ok(engine::Root {
                            interval: [
                                decode_ratio(&r.interval[0])?,
                                decode_ratio(&r.interval[1])?,
                            ],
                            kind: r.kind.clone(),
                        })
                    })
                    .collect::<Result<_>>()?,
            })
        })
        .collect()
}
pub(super) fn outward(q: &Q, upper: bool) -> Result<f64> {
    let mut f = q
        .to_f64()
        .ok_or_else(|| refused("Algebraic event range exceeds binary64"))?;
    if upper && Q::from_float(f).unwrap() < *q {
        f = f.next_up();
    }
    if !upper && Q::from_float(f).unwrap() > *q {
        f = f.next_down();
    }
    Ok(f)
}
pub fn verify_algebraic_intersections(
    a: &Curve,
    b: &Curve,
    c: &AlgebraicIntersectionCertificate,
) -> Result<VerifiedAlgebraicIntersections> {
    let (source, x, y) = input(a, b, c.self_pair, c.maximum_parameter_width)?;
    if ![2, 3].contains(&c.version) || source != c.source || c.factors.len() > 256 {
        return Err(refused("Algebraic intersection source or schema mismatch"));
    }
    let events = if let Some(axis) = c.injective_axis {
        if !c.self_pair
            || axis > 1
            || c.projection_shear != 0
            || (c.version == 2 && !c.factors.is_empty())
            || !arithmetic::monotone(&x, axis as usize)?
        {
            return Err(refused("False algebraic injectivity claim"));
        }
        if c.version == 3 {
            engine::verify_stationary(
                &x,
                &local_width(a, b, c.maximum_parameter_width),
                &decode_factors(&c.factors)?,
            )?
        } else {
            Vec::new()
        }
    } else {
        let factors = decode_factors(&c.factors)?;
        engine::verify(
            &x,
            &y,
            c.self_pair,
            &local_width(a, b, c.maximum_parameter_width),
            c.projection_shear,
            &factors,
        )?
    };
    let global = |curve: &Curve, p: &[Q; 2]| -> Result<[f64; 2]> {
        let [a, b] = curve.domain().map(|v| Q::from_float(v).unwrap());
        let lo = &a + (&b - &a) * &p[0];
        let hi = &a + (&b - &a) * &p[1];
        Ok([outward(&lo, false)?, outward(&hi, true)?])
    };
    let mut out = VerifiedAlgebraicIntersections {
        events: Vec::new(),
        singular_parameters: Vec::new(),
        stationary_census_complete: c.self_pair && (c.version == 3 || c.injective_axis.is_none()),
    };
    for event in events {
        if event.singular {
            out.singular_parameters.push(global(a, &event.first)?);
        } else {
            out.events
                .push([global(a, &event.first)?, global(b, &event.second)?]);
        }
    }
    out.events.sort_by(|a, b| {
        a[0][0]
            .total_cmp(&b[0][0])
            .then(a[1][0].total_cmp(&b[1][0]))
    });
    Ok(out)
}
#[cfg(feature = "codec")]
mod serialization {
    use super::*;
    use value_codec::{Deserialize, Serialize, Value, error, json};
    fn schema(v: &Value, keys: &[&str]) -> value_codec::Result<()> {
        if v.as_object()
            .is_none_or(|o| o.len() != keys.len() || keys.iter().any(|k| !o.contains_key(*k)))
        {
            return Err(error("Algebraic certificate schema mismatch"));
        }
        Ok(())
    }
    impl Serialize for AlgebraicRootWitness {
        fn to_value(&self) -> Value {
            json!({"interval":self.interval,"kind":self.kind})
        }
    }
    impl<'de> Deserialize<'de> for AlgebraicRootWitness {
        fn from_value(v: Value) -> value_codec::Result<Self> {
            schema(&v, &["interval", "kind"])?;
            if v["kind"]
                .as_str()
                .is_none_or(|s| !matches!(s, "event" | "outside" | "reverse" | "diagonal"))
            {
                return Err(error("Unknown algebraic root class"));
            }
            Ok(Self {
                interval: Deserialize::from_value(v["interval"].clone())?,
                kind: Deserialize::from_value(v["kind"].clone())?,
            })
        }
    }
    impl Serialize for AlgebraicFactorWitness {
        fn to_value(&self) -> Value {
            json!({"polynomial":self.polynomial,"parameterMap":self.parameter_map,"roots":self.roots})
        }
    }
    impl<'de> Deserialize<'de> for AlgebraicFactorWitness {
        fn from_value(v: Value) -> value_codec::Result<Self> {
            schema(&v, &["polynomial", "parameterMap", "roots"])?;
            if v["polynomial"].as_array().is_none_or(|a| a.len() > 257)
                || (!v["parameterMap"].is_null()
                    && v["parameterMap"].as_array().is_none_or(|a| a.len() > 257))
                || v["roots"].as_array().is_none_or(|a| a.len() > 256)
            {
                return Err(error("Algebraic factor size exceeded"));
            }
            Ok(Self {
                polynomial: Deserialize::from_value(v["polynomial"].clone())?,
                parameter_map: Deserialize::from_value(v["parameterMap"].clone())?,
                roots: Deserialize::from_value(v["roots"].clone())?,
            })
        }
    }
    impl Serialize for AlgebraicIntersectionCertificate {
        fn to_value(&self) -> Value {
            json!({"version":self.version,"source":self.source,"selfPair":self.self_pair,"maximumParameterWidth":self.maximum_parameter_width,"injectiveAxis":self.injective_axis,"projectionShear":self.projection_shear,"factors":self.factors,"scope":"represented-algebraic-curve-intersection-set"})
        }
    }
    impl<'de> Deserialize<'de> for AlgebraicIntersectionCertificate {
        fn from_value(v: Value) -> value_codec::Result<Self> {
            schema(
                &v,
                &[
                    "version",
                    "source",
                    "selfPair",
                    "maximumParameterWidth",
                    "injectiveAxis",
                    "projectionShear",
                    "factors",
                    "scope",
                ],
            )?;
            if v["scope"].as_str() != Some("represented-algebraic-curve-intersection-set")
                || v["source"].as_str().is_none_or(|s| s.len() > 66000)
                || v["factors"].as_array().is_none_or(|a| a.len() > 256)
            {
                return Err(error("Algebraic certificate scope or size mismatch"));
            }
            Ok(Self {
                version: Deserialize::from_value(v["version"].clone())?,
                source: Deserialize::from_value(v["source"].clone())?,
                self_pair: Deserialize::from_value(v["selfPair"].clone())?,
                maximum_parameter_width: Deserialize::from_value(
                    v["maximumParameterWidth"].clone(),
                )?,
                injective_axis: Deserialize::from_value(v["injectiveAxis"].clone())?,
                projection_shear: Deserialize::from_value(v["projectionShear"].clone())?,
                factors: Deserialize::from_value(v["factors"].clone())?,
            })
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn curve(p: Vec<Vec<f64>>) -> Curve {
        let n = p.len();
        Curve {
            degree: n - 1,
            knots: [vec![0.; n], vec![1.; n]].concat(),
            weights: vec![1.; n],
            control_points: p,
            periodic: false,
        }
    }
    #[test]
    fn irrational_quartic_events_and_forged_census() {
        let a = curve(vec![
            vec![0., 1.],
            vec![1., -1.],
            vec![2., 1.],
            vec![3., -1.],
            vec![4., 1.],
        ]);
        let b = curve(vec![vec![0., 0.25], vec![4., 0.25]]);
        let c = certify_algebraic_intersections(&a, &b, false, 1e-10).unwrap();
        let r = verify_algebraic_intersections(&a, &b, &c).unwrap();
        assert_eq!(r.events.len(), 2);
        for (event, t) in r
            .events
            .iter()
            .zip([(1. - 0.5f64.sqrt()) / 2., (1. + 0.5f64.sqrt()) / 2.])
        {
            assert!(event[0][0] <= t && t <= event[0][1]);
        }
        let mut bad = c.clone();
        bad.factors[0].roots.pop();
        assert!(verify_algebraic_intersections(&a, &b, &bad).is_err());
        let mut bad = c;
        bad.factors[0].parameter_map = Some(vec![["0".into(), "1".into()]]);
        assert!(verify_algebraic_intersections(&a, &b, &bad).is_err());
    }
    #[test]
    fn rational_non_dyadic_event_and_outside_fibers() {
        let a = curve(vec![vec![0., 0.], vec![3., 3.]]);
        let b = curve(vec![vec![0., 1.], vec![3., 1.]]);
        let c = certify_algebraic_intersections(&a, &b, false, 1e-12).unwrap();
        let r = verify_algebraic_intersections(&a, &b, &c).unwrap();
        assert_eq!(r.events.len(), 1);
        assert!(r.events[0][0][0] <= 1. / 3. && r.events[0][0][1] >= 1. / 3.);
    }
    #[test]
    fn separated_fibers_multiple_tangency_and_irrational_self_events() {
        let a = curve(vec![vec![0., 0.], vec![0., 1.]]);
        let b = curve(vec![vec![0.75, 0.5], vec![-1.25, 0.5], vec![0.75, 0.5]]);
        let c = certify_algebraic_intersections(&a, &b, false, 1e-12).unwrap();
        assert!(c.projection_shear > 0);
        assert_eq!(
            verify_algebraic_intersections(&a, &b, &c)
                .unwrap()
                .events
                .len(),
            2
        );
        let a = curve(vec![vec![0., 1.], vec![1.5, -2.], vec![3., 4.]]);
        let b = curve(vec![vec![0., 0.], vec![3., 0.]]);
        let c = certify_algebraic_intersections(&a, &b, false, 1e-12).unwrap();
        let r = verify_algebraic_intersections(&a, &b, &c).unwrap();
        assert_eq!(r.events.len(), 1);
        assert!(r.events[0][0][0] <= 1. / 3. && 1. / 3. <= r.events[0][0][1]);
        let a = curve(vec![
            vec![3., -1.5],
            vec![-1., 3.5],
            vec![-1., -3.5],
            vec![3., 1.5],
        ]);
        let c = certify_algebraic_intersections(&a, &a, true, 1e-12).unwrap();
        let r = verify_algebraic_intersections(&a, &a, &c).unwrap();
        assert_eq!(r.events.len(), 1);
        for (i, t) in [(1. - 0.5f64.sqrt()) / 2., (1. + 0.5f64.sqrt()) / 2.]
            .iter()
            .enumerate()
        {
            assert!(r.events[0][i][0] <= *t && *t <= r.events[0][i][1]);
        }
        let mut bad = c;
        bad.factors[0].roots[0].kind = "outside".into();
        assert!(verify_algebraic_intersections(&a, &a, &bad).is_err());
    }
    #[test]
    fn irrational_stationary_diagonal_is_counted() {
        let a = curve(vec![
            vec![0., 0.],
            vec![3., 0.],
            vec![-2., 1.],
            vec![-7., -5.],
            vec![-4., -2.],
        ]);
        let c = certify_algebraic_intersections(&a, &a, true, 1e-11).unwrap();
        let r = verify_algebraic_intersections(&a, &a, &c).unwrap();
        assert_eq!(r.singular_parameters.len(), 2);
    }
    #[test]
    fn parameter_width_uses_actual_source_domains() {
        let mut a = curve(vec![
            vec![0., 1.],
            vec![1., -1.],
            vec![2., 1.],
            vec![3., -1.],
            vec![4., 1.],
        ]);
        let mut b = curve(vec![vec![0., 0.25], vec![4., 0.25]]);
        for k in &mut a.knots {
            *k = 2. + 8. * *k;
        }
        for k in &mut b.knots {
            *k = -4. + 16. * *k;
        }
        let c = certify_algebraic_intersections(&a, &b, false, 1e-10).unwrap();
        let r = verify_algebraic_intersections(&a, &b, &c).unwrap();
        for event in r.events {
            for p in event {
                assert!(p[1] - p[0] <= 1.001e-10);
            }
        }
        let mut changed = a;
        changed.control_points[0][1] += 0.01;
        assert!(verify_algebraic_intersections(&changed, &b, &c).is_err());
    }

    #[test]
    fn injective_zero_jet_has_a_complete_nondyadic_stationary_census() {
        let a = curve(vec![
            vec![-1., 0.],
            vec![2., 0.],
            vec![-4., 0.],
            vec![8., 0.],
        ]);
        let c = certify_algebraic_intersections(&a, &a, true, 1e-12).unwrap();
        assert_eq!(c.version, 3);
        let r = verify_algebraic_intersections(&a, &a, &c).unwrap();
        assert!(r.stationary_census_complete());
        assert_eq!(r.singular_parameters.len(), 1);
        assert!(r.singular_parameters[0][0] <= 1. / 3. && 1. / 3. <= r.singular_parameters[0][1]);
        let mut bad = c;
        bad.factors.clear();
        assert!(verify_algebraic_intersections(&a, &a, &bad).is_err());
    }
}
