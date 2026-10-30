//! Exact dyadic/rational projected hull tests. No rounded projected curve exists.
use super::*;
use num_rational::BigRational as Q;
use num_traits::Zero;
type H = [Q; 4];
#[derive(Clone)]
struct Piece {
    h: Vec<H>,
}
struct Projection {
    values: Vec<Vec<Q>>,
    bounds: Vec<(Q, Q)>,
}
fn q(x: f64) -> Result<Q> {
    Q::from_float(x).ok_or_else(|| refuse("Nonfinite original projection input"))
}
fn control(c: &[Vec<f64>], w: &[f64]) -> Result<Piece> {
    let mut h = Vec::new();
    for (p, &w) in c.iter().zip(w) {
        let w = q(w)?;
        let p = [q(p[0])?, q(p[1])?, q(p[2])?];
        h.push([&p[0] * &w, &p[1] * &w, &p[2] * &w, w]);
    }
    Ok(Piece { h })
}
fn split(p: &Piece) -> Result<[Piece; 2]> {
    let mut h = p.h.clone();
    let n = h.len();
    let mut a = vec![h[0].clone()];
    let mut b = vec![h[n - 1].clone()];
    for count in (1..n).rev() {
        for i in 0..count {
            for k in 0..4 {
                h[i][k] = (&h[i][k] + &h[i + 1][k]) / Q::from_integer(2.into());
                require(
                    h[i][k].numer().bits() <= 4096 && h[i][k].denom().bits() <= 4096,
                    "Projected boundary exact bit work exhausted",
                )?;
            }
        }
        a.push(h[0].clone());
        b.push(h[count - 1].clone());
    }
    b.reverse();
    Ok([Piece { h: a }, Piece { h: b }])
}
fn same(a: &H, b: &H) -> bool {
    (0..3).all(|k| &a[k] * &b[3] == &b[k] * &a[3])
}
fn projection(
    p: &Piece,
    normals: &[[i8; 3]],
    work: &mut usize,
    max: usize,
) -> Result<Option<Projection>> {
    let mut values = Vec::new();
    let mut bounds = Vec::new();
    for normal in normals {
        let mut row = Vec::new();
        for h in &p.h {
            if *work == max {
                return Ok(None);
            }
            *work += 1;
            let mut value = Q::zero();
            for k in 0..3 {
                value += &h[k] * Q::from_integer(normal[k].into());
            }
            row.push(value / &h[3]);
        }
        let lo = row.iter().min().unwrap().clone();
        let hi = row.iter().max().unwrap().clone();
        values.push(row);
        bounds.push((lo, hi));
    }
    Ok(Some(Projection { values, bounds }))
}
fn monotone(p: &Projection) -> bool {
    p.values.iter().any(|row| {
        row.first() != row.last()
            && (row.windows(2).all(|w| w[0] <= w[1]) || row.windows(2).all(|w| w[0] >= w[1]))
    })
}
fn adjacent(a: &Projection, b: &Projection) -> bool {
    a.values.iter().zip(&b.values).any(|(a, b)| {
        let joint = a.last().unwrap();
        if joint != &b[0] {
            return false;
        }
        let left = a.iter().all(|x| x <= joint) && a.iter().any(|x| x < joint);
        let right = b.iter().all(|x| x >= joint) && b.iter().any(|x| x > joint);
        let reverse_left = a.iter().all(|x| x >= joint) && a.iter().any(|x| x > joint);
        let reverse_right = b.iter().all(|x| x <= joint) && b.iter().any(|x| x < joint);
        (left && right) || (reverse_left && reverse_right)
    })
}
pub(super) fn inspect(s: &Surface, direction: [i8; 3], max: usize) -> Result<(bool, usize, usize)> {
    let p = s.degree_u;
    let qv = s.degree_v;
    let n = s.control_points.len();
    let spans = (p..n)
        .filter(|&i| s.knots_u[i] < s.knots_u[i + 1])
        .collect::<Vec<_>>();
    let mut pieces = Vec::new();
    for &span in &spans {
        let c = (span - p..=span)
            .map(|i| s.control_points[i][0].clone())
            .collect::<Vec<_>>();
        let w = (span - p..=span)
            .map(|i| s.weights[i][0])
            .collect::<Vec<_>>();
        pieces.push(control(&c, &w)?);
    }
    for &span in spans.iter().rev() {
        let c = (span - p..=span)
            .rev()
            .map(|i| s.control_points[i][qv].clone())
            .collect::<Vec<_>>();
        let w = (span - p..=span)
            .rev()
            .map(|i| s.weights[i][qv])
            .collect::<Vec<_>>();
        pieces.push(control(&c, &w)?);
    }
    for i in 0..pieces.len() {
        require(
            same(
                pieces[i].h.last().unwrap(),
                &pieces[(i + 1) % pieces.len()].h[0],
            ),
            "Projected original boundary lost an exact join",
        )?;
    }
    let mut normals = Vec::new();
    for a in -4_i8..=4 {
        for b in -4_i8..=4 {
            if a == 0 && b == 0 {
                continue;
            }
            normals.push([a * direction[0], b * direction[1], -(a + b) * direction[2]]);
        }
    }
    let mut work = 0;
    let mut pending = 1;
    for depth in 0..=3 {
        if pieces.len() > 256 {
            return Ok((false, work, pending));
        }
        let mut cached = Vec::new();
        for piece in &pieces {
            let Some(p) = projection(piece, &normals, &mut work, max)? else {
                return Ok((false, work, pending));
            };
            cached.push(p);
        }
        pending = cached.iter().filter(|p| !monotone(p)).count();
        for a in 0..pieces.len() {
            for b in a + 1..pieces.len() {
                if work == max {
                    return Ok((false, work, pending.max(1)));
                }
                work += 1;
                let ok = if b == a + 1 {
                    adjacent(&cached[a], &cached[b])
                } else if a == 0 && b == pieces.len() - 1 {
                    adjacent(&cached[b], &cached[a])
                } else {
                    cached[a]
                        .bounds
                        .iter()
                        .zip(&cached[b].bounds)
                        .any(|((al, ah), (bl, bh))| ah < bl || bh < al)
                };
                if !ok {
                    pending += 1;
                }
            }
        }
        if pending == 0 {
            return Ok((true, work, 0));
        }
        if depth == 3 {
            return Ok((false, work, pending));
        }
        let mut children = Vec::new();
        for piece in pieces {
            children.extend(split(&piece)?);
        }
        pieces = children;
    }
    Ok((false, work, pending))
}
