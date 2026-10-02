//! Certified inverse brackets for nonlinear knot splitting. These brackets are
//! not representable exact knots and must not be promoted to exact materialization.
use super::{MapPiece, certify_reparameterization, map_pieces};
use crate::distance_bounds::Interval as I;
use crate::{Result, check, numeric, resource};
use value_codec::{Value, json};

pub(super) enum Map {
    Pieces(Vec<MapPiece>),
    Composition(Vec<Map>),
}
impl Map {
    pub(super) fn parse(value: &Value) -> Result<Self> {
        if let Some(parts) = value.get("composition").and_then(Value::as_array) {
            Ok(Self::Composition(
                parts.iter().map(Self::parse).collect::<Result<_>>()?,
            ))
        } else {
            let pieces = map_pieces(value)?;
            for piece in &pieces {
                prove_increasing(piece)?;
            }
            Ok(Self::Pieces(pieces))
        }
    }
    pub(super) fn enclosure(&self, input: I, work: &mut usize, limit: usize) -> Result<I> {
        match self {
            Self::Composition(parts) => parts
                .iter()
                .try_fold(input, |t, part| part.enclosure(t, work, limit)),
            Self::Pieces(pieces) => {
                let mut out = [f64::INFINITY, f64::NEG_INFINITY];
                for piece in pieces {
                    let lo = input.lo.max(piece.domain[0]);
                    let hi = input.hi.min(piece.domain[1]);
                    if lo > hi {
                        continue;
                    }
                    if *work >= limit {
                        return Err(resource("Knot preimage evaluation budget exhausted"));
                    }
                    *work += 1;
                    let value = enclose_piece(piece, I::new(lo, hi)?)?;
                    out[0] = out[0].min(value.lo);
                    out[1] = out[1].max(value.hi);
                }
                I::new(out[0], out[1])
            }
        }
    }
    pub(super) fn jet(&self, input: I, work: &mut usize, limit: usize) -> Result<(I, I)> {
        match self {
            Self::Composition(parts) => {
                parts
                    .iter()
                    .try_fold((input, I::point(1.)), |(value, derivative), part| {
                        let (next, factor) = part.jet(value, work, limit)?;
                        Ok((next, derivative.mul(factor)?))
                    })
            }
            Self::Pieces(pieces) => {
                let value = self.enclosure(input, work, limit)?;
                let mut bounds = [f64::INFINITY, f64::NEG_INFINITY];
                for piece in pieces {
                    let lo = input.lo.max(piece.domain[0]);
                    let hi = input.hi.min(piece.domain[1]);
                    if lo > hi {
                        continue;
                    }
                    let derivative = derivative_piece(piece, I::new(lo, hi)?)?;
                    bounds[0] = bounds[0].min(derivative.lo);
                    bounds[1] = bounds[1].max(derivative.hi);
                }
                Ok((value, I::new(bounds[0], bounds[1])?))
            }
        }
    }
}

fn de_casteljau(mut controls: Vec<I>, t: I) -> Result<I> {
    let one = I::point(1.).sub(t)?;
    for size in (1..controls.len()).rev() {
        for i in 0..size {
            controls[i] = controls[i].mul(one)?.add(controls[i + 1].mul(t)?)?;
        }
    }
    Ok(controls[0])
}
fn derivative_piece(piece: &MapPiece, input: I) -> Result<I> {
    let extent = I::point(piece.domain[1]).sub(I::point(piece.domain[0]))?;
    let t = input
        .sub(I::point(piece.domain[0]))?
        .div(extent)?
        .intersect(0., 1.)?;
    let p = piece
        .values
        .iter()
        .zip(&piece.weights)
        .map(|(&x, &w)| I::point(x).mul(I::point(w)))
        .collect::<Result<Vec<_>>>()?;
    let q: Vec<_> = piece.weights.iter().map(|&w| I::point(w)).collect();
    let differentiate = |v: &[I]| {
        v.windows(2)
            .map(|pair| pair[1].sub(pair[0])?.mul(I::point((v.len() - 1) as f64)))
            .collect::<Result<Vec<_>>>()
    };
    let dp = de_casteljau(differentiate(&p)?, t)?;
    let dq = de_casteljau(differentiate(&q)?, t)?;
    let p = de_casteljau(p, t)?;
    let q = de_casteljau(q, t)?.intersect(
        piece.weights.iter().copied().fold(f64::INFINITY, f64::min),
        piece
            .weights
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max),
    )?;
    dp.mul(q)?.sub(p.mul(dq)?)?.div(q.mul(q)?)?.div(extent)
}

