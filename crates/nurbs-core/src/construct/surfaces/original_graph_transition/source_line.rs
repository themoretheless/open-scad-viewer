//! Exact natural source-chart lines of the qualified original graph support.
use super::*;
pub struct SourceLine {
    source: Surface,
    side: usize,
    uv: [[Q; 2]; 2],
    world: Vec<[Q; 3]>,
}
impl SourceLine {
    pub fn source(&self) -> &Surface {
        &self.source
    }
    pub fn side(&self) -> usize {
        self.side
    }
    pub fn uv_points(&self) -> &[[Q; 2]; 2] {
        &self.uv
    }
    pub fn points(&self) -> &[[Q; 3]] {
        &self.world
    }
}
impl Patch {
    pub fn source_line(&self, side: usize, uv: [[Q; 2]; 2], b: &mut Budget) -> Result<SourceLine> {
        check(side < 2, "Choose an original graph source")?;
        let s = &self.sources[side];
        let n = s.degree_u;
        for p in &uv {
            for q in p {
                b.keep(q.clone())?;
            }
        }
        check(
            uv[0] != uv[1] && uv.iter().all(|p| p[1] >= Q::zero() && p[1] <= Q::one()),
            "Choose a nondegenerate source-chart line",
        )?;
        let low = b.authored(s.knots_u[n])?;
        let high = b.authored(s.knots_u[s.control_points.len()])?;
        let span = b.keep(&high - &low)?;
        check(
            uv.iter().all(|p| p[0] >= low && p[0] <= high),
            "Source line is outside original U chart",
        )?;
        let mut q = s
            .control_points
            .iter()
            .map(|r| b.authored(r[0][2]))
            .collect::<Result<Vec<_>>>()?;
        let mut h = s
            .control_points
            .iter()
            .map(|r| {
                let a = b.authored(r[0][2])?;
                let z = b.authored(r[1][2])?;
                b.keep(a - z)
            })
            .collect::<Result<Vec<_>>>()?;
        let world = if uv[0][0] == uv[1][0] {
            let t = b.keep((&uv[0][0] - &low) / &span)?;
            let q = split(&q, &t, b)?.0.last().unwrap().clone();
            let h = split(&h, &t, b)?.0.last().unwrap().clone();
            uv.iter()
                .map(|p| {
                    Ok([
                        p[0].clone(),
                        if side == 0 { -&p[1] } else { p[1].clone() },
                        b.keep(&q - &h * &p[1])?,
                    ])
                })
                .collect::<Result<Vec<_>>>()?
        } else {
            let forward = uv[0][0] < uv[1][0];
            let endpoints = if forward {
                [&uv[0], &uv[1]]
            } else {
                [&uv[1], &uv[0]]
            };
            let lo = b.keep((&endpoints[0][0] - &low) / &span)?;
            let hi = b.keep((&endpoints[1][0] - &low) / &span)?;
            q = restrict(&q, &lo, &hi, b)?;
            h = restrict(&h, &lo, &hi, b)?;
            if !forward {
                q.reverse();
                h.reverse();
            }
            let v = [uv[0][1].clone(), uv[1][1].clone()];
            let hw = product(&h, &v, b)?;
            let q = elevate(&q, n + 1, b)?;
            let x = elevate(&[uv[0][0].clone(), uv[1][0].clone()], n + 1, b)?;
            let y = elevate(&v, n + 1, b)?;
            (0..=n + 1)
                .map(|i| {
                    Ok([
                        x[i].clone(),
                        if side == 0 { -&y[i] } else { y[i].clone() },
                        b.keep(&q[i] - &hw[i])?,
                    ])
                })
                .collect::<Result<Vec<_>>>()?
        };
        Ok(SourceLine {
            source: s.clone(),
            side,
            uv,
            world,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_graph_lines_match_independent_geometry_in_both_directions() {
        let sources: [Surface; 2] = std::array::from_fn(|side| Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: [0., 0., 1.]
                .into_iter()
                .enumerate()
                .map(|(i, z)| {
                    vec![
                        vec![i as f64 / 2., 0., z],
                        vec![i as f64 / 2., if side == 0 { -1. } else { 1. }, z - 0.25],
                    ]
                })
                .collect(),
            weights: vec![vec![2.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        });
        let w = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![0.25, 0., 0.]; 2],
            weights: vec![1.; 2],
            periodic: false,
        };
        let mut b = Budget::new(100_000).unwrap();
        let p = build([&sources[0], &sources[1]], &w, &mut b).unwrap();
        for side in 0..2 {
            for uv in [
                [[ratio(1, 8), ratio(1, 4)], [ratio(7, 8), ratio(1, 2)]],
                [[ratio(7, 8), ratio(1, 2)], [ratio(1, 8), ratio(1, 4)]],
                [[ratio(1, 4), ratio(1, 1)], [ratio(1, 4), ratio(0, 1)]],
            ] {
                let line = p.source_line(side, uv.clone(), &mut b).unwrap();
                let t = ratio(1, 3);
                let point = (0..3)
                    .map(|k| {
                        split(
                            &line.points().iter().map(|p| p[k].clone()).collect::<Vec<_>>(),
                            &t,
                            &mut b,
                        )
                        .unwrap()
                        .0
                        .last()
                        .unwrap()
                        .clone()
                    })
                    .collect::<Vec<_>>();
                let x = &uv[0][0] + (&uv[1][0] - &uv[0][0]) * &t;
                let v = &uv[0][1] + (&uv[1][1] - &uv[0][1]) * t;
                assert_eq!(
                    point,
                    vec![
                        x.clone(),
                        if side == 0 { -&v } else { v.clone() },
                        &x * &x - ratio(1, 4) * v
                    ]
                );
                assert_eq!(line.source(), &sources[side]);
                assert_eq!(line.uv_points(), &uv);
            }
        }
        assert!(
            p.source_line(
                0,
                [[ratio(0, 1), ratio(0, 1)], [ratio(1, 1), ratio(2, 1)]],
                &mut b
            )
            .is_err()
        );
        assert!(
            p.source_line(
                0,
                [[ratio(0, 1), ratio(0, 1)], [ratio(1, 1), ratio(1, 1)]],
                &mut Budget::new(0).unwrap()
            )
            .is_err()
        );
    }
}
