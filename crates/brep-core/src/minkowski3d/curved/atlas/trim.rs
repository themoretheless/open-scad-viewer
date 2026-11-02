//! Embedded curvilinear triangular UV domains, without radial signed fans.
use super::*;
mod cells;
fn choose(n: usize, k: usize) -> Q {
    (0..k).fold(Q::one(), |x, i| {
        x * integer((n - i) as i64) / integer((i + 1) as i64)
    })
}
fn elevate(c: &[sb::H], d: usize) -> Vec<sb::H> {
    let n = c.len() - 1;
    (0..=d)
        .map(|i| {
            std::array::from_fn(|k| {
                (0..=n)
                    .filter(|j| *j <= i && i - j <= d - n)
                    .map(|j| &c[j][k] * choose(n, j) * choose(d - n, i - j) / choose(d, i))
                    .sum()
            })
        })
        .collect()
}
pub(super) fn chart(m: &Model, face: usize, work: &mut Work) -> Result<Net> {
    let f = &m.faces[face];
    let edges = &m.loops[f.outer].coedges;
    if !f.holes.is_empty() || edges.len() != 3 {
        return Err(fail("trimmed atlas needs one three-edge UV boundary"));
    }
    let mut curves = Vec::new();
    for e in edges {
        let c = &e.pcurve;
        let [a, b] = c.domain();
        let h = sb::curve(c, &Q::from_float(a).unwrap(), &Q::from_float(b).unwrap())?;
        curves.push(h);
    }
    disk(m, face, curves, work)
}
fn disk(m: &Model, face: usize, curves: Vec<Vec<sb::H>>, work: &mut Work) -> Result<Net> {
    let mut charts = disks(m, face, curves, work)?;
    if charts.len() != 1 {
        return Err(fail("trim chart needs a conforming multi-cell partition"));
    }
    Ok(charts.remove(0))
}
fn disks(m: &Model, face: usize, curves: Vec<Vec<sb::H>>, work: &mut Work) -> Result<Vec<Net>> {
    cells::charts(&m.faces[face].surface, disk_uv(curves, work)?, work)
}

