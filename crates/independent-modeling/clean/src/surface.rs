use crate::{
    Result,
    curve::{Curve, basis_valid, de_boor, homogeneous, homogeneous_derivative, project},
    math::*,
    require,
};
#[derive(Clone, Debug, PartialEq)]
pub struct Surface {
    degree_u: usize,
    degree_v: usize,
    knots_u: Vec<f64>,
    knots_v: Vec<f64>,
    controls: Vec<Vec<Point>>,
    weights: Vec<Vec<f64>>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Differential {
    pub point: Point,
    pub du: Point,
    pub dv: Point,
    pub duu: Point,
    pub duv: Point,
    pub dvv: Point,
}
impl Differential {
    pub fn normal(&self) -> Result<Point> {
        unit(cross(self.du, self.dv))
    }
    /// Gaussian and mean curvature from the first and second fundamental forms.
    pub fn curvatures(&self) -> Result<[f64; 2]> {
        let n = self.normal()?;
        let e = dot(self.du, self.du);
        let f = dot(self.du, self.dv);
        let g = dot(self.dv, self.dv);
        let den = e * g - f * f;
        require(den.is_finite() && den > 0., "singular surface metric")?;
        let l = dot(n, self.duu);
        let m = dot(n, self.duv);
        let nn = dot(n, self.dvv);
        let k = (l * nn - m * m) / den;
        let h = (e * nn + g * l - 2. * f * m) / (2. * den);
        require(k.is_finite() && h.is_finite(), "surface curvature overflow")?;
        Ok([k, h])
    }
}
impl Surface {
    fn hderivative(&self, u: f64, v: f64, orders: [usize; 2]) -> Result<[f64; 4]> {
        let rows = self
            .controls
            .iter()
            .zip(&self.weights)
            .map(|(ps, ws)| {
                homogeneous_derivative(
                    self.degree_v,
                    &self.knots_v,
                    &ps.iter()
                        .zip(ws)
                        .map(|(&p, &w)| homogeneous(p, w))
                        .collect::<Vec<_>>(),
                    v,
                    orders[1],
                )
            })
            .collect::<Result<Vec<_>>>()?;
        homogeneous_derivative(self.degree_u, &self.knots_u, &rows, u, orders[0])
    }
    pub fn differential(&self, u: f64, v: f64) -> Result<Differential> {
        let value = self.hderivative(u, v, [0, 0])?;
        let point = project(value)?;
        let a = self.hderivative(u, v, [1, 0])?;
        let b = self.hderivative(u, v, [0, 1])?;
        let aa = self.hderivative(u, v, [2, 0])?;
        let ab = self.hderivative(u, v, [1, 1])?;
        let bb = self.hderivative(u, v, [0, 2])?;
        let du = std::array::from_fn(|i| (a[i] - point[i] * a[3]) / value[3]);
        let dv = std::array::from_fn(|i| (b[i] - point[i] * b[3]) / value[3]);
        let duu =
            std::array::from_fn(|i| (aa[i] - 2. * du[i] * a[3] - point[i] * aa[3]) / value[3]);
        let duv = std::array::from_fn(|i| {
            (ab[i] - du[i] * b[3] - dv[i] * a[3] - point[i] * ab[3]) / value[3]
        });
        let dvv =
            std::array::from_fn(|i| (bb[i] - 2. * dv[i] * b[3] - point[i] * bb[3]) / value[3]);
        require(
            [du, dv, duu, duv, dvv].into_iter().all(finite),
            "surface derivative overflow",
        )?;
        Ok(Differential {
            point,
            du,
            dv,
            duu,
            duv,
            dvv,
        })
    }
    pub fn new(
        degrees: [usize; 2],
        knots: [Vec<f64>; 2],
        controls: Vec<Vec<Point>>,
        weights: Vec<Vec<f64>>,
    ) -> Result<Self> {
        let n = controls.len();
        let m = controls.first().map_or(0, Vec::len);
        require(n * m <= 1_000_000, "surface control budget exceeded")?;
        require(
            controls.iter().all(|r| r.len() == m)
                && weights.len() == n
                && weights.iter().all(|r| r.len() == m),
            "rectangular control grid required",
        )?;
        basis_valid(degrees[0], &knots[0], n)?;
        basis_valid(degrees[1], &knots[1], m)?;
        require(
            controls.iter().flatten().copied().all(finite)
                && weights.iter().flatten().all(|w| w.is_finite() && *w > 0.),
            "finite controls and positive weights required",
        )?;
        require(
            controls
                .iter()
                .flatten()
                .zip(weights.iter().flatten())
                .all(|(&p, &w)| homogeneous(p, w).iter().all(|v| v.is_finite())),
            "homogeneous overflow",
        )?;
        let [knots_u, knots_v] = knots;
        Ok(Self {
            degree_u: degrees[0],
            degree_v: degrees[1],
            knots_u,
            knots_v,
            controls,
            weights,
        })
    }
    pub fn domains(&self) -> [[f64; 2]; 2] {
        [
            [
                self.knots_u[self.degree_u],
                self.knots_u[self.controls.len()],
            ],
            [
                self.knots_v[self.degree_v],
                self.knots_v[self.controls[0].len()],
            ],
        ]
    }
    pub fn evaluate(&self, u: f64, v: f64) -> Result<Point> {
        let rows = self
            .controls
            .iter()
            .zip(&self.weights)
            .map(|(ps, ws)| {
                de_boor(
                    self.degree_v,
                    &self.knots_v,
                    &ps.iter()
                        .zip(ws)
                        .map(|(&p, &w)| homogeneous(p, w))
                        .collect::<Vec<_>>(),
                    v,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        project(de_boor(self.degree_u, &self.knots_u, &rows, u)?)
    }
    /// Piecewise ruled loft. Profiles must have identical rational bases.
    pub fn loft(profiles: &[Curve]) -> Result<Self> {
        require(
            (2..=1024).contains(&profiles.len()),
            "loft requires 2..1024 profiles",
        )?;
        let first = &profiles[0];
        require(
            profiles.iter().all(|c| {
                c.degree == first.degree
                    && c.knots == first.knots
                    && c.controls.len() == first.controls.len()
            }),
            "compatible profile bases required",
        )?;
        let n = profiles.len();
        let mut knots = vec![0., 0.];
        knots.extend((1..n - 1).map(|i| i as f64 / (n - 1) as f64));
        knots.extend([1., 1.]);
        Self::new(
            [1, first.degree],
            [knots, first.knots.clone()],
            profiles.iter().map(|c| c.controls.clone()).collect(),
            profiles.iter().map(|c| c.weights.clone()).collect(),
        )
    }
    pub fn extrude(profile: &Curve, vector: Point) -> Result<Self> {
        require(
            finite(vector) && norm(vector) > 0.,
            "nonzero finite extrusion",
        )?;
        Self::loft(&[
            profile.clone(),
            profile.transformed(&Affine::translation(vector)?)?,
        ])
    }
    /// Rational revolution about an arbitrary axis, preserving conic sections.
    pub fn revolve(profile: &Curve, origin: Point, axis: Point, angle: f64) -> Result<Self> {
        require(finite(origin), "finite origin required")?;
        let axis = unit(axis)?;
        let arc = Curve::ellipse_arc([0.; 3], [1.; 2], 0., angle)?;
        let mut controls = Vec::new();
        let mut weights = Vec::new();
        for (&p, &w) in arc.controls.iter().zip(&arc.weights) {
            let row = profile
                .controls
                .iter()
                .map(|&point| {
                    let d = sub(point, origin);
                    let axial = scale(axis, dot(d, axis));
                    let radial = sub(d, axial);
                    add(
                        origin,
                        add(
                            axial,
                            add(scale(radial, p[0]), scale(cross(axis, radial), p[1])),
                        ),
                    )
                })
                .collect();
            controls.push(row);
            weights.push(profile.weights.iter().map(|v| v * w).collect());
        }
        Self::new(
            [arc.degree, profile.degree],
            [arc.knots, profile.knots.clone()],
            controls,
            weights,
        )
    }
    /// Parallel-transport section frames on a polyline, joined by ruled spans.
    /// Rejects zero edges and 180-degree tangent reversals; no sweep validity claim.
    pub fn sweep(profile: &Curve, path: &[Point], up: Point, twist: f64) -> Result<Self> {
        require(
            (2..=1024).contains(&path.len())
                && path.iter().copied().all(finite)
                && twist.is_finite(),
            "invalid sweep path",
        )?;
        let tangents = path
            .windows(2)
            .map(|p| unit(sub(p[1], p[0])))
            .collect::<Result<Vec<_>>>()?;
        let mut prev = tangents[0];
        let mut u = unit(cross(up, prev))?;
        let mut profiles = Vec::new();
        for (i, &position) in path.iter().enumerate() {
            let tangent = if i == 0 {
                tangents[0]
            } else if i == path.len() - 1 {
                *tangents.last().unwrap()
            } else {
                unit(add(tangents[i - 1], tangents[i]))?
            };
            let turn = cross(prev, tangent);
            let sine = norm(turn);
            let cosine = dot(prev, tangent).clamp(-1., 1.);
            if sine > 1e-14 {
                u = rotate_vector(u, turn, sine.atan2(cosine))?;
            } else {
                require(cosine > 0., "sweep tangent reversal")?;
            }
            u = unit(sub(u, scale(tangent, dot(u, tangent))))?;
            let local_u = rotate_vector(u, tangent, twist * i as f64 / (path.len() - 1) as f64)?;
            let v = cross(tangent, local_u);
            let controls = profile
                .controls
                .iter()
                .map(|p| {
                    add(
                        position,
                        add(
                            add(scale(local_u, p[0]), scale(v, p[1])),
                            scale(tangent, p[2]),
                        ),
                    )
                })
                .collect();
            profiles.push(Curve::new(
                profile.degree,
                profile.knots.clone(),
                controls,
                profile.weights.clone(),
            )?);
            prev = tangent;
        }
        Self::loft(&profiles)
    }
    pub fn transformed(&self, t: &Affine) -> Result<Self> {
        Self::new(
            [self.degree_u, self.degree_v],
            [self.knots_u.clone(), self.knots_v.clone()],
            self.controls
                .iter()
                .map(|row| row.iter().map(|p| t.apply(*p)).collect())
                .collect(),
            self.weights.clone(),
        )
    }
    /// Uniform parameter tessellation; open seams, no tolerance certificate.
    pub fn tessellate(&self, segments: [usize; 2]) -> Result<crate::mesh::Mesh> {
        let [n, m] = segments;
        require(
            n > 0 && m > 0 && n <= 1024 && m <= 1024,
            "surface segments must be 1..1024",
        )?;
        let [[a, b], [c, d]] = self.domains();
        let mut points = Vec::new();
        for i in 0..=n {
            for j in 0..=m {
                points.push(self.evaluate(
                    if i == n {
                        b
                    } else {
                        a + (b - a) * i as f64 / n as f64
                    },
                    if j == m {
                        d
                    } else {
                        c + (d - c) * j as f64 / m as f64
                    },
                )?);
            }
        }
        let mut triangles = Vec::new();
        for i in 0..n {
            for j in 0..m {
                let q = i * (m + 1) + j;
                triangles.extend([[q, q + m + 1, q + m + 2], [q, q + m + 2, q + 1]]);
            }
        }
        crate::mesh::Mesh::new(points, triangles)
    }
}
