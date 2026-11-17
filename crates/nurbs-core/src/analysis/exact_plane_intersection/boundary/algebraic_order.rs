//! Exact numerical ordering of rational and quadratic support-clock values.
use super::{AlgebraicBoundaryClock, Refusal, Work, check};
use crate::Result;
use num_rational::BigRational as Q;
use num_traits::{Signed, Zero};
use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlgebraicBoundaryOrderReport {
    /// Groups of equal clock coordinates in increasing numerical order.
    /// Each group lists the original input indices in their original order.
    pub outcome: std::result::Result<Vec<Vec<usize>>, Refusal>,
    pub work_used: u64,
}

/// Order rational and quadratic clock values using exact polynomial signs.
/// Coincident coordinates preserve all contributing input indices.
pub fn order_algebraic_boundary_clocks(
    points: &[AlgebraicBoundaryClock],
    max_work: u64,
) -> Result<AlgebraicBoundaryOrderReport> {
    validate(points, max_work)?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let outcome = order(points, &mut work);
    Ok(AlgebraicBoundaryOrderReport {
        outcome,
        work_used: work.used,
    })
}

fn validate(points: &[AlgebraicBoundaryClock], max_work: u64) -> Result<()> {
    check(
        points.len() <= 4096 && (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Algebraic boundary ordering supports at most 4096 points and bounded work",
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IsolatedAlgebraicBoundaryClock {
    pub indices: Vec<usize>,
    pub parameter: [Q; 2],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlgebraicBoundaryIsolation {
    /// Strictly separated closed clock intervals in numerical order.
    pub clocks: Vec<IsolatedAlgebraicBoundaryClock>,
    /// One rational clock value strictly between every pair of adjacent boxes.
    pub witnesses: Vec<Q>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlgebraicBoundaryIsolationReport {
    pub outcome: std::result::Result<AlgebraicBoundaryIsolation, Refusal>,
    pub work_used: u64,
}

/// Isolate an ordered set of rational and quadratic contact coordinates.
/// Witnesses separate the supplied coordinates and can be evaluated
/// exactly on the support line.
pub fn isolate_ordered_algebraic_boundary_clocks(
    points: &[AlgebraicBoundaryClock],
    max_work: u64,
) -> Result<AlgebraicBoundaryIsolationReport> {
    validate(points, max_work)?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let outcome = isolate(points, &mut work);
    Ok(AlgebraicBoundaryIsolationReport {
        outcome,
        work_used: work.used,
    })
}

type Checked<T> = std::result::Result<T, Refusal>;

enum Key {
    Rational(Q),
    Quadratic(Quadratic),
}

struct Quadratic {
    linear: Q,
    constant: Q,
    vertex: Q,
    upper: bool,
}

fn order(points: &[AlgebraicBoundaryClock], work: &mut Work) -> Checked<Vec<Vec<usize>>> {
    let keys = points
        .iter()
        .map(|point| key(point, work))
        .collect::<Checked<Vec<_>>>()?;
    group_keys(&keys, work)
}

fn group_keys(keys: &[Key], work: &mut Work) -> Checked<Vec<Vec<usize>>> {
    let mut indices: Vec<_> = (0..keys.len()).collect();
    let mut scratch = vec![0; indices.len()];
    sort(&mut indices, &mut scratch, keys, work)?;
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for i in indices {
        if let Some(group) = groups.last_mut()
            && compare(&keys[group[0]], &keys[i], work)? == Ordering::Equal
        {
            group.push(i);
        } else {
            groups.push(vec![i]);
        }
    }
    Ok(groups)
}

fn key(point: &AlgebraicBoundaryClock, work: &mut Work) -> Checked<Key> {
    match point {
        AlgebraicBoundaryClock::Rational(value) => {
            work.charge(1 + (value.numer().bits() + value.denom().bits()) / 64)?;
            Ok(Key::Rational(value.clone()))
        }
        AlgebraicBoundaryClock::Quadratic {
            linear,
            constant,
            upper_root,
        } => {
            let four = Q::from_integer(4.into());
            let b2 = work.op(linear, linear, |a, b| a * b)?;
            let offset = work.op(&four, constant, |a, b| a * b)?;
            let discriminant = work.op(&b2, &offset, |a, b| a - b)?;
            if discriminant.is_negative() {
                return Err(Refusal::InvalidOutput);
            }
            work.charge(1 + (discriminant.numer().bits() + discriminant.denom().bits()) / 64)?;
            let candidate = Q::new(discriminant.numer().sqrt(), discriminant.denom().sqrt());
            let two = Q::from_integer(2.into());
            if work.op(&candidate, &candidate, |a, b| a * b)? == discriminant {
                let signed = if *upper_root { candidate } else { -candidate };
                let numerator = work.op(&signed, linear, |a, b| a - b)?;
                return Ok(Key::Rational(work.op(&numerator, &two, |a, b| a / b)?));
            }
            Ok(Key::Quadratic(Quadratic {
                linear: linear.clone(),
                constant: constant.clone(),
                vertex: work.op(linear, &two, |a, b| -a / b)?,
                upper: *upper_root,
            }))
        }
    }
}

fn sort(
    indices: &mut [usize],
    scratch: &mut [usize],
    keys: &[Key],
    work: &mut Work,
) -> Checked<()> {
    if indices.len() < 2 {
        return Ok(());
    }
    let half = indices.len() / 2;
    let (left, right) = indices.split_at_mut(half);
    let (left_buffer, right_buffer) = scratch.split_at_mut(half);
    sort(left, left_buffer, keys, work)?;
    sort(right, right_buffer, keys, work)?;
    let mut i = 0;
    let mut j = 0;
    let mut count = 0;
    while i < left.len() && j < right.len() {
        if compare(&keys[left[i]], &keys[right[j]], work)? != Ordering::Greater {
            scratch[count] = left[i];
            i += 1;
        } else {
            scratch[count] = right[j];
            j += 1;
        }
        count += 1;
    }
    scratch[count..count + left.len() - i].copy_from_slice(&left[i..]);
    scratch[count + left.len() - i..].copy_from_slice(&right[j..]);
    indices.copy_from_slice(scratch);
    Ok(())
}

fn compare(a: &Key, b: &Key, work: &mut Work) -> Checked<Ordering> {
    work.charge(1)?;
    match (a, b) {
        (Key::Rational(a), Key::Rational(b)) => Ok(a.cmp(b)),
        (Key::Quadratic(a), Key::Rational(b)) => root_vs_rational(a, b, work),
        (Key::Rational(a), Key::Quadratic(b)) => Ok(root_vs_rational(b, a, work)?.reverse()),
        (Key::Quadratic(a), Key::Quadratic(b)) => {
            let half = root_vs_rational(b, &a.vertex, work)?;
            if let Some(ordering) = opposite_halves(a.upper, half) {
                return Ok(ordering);
            }
            // Reduce the left polynomial modulo the right polynomial. Its
            // sign at the right root is now the sign of a linear expression.
            let slope = work.op(&a.linear, &b.linear, |a, b| a - b)?;
            let offset = work.op(&a.constant, &b.constant, |a, b| a - b)?;
            let sign = if slope.is_zero() {
                offset.cmp(&Q::zero())
            } else {
                let threshold = work.op(&offset, &slope, |a, b| -a / b)?;
                let sign = root_vs_rational(b, &threshold, work)?;
                if slope.is_negative() {
                    sign.reverse()
                } else {
                    sign
                }
            };
            Ok(if a.upper { sign.reverse() } else { sign })
        }
    }
}

fn root_vs_rational(root: &Quadratic, value: &Q, work: &mut Work) -> Checked<Ordering> {
    if let Some(ordering) = opposite_halves(root.upper, value.cmp(&root.vertex)) {
        return Ok(ordering);
    }
    let term = work.op(value, &root.linear, |a, b| a + b)?;
    let product = work.op(value, &term, |a, b| a * b)?;
    let sign = work
        .op(&product, &root.constant, |a, b| a + b)?
        .cmp(&Q::zero());
    Ok(if root.upper { sign.reverse() } else { sign })
}

fn opposite_halves(upper: bool, other_side: Ordering) -> Option<Ordering> {
    if upper && other_side != Ordering::Greater {
        Some(Ordering::Greater)
    } else if !upper && other_side != Ordering::Less {
        Some(Ordering::Less)
    } else {
        None
    }
}

fn isolate(
    points: &[AlgebraicBoundaryClock],
    work: &mut Work,
) -> Checked<AlgebraicBoundaryIsolation> {
    let keys = points
        .iter()
        .map(|point| key(point, work))
        .collect::<Checked<Vec<_>>>()?;
    let mut clocks = group_keys(&keys, work)?
        .into_iter()
        .map(|indices| {
            let parameter = initial_range(&keys[indices[0]], work)?;
            Ok(IsolatedAlgebraicBoundaryClock { indices, parameter })
        })
        .collect::<Checked<Vec<_>>>()?;
    for i in 1..clocks.len() {
        let (left, right) = clocks.split_at_mut(i);
        let a = &mut left[i - 1];
        let b = &mut right[0];
        while a.parameter[1] >= b.parameter[0] {
            refine(&keys[a.indices[0]], &mut a.parameter, work)?;
            refine(&keys[b.indices[0]], &mut b.parameter, work)?;
        }
    }
    let witnesses = clocks
        .windows(2)
        .map(|pair| midpoint(&pair[0].parameter[1], &pair[1].parameter[0], work))
        .collect::<Checked<Vec<_>>>()?;
    Ok(AlgebraicBoundaryIsolation { clocks, witnesses })
}

fn initial_range(key: &Key, work: &mut Work) -> Checked<[Q; 2]> {
    match key {
        Key::Rational(value) => Ok([value.clone(), value.clone()]),
        Key::Quadratic(root) => {
            let max = root.linear.abs().max(root.constant.abs());
            let one = Q::from_integer(1.into());
            let bound = work.op(&max, &one, |a, b| a + b)?;
            Ok(if root.upper {
                [root.vertex.clone(), bound]
            } else {
                [-bound, root.vertex.clone()]
            })
        }
    }
}

fn refine(key: &Key, range: &mut [Q; 2], work: &mut Work) -> Checked<()> {
    if let Key::Quadratic(root) = key {
        let middle = midpoint(&range[0], &range[1], work)?;
        let side = root_vs_rational(root, &middle, work)?;
        range[usize::from(side == Ordering::Less)] = middle;
    }
    Ok(())
}

fn midpoint(lo: &Q, hi: &Q, work: &mut Work) -> Checked<Q> {
    let two = Q::from_integer(2.into());
    Ok(work.op(lo, hi, |a, b| (a + b) / &two)?)
}
