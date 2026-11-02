//! Original graph chart, exact rational clocks and single implicit branch proof.
use super::*;
use num_traits::Zero;
pub(super) struct Chart {
    pub axes: [usize; 2],
    pub parameterization: OriginalQuadricGraphParameterization,
    clocks: [super::clock::Clock; 2],
    origin: [Q; 2],
    u: [Q; 2],
    v: [Q; 2],
    det: Q,
}
impl Chart {
    pub(super) fn original(s: &Surface, matrix: &[[f64; 4]; 4]) -> Result<Self> {
        s.validate()?;
        let (p, r) = (s.degree_u, s.degree_v);
        check(
            !s.periodic_u
                && !s.periodic_v
                && (1..=8).contains(&p)
                && (1..=8).contains(&r)
                && s.control_points.len() == p + 1
                && s.control_points[0].len() == r + 1
                && s.control_points.iter().flatten().all(|x| x.len() == 3),
            "Quadric graph needs one nonperiodic 3D Bezier patch",
        )?;
        check(
            s.knots_u[..=p].iter().all(|x| *x == s.knots_u[p])
                && s.knots_u[p + 1..].iter().all(|x| *x == s.knots_u[p + 1])
                && s.knots_v[..=r].iter().all(|x| *x == s.knots_v[r])
                && s.knots_v[r + 1..].iter().all(|x| *x == s.knots_v[r + 1]),
            "Quadric graph requires original clamped knots",
        )?;
        let w0 = q(s.weights[0][0]);
        // Rank-one positive weights give independent weighted Bernstein
        // clocks. Their strict monotonicity and endpoint coverage are proved
        // by the variance identity in clock.rs; no geometric assumption is
        // needed for coverage. Geometric vectors only accelerate inversion.
        check(
            (0..=p).all(|i| {
                (0..=r).all(|j| q(s.weights[i][j]) * &w0 == q(s.weights[i][0]) * q(s.weights[0][j]))
            }),
            "Quadric graph requires exactly separable original positive weights",
        )?;
        let clocks = [
            super::clock::Clock::original((0..=p).map(|i| q(s.weights[i][0])).collect()),
            super::clock::Clock::original((0..=r).map(|j| q(s.weights[0][j])).collect()),
        ];
        let parameterization = if clocks.iter().all(|c| c.identity()) {
            OriginalQuadricGraphParameterization::PolynomialAffine
        } else if clocks.iter().all(|c| c.projective()) {
            OriginalQuadricGraphParameterization::SeparableProjective
        } else {
            OriginalQuadricGraphParameterization::SeparableRational
        };
        for axes in [[0, 1], [0, 2], [1, 2]] {
            let dependent = 3 - axes[0] - axes[1];
            if matrix[dependent][dependent] != 0. {
                continue;
            }
            let pole = |i: usize, j: usize| axes.map(|k| q(s.control_points[i][j][k]));
            let origin = pole(0, 0);
            let u = std::array::from_fn(|k| pole(p, 0)[k].clone() - &origin[k]);
            let v = std::array::from_fn(|k| pole(0, r)[k].clone() - &origin[k]);
            let det = &u[0] * &v[1] - &u[1] * &v[0];
            if det.is_zero() {
                continue;
            }
            let affine = (0..=p).all(|i| {
                (0..=r).all(|j| {
                    (0..2).all(|k| {
                        pole(i, j)[k]
                            == &origin[k]
                                + &u[k] * q(i as f64) / q(p as f64)
                                + &v[k] * q(j as f64) / q(r as f64)
                    })
                })
            });
            if !affine {
                continue;
            }
            // The coefficient of the dependent coordinate is affine in the
            // projected world coordinates. Equal strict corner signs prove
            // it never vanishes on this whole parallelogram, hence each
            // projected point has exactly one implicit dependent coordinate.
            let signs: Vec<_> = [pole(0, 0), pole(p, 0), pole(0, r), pole(p, r)]
                .iter()
                .map(|xy| {
                    q(matrix[dependent][3])
                        + q(matrix[3][dependent])
                        + (0..2)
                            .map(|k| {
                                (q(matrix[dependent][axes[k]]) + q(matrix[axes[k]][dependent]))
                                    * &xy[k]
                            })
                            .sum::<Q>()
                })
                .collect();
            if !signs.iter().all(|x| *x > Q::zero()) && !signs.iter().all(|x| *x < Q::zero()) {
                continue;
            }
            return Ok(Self {
                axes,
                parameterization,
                clocks,
                origin,
                u,
                v,
                det,
            });
        }
        Err(crate::input(
            "No proved globally single-valued original quadric graph chart",
        ))
    }
    pub(super) fn inverse_clock(&self, axis: usize, a: &Q) -> [Q; 2] {
        self.clocks[axis].inverse(a)
    }
    pub(super) fn fractions(&self, p: &[Q; 3]) -> [Q; 2] {
        let x = &p[self.axes[0]] - &self.origin[0];
        let y = &p[self.axes[1]] - &self.origin[1];
        [
            (&x * &self.v[1] - &y * &self.v[0]) / &self.det,
            (&self.u[0] * &y - &self.u[1] * &x) / &self.det,
        ]
    }
}
