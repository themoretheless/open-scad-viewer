//! Complete isolated dyadic intersection events, including multiple tangencies.
//! A nonzero exact eliminant plus a checked complete census of every real fiber
//! prevents missed contacts. The self diagonal is exactly removed, not sampled.
use super::*;
use num_traits::ToPrimitive;
#[derive(Clone, Debug, PartialEq)]
pub struct CurveIntersectionCertificate {
    pub version: u32,
    pub source: String,
    pub self_pair: bool,
    pub monotone_axis: Option<u8>,
    pub projection_parameters: Vec<f64>,
    pub events: Vec<[f64; 2]>,
    pub singular_parameters: Vec<f64>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct VerifiedCurveIntersections {
    events: Vec<[f64; 2]>,
    singular_parameters: Vec<f64>,
}
impl VerifiedCurveIntersections {
    pub fn events(&self) -> &[[f64; 2]] {
        &self.events
    }
    pub fn singular_parameters(&self) -> &[f64] {
        &self.singular_parameters
    }
}
pub(super) fn input(a: &Curve, b: &Curve, self_pair: bool) -> Result<(String, Vec<[Q; 3]>, Vec<[Q; 3]>)> {
    let bits = source_bits(&[vec![a.clone()], vec![b.clone()]], 1e-7)?;
    if self_pair && source_bits(&[vec![a.clone()]], 1e-7)? != source_bits(&[vec![b.clone()]], 1e-7)?
    {
        return Err(refused("Self-intersection sources must match exactly"));
    }
    let nets = extraction::nets(&[vec![a.clone()], vec![b.clone()]])?;
    if nets.len() != 2 || nets[0].0 != 0 || nets[1].0 != 1 {
        return Err(refused(
            "Isolated intersection proof requires one active span per curve",
        ));
    }
    Ok((
        format!("curve-intersection-v1/{}/{bits}", u8::from(self_pair)),
        nets[0].1.clone(),
        nets[1].1.clone(),
    ))
}
fn float(q: &Q) -> Result<f64> {
    let f = q
        .to_f64()
        .ok_or_else(|| refused("Intersection parameter exceeds range"))?;
    if Q::from_float(f).as_ref() != Some(q) {
        return Err(refused("Intersection parameter is not exactly binary64"));
    }
    Ok(f)
}
fn qlist(v: &[f64]) -> Result<Vec<Q>> {
    if v.len() > 128
        || v.iter().any(|f| !f.is_finite() || !(0.0..=1.0).contains(f))
        || v.windows(2).any(|p| p[0] >= p[1])
    {
        return Err(refused("Intersection parameter census is not canonical"));
    }
    Ok(v.iter().map(|v| Q::from_float(*v).unwrap()).collect())
}
pub fn certify_curve_intersections(
    a: &Curve,
    b: &Curve,
    self_pair: bool,
) -> Result<CurveIntersectionCertificate> {
    let (source, x, y) = input(a, b, self_pair)?;
    let mut c = CurveIntersectionCertificate {
        version: 1,
        source,
        self_pair,
        monotone_axis: None,
        projection_parameters: Vec::new(),
        events: Vec::new(),
        singular_parameters: Vec::new(),
    };
    if self_pair {
        for axis in 0..2 {
            if arithmetic::monotone(&x, axis)? {
                c.monotone_axis = Some(axis as u8);
                verify_curve_intersections(a, b, &c)?;
                return Ok(c);
            }
        }
    }
    let equations = arithmetic::intersection_polynomials(&x, &y, self_pair)?;
    let r = arithmetic::resultant(&equations[0], &equations[1])?;
    let projections = arithmetic::rational_roots(&r)?;
    for s in projections {
        c.projection_parameters.push(float(&s)?);
        let fiber = arithmetic::fiber_gcd(&equations, &s)?;
        for t in arithmetic::rational_roots(&fiber)? {
            if self_pair && s == t {
                c.singular_parameters.push(float(&s)?);
            } else if !self_pair || s < t {
                c.events.push([float(&s)?, float(&t)?]);
            }
        }
    }
    c.events
        .sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    c.events.dedup();
    c.singular_parameters.sort_by(f64::total_cmp);
    c.singular_parameters.dedup();
    verify_curve_intersections(a, b, &c)?;
    Ok(c)
}
pub fn verify_curve_intersections(
    a: &Curve,
    b: &Curve,
    c: &CurveIntersectionCertificate,
) -> Result<VerifiedCurveIntersections> {
    let (source, x, y) = input(a, b, c.self_pair)?;
    if c.version != 1 || c.source != source || c.events.len() > 128 {
        return Err(refused("Intersection certificate source or size mismatch"));
    }
    let projections = qlist(&c.projection_parameters)?;
    let singular = qlist(&c.singular_parameters)?;
    if let Some(axis) = c.monotone_axis {
        if !c.self_pair
            || axis > 1
            || !projections.is_empty()
            || !singular.is_empty()
            || !c.events.is_empty()
            || !arithmetic::monotone(&x, axis as usize)?
        {
            return Err(refused("False injective self-intersection census"));
        }
    } else {
        let equations = arithmetic::intersection_polynomials(&x, &y, c.self_pair)?;
        let r = arithmetic::resultant(&equations[0], &equations[1])?;
        if !arithmetic::roots_accounted(&r, &projections)?
            || projections.iter().any(|s| !arithmetic::at(&r, s).is_zero())
        {
            return Err(refused(
                "Intersection projection census is incomplete or false",
            ));
        }
        let mut previous = None;
        for event in &c.events {
            if event
                .iter()
                .any(|f| !f.is_finite() || !(0.0..=1.0).contains(f))
                || c.self_pair && event[0] >= event[1]
                || previous.is_some_and(|p: [f64; 2]| {
                    p[0] > event[0] || p[0] == event[0] && p[1] >= event[1]
                })
            {
                return Err(refused("Intersection event list is not canonical"));
            }
            previous = Some(*event);
            let s = Q::from_float(event[0]).unwrap();
            let t = Q::from_float(event[1]).unwrap();
            if !projections.contains(&s)
                || c.self_pair && !projections.contains(&t)
                || equations
                    .iter()
                    .any(|p| !arithmetic::at(&arithmetic::fiber(p, &s), &t).is_zero())
            {
                return Err(refused("Claimed exact intersection event is false"));
            }
        }
        if !c.self_pair && !singular.is_empty() {
            return Err(refused("Pair certificate cannot claim self singularities"));
        }
        for s in &singular {
            if !projections.contains(s)
                || equations
                    .iter()
                    .any(|p| !arithmetic::at(&arithmetic::fiber(p, s), s).is_zero())
            {
                return Err(refused("Claimed diagonal singularity is false"));
            }
        }
        for s in &projections {
            let mut values = Vec::new();
            for event in &c.events {
                let a = Q::from_float(event[0]).unwrap();
                let b = Q::from_float(event[1]).unwrap();
                if a == *s {
                    values.push(b.clone());
                }
                if c.self_pair && b == *s {
                    values.push(a);
                }
            }
            if singular.contains(s) {
                values.push(s.clone());
            }
            if !arithmetic::roots_accounted(&arithmetic::fiber_gcd(&equations, s)?, &values)? {
                return Err(refused("Intersection fiber census omits an event"));
            }
        }
    }
    let global = |curve: &Curve, u: f64| -> Result<f64> {
        let d = curve.domain();
        float(
            &(Q::from_float(d[0]).unwrap()
                + (Q::from_float(d[1]).unwrap() - Q::from_float(d[0]).unwrap())
                    * Q::from_float(u).unwrap()),
        )
    };
    Ok(VerifiedCurveIntersections {
        events: c
            .events
            .iter()
            .map(|p| Ok([global(a, p[0])?, global(b, p[1])?]))
            .collect::<Result<_>>()?,
        singular_parameters: c
            .singular_parameters
            .iter()
            .map(|u| global(a, *u))
            .collect::<Result<_>>()?,
    })
}
#[cfg(feature = "codec")]
mod serialization {
    use super::*;
    use value_codec::{Deserialize, Serialize, Value, error, json};
    impl Serialize for CurveIntersectionCertificate {
        fn to_value(&self) -> Value {
            json!({"version":self.version,"source":self.source,"selfPair":self.self_pair,"monotoneAxis":self.monotone_axis,"projectionParameters":self.projection_parameters,"events":self.events,"singularParameters":self.singular_parameters,"scope":"represented-isolated-curve-intersection-set"})
        }
    }
    impl<'de> Deserialize<'de> for CurveIntersectionCertificate {
        fn from_value(v: Value) -> value_codec::Result<Self> {
            let keys = [
                "version",
                "source",
                "selfPair",
                "monotoneAxis",
                "projectionParameters",
                "events",
                "singularParameters",
                "scope",
            ];
            if v.as_object()
                .is_none_or(|o| o.len() != keys.len() || keys.iter().any(|k| !o.contains_key(*k)))
                || v["scope"].as_str() != Some("represented-isolated-curve-intersection-set")
                || v["source"].as_str().is_none_or(|s| s.len() > 65800)
                || ["projectionParameters", "events", "singularParameters"]
                    .iter()
                    .any(|k| v[*k].as_array().is_none_or(|a| a.len() > 128))
            {
                return Err(error("Intersection certificate schema or size mismatch"));
            }
            Ok(Self {
                version: Deserialize::from_value(v["version"].clone())?,
                source: Deserialize::from_value(v["source"].clone())?,
                self_pair: Deserialize::from_value(v["selfPair"].clone())?,
                monotone_axis: Deserialize::from_value(v["monotoneAxis"].clone())?,
                projection_parameters: Deserialize::from_value(v["projectionParameters"].clone())?,
                events: Deserialize::from_value(v["events"].clone())?,
                singular_parameters: Deserialize::from_value(v["singularParameters"].clone())?,
            })
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn curve(p: Vec<Vec<f64>>) -> Curve {
        let d = p.len() - 1;
        Curve {
            degree: d,
            knots: [vec![0.; d + 1], vec![1.; d + 1]].concat(),
            weights: vec![1.; d + 1],
            control_points: p,
            periodic: false,
        }
    }
    #[test]
    fn exact_multiple_tangency_census_and_tampering() {
        // y=(2s-1)^4, x=4s; fourth-order contact with y=0.
        let a = curve(vec![
            vec![0., 1.],
            vec![1., -1.],
            vec![2., 1.],
            vec![3., -1.],
            vec![4., 1.],
        ]);
        let b = curve(vec![vec![0., 0.], vec![4., 0.]]);
        let mut c = certify_curve_intersections(&a, &b, false).unwrap();
        assert_eq!(c.events, vec![[0.5, 0.5]]);
        verify_curve_intersections(&a, &b, &c).unwrap();
        c.events.clear();
        assert!(verify_curve_intersections(&a, &b, &c).is_err());
    }
    #[test]
    fn algebraic_event_parameters_refuse_instead_of_accepting_an_empty_census(){
        let a=curve(vec![vec![0.,1.],vec![1.,-1.],vec![2.,1.],vec![3.,-1.],vec![4.,1.]]);
        let b=curve(vec![vec![0.,0.25],vec![4.,0.25]]);
        assert!(certify_curve_intersections(&a,&b,false).is_err());
        let (source,_,_)=input(&a,&b,false).unwrap();
        let forged=CurveIntersectionCertificate{version:1,source,self_pair:false,monotone_axis:None,projection_parameters:Vec::new(),events:Vec::new(),singular_parameters:Vec::new()};
        assert!(verify_curve_intersections(&a,&b,&forged).is_err());
    }
    #[test]
    fn nonprimitive_rational_self_loop_cancels_only_verified_homogeneous_factors() {
        let poles = [[0., 0.], [13440., 2520.], [13440., -8400.], [0., 7560.]];
        let points = (0..=4)
            .map(|i| {
                (0..2)
                    .map(|axis| {
                        let left = if i < 4 {
                            (4 - i) as f64 * poles[i][axis]
                        } else {
                            0.
                        };
                        let right = if i > 0 {
                            2. * i as f64 * poles[i - 1][axis]
                        } else {
                            0.
                        };
                        (left + right) / (4 + i) as f64
                    })
                    .collect()
            })
            .collect();
        let mut a = curve(points);
        a.weights = vec![4., 5., 6., 7., 8.];
        let before = a.clone();
        let c = certify_curve_intersections(&a, &a, true).unwrap();
        assert_eq!(c.events, vec![[0.25, 0.75]]);
        verify_curve_intersections(&a, &a, &c).unwrap();
        assert_eq!(a, before);
    }
    #[test]
    fn cubic_loop_exact_self_intersection_and_stationary_diagonal() {
        // x=12s(1-s), y=12s(s-.25)(s-.75), equal at .25/.75.
        let a = curve(vec![
            vec![0., 0.],
            vec![4., 0.75],
            vec![4., -2.5],
            vec![0., 2.25],
        ]);
        let c = certify_curve_intersections(&a, &a, true).unwrap();
        assert_eq!(c.events, vec![[0.25, 0.75]]);
        let mut bad = c.clone();
        bad.projection_parameters.pop();
        assert!(verify_curve_intersections(&a, &a, &bad).is_err());
        let cusp = curve(vec![
            vec![2., 4.],
            vec![0., -2.],
            vec![-2., 0.],
            vec![4., 2.],
        ]);
        let c = certify_curve_intersections(&cusp, &cusp, true).unwrap();
        assert!(c.events.is_empty());
        assert_eq!(c.singular_parameters, vec![0.5]);
        let mut bad = c;
        bad.singular_parameters.clear();
        assert!(verify_curve_intersections(&cusp, &cusp, &bad).is_err());
    }
}