fn choose(n: usize, k: usize) -> u64 {
    (0..k.min(n - k)).fold(1, |v, i| v * (n - i) as u64 / (i + 1) as u64)
}
pub(super) fn prove_increasing(piece: &MapPiece) -> Result<[f64; 2]> {
    // Pairwise form of P'Q-PQ' avoids subtracting two large products.
    // Its degree is 2*n-2; all arithmetic encloses the authored binary64 data.
    let n = piece.values.len() - 1;
    let degree = 2 * n - 2;
    let mut coefficients = vec![I::point(0.); degree + 1];
    for i in 0..=n {
        for j in i + 1..=n {
            let k = i + j - 1;
            let scale = I::point((j - i) as f64)
                .mul(I::point(choose(n, i) as f64))?
                .mul(I::point(choose(n, j) as f64))?
                .div(I::point(choose(degree, k) as f64))?;
            let term = I::point(piece.values[j])
                .sub(I::point(piece.values[i]))?
                .mul(I::point(piece.weights[i]))?
                .mul(I::point(piece.weights[j]))?
                .mul(scale)?;
            coefficients[k] = coefficients[k].add(term)?;
        }
    }
    let mut pending = vec![(coefficients, 0usize)];
    let mut cells = 0usize;
    let mut bounds = [f64::INFINITY, f64::NEG_INFINITY];
    while let Some((coefficients, depth)) = pending.pop() {
        cells += 1;
        if cells > 4096 {
            return Err(resource("Map monotonicity subdivision budget exhausted"));
        }
        if coefficients.iter().all(|c| c.lo > 0.) {
            for c in &coefficients {
                bounds[0] = bounds[0].min(c.lo);
                bounds[1] = bounds[1].max(c.hi);
            }
            continue;
        }
        numeric(
            depth < 32 && coefficients.iter().any(|c| c.hi > 0.),
            "Map derivative lacks an outward positive certificate",
        )?;
        let mut row = coefficients;
        let mut left = vec![row[0]];
        let mut right = vec![*row.last().unwrap()];
        while row.len() > 1 {
            row = row
                .windows(2)
                .map(|p| p[0].add(p[1])?.mul(I::point(0.5)))
                .collect::<Result<Vec<_>>>()?;
            left.push(row[0]);
            right.push(*row.last().unwrap());
        }
        right.reverse();
        pending.push((right, depth + 1));
        pending.push((left, depth + 1));
    }
    let physical = I::new(bounds[0], bounds[1])?
        .div(I::point(piece.domain[1]).sub(I::point(piece.domain[0]))?)?;
    Ok([physical.lo, physical.hi])
}
fn enclose_piece(piece: &MapPiece, input: I) -> Result<I> {
    // Authored endpoints are exact real ratios (value*weight)/weight.
    if input.lo == input.hi {
        if input.lo == piece.domain[0] {
            return Ok(I::point(piece.range[0]));
        }
        if input.lo == piece.domain[1] {
            return Ok(I::point(piece.range[1]));
        }
    }
    let t = input
        .sub(I::point(piece.domain[0]))?
        .div(I::point(piece.domain[1]).sub(I::point(piece.domain[0]))?)?
        .intersect(0., 1.)?;
    let one = I::point(1.).sub(t)?;
    let mut p = piece
        .values
        .iter()
        .zip(&piece.weights)
        .map(|(&x, &w)| I::point(x).mul(I::point(w)))
        .collect::<Result<Vec<_>>>()?;
    let mut q: Vec<_> = piece.weights.iter().map(|&w| I::point(w)).collect();
    for size in (1..p.len()).rev() {
        for i in 0..size {
            p[i] = p[i].mul(one)?.add(p[i + 1].mul(t)?)?;
            q[i] = q[i].mul(one)?.add(q[i + 1].mul(t)?)?;
        }
    }
    let denominator = q[0].intersect(
        piece.weights.iter().copied().fold(f64::INFINITY, f64::min),
        piece
            .weights
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max),
    )?;
    p[0].div(denominator)?
        .intersect(piece.range[0], piece.range[1])
}

