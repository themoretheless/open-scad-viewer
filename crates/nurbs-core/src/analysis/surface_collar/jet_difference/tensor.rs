//! Outward Bernstein tensor differentiation, restriction and enclosure.
use super::*;

pub(super) fn choose(n: usize, k: usize) -> f64 {
    match (n, k) {
        (2, 1) => 2.,
        _ => 1.,
    }
}

pub(super) fn difference(p: &Poly, axis: usize, w: &mut Work) -> Result<Poly> {
    let degree = if axis == 0 {
        p.len() - 1
    } else {
        p[0].len() - 1
    };
    if degree == 0 {
        return Ok(vec![vec![I::point(0.)]]);
    }
    let (nu, nv) = if axis == 0 {
        (p.len() - 1, p[0].len())
    } else {
        (p.len(), p[0].len() - 1)
    };
    let mut out = vec![vec![I::point(0.); nv]; nu];
    for i in 0..nu {
        for j in 0..nv {
            w.charge(2)?;
            out[i][j] = (if axis == 0 { p[i + 1][j] } else { p[i][j + 1] })
                .sub(p[i][j])?
                .mul(I::point(degree as f64))?;
        }
    }
    Ok(out)
}

pub(super) fn split(p: &[I], t: I, w: &mut Work) -> Result<(Vec<I>, Vec<I>)> {
    let mut row = p.to_vec();
    let mut left = vec![row[0]];
    let mut right = vec![*row.last().unwrap()];
    w.charge(1)?;
    let complement = I::point(1.).sub(t)?;
    while row.len() > 1 {
        let mut next = Vec::with_capacity(row.len() - 1);
        for pair in row.windows(2) {
            w.charge(3)?;
            next.push(pair[0].mul(complement)?.add(pair[1].mul(t)?)?);
        }
        left.push(next[0]);
        right.push(*next.last().unwrap());
        row = next;
    }
    right.reverse();
    Ok((left, right))
}

pub(super) fn restrict(p: &[I], range: [f64; 2], w: &mut Work) -> Result<Vec<I>> {
    // Literal Bernstein endpoint identity, including differentiated polynomials.
    if range == [0., 0.] {
        return Ok(vec![p[0]]);
    }
    if range == [1., 1.] {
        return Ok(vec![*p.last().unwrap()]);
    }
    if range == [0., 1.] {
        return Ok(p.to_vec());
    }
    if range[0] == range[1] {
        let (left, _) = split(p, I::point(range[0]), w)?;
        return Ok(vec![*left.last().unwrap()]);
    }
    let mut out = if range[1] == 1. {
        p.to_vec()
    } else {
        split(p, I::point(range[1]), w)?.0
    };
    if range[0] > 0. {
        w.charge(1)?;
        out = split(&out, I::point(range[0]).div(I::point(range[1]))?, w)?.1;
    }
    Ok(out)
}

pub(super) fn enclose(p: &Poly, range: [[f64; 2]; 2], w: &mut Work) -> Result<I> {
    // Evaluate the fixed transverse endpoint BEFORE restricting the other
    // axis. Tensor-product evaluation commutes; unused columns need no work.
    if range[1] == [0., 0.] || range[1] == [1., 1.] {
        let column = if range[1] == [0., 0.] {
            0
        } else {
            p[0].len() - 1
        };
        let values = restrict(
            &p.iter().map(|row| row[column]).collect::<Vec<_>>(),
            range[0],
            w,
        )?;
        return I::new(
            values.iter().map(|x| x.lo).fold(f64::INFINITY, f64::min),
            values
                .iter()
                .map(|x| x.hi)
                .fold(f64::NEG_INFINITY, f64::max),
        );
    }
    let columns = (0..p[0].len())
        .map(|j| restrict(&p.iter().map(|r| r[j]).collect::<Vec<_>>(), range[0], w))
        .collect::<Result<Vec<_>>>()?;
    let mut lower = f64::INFINITY;
    let mut upper = f64::NEG_INFINITY;
    for i in 0..columns[0].len() {
        let row = columns.iter().map(|c| c[i]).collect::<Vec<_>>();
        for x in restrict(&row, range[1], w)? {
            lower = lower.min(x.lo);
            upper = upper.max(x.hi);
        }
    }
    I::new(lower, upper)
}
