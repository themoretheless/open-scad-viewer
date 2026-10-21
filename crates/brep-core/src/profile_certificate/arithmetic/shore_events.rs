//! Complete exact original-cell intersections with one rational chart shore.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::profile_certificate) struct ShoreRoot {
    pub primitive: Poly,
    pub root: [Q; 2],
    pub parameter: Poly,
    pub position: [[Poly; 2]; 2],
    pub order: usize,
    pub sides: [Option<i8>; 2],
}
fn verify_roots(f: &Poly, roots: &[[Q; 2]]) -> Result<()> {
    if roots.len() != parameter_order::closed_roots(f, &[Q::zero(), Q::one()])? {
        return Err(refused("Original shore root census omits or adds a root"));
    }
    let mut previous: Option<&[Q; 2]> = None;
    for root in roots {
        if root[0] < Q::zero()
            || root[1] > Q::one()
            || root[0] > root[1]
            || parameter_order::closed_roots(f, root)? != 1
            || previous
                .is_some_and(|p| p[1] > root[0] || (p[1] == root[0] && at(f, &root[0]).is_zero()))
        {
            return Err(refused(
                "Original shore isolating intervals are not disjoint selected roots",
            ));
        }
        previous = Some(root);
    }
    Ok(())
}
/// None is the proved homogeneous polynomial identity on the entire cell.
/// Some(empty) proves an empty finite root census, never budget exhaustion.
pub(in crate::profile_certificate) fn produce(
    net: &[[Q; 3]],
    domain: &[Q; 2],
    axis: usize,
    shore: &Q,
    width: &Q,
    refinements: usize,
) -> Result<Option<Vec<ShoreRoot>>> {
    if axis > 1
        || net.len() < 2
        || net.len() > 9
        || domain[0] >= domain[1]
        || width <= &Q::zero()
        || refinements > 512
        || net.iter().any(|p| p[2] <= Q::zero())
    {
        return Err(refused("Invalid original chart-shore root context"));
    }
    let w = power(net, 2);
    let h = sub(
        &power(net, axis),
        &w.iter().map(|q| q * shore).collect::<Poly>(),
    );
    event_coordinates::bounded(&h)?;
    if h.is_empty() {
        return Ok(None);
    }
    let mut f = quotient(&h, &gcd(&h, &derivative(&h))?)?;
    let scale = f
        .last()
        .cloned()
        .ok_or_else(|| refused("Original shore root primitive is empty"))?;
    for q in &mut f {
        *q /= &scale;
    }
    let roots = algebraic_events::isolates(&f, Q::zero(), Q::one(), width)?;
    verify_roots(&f, &roots)?;
    let mut out = Vec::new();
    for root in roots {
        let mut derivative_ = h.clone();
        let mut order = 0;
        let sign = loop {
            let sign = event_coordinates::sign_at(&derivative_, &f, &root, refinements)?;
            if sign != 0 {
                break sign;
            }
            order += 1;
            derivative_ = derivative(&derivative_);
            if derivative_.is_empty() {
                return Err(refused(
                    "Original shore root has no finite nonzero derivative",
                ));
            }
        };
        let before = if event_coordinates::zero_at(&vec![Q::zero(), Q::one()], &f, &root)? {
            None
        } else {
            Some(if order % 2 == 0 { sign } else { -sign })
        };
        let after = if event_coordinates::equals_one(&vec![Q::zero(), Q::one()], &f, &root)? {
            None
        } else {
            Some(sign)
        };
        let parameter = trim(vec![domain[0].clone(), &domain[1] - &domain[0]]);
        let denominator = event_coordinates::modulo(&w, &f)?;
        if event_coordinates::zero_at(&denominator, &f, &root)? {
            return Err(refused("Original shore event denominator vanishes"));
        }
        let position = [
            [
                event_coordinates::modulo(&power(net, 0), &f)?,
                denominator.clone(),
            ],
            [event_coordinates::modulo(&power(net, 1), &f)?, denominator],
        ];
        out.push(ShoreRoot {
            primitive: f.clone(),
            root,
            parameter: parameter.clone(),
            position,
            order,
            sides: [before, after],
        });
    }
    Ok(Some(out))
}
pub(in crate::profile_certificate) fn point(net: &[[Q; 3]], local: &Q) -> Result<[Q; 2]> {
    if net.len() < 2
        || net.len() > 9
        || local <= &Q::zero()
        || local >= &Q::one()
        || net.iter().any(|p| p[2] <= Q::zero())
    {
        return Err(refused("Invalid original chart interval witness"));
    }
    let w = at(&power(net, 2), local);
    if w <= Q::zero() {
        return Err(refused(
            "Original chart interval denominator is not positive",
        ));
    }
    Ok([
        at(&power(net, 0), local) / &w,
        at(&power(net, 1), local) / w,
    ])
}
/// Exact homogeneous source view. Retains degree, weights and original clock;
/// no shifted floating-point control polygon is constructed.
pub(in crate::profile_certificate) fn translated_net(
    net: &[[Q; 3]],
    offset: &[Q; 2],
) -> Result<Vec<[Q; 3]>> {
    if net.len() < 2 || net.len() > 9 || net.iter().any(|p| p[2] <= Q::zero()) {
        return Err(refused(
            "Invalid original translated homogeneous source cell",
        ));
    }
    Ok(net
        .iter()
        .map(|p| {
            [
                &p[0] + &offset[0] * &p[2],
                &p[1] + &offset[1] * &p[2],
                p[2].clone(),
            ]
        })
        .collect())
}
pub(in crate::profile_certificate) fn translated_point(
    net: &[[Q; 3]],
    domain: &[Q; 2],
    parameter: &Poly,
    f: &Poly,
    root: &[Q; 2],
    offset: &[Q; 2],
    refinements: usize,
) -> Result<[[Poly; 2]; 2]> {
    if refinements > 512
        || domain[0] >= domain[1]
        || net.len() < 2
        || net.len() > 9
        || net.iter().any(|p| p[2] <= Q::zero())
    {
        return Err(refused("Invalid original translated point source cell"));
    }
    let low = sub(parameter, &[domain[0].clone()]);
    let high = sub(&[domain[1].clone()], parameter);
    if event_coordinates::sign_at(&low, f, root, refinements)? < 0
        || event_coordinates::sign_at(&high, f, root, refinements)? < 0
    {
        return Err(refused(
            "Original translated point parameter is outside its source cell",
        ));
    }
    let local = low
        .iter()
        .map(|q| q / (&domain[1] - &domain[0]))
        .collect::<Poly>();
    let w = event_coordinates::compose(&power(net, 2), &local, f)?;
    if event_coordinates::zero_at(&w, f, root)? {
        return Err(refused("Original translated point denominator vanishes"));
    }
    let shifted = translated_net(net, offset)?;
    let mut result = std::array::from_fn(|_| [Vec::new(), Vec::new()]);
    for axis in 0..2 {
        let numerator = event_coordinates::compose(&power(&shifted, axis), &local, f)?;
        result[axis] = [numerator, w.clone()];
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn translated_homogeneous_view_preserves_exact_census_and_inverse() {
        let q = |n: i64| Q::from_integer(n.into());
        let net = vec![[q(0), q(0), q(1)], [q(2), q(3), q(3)], [q(4), q(1), q(2)]];
        let offset = [Q::new(1.into(), 3.into()), Q::new((-7).into(), 11.into())];
        let shifted = translated_net(&net, &offset).unwrap();
        assert_eq!(
            translated_net(&shifted, &[-offset[0].clone(), -offset[1].clone()]).unwrap(),
            net
        );
        assert!(shifted.iter().zip(&net).all(|(a, b)| a[2] == b[2]));
        let width = Q::new(1.into(), 1024.into());
        let original = produce(&net, &[q(2), q(5)], 0, &q(1), &width, 192)
            .unwrap()
            .unwrap();
        let moved = produce(
            &shifted,
            &[q(2), q(5)],
            0,
            &(q(1) + &offset[0]),
            &width,
            192,
        )
        .unwrap()
        .unwrap();
        assert!(!original.is_empty());
        assert_eq!(original.len(), moved.len());
        for (a, b) in original.iter().zip(&moved) {
            assert_eq!(
                (&a.primitive, &a.root, &a.parameter, a.order, a.sides),
                (&b.primitive, &b.root, &b.parameter, b.order, b.sides)
            );
        }
        assert!(translated_net(&[[q(0), q(0), q(0)], [q(1), q(1), q(1)]], &offset).is_err());
    }
    #[test]
    fn independent_shore_census_rejects_omitted_and_duplicate_roots() {
        let f = vec![Q::zero(), -Q::one(), Q::one()];
        let roots = vec![[Q::zero(), Q::zero()], [Q::one(), Q::one()]];
        verify_roots(&f, &roots).unwrap();
        assert!(verify_roots(&f, &roots[..1]).is_err());
        assert!(verify_roots(&f, &[roots[0].clone(), roots[0].clone()]).is_err());
    }

    #[test]
    fn finite_tangent_and_identity_shore_censuses_are_distinct() {
        let q = |n: i64| Q::from_integer(n.into());
        let tangent = vec![[q(1), q(0), q(1)], [q(-1), q(1), q(1)], [q(1), q(2), q(1)]];
        let roots = produce(
            &tangent,
            &[q(0), q(1)],
            0,
            &q(0),
            &Q::new(1.into(), 1024.into()),
            192,
        )
        .unwrap()
        .unwrap();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].order, 2);
        assert_eq!(roots[0].sides, [Some(1), Some(1)]);
        let line = vec![[q(0), q(0), q(1)], [q(0), q(1), q(1)]];
        assert!(
            produce(
                &line,
                &[q(0), q(1)],
                0,
                &q(0),
                &Q::new(1.into(), 1024.into()),
                192
            )
            .unwrap()
            .is_none()
        );
        assert!(
            produce(
                &line,
                &[q(0), q(1)],
                0,
                &q(2),
                &Q::new(1.into(), 1024.into()),
                192
            )
            .unwrap()
            .unwrap()
            .is_empty()
        );
    }
}
