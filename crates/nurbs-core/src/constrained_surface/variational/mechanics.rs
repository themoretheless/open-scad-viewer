//! Original reference metric and linearized membrane/second-form variations.
use super::*;
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|k| a[(k + 1) % 3] * b[(k + 2) % 3] - a[(k + 2) % 3] * b[(k + 1) % 3])
}
pub(super) struct Frame {
    pub area: f64,
    pub normal: [f64; 3],
    tangent: [[f64; 3]; 2],
    directions: [[f64; 2]; 2],
    gamma: [[f64; 2]; 3],
}
impl Frame {
    pub fn at(s: &Surface, uv: [f64; 2]) -> Result<Self> {
        let e = s.evaluate(uv[0], uv[1])?;
        let (u, v) = e
            .first_derivatives()
            .ok_or_else(|| error("Reference shell first jets unresolved"))?;
        let (uu, uv, vv) = e
            .second_derivatives()
            .ok_or_else(|| error("Reference shell second jets unresolved"))?;
        let length = norm(&u);
        let n = cross(u, v);
        let area = norm(&n);
        if length <= 0. || area <= 0. || !length.is_finite() || !area.is_finite() {
            return Err(error(
                "Reference shell metric singular or unresolved at quadrature site",
            ));
        }
        let normal = n.map(|x| x / area);
        let t1 = u.map(|x| x / length);
        let t2 = cross(normal, t1);
        let height = dot(&v, &t2);
        if !height.is_finite() || height <= 0. {
            return Err(error("Reference shell transverse metric unresolved"));
        }
        let a = [1. / length, 0.];
        let b = [-dot(&v, &t1) / (length * height), 1. / height];
        let gamma = [uu, uv, vv].map(|d| {
            [
                a[0] * dot(&d, &t1) + b[0] * dot(&d, &t2),
                b[1] * dot(&d, &t2),
            ]
        });
        if a.iter()
            .chain(&b)
            .chain(gamma.iter().flatten())
            .any(|x| !x.is_finite())
        {
            return Err(error("Reference shell connection overflow"));
        }
        Ok(Self {
            area,
            normal,
            tangent: [t1, t2],
            directions: [a, b],
            gamma,
        })
    }
    /// Affine strain offset of the trial anchor relative to the ORIGINAL field.
    pub fn offset(
        &self,
        trial: &Surface,
        reference: &Surface,
        uv: [f64; 2],
    ) -> Result<[[f64; 3]; 2]> {
        let a = trial.evaluate(uv[0], uv[1])?;
        let b = reference.evaluate(uv[0], uv[1])?;
        let (au, av) = a
            .first_derivatives()
            .ok_or_else(|| error("Trial anchor first jet unresolved"))?;
        let (bu, bv) = b
            .first_derivatives()
            .ok_or_else(|| error("Original shell first jet unresolved"))?;
        let (auu, auv, avv) = a
            .second_derivatives()
            .ok_or_else(|| error("Trial anchor second jet unresolved"))?;
        let (buu, buv, bvv) = b
            .second_derivatives()
            .ok_or_else(|| error("Original shell second jet unresolved"))?;
        let du: [f64; 3] = std::array::from_fn(|k| au[k] - bu[k]);
        let dv: [f64; 3] = std::array::from_fn(|k| av[k] - bv[k]);
        let second = [[auu, buu], [auv, buv], [avv, bvv]];
        let cov: [[f64; 3]; 3] = std::array::from_fn(|i| {
            std::array::from_fn(|k| {
                second[i][0][k]
                    - second[i][1][k]
                    - self.gamma[i][0] * du[k]
                    - self.gamma[i][1] * dv[k]
            })
        });
        let [u, v] = self.directions;
        let da: [f64; 3] = std::array::from_fn(|k| u[0] * du[k] + u[1] * dv[k]);
        let db: [f64; 3] = std::array::from_fn(|k| v[0] * du[k] + v[1] * dv[k]);
        let h = |a: [f64; 2], b: [f64; 2]| {
            (0..3)
                .map(|k| {
                    self.normal[k]
                        * (a[0] * b[0] * cov[0][k]
                            + (a[0] * b[1] + a[1] * b[0]) * cov[1][k]
                            + a[1] * b[1] * cov[2][k])
                })
                .sum::<f64>()
        };
        Ok([
            [
                dot(&self.tangent[0], &da),
                dot(&self.tangent[1], &db),
                0.5 * (dot(&self.tangent[0], &db) + dot(&self.tangent[1], &da)),
            ],
            [h(u, u), h(v, v), h(u, v)],
        ])
    }
    pub fn strains(&self, jets: &[[f64; 6]]) -> [[Vec<f64>; 3]; 2] {
        let n = jets.len();
        let mut membrane: [Vec<f64>; 3] = std::array::from_fn(|_| vec![0.; 3 * n]);
        let mut bending: [Vec<f64>; 3] = std::array::from_fn(|_| vec![0.; 3 * n]);
        let [a, b] = self.directions;
        for (i, r) in jets.iter().enumerate() {
            let da = a[0] * r[1] + a[1] * r[2];
            let db = b[0] * r[1] + b[1] * r[2];
            let cov: [f64; 3] = std::array::from_fn(|k| {
                r[k + 3] - self.gamma[k][0] * r[1] - self.gamma[k][1] * r[2]
            });
            let hess = |a: [f64; 2], b: [f64; 2]| {
                a[0] * b[0] * cov[0] + (a[0] * b[1] + a[1] * b[0]) * cov[1] + a[1] * b[1] * cov[2]
            };
            let kappa = [hess(a, a), hess(b, b), hess(a, b)];
            for k in 0..3 {
                membrane[0][k * n + i] = self.tangent[0][k] * da;
                membrane[1][k * n + i] = self.tangent[1][k] * db;
                membrane[2][k * n + i] = 0.5 * (self.tangent[0][k] * db + self.tangent[1][k] * da);
                for j in 0..3 {
                    bending[j][k * n + i] = self.normal[k] * kappa[j];
                }
            }
        }
        [membrane, bending]
    }
}
