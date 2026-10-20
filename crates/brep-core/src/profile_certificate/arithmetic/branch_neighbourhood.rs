//! Exact contact-free monotone source branch neighbourhood, before a rational cut.
use super::*;
use event_coordinates as exact;
use parameter_order::{closed_roots, range, refine};
#[derive(Clone)]
pub(in crate::profile_certificate) struct ParameterValue {
    pub primitive: Poly,
    pub value: Poly,
    pub interval: [Q; 2],
}
#[derive(Clone, Debug, PartialEq)]
pub(in crate::profile_certificate) struct Neighbourhood {
    pub root: [Q; 2],
    pub parameter_box: [Q; 2],
    pub cut: Q,
    pub cut_point: [Q; 2],
    pub contact_critical: bool,
    pub steps: usize,
}
pub(super) fn local(p: &Poly, domain: &[Q; 2]) -> Poly {
    sub(p, &[domain[0].clone()])
        .iter()
        .map(|q| q / (&domain[1] - &domain[0]))
        .collect()
}
pub(super) fn box_at(p: &Poly, f: &Poly, r: &[Q; 2]) -> Result<[Q; 2]> {
    if exact::zero_at(p, f, r)? {
        return Ok([Q::zero(), Q::zero()]);
    }
    if exact::equals_one(p, f, r)? {
        return Ok([Q::one(), Q::one()]);
    }
    let b = range(p, r);
    Ok([b[0].clone().max(Q::zero()), b[1].clone().min(Q::one())])
}
pub(in crate::profile_certificate) fn prove(
    net: &[[Q; 3]],
    domain: &[Q; 2],
    contact: ParameterValue,
    mut neighbour: Option<ParameterValue>,
    direction: i8,
    axis: usize,
    axis_sign: i8,
    max_steps: usize,
) -> Result<Neighbourhood> {
    if axis > 1
        || !matches!(direction, -1 | 1)
        || !matches!(axis_sign, -1 | 1)
        || max_steps == 0
        || max_steps > 512
        || domain[0] >= domain[1]
        || net.iter().any(|h| h[2] <= Q::zero())
    {
        return Err(refused(
            "Invalid original monotone branch neighbourhood context",
        ));
    }
    let powers = [power(net, 0), power(net, 1), power(net, 2)];
    let critical = sub(
        &product(&derivative(&powers[axis]), &powers[2]),
        &product(&powers[axis], &derivative(&powers[2])),
    );
    if critical.is_empty() {
        return Err(refused("Selected branch coordinate is constant"));
    }
    let p = local(&contact.value, domain);
    let mut root = contact.interval.clone();
    let contact_critical = exact::zero_at(
        &exact::compose(&critical, &p, &contact.primitive)?,
        &contact.primitive,
        &root,
    )?;
    let mut cut = if direction > 0 { Q::one() } else { Q::zero() };
    for step in 1..=max_steps {
        let b = box_at(&p, &contact.primitive, &root)?;
        if b[0] > b[1] {
            return Err(refused(
                "Original contact parameter enclosure leaves its actual source cell",
            ));
        }
        if (direction > 0 && b[1] < cut) || (direction < 0 && b[0] > cut) {
            cut =
                (cut + if direction > 0 {
                    b[1].clone()
                } else {
                    b[0].clone()
                }) / Q::from_integer(2.into());
            let natural = &domain[0] + (&domain[1] - &domain[0]) * &cut;
            let separated = neighbour.as_ref().is_none_or(|n| {
                let r = range(&n.value, &n.interval);
                if direction > 0 {
                    natural < r[0]
                } else {
                    natural > r[1]
                }
            });
            let hull = [b[0].clone().min(cut.clone()), b[1].clone().max(cut.clone())];
            if separated && closed_roots(&critical, &hull)? == usize::from(contact_critical) {
                let sign = at(&critical, &cut).cmp(&Q::zero());
                let expected = if axis_sign * direction > 0 {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Less
                };
                if sign == expected {
                    let w = at(&powers[2], &cut);
                    if w <= Q::zero() {
                        return Err(refused("Original cut denominator is not positive"));
                    }
                    return Ok(Neighbourhood {
                        root,
                        parameter_box: b,
                        cut: natural,
                        cut_point: [at(&powers[0], &cut) / &w, at(&powers[1], &cut) / w],
                        contact_critical,
                        steps: step,
                    });
                }
            }
        }
        if step < max_steps {
            refine(&contact.primitive, &mut root)?;
            if let Some(n) = &mut neighbour {
                refine(&n.primitive, &mut n.interval)?;
            }
        }
    }
    Err(refused(
        "Original contact-free monotone neighbourhood remains unproven within allowance",
    ))
}