/// Enclose every requested inverse of a certified increasing rational map.
/// `max_work` bounds leaf evaluations across all roots and nested factors.
/// Any unresolved root refuses; midpoint estimates are never called exact.
pub fn bound_reparameterization_preimages(
    mapping: &Value,
    values: &[f64],
    parameter_tolerance: f64,
    max_work: usize,
) -> Result<Value> {
    check(
        parameter_tolerance.is_finite() && parameter_tolerance > 0.,
        "Preimage tolerance must be positive",
    )?;
    check(
        values.len() <= 256 && max_work > 0 && max_work <= 1_000_000,
        "Preimage request exceeds its resource bounds",
    )?;
    let certificate = certify_reparameterization(mapping, None)?;
    let domain: [f64; 2] = value_codec::from_value(certificate["domain"].clone())
        .map_err(|e| crate::input(e.to_string()))?;
    let range: [f64; 2] = value_codec::from_value(certificate["range"].clone())
        .map_err(|e| crate::input(e.to_string()))?;
    let map = Map::parse(mapping)?;
    let mut work = 0;
    let mut roots = Vec::new();
    for &value in values {
        check(
            value.is_finite() && value >= range[0] && value <= range[1],
            "Requested knot lies outside the map range",
        )?;
        let mut bracket = if value == range[0] {
            I::point(domain[0])
        } else if value == range[1] {
            I::point(domain[1])
        } else {
            I::new(domain[0], domain[1])?
        };
        while bracket.hi > bracket.lo && (bracket.hi - bracket.lo).next_up() > parameter_tolerance {
            let before = bracket;
            // Quarter points let us contract even if the midpoint evaluation
            // straddles the knot. Every update uses a strict outward sign.
            for fraction in [0.25, 0.5, 0.75] {
                let parameter = before.lo + (before.hi - before.lo) * fraction;
                if parameter <= bracket.lo || parameter >= bracket.hi {
                    continue;
                }
                let image = map.enclosure(I::point(parameter), &mut work, max_work)?;
                if image.hi < value {
                    bracket.lo = parameter;
                } else if image.lo > value {
                    bracket.hi = parameter;
                }
            }
            if bracket.lo == before.lo && bracket.hi == before.hi {
                return Err(crate::numeric_err(
                    "Knot preimage unresolved at binary64 interval precision",
                ));
            }
        }
        roots.push(json!({"value":value,"parameterInterval":[bracket.lo,bracket.hi],"exact":bracket.lo==bracket.hi}));
    }
    Ok(
        json!({"preimages":roots,"certificate":{"operation":"bound-reparameterization-preimages",
        "method":"outward-rational-de-Casteljau-monotone-bracketing","parameterTolerance":parameter_tolerance,
        "leafEvaluations":work,"maxWork":max_work,"allRootsEnclosed":true,"mapCertificate":certificate}}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn polynomial() -> Value {
        json!({"pieces":[{"domain":[0.,1.],"range":[0.,1.],"controlValues":[0.,0.25,1.],"weights":[1.,1.,1.]}]})
    }
    fn interval(result: &Value, i: usize) -> [f64; 2] {
        value_codec::from_value(result["preimages"][i]["parameterInterval"].clone()).unwrap()
    }
    #[test]
    fn irrational_knot_inverse_is_enclosed_not_promoted() {
        let report =
            bound_reparameterization_preimages(&polynomial(), &[0., 0.5, 1.], 1e-12, 1000).unwrap();
        let [a, b] = interval(&report, 1);
        let independent = (5f64.sqrt() - 1.) / 2.;
        assert!(a < independent && independent < b && b - a <= 1e-12);
        assert_eq!(report["preimages"][1]["exact"], false);
        assert_eq!(interval(&report, 0), [0., 0.]);
        assert_eq!(interval(&report, 2), [1., 1.]);
    }
    #[test]
    fn rational_nested_factors_preserve_authored_order() {
        let rational = json!({"pieces":[{"domain":[0.,1.],"range":[0.,1.],"controlValues":[0.,1.],"weights":[1.,2.]}]});
        let mapping = json!({"composition":[polynomial(),rational]});
        let report = bound_reparameterization_preimages(&mapping, &[0.5], 1e-11, 1000).unwrap();
        // 2*p/(1+p)=1/2 => p=1/3; p=(t+t*t)/2.
        let independent = ((11f64 / 3.).sqrt() - 1.) / 2.;
        let [a, b] = interval(&report, 0);
        assert!(a < independent && independent < b);
    }
    #[test]
    fn piece_boundaries_and_nonunit_domains_are_enclosed() {
        let mapping = json!({"pieces":[
            {"domain":[2.,3.],"range":[4.,5.],"controlValues":[4.,4.25,5.],"weights":[1.,1.,1.]},
            {"domain":[3.,6.],"range":[5.,8.],"controlValues":[5.,8.],"weights":[2.,1.]}
        ]});
        let report =
            bound_reparameterization_preimages(&mapping, &[4.5, 5., 6.5], 1e-11, 2000).unwrap();
        for (i, expected) in [2. + (5f64.sqrt() - 1.) / 2., 3., 5.]
            .into_iter()
            .enumerate()
        {
            let [a, b] = interval(&report, i);
            assert!(a <= expected && expected <= b && b - a <= 1e-11);
        }
    }
    #[test]
    fn work_exhaustion_and_invalid_requests_refuse() {
        assert!(bound_reparameterization_preimages(&polynomial(), &[0.5], 1e-12, 1).is_err());
        assert!(bound_reparameterization_preimages(&polynomial(), &[1.1], 1e-12, 1000).is_err());
        assert!(bound_reparameterization_preimages(&polynomial(), &[0.5], 0., 1000).is_err());
        assert!(bound_reparameterization_preimages(&polynomial(), &[0.5], 1e-30, 1000).is_err());
    }
}

#[cfg(test)]
mod monotonicity_tests {
    use super::*;
    #[test]
    fn subdivision_proves_positive_derivative_with_negative_control() {
        let piece = MapPiece {
            domain: [0., 1.],
            range: [0., 1.],
            values: vec![0., 1., 0., 1.],
            weights: vec![1.; 4],
        };
        // This derivative vanishes at 1/2 and must refuse.
        assert!(prove_increasing(&piece).is_err());
        let piece = MapPiece {
            values: vec![0., 0.6, 0.4, 1.],
            ..piece
        };
        // Derivative controls [1.8,-0.6,1.8], minimum 0.6.
        prove_increasing(&piece).unwrap();
    }
    #[test]
    fn pairwise_derivative_avoids_large_weight_cancellation() {
        let piece = MapPiece {
            domain: [0., 1.],
            range: [0., 1.],
            values: vec![0., 1.],
            weights: vec![1e-12, 1e12],
        };
        prove_increasing(&piece).unwrap();
    }
}

#[cfg(test)]
mod public_monotonicity_tests {
    use super::*;
    #[test]
    fn public_certificate_accepts_subdivision_and_rejects_stationary_map() {
        let mapping = json!({"pieces":[{"domain":[0.,1.],"range":[0.,1.],
            "controlValues":[0.,0.6,0.4,1.],"weights":[1.,1.,1.,1.]}]});
        let certificate = certify_reparameterization(&mapping, None).unwrap();
        assert!(
            certificate["pieces"][0]["derivativeNumeratorBounds"][0]
                .as_f64()
                .unwrap()
                > 0.
        );
        let report = bound_reparameterization_preimages(&mapping, &[0.5], 1e-10, 1000).unwrap();
        let root = &report["preimages"][0]["parameterInterval"];
        assert!(root[0].as_f64().unwrap() <= 0.5 && root[1].as_f64().unwrap() >= 0.5);
        let stationary = json!({"pieces":[{"domain":[0.,1.],"range":[0.,1.],
            "controlValues":[0.,1.,0.,1.],"weights":[1.,1.,1.,1.]}]});
        assert!(certify_reparameterization(&stationary, None).is_err());
    }
}