pub(super) fn disk_uv(curves: Vec<Vec<sb::H>>, work: &mut Work) -> Result<Net> {
    disk_uv_parameter(curves, work, false)
}
pub(super) fn disk_uv_parameter(
    mut curves: Vec<Vec<sb::H>>,
    work: &mut Work,
    stationary: bool,
) -> Result<Net> {
    if curves.len() != 3 {
        return Err(fail("UV disk needs three edges"));
    }
    // Choose common corner gauges from the highest-degree boundaries first.
    // A positive linear homogeneous factor adjusts any remaining endpoints
    // without changing either the curve image or its parameter clock. This
    // keeps a rational quadratic arc quadratic when its straight neighbours
    // alone need degree elevation.
    let mut weights: [Option<Q>; 3] = std::array::from_fn(|_| None);
    let mut order = [0, 1, 2];
    order.sort_by_key(|k| std::cmp::Reverse(curves[*k].len()));
    for k in order {
        weights[k].get_or_insert_with(|| curves[k][0][3].clone());
        weights[(k + 1) % 3].get_or_insert_with(|| curves[k].last().unwrap()[3].clone());
    }
    let weights = weights.map(Option::unwrap);
    for (k, h) in curves.iter_mut().enumerate() {
        let (g0, g1) = (
            &weights[k] / &h[0][3],
            &weights[(k + 1) % 3] / &h.last().unwrap()[3],
        );
        if g0 == g1 {
            for p in h {
                for q in p {
                    *q *= &g0;
                }
            }
        } else {
            let n = h.len() - 1;
            *h = (0..=n + 1)
                .map(|i| {
                    std::array::from_fn(|k| {
                        let t = Q::new(i.into(), (n + 1).into());
                        (if i <= n {
                            (Q::one() - &t) * &h[i][k] * &g0
                        } else {
                            Q::zero()
                        }) + (if i > 0 {
                            &t * &h[i - 1][k] * &g1
                        } else {
                            Q::zero()
                        })
                    })
                })
                .collect();
        }
    }
    let d = curves.iter().map(|h| h.len() - 1).max().unwrap();
    if d == 0 || d > 8 {
        return Err(fail("trimmed atlas boundary degree budget exceeded"));
    }
    let curves = curves.iter().map(|c| elevate(c, d)).collect::<Vec<_>>();
    for k in 0..3 {
        if curves[k].last().unwrap() != &curves[(k + 1) % 3][0] {
            return Err(fail("trimmed atlas UV boundary is not exactly joined"));
        }
    }
    let c = curves.iter().map(|c| sb::point(&c[0])).collect::<Vec<_>>();
    let mut uv = (0..=d)
        .map(|i| {
            (0..=d - i)
                .map(|j| {
                    std::array::from_fn(|k| {
                        let counts = [d - i - j, i, j];
                        (0..3)
                            .map(|v| {
                                integer(counts[v] as i64)
                                    * &weights[v]
                                    * (if k == 3 { Q::one() } else { c[v][k].clone() })
                            })
                            .sum::<Q>()
                            / integer(d as i64)
                    })
                })
                .collect()
        })
        .collect::<Net>();
    for i in 0..=d {
        uv[i][0] = curves[0][i].clone();
        uv[d - i][i] = curves[1][i].clone();
        uv[0][d - i] = curves[2][i].clone();
    }
    let motion = bernstein::homotopy(&uv, work)?;
    let p = Patch {
        ids: [0, 1, 2],
        derivatives: bernstein::derivatives(&uv, work)?,
        h: uv.clone(),
        motion,
    };
    let n = [Q::zero(), Q::zero(), Q::one()];
    if !regular_parameter_disk(&p, &n, work, stationary)? {
        return Err(fail("trimmed atlas UV disk Jacobian is unproved"));
    }
    let edge0 = bernstein::edges(&p.motion[0]);
    let edge1 = bernstein::edges(&p.motion[1]);
    let curves = (0..3)
        .map(|i| [edge0[i].clone(), edge1[i].clone()])
        .collect::<Vec<_>>();
    for i in 0..3 {
        if !monotone_parameter(&curves[i], &n, work, stationary)? {
            return Err(fail("trimmed atlas UV boundary monotonicity is unproved"));
        }
        for j in i + 1..3 {
            let vertex = if j == i + 1 { j } else { 0 };
            if !curve_pair(&curves[i], &curves[j], Some(&c[vertex]), &n, work)? {
                return Err(fail("trimmed atlas UV Jordan boundary is unproved"));
            }
        }
    }
    Ok(uv)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn curved_uv_triangle_is_composed_exactly_and_folds_refuse() {
        let mut m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let outer = m.faces[0].outer;
        let mut coedges = m.loops[outer].coedges[..3].to_vec();
        coedges[0].pcurve = crate::line(vec![0., 0.], vec![1., 0.]);
        coedges[1].pcurve = crate::Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1., 0.], vec![0.625, 0.625], vec![0., 1.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        coedges[2].pcurve = crate::line(vec![0., 1.], vec![0., 0.]);
        m.loops[outer].coedges = coedges;
        let h = chart(&m, 0, &mut Work::default()).unwrap();
        // The lifted second edge is the original curve, not its chord.
        let edge = bernstein::edges(&h)[1].clone();
        let expected = vec![
            [Q::zero(), Q::one(), Q::zero(), Q::one()],
            [
                Q::new(5.into(), 8.into()),
                Q::new(5.into(), 8.into()),
                Q::zero(),
                Q::one(),
            ],
            [Q::one(), Q::zero(), Q::zero(), Q::one()],
        ];
        assert!(sb::same(&edge, &expected));
        m.loops[outer].coedges[1].pcurve.control_points[1] = vec![-1., -1.];
        assert!(chart(&m, 0, &mut Work::default()).is_err());
        m.faces[0].holes.push(outer);
        assert!(chart(&m, 0, &mut Work::default()).is_err());
    }
}

mod region;
pub(super) fn charts(m: &Model, face: usize, work: &mut Work) -> Result<Vec<Net>> {
    let f = &m.faces[face];
    if f.holes.is_empty() && m.loops[f.outer].coedges.len() == 3 {
        return Ok(vec![chart(m, face, work)?]);
    }
    region::charts(m, face, work)
}
