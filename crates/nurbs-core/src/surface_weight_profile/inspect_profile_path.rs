//! surface weight profile operations: inspect_profile_path.
use super::*;
impl<'source> WeightProfile<'source> {
/// Complete angular comparison of two exact proposed weight profiles.
    /// Shared position and body ownership still require separate admission.
    pub fn inspect_profile_path(
        &self,
        other: &WeightProfile<'_>,
        endpoints: [[[f64; 2]; 2]; 2],
        threshold: f64,
        max_cells: usize,
        max_spans: usize,
    ) -> Result<ProfilePathReport> {
        self.inspect_path(
            other.source,
            Some(other),
            endpoints,
            threshold,
            max_cells,
            max_spans,
            None,
        )
    }
pub(super) fn inspect_path(
        &self,
        original: &Surface,
        other: Option<&WeightProfile<'_>>,
        endpoints: [[[f64; 2]; 2]; 2],
        threshold: f64,
        max_cells: usize,
        max_spans: usize,
        pole: Option<bool>,
    ) -> Result<ProfilePathReport> {
        check(
            (1..=100000).contains(&max_cells)
                && (1..=100000).contains(&max_spans)
                && threshold.is_finite()
                && (0. ..1.).contains(&threshold),
            "Bound complete profile path work",
        )?;
        for (side, s) in [self.source, original].into_iter().enumerate() {
            let rectangle = std::array::from_fn(|a| {
                [
                    endpoints[side][0][a].min(endpoints[side][1][a]),
                    endpoints[side][0][a].max(endpoints[side][1][a]),
                ]
            });
            crate::normal_alignment::validate_rectangle(s, rectangle, max_spans)?;
        }
        let mut out = ProfilePathReport {
            aligned: None,
            cells: 0,
            spans: 0,
            accepted_cells: 0,
            upper_sine_squared: None,
            pending: vec![[0., 1.]],
            reason: "profile-path-work-limit",
        };
        let mut upper = 0_f64;
        while let Some(q) = out.pending.pop() {
            if out.cells == max_cells || out.spans == max_spans {
                out.pending.push(q);
                return Ok(out);
            }
            let mut domains = [[[0.; 2]; 2]; 2];
            for side in 0..2 {
                for axis in 0..2 {
                    let a = endpoints[side][0][axis];
                    let b = endpoints[side][1][axis];
                    let r = if a == b {
                        I::point(a)
                    } else {
                        I::point(a)
                            .add(I::new(q[0], q[1])?.mul(I::point(b).sub(I::point(a))?)?)?
                            .intersect(a.min(b), a.max(b))?
                    };
                    domains[side][axis] = [r.lo, r.hi];
                }
            }
            out.cells += 1;
            let r = if let Some(high)=pole {
                let (a,used)=self.factored_pole_normal_bounds(domains[0],high,max_spans-out.spans)?;
                out.spans+=used;
                let Some(a)=a else {
                    out.pending.push(q);out.reason="profile-path-pole-factor-unproven";return Ok(out);
                };
                let (b,used)=crate::normal_alignment::normal_bounds(original,domains[1],max_spans-out.spans)?;
                out.spans+=used;
                let Some(b)=b else {
                    out.pending.push(q);return Ok(out);
                };
                crate::normal_alignment::inspect_pair_bounds([a.components,b],threshold)?
            } else {self.inspect_normal_pair(
                original,
                other,
                domains,
                threshold,
                max_spans - out.spans,
            )?};
            out.spans += r.spans;
            if r.aligned == Some(true) {
                upper = upper.max(r.sine_squared_interval.unwrap()[1]);
                out.accepted_cells += 1;
                continue;
            }
            if r.aligned == Some(false) {
                out.pending.push(q);
                out.aligned = Some(false);
                out.reason = "profile-path-angular-mismatch";
                return Ok(out);
            }
            if r.reason == "span-limit" {
                out.pending.push(q);
                return Ok(out);
            }
            let mid = q[0] + (q[1] - q[0]) * 0.5;
            if mid <= q[0] || mid >= q[1] {
                out.pending.push(q);
                out.reason = "profile-path-resolution-limit";
                return Ok(out);
            }
            out.pending.push([mid, q[1]]);
            out.pending.push([q[0], mid]);
        }
        out.aligned = Some(true);
        out.upper_sine_squared = Some(upper);
        out.reason = if pole.is_some() {"profile-path-factored-limit-angles-qualified"}else{"profile-path-tangent-planes-qualified"};
        Ok(out)
    }
/// Compare limiting normal directions after removing both longitudinal
    /// and transverse collapse factors. This never admits a regular G1 pole.
    pub fn factored_pole_limits(&self,high:bool,threshold:f64,max_spans:usize)->Result<(Option<ProfilePoleLimits>,usize)> {
        check(threshold.is_finite() && (0. ..1.).contains(&threshold) && max_spans<=100000,
            "Bound exact factored pole diagnostics")?;
        let s=self.source;
        let u=s.knots_u[if high {s.control_points.len()}else{s.degree_u}];
        let vs=[s.knots_v[s.degree_v],s.knots_v[s.control_points[0].len()]];
        let mut spent=0;let mut normals=Vec::new();
        for v in vs {
            let (normal,used)=self.factored_pole_normal_bounds([[u;2],[v;2]],high,max_spans-spent)?;
            spent+=used;
            let Some(normal)=normal else {return Ok((None,spent));};
            normals.push(normal);
        }
        check(normals[0].point==normals[1].point,"Use one exact collapsed pole")?;
        let components=[normals[0].components,normals[1].components];
        let compared=crate::normal_alignment::inspect_pair_bounds(components,threshold)?;
        Ok((Some(ProfilePoleLimits{point:normals[0].point,components,
            sine_squared_interval:compared.sine_squared_interval,
            distinct_tangent_limits:compared.aligned==Some(false)}),spent))
    }
/// First transverse factor diagnostic; higher longitudinal collapse can
    /// require `factored_pole_limits`. Neither method admits pole immersion.
    pub fn pole_limits(
        &self,
        high: bool,
        threshold: f64,
        max_spans: usize,
    ) -> Result<(Option<ProfilePoleLimits>, usize)> {
        check(
            threshold.is_finite() && (0. ..1.).contains(&threshold) && max_spans <= 100000,
            "Bound proposed pole diagnostics",
        )?;
        let s = self.source;
        let degrees = [s.degree_u, s.degree_v];
        let counts = [s.control_points.len(), s.control_points[0].len()];
        let knots = [&s.knots_u, &s.knots_v];
        let row = &s.control_points[if high { counts[0] - 1 } else { 0 }];
        if !row.iter().all(|p| p == &row[0]) {
            return Ok((None, 0));
        }
        let u = knots[0][if high { counts[0] } else { degrees[0] }];
        let vs = [knots[1][degrees[1]], knots[1][counts[1]]];
        let span = [[knots[0][degrees[0]], knots[0][counts[0]]], vs];
        let mut spent = 0;
        let mut normals = [[[0.; 2]; 3]; 2];
        for end in 0..2 {
            let domain = [[u, u], [vs[end], vs[end]]];
            let (direction, used) =
                self.boundary_direction(domain, 1, vs[end], max_spans - spent)?;
            spent += used;
            let Some(direction) = direction else {
                return Ok((None, spent));
            };
            if !matches!(direction.factor(),Some((factor_high,power)) if factor_high==high&&power>0)
            {
                return Ok((None, spent));
            }
            if spent == max_spans {
                return Ok((None, spent));
            }
            let net = self.homogeneous_on(degrees, span)?;
            let j = crate::normal_alignment::jacobian_net_on(net, degrees, span, domain, &row[0])?;
            spent += 1;
            let transverse = direction.components();
            for k in 0..3 {
                let a = (k + 1) % 3;
                let b = (k + 2) % 3;
                let n = j[a][0]
                    .mul(I::new(transverse[b][0], transverse[b][1])?)?
                    .sub(j[b][0].mul(I::new(transverse[a][0], transverse[a][1])?)?)?;
                normals[end][k] = [n.lo, n.hi];
            }
        }
        let compared = crate::normal_alignment::inspect_pair_bounds(normals, threshold)?;
        Ok((
            Some(ProfilePoleLimits {
                point: [row[0][0], row[0][1], row[0][2]],
                components: normals,
                sine_squared_interval: compared.sine_squared_interval,
                distinct_tangent_limits: compared.aligned == Some(false),
            }),
            spent,
        ))
    }
pub(super) fn homogeneous_centered_on(&self,span:[usize;2],domain:[[f64;2];2],origin:&[f64])
        ->Result<Vec<Vec<[I;4]>>> {
        crate::curve_surface_composition::surface_net_on_with_weights_centered(
            self.source,span,domain,
            |i,j| I::point(self.row[i]).mul(I::point(self.column[j]))?.div(I::point(self.pivot)),
            std::array::from_fn(|k|origin[k]),
        )
    }
pub(crate) fn homogeneous_on(
        &self,
        span: [usize; 2],
        domain: [[f64; 2]; 2],
    ) -> Result<Vec<Vec<[I; 4]>>> {
        crate::curve_surface_composition::surface_net_on_with_weights(
            self.source,
            span,
            domain,
            |i, j| {
                I::point(self.row[i])
                    .mul(I::point(self.column[j]))?
                    .div(I::point(self.pivot))
            },
        )
    }
/// Original control positions with exact separable weights. Constant endpoint
    /// rows may be different points: separability cancels each row's transverse
    /// chord exactly. Removed normalized free factors are nonnegative.
    pub fn boundary_direction<'profile>(
        &'profile self,
        endpoints: [[f64; 2]; 2],
        axis: usize,
        boundary: f64,
        max_spans: usize,
    ) -> Result<(Option<ProfileDirection<'profile, 'source>>, usize)> {
        check(axis < 2, "Choose a proposed profile secant coordinate")?;
        crate::normal_alignment::validate_rectangle(self.source, endpoints, max_spans)?;
        let s = self.source;
        let degrees = [s.degree_u, s.degree_v];
        let counts = [s.control_points.len(), s.control_points[0].len()];
        let knots = [&s.knots_u, &s.knots_v];
        let free = 1 - axis;
        let p = degrees[free];
        let q = degrees[axis];
        let span = [
            [knots[free][p], knots[free][counts[free]]],
            [knots[axis][q], knots[axis][counts[axis]]],
        ];
        check(
            boundary == span[1][0] || boundary == span[1][1],
            "Use a natural proposed profile boundary",
        )?;
        if p == 0
            || p > 8
            || q == 0
            || q > 8
            || counts[free] != p + 1
            || counts[axis] != q + 1
            || !(0..2).all(|a| {
                knots[a][..=degrees[a]]
                    .iter()
                    .all(|x| *x == knots[a][degrees[a]])
                    && knots[a][degrees[a] + 1..]
                        .iter()
                        .all(|x| *x == knots[a][counts[a]])
            })
        {
            return Ok((None, 0));
        }
        let cp = |i: usize, j: usize| {
            if axis == 1 {
                &s.control_points[i][j]
            } else {
                &s.control_points[j][i]
            }
        };
        let mut factor = None;
        for high in [false, true] {
            let mut power = 0;
            for step in 0..=p {
                let i = if high { p - step } else { step };
                if !(0..=q).all(|j| cp(i, j) == cp(i, 0)) {
                    break;
                }
                power += 1;
            }
            if power > p {
                return Ok((None, 0));
            }
            if power > 0 {
                factor = Some((high, power));
                break;
            }
        }
        let high_boundary = boundary == span[1][1];
        let b = if high_boundary { q } else { 0 };
        let mut net = vec![vec![[I::point(0.); 4]; q]; p + 1];
        for i in 0..=p {
            for j in 0..=q {
                if j == b {
                    continue;
                }
                let (u, v) = if axis == 1 { (i, j) } else { (j, i) };
                let weight = I::point(self.row[u])
                    .mul(I::point(self.column[v]))?
                    .div(I::point(self.pivot))?;
                let division =
                    I::point(q as f64)
                        .div(I::point(if high_boundary { q - j } else { j } as f64))?;
                let column = if high_boundary { j } else { j - 1 };
                for k in 0..3 {
                    net[i][column][k] = if cp(i, j)[k] == cp(i, b)[k] {
                        I::point(0.)
                    } else {
                        I::point(cp(i, j)[k])
                            .sub(I::point(cp(i, b)[k]))?
                            .mul(weight)?
                            .mul(division)?
                    };
                }
            }
        }
        let mut degree = p;
        if let Some((high, power)) = factor {
            let mut reduced = Vec::new();
            for row in 0..=p - power {
                let original = if high { row } else { row + power };
                let scale = I::point(choose(p, original)).div(I::point(choose(p - power, row)))?;
                let mut values = net[original].clone();
                for h in &mut values {
                    for k in 0..3 {
                        h[k] = h[k].mul(scale)?;
                    }
                }
                reduced.push(values);
            }
            net = reduced;
            degree -= power;
        }
        let domain = [endpoints[free], endpoints[axis]];
        let numerator =
            crate::normal_alignment::restrict_hull(&net, [degree, q - 1], span, domain)?;
        let original_span = [
            [s.knots_u[s.degree_u], s.knots_u[counts[0]]],
            [s.knots_v[s.degree_v], s.knots_v[counts[1]]],
        ];
        let homogeneous = self.homogeneous_on(degrees, original_span)?;
        let values = crate::normal_alignment::restrict_hull(
            &homogeneous,
            degrees,
            original_span,
            endpoints,
        )?;
        let scale = I::point(if high_boundary { -1. } else { 1. })
            .div(I::point(span[1][1]).sub(I::point(span[1][0]))?)?;
        let mut components = [[0.; 2]; 3];
        for k in 0..3 {
            let r = numerator[k].mul(scale)?.div(values[3])?;
            components[k] = [r.lo, r.hi];
        }
        Ok((
            Some(ProfileDirection {
                profile: self,
                endpoints,
                axis,
                boundary,
                factor,
                components,
            }),
            1,
        ))
    }
