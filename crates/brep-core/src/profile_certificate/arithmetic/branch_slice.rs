//! Intrinsic tangent ordering on a common exact rational coordinate slice.
//! Inputs are contact-free monotone neighbourhoods of the same owned point.
use super::*;
use branch_neighbourhood::{ParameterValue, box_at, local};
use event_coordinates as exact;
use parameter_order::{fraction_range, refine};
#[derive(Clone)]
pub(in crate::profile_certificate) struct SliceBranch {
    pub net: Vec<[Q; 3]>,
    pub domain: [Q; 2],
    pub contact: ParameterValue,
    pub cut: Q,
    pub cut_point: [Q; 2],
    pub direction: i8,
}
struct Value {
    f: Poly,
    root: [Q; 2],
    numerator: Poly,
    denominator: Poly,
}
fn choose(
    a: &SliceBranch,
    b: &SliceBranch,
    axis: usize,
    sign: i8,
    max_refinements: usize,
) -> Result<Q> {
    let p = local(&a.contact.value, &a.domain);
    let f = &a.contact.primitive;
    let n = exact::compose(&power(&a.net, axis), &p, f)?;
    let d = exact::compose(&power(&a.net, 2), &p, f)?;
    let mut root = a.contact.interval.clone();
    let end = if sign > 0 {
        a.cut_point[axis].clone().min(b.cut_point[axis].clone())
    } else {
        a.cut_point[axis].clone().max(b.cut_point[axis].clone())
    };
    for step in 0..=max_refinements {
        if let Some(r) = fraction_range(&n, &d, &root) {
            if sign > 0 && r[1] < end {
                return Ok((&r[1] + end) / Q::from_integer(2.into()));
            }
            if sign < 0 && r[0] > end {
                return Ok((&r[0] + end) / Q::from_integer(2.into()));
            }
        }
        if step < max_refinements {
            refine(f, &mut root)?;
        }
    }
    Err(refused(
        "Common original tangent slice remains unproven within allowance",
    ))
}
fn value(a: &SliceBranch, axis: usize, q: &Q, max_refinements: usize) -> Result<Value> {
    let powers = [power(&a.net, 0), power(&a.net, 1), power(&a.net, 2)];
    let h = sub(
        &powers[axis],
        &powers[2].iter().map(|v| v * q).collect::<Poly>(),
    );
    if h.is_empty() {
        return Err(refused(
            "Original monotone slice coordinate became constant",
        ));
    }
    let mut candidates =
        algebraic_events::isolates(&h, Q::zero(), Q::one(), &Q::new(1.into(), 1024.into()))?;
    let p = local(&a.contact.value, &a.domain);
    let mut contact = a.contact.interval.clone();
    let cut = (&a.cut - &a.domain[0]) / (&a.domain[1] - &a.domain[0]);
    for step in 0..=max_refinements {
        let b = box_at(&p, &a.contact.primitive, &contact)?;
        let selected = candidates
            .iter()
            .filter(|r| {
                if a.direction > 0 {
                    r[0] > b[1] && r[1] < cut
                } else {
                    r[1] < b[0] && r[0] > cut
                }
            })
            .collect::<Vec<_>>();
        if selected.len() > 1 {
            return Err(refused("Original monotone branch has multiple slice roots"));
        }
        if let Some(root) = selected.first() {
            return Ok(Value {
                f: h,
                root: (*root).clone(),
                numerator: powers[1 - axis].clone(),
                denominator: powers[2].clone(),
            });
        }
        if step < max_refinements {
            refine(&a.contact.primitive, &mut contact)?;
            for r in &mut candidates {
                refine(&h, r)?;
            }
        }
    }
    Err(refused(
        "Unique original branch slice root remains unproven within allowance",
    ))
}
pub(in crate::profile_certificate) fn compare(
    a: SliceBranch,
    b: SliceBranch,
    axis: usize,
    sign: i8,
    refinements: usize,
    matrix: usize,
) -> Result<std::cmp::Ordering> {
    if axis > 1 || !matches!(sign, -1 | 1) {
        return Err(refused("Invalid original tangent slice axis"));
    }
    let q = choose(&a, &b, axis, sign, refinements)?;
    let a = value(&a, axis, &q, refinements)?;
    let b = value(&b, axis, &q, refinements)?;
    let result = parameter_order::compare_rational(
        &a.f,
        [&a.numerator, &a.denominator],
        &a.root,
        &b.f,
        [&b.numerator, &b.denominator],
        &b.root,
        refinements,
        matrix,
    )?;
    if result == std::cmp::Ordering::Equal {
        return Err(refused(
            "Original contact-free tangent slices are not separated",
        ));
    }
    Ok(if (axis == 0 && sign > 0) || (axis == 1 && sign < 0) {
        result
    } else {
        result.reverse()
    })
}
