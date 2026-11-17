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
    // A forward full-period trim already has the authored periodic
    // representation. Its world curve may have an independently remapped clock.
    if curve.periodic && trim.pcurve_parameter == curve.domain() {
        return Ok(vec![curve.clone()]);
    }
    // The pcurve already stores literal lifted coordinates. Its endpoint
    // lift metadata is validated by the caller against the surface chart.
    restrict_pcurve(
        curve,
        trim.pcurve_parameter.map(|v| Q::from_float(v).unwrap()),
    )
}

pub(crate) fn restrict_pcurve(curve: &Curve, [a, b]: [Q; 2]) -> Result<Vec<Curve>> {
    curve.validate()?;
    let [lo, hi] = [(&a).min(&b).clone(), (&a).max(&b).clone()];
    let [start, end] = curve.domain().map(|v| Q::from_float(v).unwrap());
    if lo < start || hi > end || lo == hi {
        return Err(unsupported("Exact pcurve restriction needs a nonempty native interval"));
    }
    let native;
    let curve = if curve.periodic {
        if !nurbs_core::curve_partition_agreement::verify_periodic_source_closure(curve)? {
            return Err(unsupported("Exact periodic pcurve restriction needs a closed native period"));
        }
        // Within the native period, ordinary B-spline evaluation uses the
        // same active poles. Exact closure identifies the endpoint limits.
        let mut basis = curve.clone();
        basis.periodic = false;
        native = basis;
        &native
    } else {
        curve
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn periodic_pcurve_trim_keeps_order_and_exact_endpoints() {
        let curve = Curve {
            degree: 1,
            periodic: true,
            knots: vec![-0.25, 0., 0.25, 0.5, 0.75, 1., 1.25],
            control_points: [[0., 0.], [1., 0.], [1., 1.], [0., 1.], [0., 0.]]
                .map(Vec::from)
                .to_vec(),
            weights: vec![1.; 5],
        };
        let range = |ends: [f64; 2]| ends.map(|v| Q::from_float(v).unwrap());
        let forward = restrict_pcurve(&curve, range([0.125, 1.])).unwrap();
        assert_eq!(forward.len(), 4);
        assert!(forward.iter().all(|piece| !piece.periodic));
        assert_eq!(forward[0].control_points[0], vec![0.5, 0.]);
        assert_eq!(forward[3].control_points[1], vec![0., 0.]);
        assert!(forward.windows(2).all(|pair| {
            pair[0].control_points.last() == pair[1].control_points.first()
        }));
        let reverse = restrict_pcurve(&curve, range([1., 0.125])).unwrap();
        assert_eq!(reverse[0].control_points[0], vec![0., 0.]);
        assert_eq!(reverse[3].control_points[1], vec![0.5, 0.]);
        assert!(restrict_pcurve(&curve, range([0.5, 0.5])).is_err());
        assert!(restrict_pcurve(&curve, range([0.5, 1.25])).is_err());
    }
}
