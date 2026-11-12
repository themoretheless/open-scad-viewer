//! Original graph chart, exact rational clocks and single implicit branch proof.
use super::*;
use num_traits::Zero;
pub(super) struct Chart {
    pub axes: [usize; 2],
    pub parameterization: OriginalQuadricGraphParameterization,
    joint: Option<super::joint::Joint>,
    triangular: Option<super::triangular::Triangular>,
    coupled: Option<[Q; 4]>,
    coupled_tensor: Option<Vec<Vec<Q>>>,
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
        let (nu, nv) = (s.control_points.len(), s.control_points[0].len());
        let single = nu == p + 1 && nv == r + 1;
        check(
            !s.periodic_u
                && !s.periodic_v
                && (1..=8).contains(&p)
                && (1..=8).contains(&r)
                && nu <= 256
                && nv <= 256
                && s.control_points.iter().flatten().all(|x| x.len() == 3),
            "Quadric graph needs a bounded nonperiodic 3D B-spline patch",
        )?;
        check(
            s.knots_u[..=p].iter().all(|x| *x == s.knots_u[p])
                && s.knots_u[nu..].iter().all(|x| *x == s.knots_u[nu])
                && s.knots_v[..=r].iter().all(|x| *x == s.knots_v[r])
                && s.knots_v[nv..].iter().all(|x| *x == s.knots_v[nv]),
            "Quadric graph requires original clamped knots",
        )?;
        let w0 = q(s.weights[0][0]);
        // Rank-one positive weights give independent weighted Bernstein
        // clocks. Their strict monotonicity and endpoint coverage are proved
        // by the variance identity in clock.rs; no geometric assumption is
        // needed for coverage. Geometric vectors only accelerate inversion.
        let separable = (0..nu).all(|i| {
            (0..nv).all(|j| q(s.weights[i][j]) * &w0 == q(s.weights[i][0]) * q(s.weights[0][j]))
        });
        let coupled = !separable
            && single
            && (0..3).all(|i| (0..3).all(|j| q(matrix[i][j]) + q(matrix[j][i]) == q(0.)));

