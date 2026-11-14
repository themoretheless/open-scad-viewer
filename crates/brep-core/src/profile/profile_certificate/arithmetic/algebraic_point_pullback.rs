//! Complete finite preimages of one selected exact algebraic point on a source cell.
use super::*;
use event_coordinates as coordinates;
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::profile::profile_certificate) struct Preimage {
    pub primitive: Poly,
    pub interval: [Q; 2],
    pub source_primitive: Poly,
    pub local_parameter: Poly,
    pub position: [[Poly; 2]; 2],
}
fn affine(p: &Poly, a: &Q, b: &Q) -> Result<Poly> {
    p.iter().rev().try_fold(Vec::new(), |v, c| {
        let mut r = product(&v, &[a.clone(), b - a]);
        if r.is_empty() {
            r.push(c.clone());
        } else {
            r[0] += c;
        }
        let r = trim(r);
        coordinates::bounded(&r)?;
        Ok(r)
    })
}
const MAX_NET: usize = 9;
fn verify_input(
    f: &Poly,
    root: &[Q; 2],
    point: &[[Poly; 2]; 2],
    net: &[[Q; 3]],
    width: &Q,
) -> Result<()> {
    coordinates::bounded(f)?;
    if f.len() < 2
        || root[0] > root[1]
        || width <= &Q::zero()
        || !(2..=MAX_NET).contains(&net.len())
        || net.iter().any(|p| p[2] <= Q::zero())
        || gcd(f, &derivative(f))?.len() != 1
        || parameter_order::closed_roots(f, root)? != 1
    {
        return Err(refused(
            "Invalid selected algebraic point or original source cell",
        ));
    }
    for coordinate in point {
        for p in coordinate {
            coordinates::bounded(p)?;
        }
        if coordinates::zero_at(&coordinate[1], f, root)? {
            return Err(refused("Selected algebraic point denominator vanishes"));
        }
    }
    Ok(())
}
/// `x(t) * den - num * w(t) = 0` along one axis of the source cell, with the
/// point coordinate `num / den` in the joint primitive variable.
fn implicit_equation(net: &[[Q; 3]], axis: usize, coordinate: &[Poly; 2]) -> Vec<Poly> {
    let x = power(net, axis);
    let w = power(net, 2);
    let mut equation = Vec::new();
    for i in 0..x.len().max(w.len()) {
        let xp = x.get(i).cloned().unwrap_or_else(Q::zero);
        let wp = w.get(i).cloned().unwrap_or_else(Q::zero);
        equation.push(sub(
            &coordinate[1].iter().map(|q| q * &xp).collect::<Poly>(),
            &coordinate[0].iter().map(|q| q * &wp).collect::<Poly>(),
        ));
    }
    while equation.last().is_some_and(Vec::is_empty) {
        equation.pop();
    }
    equation
}
/// alpha = lo + (hi - lo) * s, where s is the first coordinate of the joint
/// system.
fn source_parameter(s: &Poly, root: &[Q; 2]) -> Result<Poly> {
    let source_primitive = affine(s, &Q::zero(), &Q::one())?;
    let mut alpha = source_primitive
        .iter()
        .map(|q| q * (&root[1] - &root[0]))
        .collect::<Poly>();
    if alpha.is_empty() {
        alpha.push(root[0].clone());
    } else {
        alpha[0] += &root[0];
    }
    Ok(trim(alpha))
}
/// The point and the source cell at one joint root, when the source cell
/// passes through the point there on both axes.
fn root_position(
    transformed: &[[Poly; 2]],
    net: &[[Q; 3]],
    (s, t): (&Poly, &Poly),
    primitive: &Poly,
    interval: &[Q; 2],
) -> Result<Option<[[Poly; 2]; 2]>> {
    let compose = |p: &Poly, map: &Poly| coordinates::compose(p, map, primitive);
    let w = power(net, 2);
    let mut position = std::array::from_fn::<_, 2, _>(|_| [Vec::new(), Vec::new()]);
    let mut matches = true;
    for axis in 0..2 {
        let n = compose(&transformed[axis][0], s)?;
        let den = compose(&transformed[axis][1], s)?;
        let right = compose(&power(net, axis), t)?;
        let weight = compose(&w, t)?;
        if coordinates::zero_at(&den, primitive, interval)?
            || coordinates::zero_at(&weight, primitive, interval)?
        {
            return Err(refused("Point-preimage coordinate denominator vanishes"));
        }
        let difference = coordinates::modulo(
            &sub(&product(&n, &weight), &product(&right, &den)),
            primitive,
        )?;
        if !coordinates::zero_at(&difference, primitive, interval)? {
            matches = false;
        }
        position[axis] = [n, den];
    }
    Ok(matches.then_some(position))
}
/// Every result keeps a joint primitive rather than assuming the new parameter
/// belongs to the input point's number field. Completeness is relative to the
/// actual closed source cell [0,1], and the one selected input primitive root.
pub(in crate::profile::profile_certificate) fn pullback(
    f: &Poly,
    root: &[Q; 2],
    point: &[[Poly; 2]; 2],
    net: &[[Q; 3]],
    width: &Q,
) -> Result<Vec<Preimage>> {
    verify_input(f, root, point, net, width)?;
    let transformed = point
        .iter()
        .map(|p| {
            Ok([
                affine(&p[0], &root[0], &root[1])?,
                affine(&p[1], &root[0], &root[1])?,
            ])
        })
        .collect::<Result<Vec<_>>>()?;
    let primitive = if root[0] == root[1] {
        vec![Q::zero(), Q::one()]
    } else {
        affine(f, &root[0], &root[1])?
    };
    let d = tangent_numerators(net);
    let axis = (0..2)
        .find(|&axis| d[axis].iter().any(|v| !v.is_zero()))
        .ok_or_else(|| refused("Constant source cell has no finite point-preimage census"))?;
    let equation = implicit_equation(net, axis, &transformed[axis]);
    let (shear, factors) =
        algebraic_events::produce_equations(&[vec![primitive], equation], false, width)?;
    let mut out = Vec::new();
    for factor in factors {
        let Some(t) = factor.map else { continue };
        let s = coordinates::first_parameter(&t, shear);
        let alpha = source_parameter(&s, root)?;
        for r in factor.roots.into_iter().filter(|r| r.kind == "event") {
            let position =
                root_position(&transformed, net, (&s, &t), &factor.polynomial, &r.interval)?;
            if let Some(position) = position {
                out.push(Preimage {
                    primitive: factor.polynomial.clone(),
                    interval: r.interval,
                    source_primitive: alpha.clone(),
                    local_parameter: t.clone(),
                    position,
                });
            }
        }
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn q(n: i64, d: i64) -> Q {
        Q::new(n.into(), d.into())
    }
    #[test]
    fn irrational_point_pullback_owns_joint_sixth_root_extension() {
        let f = vec![-q(1, 2), Q::zero(), Q::one()];
        let root = [q(7, 10), q(8, 10)];
        let point = [
            [vec![Q::zero(), Q::one()], vec![Q::one()]],
            [Vec::new(), vec![Q::one()]],
        ];
        let net = vec![
            [Q::zero(), Q::zero(), Q::one()],
            [Q::zero(), Q::zero(), Q::one()],
            [Q::zero(), Q::zero(), Q::one()],
            [Q::one(), Q::zero(), Q::one()],
        ];
        let r = pullback(&f, &root, &point, &net, &q(1, 1024)).unwrap();
        assert_eq!(r.len(), 1);
        let p = &r[0];
        let t = coordinates::compose(
            &vec![Q::zero(), Q::zero(), Q::zero(), Q::one()],
            &p.local_parameter,
            &p.primitive,
        )
        .unwrap();
        assert!(
            coordinates::zero_at(&sub(&t, &p.source_primitive), &p.primitive, &p.interval).unwrap()
        );
        assert!(p.primitive.len() >= 7);
        let mut wrong = point.clone();
        wrong[1][0] = vec![Q::one()];
        assert!(
            pullback(&f, &root, &wrong, &net, &q(1, 1024))
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn rational_point_has_complete_multiple_original_traversals() {
        let f = vec![Q::zero(), Q::one()];
        let root = [Q::zero(), Q::zero()];
        let point = [
            [vec![q(1, 2)], vec![Q::one()]],
            [Vec::new(), vec![Q::one()]],
        ];
        let net = vec![
            [Q::zero(), Q::zero(), Q::one()],
            [q(2, 1), Q::zero(), Q::one()],
            [Q::zero(), Q::zero(), Q::one()],
        ];
        assert_eq!(
            pullback(&f, &root, &point, &net, &q(1, 1024))
                .unwrap()
                .len(),
            2
        );
        let mut wrong = point.clone();
        wrong[0][1] = Vec::new();
        assert!(pullback(&f, &root, &wrong, &net, &q(1, 1024)).is_err());
        assert!(pullback(&f, &[q(-1, 1), q(1, 1)], &point, &net, &q(0, 1)).is_err());
    }
}
