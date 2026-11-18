//! Exact selected-parameter critical set, including common-image branch ends.
use super::*;
use a::bivariate::{self as bi, Bi};
fn residual(e: &[Bi; 2]) -> Result<Vec<Q>> {
    if e[0].is_empty() || e[1].is_empty() {
        let other = if e[0].is_empty() { &e[1] } else { &e[0] };
        if other.len() != 1 || other[0].is_empty() {
            return Err(refused("Unresolved positive-dimensional residual relation"));
        }
        return Ok(other[0].clone());
    }
    let p = bi::resultant(&e[0], &e[1])?;
    if p.is_empty() {
        return Err(refused("Unresolved exact material pair projection"));
    }
    Ok(p)
}
pub(super) fn pair(x: &[[Q; 3]], y: &[[Q; 3]]) -> Result<Vec<Q>> {
    if let Some((_, remainder)) = bi::component_system(x, y, false)? {
        let h = bi::image_relation(x, y)?;
        let common = bi::image_relation_projection(x, y, &h)?;
        bi::combine_image_projections(&[common, residual(&remainder)?])
    } else {
        residual(&bi::intersection_polynomials(x, y, false)?)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Root {
    pub factor: usize,
    pub interval: [ExactRatio; 2],
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Partition {
    pub selected: usize,
    pub projections: Vec<crate::profile_certificate::ExactPolynomial>,
    pub roots: Vec<Root>,
    pub samples: Vec<ExactRatio>,
}
pub(super) fn partition(
    selected: usize,
    terms: &[Vec<Q>],
    limits: Limits,
) -> Result<(Partition, Vec<Q>)> {
    // Pairwise coprime squarefree factors have exactly the union of the original
    // projection roots. Do not inflate a global product before Sturm isolation.
    let mut factors: Vec<Vec<Q>> = Vec::new();
    let mut degree = 0;
    for term in terms {
        let mut p = bi::combine_image_projections(&[term.clone()])?;
        for previous in &factors {
            p = a::quotient(&p, &a::gcd(&p, previous)?)?;
        }
        if p.len() > 1 {
            degree += p.len() - 1;
            if degree > 1024 {
                return Err(refused("Exact material projection degree exhausted"));
            }
            factors.push(p);
        }
    }
    let mut roots: Vec<(usize, [Q; 2])> = Vec::new();
    let identity = vec![Q::zero(), Q::one()];
    for (factor, p) in factors.iter().enumerate() {
        for root in
            a::algebraic_events::isolates(p, Q::zero(), Q::one(), &Q::new(1.into(), 1024.into()))?
        {
            let mut insertion = roots.len();
            for (j, (other, interval)) in roots.iter().enumerate() {
                let order = a::parameter_order::compare(
                    a::parameter_order::ParameterRef::new(p, &identity, &root),
                    a::parameter_order::ParameterRef::new(&factors[*other], &identity, interval),
                    limits.refinements,
                    64,
                )?;
                if order == std::cmp::Ordering::Equal {
                    return Err(refused("Coprime projection roots unexpectedly coincide"));
                }
                if order == std::cmp::Ordering::Less {
                    insertion = j;
                    break;
                }
            }
            roots.insert(insertion, (factor, root));
        }
    }
    let mut separated = false;
    for step in 0..=limits.refinements {
        let left = roots
            .first()
            .is_none_or(|(_, r)| r[0] > Q::zero() || r[1].is_zero());
        let right = roots
            .last()
            .is_none_or(|(_, r)| r[1] < Q::one() || r[0].is_one());
        if left && right && roots.windows(2).all(|r| r[0].1[1] < r[1].1[0]) {
            separated = true;
            break;
        }
        if step < limits.refinements {
            for (factor, r) in &mut roots {
                a::parameter_order::refine(&factors[*factor], r)?;
            }
        }
    }
    if !separated {
        return Err(refused("Exact material cut ordering exhausted"));
    }
    let mut samples = Vec::new();
    let mut previous = Q::zero();
    for (_, root) in &roots {
        if previous < root[0] {
            samples.push((&previous + &root[0]) / Q::from_integer(2.into()));
        }
        previous = root[1].clone();
    }
    if previous < Q::one() {
        samples.push((previous + Q::one()) / Q::from_integer(2.into()));
    }
    if samples.is_empty() {
        return Err(refused("Exact material cell has no open atom"));
    }
    Ok((
        Partition {
            selected,
            projections: factors
                .iter()
                .map(|p| crate::profile_certificate::algebraic_intersections::polynomial(p))
                .collect(),
            roots: roots
                .iter()
                .map(|(factor, r)| Root {
                    factor: *factor,
                    interval: r.each_ref().map(ratio),
                })
                .collect(),
            samples: samples.iter().map(ratio).collect(),
        },
        samples,
    ))
}
