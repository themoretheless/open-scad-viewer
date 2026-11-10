//! Homogeneous blowdown of a complete collapsed tensor side. The quotient
//! triangle retains the singular vertex with a proved uniform limit.
use super::*;
fn choose(n: usize, k: usize) -> Q {
    (0..k).fold(Q::one(), |q, i| {
        q * integer((n - i) as i64) / integer((i + 1) as i64)
    })
}
pub(super) fn chart(h: &sb::Net, work: &mut Work) -> Result<Option<Net>> {
    let (n, m) = (h.len() - 1, h[0].len() - 1);
    if n == 0 || m == 0 || n + m > 8 {
        return Ok(None);
    }
    if h.iter().flatten().any(|p| p[3] <= Q::zero()) {
        return Ok(None);
    }
    let pole = sb::point(&h[0][0]);
    if h.iter().any(|r| sb::point(&r[0]) != pole) {
        return Ok(None);
    }
    // u=y/(x+y), v=x+y, multiply all homogeneous coordinates by (x+y)^n.
    // Tensor Bernstein basis becomes positive triangular Bernstein coefficients.
    let d = n + m;
    let mut out = (0..=d)
        .map(|i| vec![std::array::from_fn(|_| Q::zero()); d + 1 - i])
        .collect::<Net>();
    for i in 0..=n {
        for j in 0..=m {
            for k in 0..=j {
                work.charge(1)?;
                let (x, y) = (n - i + k, i + j - k);
                let factor = choose(n, i) * choose(m, j) * choose(j, k)
                    / (choose(d, x + y) * choose(x + y, x));
                for c in 0..4 {
                    out[x][y][c] += &h[i][j][c] * &factor;
                }
            }
        }
    }
    transport::check(&out)?;
    Ok(Some(out))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zero_homogeneous_edge_controls_do_not_create_infinite_points() {
        let h = vec![
            [integer(0), integer(0), integer(0), integer(1)],
            std::array::from_fn(|_| Q::zero()),
            [integer(1), integer(0), integer(0), integer(1)],
        ];
        let c = [h.clone(), h.clone()];
        assert_eq!(
            curve_points(&c, &[integer(0), integer(0), integer(1)]).len(),
            4
        );
        // A regular rational segment with stationary endpoint clocks may refuse
        // the strict monotonicity certificate, but must never divide by zero.
        let _ = segment_seam(&h, &h, &mut Work::default()).unwrap();
    }
    #[test]
    fn rational_cone_has_one_uniform_apex_and_exact_edges() {
        // Rational nonplanar base E(u)=(u,u²,1)/(1+u²), S=v E(u).
        let base = vec![
            [integer(0), integer(0), integer(1), integer(1)],
            [
                Q::new(1.into(), 2.into()),
                integer(0),
                integer(1),
                integer(1),
            ],
            [integer(1), integer(1), integer(1), integer(2)],
        ];
        let h = base
            .iter()
            .map(|p| {
                vec![
                    [integer(0), integer(0), integer(0), p[3].clone()],
                    p.clone(),
                ]
            })
            .collect::<sb::Net>();
        let a = chart(&h, &mut Work::default()).unwrap().unwrap();
        assert!(a[0][0].iter().all(Q::is_zero));
        assert_eq!(
            bernstein::corners(&a)[0],
            [integer(0), integer(0), integer(0)]
        );
        for e in bernstein::edges(&a) {
            assert!(e.iter().all(|p| p[3] > Q::zero()));
        }
        // Independent exact evaluation of tensor source and quotient chart.
        for (u, v) in [
            (Q::new(1.into(), 3.into()), Q::new(1.into(), 2.into())),
            (Q::new(2.into(), 5.into()), Q::new(3.into(), 4.into())),
        ] {
            let x = &v * (Q::one() - &u);
            let y = &v * &u;
            let z = Q::one() - &v;
            let mut value: [Q; 4] = std::array::from_fn(|_| Q::zero());
            let d = a.len() - 1;
            for (i, r) in a.iter().enumerate() {
                for (j, p) in r.iter().enumerate() {
                    let basis = choose(d, i + j)
                        * choose(i + j, i)
                        * x.pow(i as i32)
                        * y.pow(j as i32)
                        * z.pow((d - i - j) as i32);
                    for k in 0..4 {
                        value[k] += &p[k] * &basis;
                    }
                }
            }
            let mut expected: [Q; 4] = std::array::from_fn(|_| Q::zero());
            for (i, p) in base.iter().enumerate() {
                let b = choose(2, i) * u.pow(i as i32) * (Q::one() - &u).pow((2 - i) as i32);
                for k in 0..4 {
                    expected[k] += &p[k] * &b;
                }
            }
            let source: P = std::array::from_fn(|k| &v * &expected[k] / &expected[3]);
            assert_eq!(sb::point(&value), source);
        }
        let mut bad = a.clone();
        bad[1][1][0] += Q::one();
        assert!(transport::check(&bad).is_err());
        let mut bad = a;
        bad[0][0][0] = Q::one();
        assert!(transport::check(&bad).is_err());
    }
}
