//! Proposal only: the complete transverse census still authorizes every probe.
use super::*;
/// Choose a scale from exact normal-line intersections instead of starting at
/// one world unit. All nonzero transverse distances contribute. Unresolved
/// bounds refuse a proposal; no branch or center ownership is certified here.
pub(in crate::profile_certificate) fn suggest(
    nets: &[Vec<[Q; 3]>],
    selected: usize,
    t: &Q,
    refinements: usize,
) -> Result<Q> {
    if selected >= nets.len() || refinements > 512 || t <= &Q::zero() || t >= &Q::one() {
        return Err(refused("Invalid transverse radius proposal"));
    }
    let powers = [
        power(&nets[selected], 0),
        power(&nets[selected], 1),
        power(&nets[selected], 2),
    ];
    let h = powers.clone().map(|p| at(&p, t));
    if h[2] <= Q::zero() {
        return Err(refused("Nonpositive radius proposal denominator"));
    }
    let point = [&h[0] / &h[2], &h[1] / &h[2]];
    let d: [Q; 2] = std::array::from_fn(|a| {
        at(&derivative(&powers[a]), t) * &h[2] - &h[a] * at(&derivative(&powers[2]), t)
    });
    let normal = [-d[1].clone(), d[0].clone()];
    let length = &normal[0] * &normal[0] + &normal[1] * &normal[1];
    if length.is_zero() {
        return Err(refused("Singular transverse radius proposal"));
    }
    let mut radius = Q::one();
    for net in nets {
        if net.iter().any(|h| h[2] <= Q::zero()) {
            return Err(refused("Nonpositive radius proposal weight"));
        }
        let w = power(net, 2);
        let x = sub(&power(net, 0), &w.iter().map(|v| v * &point[0]).collect::<Poly>());
        let y = sub(&power(net, 1), &w.iter().map(|v| v * &point[1]).collect::<Poly>());
        let linear = |a: &Q, b: &Q| {
            let mut p = vec![Q::zero(); x.len().max(y.len())];
            for (i, v) in x.iter().enumerate() {
                p[i] += v * a;
            }
            for (i, v) in y.iter().enumerate() {
                p[i] += v * b;
            }
            trim(p)
        };
        let f = linear(&d[0], &d[1]);
        let lateral = linear(&normal[0], &normal[1]);
        if f.is_empty() {
            let values = net
                .iter()
                .map(|h| {
                    ((&h[0] - &point[0] * &h[2]) * &normal[0]
                        + (&h[1] - &point[1] * &h[2]) * &normal[1])
                        / &h[2]
                })
                .collect::<Vec<_>>();
            if !(values.iter().all(|v| v > &Q::zero()) || values.iter().all(|v| v < &Q::zero())) {
                return Err(refused("Unresolved coincident normal-line radius proposal"));
            }
            let distance = values.iter().map(Q::abs).min().unwrap();
            radius = radius.min(distance / (&length * Q::from_integer(4.into())));
            continue;
        }
        for mut root in
            algebraic_events::isolates(&f, Q::zero(), Q::one(), &Q::new(1.into(), 1024.into()))?
        {
            if zero_at(&lateral, &f, &root)? {
                continue;
            }
            let mut distance = None;
            for step in 0..=refinements {
                let r = parameter_order::range(&lateral, &root);
                let lower = if r[0] > Q::zero() {
                    Some(r[0].clone())
                } else if r[1] < Q::zero() {
                    Some(-&r[1])
                } else {
                    None
                };
                if let Some(lower) = lower {
                    let weight = parameter_order::range(&w, &root)[1].clone();
                    if weight > Q::zero() {
                        distance = Some(lower / weight);
                        break;
                    }
                }
                if step < refinements {
                    parameter_order::refine(&f, &mut root)?;
                }
            }
            let distance =
                distance.ok_or_else(|| refused("Transverse radius distance remains unresolved"))?;
            radius = radius.min(distance / (&length * Q::from_integer(4.into())));
        }
    }
    Ok(radius)
}