        let coupled_tensor = (coupled && (p > 1 || r > 1)).then(|| {
            s.weights
                .iter()
                .map(|row| row.iter().map(|w| q(*w)).collect())
                .collect()
        });
        let coupled_weights = (coupled && p == 1 && r == 1).then(|| {
            [
                q(s.weights[0][0]),
                q(s.weights[0][1]),
                q(s.weights[1][0]),
                q(s.weights[1][1]),
            ]
        });
        // Exact Greville linear precision certifies the polynomial multi-span
        // projection directly in the original basis, without extracted poles.
        let greville = |knots: &[f64], d: usize, n: usize| -> Vec<Q> {
            (0..n)
                .map(|i| {
                    ((i + 1..=i + d).map(|k| q(knots[k])).sum::<Q>() / q(d as f64) - q(knots[d]))
                        / (q(knots[n]) - q(knots[d]))
                })
                .collect()
        };
        let gu = greville(&s.knots_u, p, nu);
        let gv = greville(&s.knots_v, r, nv);
        let constant = s.weights.iter().flatten().all(|w| q(*w) == w0);
        let clocks = if !separable || (!single && constant) {
            [
                super::clock::Clock::original(vec![q(1.), q(1.)]),
                super::clock::Clock::original(vec![q(1.), q(1.)]),
            ]
        } else if !single {
            [
                super::clock::Clock::bspline(
                    p,
                    &s.knots_u,
                    (0..nu).map(|i| q(s.weights[i][0])).collect(),
                    &gu,
                )?,
                super::clock::Clock::bspline(
                    r,
                    &s.knots_v,
                    (0..nv).map(|j| q(s.weights[0][j])).collect(),
                    &gv,
                )?,
            ]
        } else {
            [
                super::clock::Clock::original((0..=p).map(|i| q(s.weights[i][0])).collect()),
                super::clock::Clock::original((0..=r).map(|j| q(s.weights[0][j])).collect()),
            ]
        };
        let parameterization = if coupled && (p > 1 || r > 1) {
            OriginalQuadricGraphParameterization::CoupledBernstein
        } else if coupled {
            OriginalQuadricGraphParameterization::CoupledBilinear
        } else if clocks.iter().all(|c| c.identity()) {
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
            let u = std::array::from_fn(|k| pole(nu - 1, 0)[k].clone() - &origin[k]);
            let v = std::array::from_fn(|k| pole(0, nv - 1)[k].clone() - &origin[k]);
            let det = &u[0] * &v[1] - &u[1] * &v[0];
            if det.is_zero() {
                continue;
            }
            let frame = AffineFrame { origin, u, v, det };
            let AffineFrame { origin, u, v, .. } = &frame;
            let affine = (0..nu).all(|i| {
                (0..nv).all(|j| {
                    (0..2).all(|k| pole(i, j)[k] == &origin[k] + &u[k] * &gu[i] + &v[k] * &gv[j])
                })
            });
            let ordinary = affine && (separable || coupled);
            let homogeneous =
                !ordinary && super::homogeneous_affine::prove(s, axes, &origin, &u, &v);
            let boundary_clocks = if !ordinary && !homogeneous {
                super::homogeneous_separable::prove(s, axes, &frame)
            } else {
                None
            };
            let independent = boundary_clocks.is_some();
            let triangular = if !ordinary && !homogeneous && !independent {
                super::triangular::Triangular::prove(s, axes, &frame)
            } else {
                None
            };
            let joint = if !ordinary && !homogeneous && !independent && triangular.is_none() {
                super::joint::Joint::prove(s, axes, &frame)
            } else {
                None
            };
            if !ordinary && !homogeneous && !independent && triangular.is_none() && joint.is_none()
            {
                continue;
            }
            // The coefficient of the dependent coordinate is affine in the
            // projected world coordinates. Equal strict corner signs prove
            // it never vanishes on this whole parallelogram, hence each
            // projected point has exactly one implicit dependent coordinate.
            let signs: Vec<_> = [
                pole(0, 0),
                pole(nu - 1, 0),
                pole(0, nv - 1),
                pole(nu - 1, nv - 1),
            ]
            .iter()
            .map(|xy| {
                q(matrix[dependent][3])
                    + q(matrix[3][dependent])
                    + (0..2)
                        .map(|k| {
                            (q(matrix[dependent][axes[k]]) + q(matrix[axes[k]][dependent])) * &xy[k]
                        })
                        .sum::<Q>()
            })
            .collect();
            if !signs.iter().all(|x| *x > Q::zero()) && !signs.iter().all(|x| *x < Q::zero()) {
                continue;
            }
            return Ok(Self {
                axes,
                parameterization: if joint.is_some() {
                    OriginalQuadricGraphParameterization::JointJacobian
                } else if triangular.is_some() {
                    OriginalQuadricGraphParameterization::TriangularRational
                } else if independent {
                    OriginalQuadricGraphParameterization::HomogeneousSeparable
                } else if homogeneous {
                    OriginalQuadricGraphParameterization::HomogeneousAffine
                } else {
                    parameterization
                },
                clocks: if let Some(clocks) = boundary_clocks {
                    clocks
                } else if homogeneous {
                    [
                        super::clock::Clock::original(vec![q(1.), q(1.)]),
                        super::clock::Clock::original(vec![q(1.), q(1.)]),
                    ]
                } else {
                    clocks
                },
                coupled: if homogeneous || independent {
                    None
                } else {
                    coupled_weights
                },
                coupled_tensor: if homogeneous || independent {
                    None
                } else {
                    coupled_tensor
                },
                joint,
                triangular,
                origin: frame.origin,
                u: frame.u,
                v: frame.v,
                det: frame.det,
            });
        }
        Err(crate::input(
            "No proved globally single-valued original quadric graph chart",
        ))
    }
    pub(super) fn inverse_clocks(&self, r: &[[Q; 2]; 2]) -> Result<[[Q; 2]; 2]> {
        if let Some(j) = &self.joint {
            return j.enclosure(r);
        }
        if let Some(t) = &self.triangular {
            return t.enclosure(r);
        }
        if let Some(w) = &self.coupled_tensor {
            return super::coupled_bernstein::enclosure(w, r);
        }
        if let Some(w) = &self.coupled {
            return Ok(super::coupled::enclosure(w, r));
        }
        Ok(std::array::from_fn(|axis| {
            let lo = self.clocks[axis].inverse(&r[axis][0]);
            let hi = if r[axis][0] == r[axis][1] {
                lo.clone()
            } else {
                self.clocks[axis].inverse(&r[axis][1])
            };
            [lo[0].clone(), hi[1].clone()]
        }))
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
