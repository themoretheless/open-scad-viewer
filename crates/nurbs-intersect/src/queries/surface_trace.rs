use super::*;
#[derive(Clone, Debug)]
pub struct SurfacePoint {
    pub uv: [f64; 2],
    pub point: [f64; 3],
    pub plane_residual: f64,
}

#[derive(Clone, Debug)]
pub enum SurfaceTrace {
    /// A ruled trace linear in U, parameterized by original V.
    RuledU {
        surface: Surface,
        plane: Plane,
        v_interval: [f64; 2],
    },
    /// Affine line in the surface's original parameter coordinates.
    Line {
        surface: Surface,
        plane: Plane,
        start: [f64; 2],
        end: [f64; 2],
    },
    /// A retained ruled rational surface and plane define v(u) exactly as a
    /// procedural trace. This avoids replacing a conic with fitted samples.
    /// This variant is linear in V; RuledU retains the transposed case.
    Ruled {
        surface: Surface,
        plane: Plane,
        u_interval: [f64; 2],
    },
}
impl SurfaceTrace {
    /// Ordered rational pieces on the original [0,1] trace-fraction domain.
    /// Knot-crossing endpoints remain independent numerical values: this API
    /// does not reconcile them or grant permission to sew topology.
    pub fn to_curve_segments(&self) -> Result<Vec<Curve>> {
        self.evaluate(0.)?;
        self.evaluate(1.)?;
        match self {
            Self::Ruled {
                surface,
                plane,
                u_interval,
            } => return ruled_segments(surface, *plane, *u_interval),
            Self::RuledU {
                surface,
                plane,
                v_interval,
            } => return ruled_segments(&transpose_surface(surface), *plane, *v_interval),
            _ => (),
        }
        if let Self::Line {
            surface,
            plane,
            start,
            end,
        } = self
            && start[0] != end[0]
            && start[1] != end[1]
        {
            if surface.degree_u + surface.degree_v > 25 {
                return Err(invalid(
                    "UV diagonal conversion requires result degree at most 25",
                ));
            }
            return diagonal_segments(surface, *plane, *start, *end);
        }
        let mut curve = self.to_curve()?;
        let [lo, hi] = curve.domain();
        for knot in &mut curve.knots {
            *knot = (*knot - lo) / (hi - lo);
        }
        curve.validate()?;
        Ok(vec![curve])
    }
    /// Algebraic conversion of supported retained traces, never sample fitting.
    /// Floating-point coefficient arithmetic is numerical, not certified exact.
    /// Fraction maps linearly to the returned curve's active knot domain.
    pub fn to_curve(&self) -> Result<Curve> {
        let start_point = self.evaluate(0.)?.point;
        let end_point = self.evaluate(1.)?.point;
        match self {
            Self::RuledU {
                surface,
                plane,
                v_interval,
            } => Self::Ruled {
                surface: transpose_surface(surface),
                plane: *plane,
                u_interval: *v_interval,
            }
            .to_curve(),
            Self::Line {
                surface,
                plane,
                start,
                end,
            } => {
                let (axis, constant, range) = if start[1] == end[1] && start[0] != end[0] {
                    (nurbs_core::surface::Axis::V, start[1], [start[0], end[0]])
                } else if start[0] == end[0] && start[1] != end[1] {
                    (nurbs_core::surface::Axis::U, start[0], [start[1], end[1]])
                } else {
                    if affine_frame(surface).is_some() || start == end {
                        return Curve::from_polyline(vec![
                            start_point.to_vec(),
                            end_point.to_vec(),
                        ]);
                    }
                    let degree = surface.degree_u + surface.degree_v;
                    if degree > 25 {
                        return Err(invalid(
                            "UV diagonal conversion requires result degree at most 25",
                        ));
                    }
                    let piece = surface.trim([
                        start[0].min(end[0]),
                        start[0].max(end[0]),
                        start[1].min(end[1]),
                        start[1].max(end[1]),
                    ])?;
                    let p = piece.degree_u;
                    let q = piece.degree_v;
                    if piece.control_points.len() != p + 1 || piece.control_points[0].len() != q + 1
                    {
                        return diagonal_multispan(surface, *plane, *start, *end);
                    }
                    // Restrict the homogeneous tensor product to u(t), v(t).
                    // B_i^p(t) B_j^q(t) = C(p,i) C(q,j) / C(p+q,i+j) B_(i+j)^(p+q)(t).
                    let binomial = |n: usize, k: usize| {
                        (0..k).fold(1_f64, |value, i| value * (n - i) as f64 / (i + 1) as f64)
                    };
                    let scale = piece.weights.iter().flatten().copied().fold(0., f64::max);
                    let mut controls = vec![[0.; 4]; degree + 1];
                    for i in 0..=p {
                        for j in 0..=q {
                            let u = if start[0] > end[0] { p - i } else { i };
                            let v = if start[1] > end[1] { q - j } else { j };
                            let factor = binomial(p, i) * binomial(q, j) / binomial(degree, i + j);
                            let weight = factor * (piece.weights[u][v] / scale);
                            for axis in 0..3 {
                                controls[i + j][axis] += weight * piece.control_points[u][v][axis];
                            }
                            controls[i + j][3] += weight;
                        }
                    }
                    let curve = Curve {
                        degree,
                        knots: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
                        control_points: controls
                            .iter()
                            .enumerate()
                            .map(|(index, h)| {
                                // At either end the tensor basis has exactly one
                                // nonzero term. Copy its Cartesian coefficient;
                                // a redundant multiply/divide introduces drift.
                                if index == 0 || index == degree {
                                    let last = index == degree;
                                    let u = if (start[0] > end[0]) != last { p } else { 0 };
                                    let v = if (start[1] > end[1]) != last { q } else { 0 };
                                    piece.control_points[u][v].clone()
                                } else {
                                    (0..3).map(|axis| h[axis] / h[3]).collect()
                                }
                            })
                            .collect(),
                        weights: controls.iter().map(|h| h[3]).collect(),
                        periodic: false,
                    };
                    curve.validate()?;
                    return Ok(curve);
                };
                let curve = surface
                    .iso(axis, constant)?
                    .trim(range[0].min(range[1]), range[0].max(range[1]))?;
                if range[0] > range[1] {
                    curve.reverse()
                } else {
                    Ok(curve)
                }
            }
            Self::Ruled {
                surface,
                plane,
                u_interval,
            } => {
                let degree = surface.degree_u;
                if degree > 12 || u_interval[0] == u_interval[1] {
                    return Err(invalid(
                        "Trace conversion requires nonzero interval and result degree at most 25",
                    ));
                }
                let domain = surface_domain(surface);
                let lo = u_interval[0].min(u_interval[1]);
                let hi = u_interval[0].max(u_interval[1]);
                let piece = surface.trim([lo, hi, domain[2], domain[3]])?;
                if piece.control_points.len() != degree + 1 {
                    let mut breaks: Vec<f64> = piece
                        .knots_u
                        .iter()
                        .copied()
                        .filter(|u| *u >= lo && *u <= hi)
                        .collect();
                    breaks.dedup();
                    let result_degree = 2 * degree;
                    if (breaks.len() - 1) * result_degree + 1 > 256 {
                        return Err(invalid("Converted trace exceeds 256 control points"));
                    }
                    let mut result: Option<Curve> = None;
                    for span in breaks.windows(2) {
                        let segment = Self::Ruled {
                            surface: piece.trim([span[0], span[1], domain[2], domain[3]])?,
                            plane: *plane,
                            u_interval: [span[0], span[1]],
                        }
                        .to_curve()?;
                        if let Some(joined) = &mut result {
                            // Do not weld geometrically nearby endpoints: only identical
                            // Cartesian endpoint values can share a control point here.
                            if joined.control_points.last() != segment.control_points.first() {
                                return Err(invalid(
                                    "Converted span endpoints do not coincide numerically",
                                ));
                            }
                            let factor = joined.weights.last().unwrap() / segment.weights[0];
                            joined.knots.pop();
                            joined
                                .knots
                                .extend_from_slice(&segment.knots[result_degree + 1..]);
                            joined
                                .control_points
                                .extend_from_slice(&segment.control_points[1..]);
                            joined
                                .weights
                                .extend(segment.weights[1..].iter().map(|w| w * factor));
                        } else {
                            result = Some(segment);
                        }
                    }
                    let mut curve = result.ok_or_else(|| invalid("Trace has no nonzero spans"))?;
                    let scale = curve.weights.iter().copied().fold(0., f64::max);
                    for weight in &mut curve.weights {
                        *weight /= scale;
                    }
                    curve.validate()?;
                    return if u_interval[0] > u_interval[1] {
                        curve.reverse()
                    } else {
                        Ok(curve)
                    };
                }
                let plane = plane.normalized()?;
                let scale = piece.weights.iter().flatten().copied().fold(0., f64::max);
                let homogeneous = (0..=degree)
                    .map(|i| {
                        std::array::from_fn::<_, 2, _>(|j| {
                            let w = piece.weights[i][j] / scale;
                            [
                                piece.control_points[i][j][0] * w,
                                piece.control_points[i][j][1] * w,
                                piece.control_points[i][j][2] * w,
                                w,
                            ]
                        })
                    })
                    .collect::<Vec<_>>();
                let values = homogeneous
                    .iter()
                    .map(|row| {
                        row.map(|h| dot(plane.normal, [h[0], h[1], h[2]]) - plane.offset * h[3])
                    })
                    .collect::<Vec<_>>();
                if values.iter().flatten().any(|v| !v.is_finite()) {
                    return Err(invalid("Nonfinite ruled boundary residual"));
                }
                admit_ruled_parameter_bounds(ruled_boundary_bounds(&piece, plane))?;
                let binomial = |n: usize, k: usize| {
                    (0..k).fold(1_f64, |value, i| value * (n - i) as f64 / (i + 1) as f64)
                };
                let mut controls = vec![[0.; 4]; 2 * degree + 1];
                for i in 0..=degree {
                    for j in 0..=degree {
                        let factor =
                            binomial(degree, i) * binomial(degree, j) / binomial(2 * degree, i + j);
                        for axis in 0..4 {
                            controls[i + j][axis] += factor
                                * (homogeneous[i][1][axis] * values[j][0]
                                    - homogeneous[i][0][axis] * values[j][1]);
                        }
                    }
                }
                let sign = if controls.iter().all(|h| h[3] > 0.) {
                    1.
                } else if controls.iter().all(|h| h[3] < 0.) {
                    -1.
                } else {
                    return Err(invalid(
                        "Converted trace has no positive-weight representation in this basis",
                    ));
                };
                let scale = controls.iter().map(|h| h[3].abs()).fold(0., f64::max);
                let curve = Curve {
                    degree: 2 * degree,
                    knots: [vec![lo; 2 * degree + 1], vec![hi; 2 * degree + 1]].concat(),
                    control_points: controls
                        .iter()
                        .map(|h| (0..3).map(|i| h[i] / h[3]).collect())
                        .collect(),
                    weights: controls.iter().map(|h| sign * h[3] / scale).collect(),
                    periodic: false,
                };
                curve.validate()?;
                if u_interval[0] > u_interval[1] {
                    curve.reverse()
                } else {
                    Ok(curve)
                }
            }
        }
    }
    /// `fraction` is in [0,1]. The source surface remains authoritative.
    pub fn evaluate(&self, fraction: f64) -> Result<SurfacePoint> {
        if !fraction.is_finite() || !(0. ..=1.).contains(&fraction) {
            return Err(invalid("Trace fraction must be in [0,1]"));
        }
        if let Self::RuledU {
            surface,
            plane,
            v_interval,
        } = self
        {
            surface.validate()?;
            let mut point = Self::Ruled {
                surface: transpose_surface(surface),
                plane: *plane,
                u_interval: *v_interval,
            }
            .evaluate(fraction)?;
            point.uv.swap(0, 1);
            return Ok(point);
        }
        let (surface, plane) = match self {
            Self::Line { surface, plane, .. }
            | Self::Ruled { surface, plane, .. }
            | Self::RuledU { surface, plane, .. } => (surface, *plane),
        };
        surface.validate()?;
        let plane = plane.normalized()?;
        let uv = match self {
            Self::RuledU { .. } => unreachable!("Handled above"),
            Self::Line { start, end, .. } => {
                let domain = surface_domain(surface);
                if ![start, end].iter().all(|uv| {
                    (0..2).all(|axis| {
                        uv[axis].is_finite()
                            && uv[axis] >= domain[axis * 2]
                            && uv[axis] <= domain[axis * 2 + 1]
                    })
                }) {
                    return Err(invalid(
                        "Line trace endpoints must lie in the source domain",
                    ));
                }
                std::array::from_fn(|i| start[i] + fraction * (end[i] - start[i]))
            }
            Self::Ruled {
                surface: source,
                u_interval,
                ..
            } => {
                if source.degree_v != 1 || source.control_points[0].len() != 2 {
                    return Err(invalid("Ruled trace requires a surface linear in V"));
                }
                let domain = surface_domain(source);
                if !u_interval
                    .iter()
                    .all(|u| u.is_finite() && *u >= domain[0] && *u <= domain[1])
                {
                    return Err(invalid(
                        "Ruled trace interval must lie in the source domain",
                    ));
                }
                let u = u_interval[0] + fraction * (u_interval[1] - u_interval[0]);
                let bottom = source.evaluate(u, domain[2])?.point;
                let top = source.evaluate(u, domain[3])?.point;
                // Linear interpolation occurs in homogeneous coordinates.
                // Both boundary weights must use the same normalization.
                let basis = nurbs_core::curve::basis(
                    source.degree_u,
                    &source.knots_u,
                    source.control_points.len(),
                    u,
                    source.periodic_u,
                )?;
                let scale = source.weights.iter().flatten().copied().fold(0., f64::max);
                let weights: [f64; 2] = std::array::from_fn(|column| {
                    basis
                        .basis
                        .iter()
                        .zip(&source.weights)
                        .map(|(b, w)| b * (w[column] / scale))
                        .sum()
                });
                let a = weights[0] * plane.distance(bottom);
                let b = weights[1] * plane.distance(top);
                let denominator = a - b;
                if !denominator.is_finite() || denominator == 0. {
                    return Err(invalid("Singular ruled intersection trace"));
                }
                let v = domain[2] + a / denominator * (domain[3] - domain[2]);
                if v < domain[2] || v > domain[3] {
                    return Err(invalid("Ruled trace left the source domain"));
                }
                [u, v]
            }
        };
        let point = surface.evaluate(uv[0], uv[1])?.point;
        Ok(SurfacePoint {
            uv,
            point,
            plane_residual: plane.distance(point).abs(),
        })
    }
}
