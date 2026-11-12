//! First nonzero ORIGINAL rational derivative germ at an algebraic parameter.
use super::*;
use event_coordinates as exact;
/// Original source cell, its algebraic parameter `parameter mod f` isolated in
/// `root`, and the side of the root the branch leaves towards.
#[derive(Clone, Copy)]
pub(in crate::profile::profile_certificate) struct BranchGerm<'a> {
    pub net: &'a [[Q; 3]],
    pub domain: &'a [Q; 2],
    pub parameter: &'a Poly,
    pub f: &'a Poly,
    pub root: &'a [Q; 2],
    pub direction: i8,
}
pub(in crate::profile::profile_certificate) fn leading(
    germ: BranchGerm<'_>,
    max_refinements: usize,
) -> Result<(usize, [Poly; 2], [i8; 2])> {
    let BranchGerm {
        net,
        domain,
        parameter,
        f,
        root,
        direction,
    } = germ;
    if domain[0] >= domain[1]
        || net.iter().any(|h| h[2] <= Q::zero())
        || !matches!(direction, -1 | 1)
    {
        return Err(refused(
            "Original branch requires a positive-weight nonempty source cell",
        ));
    }
    let local = sub(parameter, &[domain[0].clone()])
        .into_iter()
        .map(|v| v / (&domain[1] - &domain[0]))
        .collect::<Poly>();
    let mut d = [power(net, 0), power(net, 1), power(net, 2)];
    let initial = [
        exact::compose(&d[0], &local, f)?,
        exact::compose(&d[1], &local, f)?,
        exact::compose(&d[2], &local, f)?,
    ];
    if exact::zero_at(&initial[2], f, root)? {
        return Err(refused("Original rational germ denominator vanishes"));
    }
    for order in 1..net.len() {
        d = d.map(|p| derivative(&p));
        let w = exact::compose(&d[2], &local, f)?;
        let mut leading = [Vec::new(), Vec::new()];
        for axis in 0..2 {
            leading[axis] = exact::modulo(
                &sub(
                    &product(&exact::compose(&d[axis], &local, f)?, &initial[2]),
                    &product(&initial[axis], &w),
                ),
                f,
            )?;
            if direction < 0 && order % 2 == 1 {
                for v in &mut leading[axis] {
                    *v = -v.clone();
                }
            }
        }
        let signs = [
            exact::sign_at(&leading[0], f, root, max_refinements)?,
            exact::sign_at(&leading[1], f, root, max_refinements)?,
        ];
        if signs != [0, 0] {
            return Ok((order, leading, signs));
        }
    }
    Err(refused(
        "Original branch is constant or has no finite nonzero germ",
    ))
}

/// First nonzero coordinate displacement sign, including terms beyond the
/// leading vector. Zero means the actual coordinate is locally constant.
pub(in crate::profile::profile_certificate) fn component_sign(
    germ: BranchGerm<'_>,
    axis: usize,
    max_refinements: usize,
) -> Result<i8> {
    let BranchGerm {
        net,
        domain,
        parameter,
        f,
        root,
        direction,
    } = germ;
    if axis > 1 {
        return Err(refused("Invalid original germ component"));
    }
    let local = sub(parameter, &[domain[0].clone()])
        .iter()
        .map(|q| q / (&domain[1] - &domain[0]))
        .collect::<Poly>();
    let mut x = power(net, axis);
    let mut w = power(net, 2);
    let x0 = exact::compose(&x, &local, f)?;
    let w0 = exact::compose(&w, &local, f)?;
    for order in 1..net.len() {
        x = derivative(&x);
        w = derivative(&w);
        let mut n = exact::modulo(
            &sub(
                &product(&exact::compose(&x, &local, f)?, &w0),
                &product(&x0, &exact::compose(&w, &local, f)?),
            ),
            f,
        )?;
        if direction < 0 && order % 2 == 1 {
            for q in &mut n {
                *q = -q.clone();
            }
        }
        let s = exact::sign_at(&n, f, root, max_refinements)?;
        if s != 0 {
            return Ok(s);
        }
    }
    Ok(0)
}
