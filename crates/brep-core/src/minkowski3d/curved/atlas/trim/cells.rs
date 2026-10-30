//! Exact convex clipping of affine UV triangles against authored knot cells.
//! Curved UV maps crossing cell boundaries require a separate branch proof.
use super::*;
mod curved;
fn clip(poly: Vec<P>, axis: usize, limit: &Q, lower: bool, work: &mut Work) -> Result<Vec<P>> {
    let mut out = Vec::new();
    if poly.is_empty() {
        return Ok(out);
    }
    let inside = |p: &P| {
        if lower {
            p[axis] >= *limit
        } else {
            p[axis] <= *limit
        }
    };
    for (a, b) in poly
        .iter()
        .zip(poly.iter().cycle().skip(1))
        .take(poly.len())
    {
        work.charge(1)?;
        let (ia, ib) = (inside(a), inside(b));
        if ia {
            out.push(a.clone());
        }
        if ia != ib {
            let t = (limit - &a[axis]) / (&b[axis] - &a[axis]);
            let mut p = std::array::from_fn(|k| &a[k] + &t * (&b[k] - &a[k]));
            p[axis] = limit.clone();
            out.push(p);
        }
    }
    out.dedup();
    if out.len() > 1 && out.first() == out.last() {
        out.pop();
    }
    Ok(out)
}
fn intervals(knots: &[f64], degree: usize, count: usize) -> Vec<[Q; 2]> {
    (degree..count)
        .filter(|i| knots[*i] < knots[*i + 1])
        .map(|i| {
            [
                Q::from_float(knots[i]).unwrap(),
                Q::from_float(knots[i + 1]).unwrap(),
            ]
        })
        .collect()
}
fn compose_cell(
    s: &crate::Surface,
    uv: &Net,
    u: &[Q; 2],
    v: &[Q; 2],
    work: &mut Work,
) -> Result<Net> {
    let mut uv = uv.clone();
    for p in uv.iter_mut().flatten() {
        for (axis, range) in [u, v].into_iter().enumerate() {
            p[axis] = (&p[axis] - &range[0] * &p[3]) / (&range[1] - &range[0]);
            if p[axis] < Q::zero() || p[axis] > p[3] {
                return Err(fail("trim leaves selected knot cell"));
            }
        }
    }
    bernstein::compose(&sb::cell(s, u.clone(), v.clone())?, &uv, work)
}
pub(super) fn charts(s: &crate::Surface, uv: Net, work: &mut Work) -> Result<Vec<Net>> {
    s.validate()?;
    let us = intervals(&s.knots_u, s.degree_u, s.control_points.len());
    let vs = intervals(&s.knots_v, s.degree_v, s.control_points[0].len());
    if us.len().saturating_mul(vs.len()) > 128 {
        return Err(fail("trim knot cell budget exceeded"));
    }
    // Preserve the old parameterization whenever the complete control hull
    // lies in one cell, even if this is a nonlinear rational disk.
    for u in &us {
        for v in &vs {
            work.charge(uv.iter().map(Vec::len).sum())?;
            if uv.iter().flatten().all(|p| {
                [u, v]
                    .iter()
                    .enumerate()
                    .all(|(k, r)| p[k] >= r[0].clone() * &p[3] && p[k] <= r[1].clone() * &p[3])
            }) {
                return Ok(vec![compose_cell(s, &uv, u, v, work)?]);
            }
        }
    }
    if uv.len() != 2 {
        let mut trial = Work(work.0);
        match curved::charts(s, &uv, &us, &vs, &mut trial) {
            Ok(charts) => {
                *work = trial;
                return Ok(charts);
            }
            Err(original) => {
                if let Some(linear) = conform::planar_chart(&uv, work)? {
                    return charts(s, linear, work);
                }
                return Err(original);
            }
        }
    }
    let triangle = bernstein::corners(&uv).to_vec();
    let domain = [
        us.first().unwrap()[0].clone(),
        us.last().unwrap()[1].clone(),
        vs.first().unwrap()[0].clone(),
        vs.last().unwrap()[1].clone(),
    ];
    if triangle
        .iter()
        .any(|p| p[0] < domain[0] || p[0] > domain[1] || p[1] < domain[2] || p[1] > domain[3])
    {
        return Err(fail("trim triangle leaves the authored surface domain"));
    }
    let side =
        |a: &P, b: &P, c: &P| (&b[0] - &a[0]) * (&c[1] - &a[1]) - (&b[1] - &a[1]) * (&c[0] - &a[0]);
    let area = side(&triangle[0], &triangle[1], &triangle[2]);
    if area <= Q::zero() {
        return Err(fail("trim triangle has false orientation"));
    }
    let mut covered = Q::zero();
    let mut charts = Vec::new();
    for u in &us {
        for v in &vs {
            let mut poly = triangle.clone();
            for (axis, range) in [u, v].iter().enumerate() {
                poly = clip(poly, axis, &range[0], true, work)?;
                poly = clip(poly, axis, &range[1], false, work)?;
            }
            for i in 1..poly.len().saturating_sub(1) {
                let points = [poly[0].clone(), poly[i].clone(), poly[i + 1].clone()];
                let a = side(&points[0], &points[1], &points[2]);
                if a.is_zero() {
                    continue;
                }
                if a < Q::zero() {
                    return Err(fail("clipped trim triangle is reversed"));
                }
                covered += a;
                let h = |p: &P| [p[0].clone(), p[1].clone(), Q::zero(), Q::one()];
                let net = vec![vec![h(&points[0]), h(&points[2])], vec![h(&points[1])]];
                charts.push(compose_cell(s, &net, u, v, work)?);
                if charts.len() > 512 {
                    return Err(fail("clipped trim triangle budget exceeded"));
                }
            }
        }
    }
    // Cell interiors are disjoint. Each clipped polygon lies inside the source
    // triangle. Exact oriented area equality verifies complete coverage.
    if covered != area {
        return Err(fail("knot cell partition has incomplete UV coverage"));
    }
    Ok(charts)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn split_surface() -> crate::Surface {
        let mut s = crate::cuboid([0.; 3], [1.; 3]).unwrap().faces[0]
            .surface
            .clone();
        let mid = s.control_points[0]
            .iter()
            .zip(&s.control_points[1])
            .map(|(a, b)| a.iter().zip(b).map(|(a, b)| (a + b) / 2.).collect())
            .collect();
        s.control_points.insert(1, mid);
        let mid = s.weights[0]
            .iter()
            .zip(&s.weights[1])
            .map(|(a, b)| (a + b) / 2.)
            .collect();
        s.weights.insert(1, mid);
        s.knots_u = vec![0., 0., 0.5, 1., 1.];
        s
    }
    #[test]
    fn nonlinear_irrational_knot_pullback_uses_a_proved_image_clock() {
        let s = split_surface();
        // x=(u+u²)/2, y=v(1+u/2). The u=.5 source knot has
        // irrational preimage (sqrt(5)-1)/2, yet the complete image
        // is exactly the corner triangle (proved before replacement).
        let uv = (0..=2)
            .map(|i| {
                (0..=2 - i)
                    .map(|j| {
                        [
                            Q::new(i.into(), 4.into())
                                + if i == 2 {
                                    Q::new(1.into(), 2.into())
                                } else {
                                    Q::zero()
                                },
                            Q::new(j.into(), 2.into())
                                + if i == 1 && j == 1 {
                                    Q::new(1.into(), 4.into())
                                } else {
                                    Q::zero()
                                },
                            Q::zero(),
                            Q::one(),
                        ]
                    })
                    .collect()
            })
            .collect::<Net>();
        let uv = disk_uv(bernstein::edges(&uv).to_vec(), &mut Work::default()).unwrap();
        let result = charts(&s, uv, &mut Work::default()).unwrap();
        assert_eq!(result.len(), 3);
        let point = sb::at(&s, &[Q::new(1.into(), 2.into()), Q::zero()]).unwrap();
        assert!(result
            .iter()
            .flat_map(bernstein::corners)
            .any(|p| p == point));
    }
    #[test]
    fn curved_boundary_crosses_a_knot_without_becoming_a_chord() {
        let s = split_surface();
        let mut uv = (0..=2)
            .map(|i| {
                (0..=2 - i)
                    .map(|j| {
                        [
                            Q::new(i.into(), 2.into()),
                            Q::new(j.into(), 2.into()),
                            Q::zero(),
                            Q::one(),
                        ]
                    })
                    .collect()
            })
            .collect::<Net>();
        uv[1][0][1] += Q::new(1.into(), 8.into());
        let uv = disk_uv(bernstein::edges(&uv).to_vec(), &mut Work::default()).unwrap();
        let result = charts(&s, uv.clone(), &mut Work::default()).unwrap();
        assert_eq!(result.len(), 3);
        assert!(result.iter().any(|h| h.len() > 3));
        // The exact point at the middle of the curved bottom edge is retained.
        let expected = sb::at(
            &s,
            &[Q::new(1.into(), 2.into()), Q::new(1.into(), 16.into())],
        )
        .unwrap();
        assert!(result
            .iter()
            .flat_map(bernstein::corners)
            .any(|p| p == expected));
        // Make x nonlinear too: a general algebraic pullback still refuses.
        let mut bad = uv;
        bad[1][0][0] += Q::new(1.into(), 32.into());
        assert!(charts(&s, bad, &mut Work::default()).is_err());
    }
    #[test]
    fn affine_triangle_crosses_internal_surface_knots_exactly() {
        let m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let mut s = m.faces[0].surface.clone();
        assert_eq!(s.degree_u, 1);
        let mid = s.control_points[0]
            .iter()
            .zip(&s.control_points[1])
            .map(|(a, b)| a.iter().zip(b).map(|(a, b)| (a + b) / 2.).collect())
            .collect();
        s.control_points.insert(1, mid);
        let mid = s.weights[0]
            .iter()
            .zip(&s.weights[1])
            .map(|(a, b)| (a + b) / 2.)
            .collect();
        s.weights.insert(1, mid);
        s.knots_u = vec![0., 0., 0.5, 1., 1.];
        let uv = vec![
            vec![
                [integer(0), integer(0), integer(0), integer(1)],
                [integer(0), integer(1), integer(0), integer(1)],
            ],
            vec![[integer(1), integer(0), integer(0), integer(1)]],
        ];
        let result = charts(&s, uv.clone(), &mut Work::default()).unwrap();
        assert_eq!(result.len(), 3);
        let mut bad = uv;
        bad[1][0][0] = integer(2);
        assert!(charts(&s, bad, &mut Work::default()).is_err());
    }
}
