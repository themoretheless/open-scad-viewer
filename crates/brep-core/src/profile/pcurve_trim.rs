//! Exact planar coedge restriction and Bernstein materialization.
use crate::profile_certificate::exact_curve_bernstein_interval;
use crate::{Coedge, Curve, Error, Result};
use num_bigint::BigInt;
use num_rational::BigRational as Q;
use num_traits::{ToPrimitive, Zero};

type Point = [Q; 3];

fn unsupported(message: &str) -> Error {
    Error::new("BREP_UNSUPPORTED_OPERATION", message)
}

fn representable(value: &Q) -> Result<f64> {
    value
        .to_f64()
        .filter(|&v| Q::from_float(v).as_ref() == Some(value))
        .ok_or_else(|| unsupported("Exact pcurve coefficient exceeds the BRep representation"))
}

fn gcd(mut a: BigInt, mut b: BigInt) -> BigInt {
    while !b.is_zero() {
        let remainder = a % &b;
        a = b;
        b = remainder;
    }
    a
}

fn represented_weights(poles: &[Point]) -> Result<Vec<f64>> {
    let weights = || poles.iter().map(|p| &p[2]);
    let accepted = |weight: &Q| {
        representable(weight).and_then(|value| {
            if (1e-12..=1e12).contains(&value) {
                Ok(value)
            } else {
                Err(unsupported("Exact fragment weight exceeds the BRep range"))
            }
        })
    };
    if let Ok(values) = weights().map(accepted).collect() {
        return Ok(values);
    }
    let common = weights().fold(BigInt::from(1), |d, w| {
        &d / gcd(d.clone(), w.denom().clone()) * w.denom()
    });
    let integer: Vec<_> = weights()
        .map(|w| w.numer() * (&common / w.denom()))
        .collect();
    let divisor = integer.iter().cloned().reduce(gcd).unwrap();
    let primitive: Vec<_> = integer.iter().map(|w| w / &divisor).collect();
    let max = primitive.iter().max().unwrap();
    let mut shift = max.bits().saturating_sub(40);
    let limit = BigInt::from(1_000_000_000_000_u64);
    if max > &(&limit << shift) {
        shift += 1;
    }
    let scale = BigInt::from(1) << shift;
    primitive
        .into_iter()
        .map(|w| accepted(&Q::new(w, scale.clone())))
        .collect()
}

pub(crate) fn materialize_span(curve: &Curve, fragment: &[Point], interval: &[Q]) -> Result<Curve> {
    let mut piece = curve.clone();
    piece.weights = represented_weights(fragment)?;
    piece.control_points.truncate(fragment.len());
    // Affine reclocking preserves the exact homogeneous
    // fragment even if either source parameter is non-dyadic.
    let knots = match (representable(&interval[0]), representable(&interval[1])) {
        (Ok(a), Ok(b)) => [a, b],
        _ => [0., 1.],
    };
    piece.knots = knots
        .into_iter()
        .flat_map(|v| std::iter::repeat_n(v, curve.degree + 1))
        .collect();
    for (i, [x, y, w]) in fragment.iter().enumerate() {
        piece.control_points[i] = vec![representable(&(x / w))?, representable(&(y / w))?];
    }
    piece.validate()?;
    Ok(piece)
}

pub(crate) fn active_pcurves(coedge: &Coedge) -> Result<Vec<Curve>> {
    let curve = &coedge.pcurve;
    let Some(trim) = coedge.trim() else {
        return Ok(vec![curve.clone()]);
    };
    // The pcurve already stores literal lifted coordinates. Its endpoint
    // lift metadata is validated by the caller against the surface chart.
    restrict_pcurve(
        curve,
        trim.pcurve_parameter.map(|v| Q::from_float(v).unwrap()),
    )
}

pub(crate) fn restrict_pcurve(curve: &Curve, [a, b]: [Q; 2]) -> Result<Vec<Curve>> {
    let [lo, hi] = [(&a).min(&b).clone(), (&a).max(&b).clone()];
    let mut clocks = vec![lo.clone(), hi.clone()];
    clocks.extend(
        curve
            .knots
            .iter()
            .map(|&t| Q::from_float(t).unwrap())
            .filter(|t| &lo < t && t < &hi),
    );
    clocks.sort();
    clocks.dedup();
    if a > b {
        clocks.reverse();
    }
    clocks
        .windows(2)
        .map(|clock| {
            let interval = [clock[0].clone(), clock[1].clone()];
            let net = exact_curve_bernstein_interval(curve, &interval[0], &interval[1])?;
            // On a reversed trim the net is already reversed. Its new natural
            // domain must still increase.
            let mut domain = interval;
            domain.sort();
            materialize_span(curve, &net, &domain)
        })
        .collect()
}