/// Prove a complete natural boundary unchanged using original coefficient
    /// identity or its constant position. No nearby weight or sampled point is
    /// an identity. Unclamped boundaries and incomplete work refuse.
    pub fn preserved_boundary<'profile>(
        &'profile self,
        axis: usize,
        high: bool,
        max_controls: usize,
    ) -> Result<(Option<ProfileBoundary<'profile, 'source>>, usize)> {
        check(
            axis < 2 && max_controls <= 1_000_000,
            "Choose an original bounded boundary proof",
        )?;
        let s = self.source;
        let knots = [&s.knots_u, &s.knots_v];
        let degrees = [s.degree_u, s.degree_v];
        let counts = [s.control_points.len(), s.control_points[0].len()];
        let boundary = if high {
            knots[axis][counts[axis]]
        } else {
            knots[axis][degrees[axis]]
        };
        let clamp = if high {
            &knots[axis][counts[axis]..]
        } else {
            &knots[axis][..=degrees[axis]]
        };
        if !clamp.iter().all(|x| *x == boundary) {
            return Ok((None, 0));
        }
        let fixed = if high { counts[axis] - 1 } else { 0 };
        let position = if axis == 0 {
            &s.control_points[fixed][0]
        } else {
            &s.control_points[0][fixed]
        };
        let mut constant = true;
        let mut weights_equal = true;
        let mut spent = 0;
        for free in 0..counts[1 - axis] {
            if spent == max_controls {
                return Ok((None, spent));
            }
            let (i, j) = if axis == 0 {
                (fixed, free)
            } else {
                (free, fixed)
            };
            constant &= s.control_points[i][j] == *position;
            weights_equal &= if axis == 0 {
                exact_positive_products_equal(self.row[fixed],self.column[j],s.weights[i][j],self.pivot)
            } else {
                exact_positive_products_equal(self.column[fixed],self.row[i],s.weights[i][j],self.pivot)
            };
            spent += 1;
        }
        if !constant && !weights_equal {
            return Ok((None, spent));
        }
        Ok((
            Some(ProfileBoundary {
                profile: self,
                axis,
                boundary,
            }),
            spent,
        ))
    }
pub fn within_position_tolerance(&self, tolerance: f64) -> Result<bool> {
        check(
            tolerance.is_finite() && tolerance >= 0.,
            "Use a finite nonnegative positional tolerance",
        )?;
        let mut square = I::point(0.);
        for x in self.axis_deviation_upper {
            square = square.add(I::point(x).mul(I::point(x))?)?;
        }
        let required = I::point(tolerance).mul(I::point(tolerance))?;
        Ok(square.hi <= required.lo)
    }
}
