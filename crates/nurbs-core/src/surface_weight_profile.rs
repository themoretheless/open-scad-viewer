//! Positional error bound for an exact separable original rational weight profile.
use crate::{Result, check, interval_eval::Interval as I, surface::Surface};
/// A proposal certificate, not a replacement Surface or a B-rep admission.
/// The target weights are the exact expressions row[i]*column[j]/pivot;
/// flattening those expressions to binary64 is not covered by this certificate.
pub struct WeightProfile<'a> {
    source: &'a Surface,
    row: Vec<f64>,
    column: Vec<f64>,
    pivot: f64,
    axis_deviation_upper: [f64; 3],
    weight_deviation_upper: f64,
}
/// Direction certificate for the exact proposed profile, not its original Surface.
pub struct ProfileDirection<'a, 's> {
    profile: &'a WeightProfile<'s>,
    endpoints: [[f64; 2]; 2],
    axis: usize,
    boundary: f64,
    factor: Option<(bool, usize)>,
    components: [[f64; 2]; 3],
}
impl ProfileDirection<'_, '_> {
    pub fn profile(&self) -> &WeightProfile<'_> {
        self.profile
    }
    pub fn endpoints(&self) -> [[f64; 2]; 2] {
        self.endpoints
    }
    pub fn axis(&self) -> usize {
        self.axis
    }
    pub fn boundary(&self) -> f64 {
        self.boundary
    }
    pub fn factor(&self) -> Option<(bool, usize)> {
        self.factor
    }
    pub fn components(&self) -> [[f64; 2]; 3] {
        self.components
    }
}
/// Pointwise equality of one original natural boundary and the proposal.
pub struct ProfileBoundary<'a, 's> {
    profile: &'a WeightProfile<'s>,
    axis: usize,
    boundary: f64,
}
impl ProfileBoundary<'_, '_> {
    pub fn profile(&self) -> &WeightProfile<'_> {
        self.profile
    }
    pub fn axis(&self) -> usize {
        self.axis
    }
    pub fn boundary(&self) -> f64 {
        self.boundary
    }
}
/// Complete angular coverage of two affine parameter lines. This does not
/// establish that the lines are the same B-rep edge.
pub struct ProfilePathReport {
    pub aligned: Option<bool>,
    pub cells: usize,
    pub spans: usize,
    pub accepted_cells: usize,
    pub upper_sine_squared: Option<f64>,
    pub pending: Vec<[f64; 2]>,
    pub reason: &'static str,
}
/// Diagnostic limiting planes for a collapsed natural U boundary of the exact
/// proposed profile. Distinct limits cannot authorize a regular G1 pole.
pub struct ProfilePoleLimits {
    pub point: [f64; 3],
    pub components: [[[f64; 2]; 3]; 2],
    pub sine_squared_interval: Option<[f64; 2]>,
    pub distinct_tangent_limits: bool,
}
/// Exact iso-parameter cut carrier on one immutable proposed profile.
/// This view keeps source/factor ownership instead of flattening new weights.
pub struct ProfileSection<'p, 's> {
    profile: &'p WeightProfile<'s>,
    axis: usize,
    parameter: f64,
    range: [f64; 2],
}
impl ProfileSection<'_, '_> {
    pub fn profile(&self) -> &WeightProfile<'_> {
        self.profile
    }
    pub fn axis(&self) -> usize {
        self.axis
    }
    pub fn parameter(&self) -> f64 {
        self.parameter
    }
    pub fn range(&self) -> [f64; 2] {
        self.range
    }
    fn rectangle(&self, range: [f64; 2]) -> Result<[[f64; 2]; 2]> {
        check(
            range.iter().all(|x| x.is_finite())
                && range[0] <= range[1]
                && range[0] >= self.range[0]
                && range[1] <= self.range[1],
            "Keep section parameters in its owned range",
        )?;
        let mut domain = [[self.parameter; 2]; 2];
        domain[1 - self.axis] = range;
        Ok(domain)
    }
    pub fn enclose(
        &self,
        range: [f64; 2],
        max_spans: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        self.profile.enclose(self.rectangle(range)?, max_spans)
    }
    /// Complete derivative of the exact section with respect to its free
    /// original parameter, including both one-sided knot limits.
    pub fn tangent_bounds(
        &self,
        range: [f64; 2],
        max_spans: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        let (bounds, spent) = self
            .profile
            .derivative_bounds(self.rectangle(range)?, max_spans)?;
        Ok((
            bounds.map(|b| std::array::from_fn(|k| b[k][1 - self.axis])),
            spent,
        ))
    }
    /// Transverse support derivative for constructing a boundary-compatible
    /// transition. This does not certify any independently proposed cap.
    pub fn transverse_bounds(
        &self,
        range: [f64; 2],
        max_spans: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        let (bounds, spent) = self
            .profile
            .derivative_bounds(self.rectangle(range)?, max_spans)?;
        Ok((
            bounds.map(|b| std::array::from_fn(|k| b[k][self.axis])),
            spent,
        ))
    }
    /// Normals belong to the incident proposed support, not an as-yet unauthored cap.
    pub fn support_normals(
        &self,
        range: [f64; 2],
        max_spans: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        self.profile
            .normal_bounds(self.rectangle(range)?, max_spans)
    }
}
/// Exact cubic Hermite end-transition expression attached to one immutable
/// profile section. Side rails and the collapsed tip are not B-rep admissions.
pub struct ProfileEndTransition<'p, 's> {
    section: ProfileSection<'p, 's>,
    target: [f64; 3],
    transverse_scale: f64,
}
impl<'p, 's> ProfileSection<'p, 's> {
    pub fn end_transition(
        self,
        target: [f64; 3],
        transverse_scale: f64,
    ) -> Result<ProfileEndTransition<'p, 's>> {
        check(
            target.iter().all(|x| x.is_finite())
                && transverse_scale.is_finite()
                && transverse_scale != 0.,
            "Require a finite target and nonzero transverse scale",
        )?;
        Ok(ProfileEndTransition {
            section: self,
            target,
            transverse_scale,
        })
    }
}
impl ProfileEndTransition<'_, '_> {
    pub fn section(&self) -> &ProfileSection<'_, '_> {
        &self.section
    }
    pub fn target(&self) -> [f64; 3] {
        self.target
    }
    pub fn transverse_scale(&self) -> f64 {
        self.transverse_scale
    }
    /// H(s,t)=(2s^3-3s^2+1)S(t)+(s^3-2s^2+s)l D(t)
    ///          +(-2s^3+3s^2)P. All operations retain exact source expressions.
    pub fn enclose(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        max_spans: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        self.section.rectangle(free)?;
        check(max_spans <= 100000, "Bound transition span work")?;
        check(
            across.iter().all(|x| x.is_finite())
                && across[0] >= 0.
                && across[1] <= 1.
                && across[0] <= across[1],
            "Keep transition parameters inside [0,1]",
        )?;
        if across == [0., 0.] {
            return self.section.enclose(free, max_spans);
        }
        if across == [1., 1.] {
            return Ok((Some(self.target.map(|x| [x, x])), 0));
        }
        let (source, spent) = self.section.enclose(free, max_spans)?;
        let Some(source) = source else {
            return Ok((None, spent));
        };
        if spent == max_spans {
            return Ok((None, spent));
        }
        let (derivative, more) = self.section.transverse_bounds(free, max_spans - spent)?;
        let Some(derivative) = derivative else {
            return Ok((None, spent + more));
        };
        let s = I::new(across[0], across[1])?;
        let s2 = s.mul(s)?;
        let s3 = s2.mul(s)?;
        let h00 = I::point(2.)
            .mul(s3)?
            .sub(I::point(3.).mul(s2)?)?
            .add(I::point(1.))?;
        let h10 = s3
            .sub(I::point(2.).mul(s2)?)?
            .add(s)?
            .mul(I::point(self.transverse_scale))?;
        let h01 = I::point(3.).mul(s2)?.sub(I::point(2.).mul(s3)?)?;
        let mut bounds = [[0.; 2]; 3];
        for k in 0..3 {
            let value = h00
                .mul(I::new(source[k][0], source[k][1])?)?
                .add(h10.mul(I::new(derivative[k][0], derivative[k][1])?)?)?
                .add(h01.mul(I::point(self.target[k]))?)?;
            bounds[k] = [value.lo, value.hi];
        }
        Ok((Some(bounds), spent + more))
    }
    /// At s=0, H=S, dH/dt=dS/dt and dH/ds=l D exactly.
    /// A nonzero support normal therefore proves the same regular tangent
    /// plane along this owned boundary. Nothing is claimed at the tip or rails.
    pub fn regular_join(&self, free: [f64; 2], max_spans: usize) -> Result<(Option<bool>, usize)> {
        let (normal, spent) = self.section.support_normals(free, max_spans)?;
        Ok((
            normal.and_then(|n| n.iter().any(|r| r[0] > 0. || r[1] < 0.).then_some(true)),
            spent,
        ))
    }
}
fn choose(n: usize, k: usize) -> f64 {
    let mut x = 1u64;
    for i in 0..k.min(n - k) {
        x = x * (n - i) as u64 / (i + 1) as u64;
    }
    x as f64
}
impl<'source> WeightProfile<'source> {
    /// Exact new cut boundary for subsequent endpoint surface construction.
    /// Creating the carrier does not trim the face or sew a closed Body.
    pub fn section(&self, axis: usize, parameter: f64) -> Result<ProfileSection<'_, 'source>> {
        check(
            axis < 2 && parameter.is_finite(),
            "Choose a finite profile section coordinate",
        )?;
        let s = self.source;
        let knots = [&s.knots_u, &s.knots_v];
        let degrees = [s.degree_u, s.degree_v];
        let counts = [s.control_points.len(), s.control_points[0].len()];
        let range = [
            knots[1 - axis][degrees[1 - axis]],
            knots[1 - axis][counts[1 - axis]],
        ];
        check(
            parameter >= knots[axis][degrees[axis]] && parameter <= knots[axis][counts[axis]],
            "Section coordinate lies inside the original natural chart",
        )?;
        Ok(ProfileSection {
            profile: self,
            axis,
            parameter,
            range,
        })
    }
    pub fn source(&self) -> &Surface {
        self.source
    }
    pub fn factors(&self) -> (&[f64], &[f64], f64) {
        (&self.row, &self.column, self.pivot)
    }
    pub fn axis_deviation_upper(&self) -> [f64; 3] {
        self.axis_deviation_upper
    }
    pub fn weight_deviation_upper(&self) -> f64 {
        self.weight_deviation_upper
    }
    /// Certified enclosure of the exact proposed profile over a complete
    /// original parameter rectangle. No target weight is rounded to f64.
    pub fn enclose(
        &self,
        domain: [[f64; 2]; 2],
        max_spans: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        crate::normal_alignment::validate_rectangle(self.source, domain, max_spans)?;
        let s = self.source;
        let degrees = [s.degree_u, s.degree_v];
        let counts = [s.control_points.len(), s.control_points[0].len()];
        let knots = [&s.knots_u, &s.knots_v];
        let mut hull = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
        let mut spent = 0;
        for u in degrees[0]..counts[0] {
            for v in degrees[1]..counts[1] {
                let span = [
                    [knots[0][u], knots[0][u + 1]],
                    [knots[1][v], knots[1][v + 1]],
                ];
                if span.iter().any(|a| a[0] == a[1]) {
                    continue;
                }
                let section = std::array::from_fn(|a| {
                    [span[a][0].max(domain[a][0]), span[a][1].min(domain[a][1])]
                });
                if section.iter().any(|a| a[0] > a[1]) {
                    continue;
                }
                if spent == max_spans {
                    return Ok((None, spent));
                }
                let net = self.homogeneous_on([u, v], span)?;
                let values = crate::normal_alignment::restrict_hull(&net, degrees, span, section)?;
                for k in 0..3 {
                    let r = values[k].div(values[3])?;
                    hull[k][0] = hull[k][0].min(r.lo);
                    hull[k][1] = hull[k][1].max(r.hi);
                }
                spent += 1;
            }
        }
        if spent == 0 {
            return Ok((None, 0));
        }
        Ok((Some(hull), spent))
    }
    /// Quotient derivatives of the exact factored homogeneous expression.
    /// Every original span intersecting the rectangle contributes; incomplete
    /// work returns no certificate. No replacement binary64 weights are made.
    pub fn derivative_bounds(
        &self,
        domain: [[f64; 2]; 2],
        max_spans: usize,
    ) -> Result<(Option<[[[f64; 2]; 2]; 3]>, usize)> {
        crate::normal_alignment::validate_rectangle(self.source, domain, max_spans)?;
        let s = self.source;
        let degrees = [s.degree_u, s.degree_v];
        let counts = [s.control_points.len(), s.control_points[0].len()];
        let knots = [&s.knots_u, &s.knots_v];
        let mut hull = [[[f64::INFINITY, f64::NEG_INFINITY]; 2]; 3];
        let mut spent = 0;
        for u in degrees[0]..counts[0] {
            for v in degrees[1]..counts[1] {
                let span = [
                    [knots[0][u], knots[0][u + 1]],
                    [knots[1][v], knots[1][v + 1]],
                ];
                if span.iter().any(|r| r[0] == r[1]) {
                    continue;
                }
                let section = std::array::from_fn(|a| {
                    [span[a][0].max(domain[a][0]), span[a][1].min(domain[a][1])]
                });
                if section.iter().any(|r| r[0] > r[1]) {
                    continue;
                }
                if spent == max_spans {
                    return Ok((None, spent));
                }
                let values = crate::normal_alignment::jacobian_net_on(
                    self.homogeneous_on([u, v], span)?,
                    degrees,
                    span,
                    section,
                    &s.control_points[u][v],
                )?;
                for k in 0..3 {
                    for axis in 0..2 {
                        hull[k][axis][0] = hull[k][axis][0].min(values[k][axis].lo);
                        hull[k][axis][1] = hull[k][axis][1].max(values[k][axis].hi);
                    }
                }
                spent += 1;
            }
        }
        Ok((if spent == 0 { None } else { Some(hull) }, spent))
    }
    /// Complete rectangle enclosure of the exact profile's unnormalized normal.
    /// Includes both one-sided limits at knots; zero-containing bounds do not
    /// establish regularity. A missing span never returns a partial certificate.
    pub fn normal_bounds(
        &self,
        domain: [[f64; 2]; 2],
        max_spans: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        crate::normal_alignment::validate_rectangle(self.source, domain, max_spans)?;
        let s = self.source;
        let degrees = [s.degree_u, s.degree_v];
        let counts = [s.control_points.len(), s.control_points[0].len()];
        let knots = [&s.knots_u, &s.knots_v];
        let mut hull = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
        let mut spent = 0;
        for u in degrees[0]..counts[0] {
            for v in degrees[1]..counts[1] {
                let span = [
                    [knots[0][u], knots[0][u + 1]],
                    [knots[1][v], knots[1][v + 1]],
                ];
                if span.iter().any(|a| a[0] == a[1]) {
                    continue;
                }
                let section = std::array::from_fn(|a| {
                    [span[a][0].max(domain[a][0]), span[a][1].min(domain[a][1])]
                });
                if section.iter().any(|a| a[0] > a[1]) {
                    continue;
                }
                if spent == max_spans {
                    return Ok((None, spent));
                }
                let net = self.homogeneous_on([u, v], span)?;
                let j = crate::normal_alignment::jacobian_net_on(
                    net,
                    degrees,
                    span,
                    section,
                    &s.control_points[u][v],
                )?;
                for k in 0..3 {
                    let a = (k + 1) % 3;
                    let b = (k + 2) % 3;
                    let n = j[a][0].mul(j[b][1])?.sub(j[b][0].mul(j[a][1])?)?;
                    hull[k][0] = hull[k][0].min(n.lo);
                    hull[k][1] = hull[k][1].max(n.hi);
                }
                spent += 1;
            }
        }
        Ok((if spent == 0 { None } else { Some(hull) }, spent))
    }
    /// Compare complete proposed-profile and original-surface normal bounds.
    /// This proves only tangent-plane angle over the supplied rectangles;
    /// shared position, seam ownership and orientation require separate gates.
    pub fn inspect_original_normals(
        &self,
        original: &Surface,
        domains: [[[f64; 2]; 2]; 2],
        max_sine_squared: f64,
        max_spans: usize,
    ) -> Result<crate::normal_alignment::PairReport> {
        self.inspect_normal_pair(original, None, domains, max_sine_squared, max_spans)
    }
    /// Complete rectangle angle bounds for two exact proposed profiles.
    pub fn inspect_profile_normals(
        &self,
        other: &WeightProfile<'_>,
        domains: [[[f64; 2]; 2]; 2],
        threshold: f64,
        max_spans: usize,
    ) -> Result<crate::normal_alignment::PairReport> {
        self.inspect_normal_pair(other.source, Some(other), domains, threshold, max_spans)
    }
    fn inspect_normal_pair(
        &self,
        original: &Surface,
        profile: Option<&WeightProfile<'_>>,
        domains: [[[f64; 2]; 2]; 2],
        max_sine_squared: f64,
        max_spans: usize,
    ) -> Result<crate::normal_alignment::PairReport> {
        check(
            max_sine_squared.is_finite() && (0. ..1.).contains(&max_sine_squared),
            "Finite squared-sine threshold below one",
        )?;
        crate::normal_alignment::validate_rectangle(original, domains[1], max_spans)?;
        let (a, used) = self.normal_bounds(domains[0], max_spans)?;
        let mut out = crate::normal_alignment::PairReport {
            aligned: None,
            sine_squared_interval: None,
            normal_components: [a, None],
            spans: used,
            reason: "span-limit",
        };
        if a.is_none() || used == max_spans {
            return Ok(out);
        }
        let (b, spent) = match profile {
            Some(p) => p.normal_bounds(domains[1], max_spans - used)?,
            None => crate::normal_alignment::normal_bounds(original, domains[1], max_spans - used)?,
        };
        out.spans += spent;
        out.normal_components[1] = b;
        if let (Some(a), Some(b)) = (a, b) {
            let mut compared =
                crate::normal_alignment::inspect_pair_bounds([a, b], max_sine_squared)?;
            compared.spans = out.spans;
            Ok(compared)
        } else {
            Ok(out)
        }
    }
    /// Adaptively cover the entire [0,1] correspondence of two affine UV lines.
    /// Endpoint coordinates are exact binary64 inputs; interval arithmetic
    /// encloses every mapped point. Both endpoints stay in interval coverage.
    pub fn inspect_original_path(
        &self,
        original: &Surface,
        endpoints: [[[f64; 2]; 2]; 2],
        threshold: f64,
        max_cells: usize,
        max_spans: usize,
    ) -> Result<ProfilePathReport> {
        self.inspect_path(original, None, endpoints, threshold, max_cells, max_spans)
    }
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
        )
    }
    fn inspect_path(
        &self,
        original: &Surface,
        other: Option<&WeightProfile<'_>>,
        endpoints: [[[f64; 2]; 2]; 2],
        threshold: f64,
        max_cells: usize,
        max_spans: usize,
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
            let r = self.inspect_normal_pair(
                original,
                other,
                domains,
                threshold,
                max_spans - out.spans,
            )?;
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
        out.reason = "profile-path-tangent-planes-qualified";
        Ok(out)
    }
    /// Compare factored one-sided transverse directions at both ends of a
    /// collapsed U rail. The ordinary U derivative is combined with the exact
    /// removed transverse factor; this is a pole diagnostic, not regularity.
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
                self.row[fixed] == self.pivot && s.weights[i][j] == self.column[j]
            } else {
                self.column[fixed] == self.pivot && s.weights[i][j] == self.row[i]
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
/// Cover every original control. Whole-domain positional deviation follows from
/// positive rational basis weights, without sampling or derivative assumptions.
pub fn qualify_weight_profile<'a>(
    s: &'a Surface,
    anchor: [usize; 2],
    max_controls: usize,
) -> Result<(Option<WeightProfile<'a>>, usize)> {
    s.validate()?;
    check(
        max_controls <= 1_000_000 && anchor[0] < s.weights.len() && anchor[1] < s.weights[0].len(),
        "Choose an original profile anchor and bounded control work",
    )?;
    if s.periodic_u || s.periodic_v {
        return Ok((None, 0));
    }
    let row: Vec<_> = s.weights.iter().map(|r| r[anchor[1]]).collect();
    let column = s.weights[anchor[0]].clone();
    let pivot = s.weights[anchor[0]][anchor[1]];
    let mut delta = 0_f64;
    let mut lower = f64::INFINITY;
    let mut ranges = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    let mut spent = 0;
    for i in 0..row.len() {
        for j in 0..column.len() {
            if spent == max_controls {
                return Ok((None, spent));
            }
            let target = I::point(row[i])
                .mul(I::point(column[j]))?
                .div(I::point(pivot))?;
            let difference = target.sub(I::point(s.weights[i][j]))?;
            delta = delta.max(difference.lo.abs()).max(difference.hi.abs());
            lower = lower.min(target.lo);
            for k in 0..3 {
                let coordinate = s.control_points[i][j][k];
                ranges[k][0] = ranges[k][0].min(coordinate);
                ranges[k][1] = ranges[k][1].max(coordinate);
            }
            spent += 1;
        }
    }
    if lower <= 0. {
        return Ok((None, spent));
    }
    // T-S = sum B_ij (targetWeight-originalWeight)(P_ij-S)/targetW.
    // Positive original weights place S in the original control convex hull;
    // sum B_ij=1 and targetW>=min(targetWeight). No chart samples enter proof.
    let mut axis_deviation_upper = [0.; 3];
    for k in 0..3 {
        let extent = I::point(ranges[k][1]).sub(I::point(ranges[k][0]))?;
        axis_deviation_upper[k] = extent.mul(I::point(delta))?.div(I::point(lower))?.hi;
    }
    Ok((
        Some(WeightProfile {
            source: s,
            row,
            column,
            pivot,
            axis_deviation_upper,
            weight_deviation_upper: delta,
        }),
        spent,
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn surface() -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn adaptive_profile_path_retains_pending_coverage() {
        let s = surface();
        let profile = qualify_weight_profile(&s, [0, 0], 4).unwrap().0.unwrap();
        let line = [[[0., 0.], [1., 0.]], [[0., 0.], [1., 0.]]];
        let r = profile
            .inspect_original_path(&s, line, 1e-12, 1, 2)
            .unwrap();
        assert_eq!(r.aligned, Some(true));
        assert!(r.pending.is_empty());
        let r = profile
            .inspect_original_path(&s, line, 1e-12, 1, 1)
            .unwrap();
        assert_eq!(r.aligned, None);
        assert_eq!(r.pending, vec![[0., 1.]]);
        let mut tilted = surface();
        for row in &mut tilted.control_points {
            for p in row {
                p[2] = p[1];
            }
        }
        let r = profile
            .inspect_original_path(&tilted, line, 1e-12, 100, 200)
            .unwrap();
        assert_eq!(r.aligned, Some(false));
        assert_eq!(r.pending, vec![[0., 1.]]);
    }
    #[test]
    fn exact_section_carrier_keeps_profile_identity_and_complete_span_budget() {
        let mut s = surface();
        s.weights = vec![vec![2., 6.], vec![8., 25.]];
        let p = qualify_weight_profile(&s, [0, 0], 4).unwrap().0.unwrap();
        let cut = p.section(0, 0.5).unwrap();
        assert!(std::ptr::eq(cut.profile(), &p));
        assert_eq!(
            (cut.axis(), cut.parameter(), cut.range()),
            (0, 0.5, [0., 1.])
        );
        for v in [0., 0.3, 1.] {
            let r = cut.enclose([v, v], 1).unwrap().0.unwrap();
            let expected = [0.8, 3. * v / (1. + 2. * v), 0.];
            let tangent = cut.tangent_bounds([v, v], 1).unwrap().0.unwrap();
            let transverse = cut.transverse_bounds([v, v], 1).unwrap().0.unwrap();
            for (bounds, expected) in [
                (tangent, [0., 3. / (1. + 2. * v).powi(2), 0.]),
                (transverse, [0.64, 0., 0.]),
            ] {
                for k in 0..3 {
                    assert!(bounds[k][0] <= expected[k] && expected[k] <= bounds[k][1]);
                }
            }
            for k in 0..3 {
                assert!(r[k][0] <= expected[k] && r[k][1] >= expected[k]);
            }
        }
        assert!(cut.support_normals([0.3, 0.3], 1).unwrap().0.unwrap()[2][0] > 0.);
        assert!(cut.enclose([-0.1, 0.5], 1).is_err());
        assert!(p.section(2, 0.5).is_err());
        assert!(p.section(0, 1.1).is_err());
        assert!(
            p.section(0, 0.5)
                .unwrap()
                .end_transition([0.; 3], 0.)
                .is_err()
        );
        let transition = p
            .section(0, 0.5)
            .unwrap()
            .end_transition([1., 1., 1.], 0.25)
            .unwrap();
        assert_eq!(
            transition
                .enclose([0., 1.], [1., 1.], 0)
                .unwrap()
                .0
                .unwrap(),
            [[1., 1.]; 3]
        );
        assert!(
            transition
                .enclose([0.3, 0.3], [0.5, 0.5], 1)
                .unwrap()
                .0
                .is_none()
        );
        for (t, v) in [(0., 0.2), (0.5, 0.3), (1., 0.8)] {
            let bounds = transition.enclose([v, v], [t, t], 2).unwrap().0.unwrap();
            let source = [0.8, 3. * v / (1. + 2. * v), 0.];
            let derivative = [0.64, 0., 0.];
            let h00 = 2. * t * t * t - 3. * t * t + 1.;
            let h10 = t * t * t - 2. * t * t + t;
            let h01 = 3. * t * t - 2. * t * t * t;
            for k in 0..3 {
                let value = h00 * source[k] + h10 * 0.25 * derivative[k] + h01;
                assert!(bounds[k][0] <= value && value <= bounds[k][1]);
            }
        }
        assert_eq!(
            transition.regular_join([0.2, 0.4], 1).unwrap().0,
            Some(true)
        );
        assert!(transition.regular_join([0.2, 0.4], 0).is_err());
        s.knots_v = vec![0., 0., 0.5, 1., 1.];
        for row in &mut s.control_points {
            let x = row[0][0];
            row.insert(1, vec![x, 0.5, 0.]);
        }
        for row in &mut s.weights {
            row.insert(1, 4.);
        }
        let p = qualify_weight_profile(&s, [0, 0], 6).unwrap().0.unwrap();
        let cut = p.section(0, 0.5).unwrap();
        assert!(cut.enclose([0., 1.], 1).unwrap().0.is_none());
        assert!(cut.support_normals([0., 1.], 1).unwrap().0.is_none());
        assert!(cut.enclose([0., 1.], 2).unwrap().0.is_some());
        assert!(cut.tangent_bounds([0., 1.], 1).unwrap().0.is_none());
        assert!(cut.transverse_bounds([0., 1.], 1).unwrap().0.is_none());
        assert!(cut.tangent_bounds([0., 1.], 2).unwrap().0.is_some());
        assert!(cut.transverse_bounds([0., 1.], 2).unwrap().0.is_some());
    }
    #[test]
    fn exact_profile_pole_keeps_distinct_limiting_planes() {
        let mut s = surface();
        s.degree_u = 2;
        s.knots_u = vec![0., 0., 0., 1., 1., 1.];
        s.degree_v = 2;
        s.knots_v = vec![0., 0., 0., 1., 1., 1.];
        s.control_points = vec![
            vec![vec![0., 0., 0.]; 3],
            vec![vec![0.5, 0., 0.]; 3],
            vec![vec![1., 0., 0.], vec![1., 0.5, 0.], vec![1., 1., 1.]],
        ];
        s.weights = vec![vec![1.; 3]; 3];
        let profile = qualify_weight_profile(&s, [0, 0], 9).unwrap().0.unwrap();
        let (limits, spent) = profile.pole_limits(false, 1e-6, 4).unwrap();
        let limits = limits.unwrap();
        assert_eq!(spent, 4);
        assert_eq!(limits.point, [0.; 3]);
        assert!(limits.distinct_tangent_limits);
        assert!(profile.pole_limits(false, 1e-6, 3).unwrap().0.is_none());
        assert!(profile.pole_limits(true, 1e-6, 4).unwrap().0.is_none());
    }
    #[test]
    fn two_corrected_profiles_use_their_actual_normals() {
        let mut a = surface();
        a.control_points[1][1][2] = 1.;
        a.weights[1][1] = 1.5;
        let mut b = a.clone();
        b.weights[1][1] = 2.;
        // Same factor recipe makes the proposed surfaces equal, despite
        // different original interior weights.
        let p = qualify_weight_profile(&a, [0, 0], 4).unwrap().0.unwrap();
        let q = qualify_weight_profile(&b, [0, 0], 4).unwrap().0.unwrap();
        let line = [[[0., 0.5], [1., 0.5]], [[0., 0.5], [1., 0.5]]];
        let r = p.inspect_profile_path(&q, line, 1e-4, 1000, 2000).unwrap();
        assert_eq!(r.aligned, Some(true));
        assert!(r.pending.is_empty());
        let r = p.inspect_profile_path(&q, line, 1e-4, 1, 1).unwrap();
        assert_eq!(r.aligned, None);
        assert!(!r.pending.is_empty());
        b.control_points[1][0][2] = 2.;
        let tilted = qualify_weight_profile(&b, [0, 0], 4).unwrap().0.unwrap();
        assert_eq!(
            p.inspect_profile_path(&tilted, line, 1e-4, 1000, 2000)
                .unwrap()
                .aligned,
            Some(false)
        );
    }
    #[test]
    fn adaptive_curved_path_covers_all_subintervals() {
        let mut s = surface();
        s.control_points[1][1][2] = 1.;
        let profile = qualify_weight_profile(&s, [0, 0], 4).unwrap().0.unwrap();
        let line = [[[0., 0.5], [1., 0.5]], [[0., 0.5], [1., 0.5]]];
        let limited = profile
            .inspect_original_path(&s, line, 1e-4, 2, 1000)
            .unwrap();
        assert_eq!(limited.aligned, None);
        assert!(!limited.pending.is_empty());
        let full = profile
            .inspect_original_path(&s, line, 1e-4, 1000, 2000)
            .unwrap();
        assert_eq!(full.aligned, Some(true));
        assert!(full.pending.is_empty());
        assert!(full.cells > 1 && full.accepted_cells > 1);
        assert!(full.upper_sine_squared.unwrap() <= 1e-4);
    }
    #[test]
    fn proposed_original_tangent_planes_require_complete_work() {
        let s = surface();
        let profile = qualify_weight_profile(&s, [0, 0], 4).unwrap().0.unwrap();
        let domains = [[[0., 1.], [0., 0.]], [[0., 1.], [0., 0.]]];
        let aligned = profile
            .inspect_original_normals(&s, domains, 1e-12, 2)
            .unwrap();
        assert_eq!(aligned.aligned, Some(true));
        assert_eq!(aligned.spans, 2);
        let short = profile
            .inspect_original_normals(&s, domains, 1e-12, 1)
            .unwrap();
        assert_eq!(short.aligned, None);
        assert_eq!(short.reason, "span-limit");
        let mut tilted = surface();
        for row in &mut tilted.control_points {
            for p in row {
                p[2] = p[1];
            }
        }
        let mismatch = profile
            .inspect_original_normals(&tilted, domains, 1e-12, 2)
            .unwrap();
        assert_eq!(mismatch.aligned, Some(false));
        let mut collapsed = surface();
        for row in &mut collapsed.control_points {
            for p in row {
                p[1] = 0.;
            }
        }
        assert_eq!(
            profile
                .inspect_original_normals(&collapsed, domains, 1e-12, 2)
                .unwrap()
                .aligned,
            None
        );
        assert!(
            profile
                .inspect_original_normals(&s, domains, f64::NAN, 2)
                .is_err()
        );
    }
    #[test]
    fn profile_normals_cover_exact_rational_derivatives() {
        let mut s = surface();
        s.weights = vec![vec![2., 6.], vec![8., 25.]];
        let profile = qualify_weight_profile(&s, [0, 0], 4).unwrap().0.unwrap();
        for u in [0., 0.2, 0.5, 1.] {
            for v in [0., 0.3, 0.7, 1.] {
                let (normal, spent) = profile.normal_bounds([[u, u], [v, v]], 1).unwrap();
                assert_eq!(spent, 1);
                let n = normal.unwrap();
                // Target separability: x=4u/(1+3u), y=3v/(1+2v).
                let expected = 12. / ((1. + 3. * u).powi(2) * (1. + 2. * v).powi(2));
                assert!(n[0][0] <= 0. && n[0][1] >= 0. && n[1][0] <= 0. && n[1][1] >= 0.);
                assert!(n[2][0] <= expected && n[2][1] >= expected && n[2][0] > 0.);
            }
        }
        s.knots_u = vec![0., 0., 0.5, 1., 1.];
        s.control_points
            .insert(1, vec![vec![0.5, 0., 0.], vec![0.5, 1., 0.]]);
        s.weights.insert(1, vec![4., 12.]);
        let profile = qualify_weight_profile(&s, [0, 0], 6).unwrap().0.unwrap();
        let (missing, spent) = profile.normal_bounds([[0.2, 0.8], [0.3, 0.7]], 1).unwrap();
        assert!(missing.is_none());
        assert_eq!(spent, 1);
        assert!(
            profile
                .normal_bounds([[0.5, 0.5], [0.5, 0.5]], 2)
                .unwrap()
                .0
                .is_some()
        );
    }
    #[test]
    fn tiny_weight_change_is_bounded_without_mutating_source() {
        let mut s = surface();
        s.weights[1][1] = 1_f64.next_up();
        let original = s.weights.clone();
        let (proof, spent) = qualify_weight_profile(&s, [0, 0], 4).unwrap();
        let proof = proof.unwrap();
        assert_eq!(spent, 4);
        assert!(std::ptr::eq(proof.source(), &s));
        assert!(proof.within_position_tolerance(1e-12).unwrap());
        assert!(!proof.within_position_tolerance(0.).unwrap());
        assert!(proof.within_position_tolerance(f64::NAN).is_err());
        let (row, column, pivot) = proof.factors();
        assert_eq!(row, &[1., 1.]);
        assert_eq!(column, &[1., 1.]);
        assert_eq!(pivot, 1.);
        let bound = proof.axis_deviation_upper();
        for u in [0.1, 0.4, 0.9] {
            for v in [0.1, 0.4, 0.9] {
                let p = s.evaluate(u, v).unwrap().point;
                assert!((p[0] - u).abs() <= bound[0] && (p[1] - v).abs() <= bound[1]);
            }
        }
        assert_eq!(s.weights, original);
    }
    #[test]
    fn missing_controls_and_large_changes_do_not_pass_tolerance() {
        let mut s = surface();
        s.weights[1][1] = 2.;
        let (proof, spent) = qualify_weight_profile(&s, [0, 0], 3).unwrap();
        assert!(proof.is_none());
        assert_eq!(spent, 3);
        let proof = qualify_weight_profile(&s, [0, 0], 4).unwrap().0.unwrap();
        assert!(!proof.within_position_tolerance(1e-7).unwrap());
        assert!(qualify_weight_profile(&s, [2, 0], 4).is_err());
        assert!(qualify_weight_profile(&s, [0, 0], 0).unwrap().0.is_none());
    }
    #[test]
    fn exact_profile_geometry_matches_independent_separable_formula() {
        let mut s = surface();
        s.weights = vec![vec![1., 3.], vec![2., 6.]];
        let proof = qualify_weight_profile(&s, [0, 0], 4).unwrap().0.unwrap();
        for u in [0.2, 0.37, 0.4] {
            for v in [0.3, 0.41, 0.5] {
                let (box_, spent) = proof.enclose([[u, u], [v, v]], 1).unwrap();
                let box_ = box_.unwrap();
                assert_eq!(spent, 1);
                let expected = [2. * u / (1. + u), 3. * v / (1. + 2. * v), 0.];
                for (r, x) in box_.into_iter().zip(expected) {
                    assert!(r[0] <= x && x <= r[1], "{r:?} misses {x}");
                }
            }
        }
        assert!(proof.enclose([[-0.1, 0.2], [0., 1.]], 1).is_err());
        assert!(proof.enclose([[0., 1.]; 2], 0).is_err());
    }
    #[test]
    fn profile_geometry_never_returns_partial_span_union() {
        let mut s = surface();
        s.knots_u = vec![0., 0., 0.5, 1., 1.];
        s.control_points
            .insert(1, vec![vec![0.5, 0., 0.], vec![0.5, 1., 0.]]);
        s.weights = vec![vec![1.; 2]; 3];
        let proof = qualify_weight_profile(&s, [0, 0], 6).unwrap().0.unwrap();
        let (missing, spent) = proof.enclose([[0.2, 0.8], [0.3, 0.7]], 1).unwrap();
        assert!(missing.is_none());
        assert_eq!(spent, 1);
        let (bounds, spent) = proof.enclose([[0.2, 0.8], [0.3, 0.7]], 2).unwrap();
        let bounds = bounds.unwrap();
        assert_eq!(spent, 2);
        assert!(
            bounds[0][0] <= 0.2
                && bounds[0][1] >= 0.8
                && bounds[1][0] <= 0.3
                && bounds[1][1] >= 0.7
        );
    }
    #[test]
    fn exact_profile_separability_removes_constant_rows_at_different_points() {
        let mut s = surface();
        s.degree_u = 2;
        s.knots_u = vec![0., 0., 0., 1., 1., 1.];
        s.control_points = vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![0.5, 0., 0.]; 2],
            vec![vec![1., 0., 0.]; 2],
        ];
        s.weights = vec![vec![1., 0.7], vec![1., 0.7_f64.next_up()], vec![1., 0.7]];
        let profile = qualify_weight_profile(&s, [0, 0], 6).unwrap().0.unwrap();
        let proof = profile
            .boundary_direction([[0.99, 1.], [0.2, 0.3]], 1, 1., 1)
            .unwrap()
            .0
            .unwrap();
        assert!(std::ptr::eq(proof.profile(), &profile));
        assert_eq!(proof.factor(), Some((true, 2)));
        let r = proof.components()[1];
        assert!(r[0] > 0.);
        // Profile y=(1-u)^2 * 0.7v/(1-0.3v); its boundary secant
        // divided by (1-u)^2 equals 1/(1-0.3v), including u=1.
        for v in [0.2, 0.25, 0.3] {
            let expected = 1. / (1. - 0.3 * v);
            assert!(r[0] <= expected && expected <= r[1]);
        }
        assert!(
            profile
                .boundary_direction([[0.99, 1.], [0.2, 0.3]], 1, 1., 0)
                .is_err()
        );
    }
    #[test]
    fn preserved_boundaries_are_exact_and_budgeted() {
        let mut s = surface();
        s.weights[1][1] = 1_f64.next_up();
        let proof = qualify_weight_profile(&s, [0, 0], 4).unwrap().0.unwrap();
        for axis in [0, 1] {
            let (boundary, spent) = proof.preserved_boundary(axis, false, 2).unwrap();
            let boundary = boundary.unwrap();
            assert_eq!(spent, 2);
            assert!(std::ptr::eq(boundary.profile(), &proof));
            assert_eq!(boundary.axis(), axis);
            assert_eq!(boundary.boundary(), 0.);
            assert!(proof.preserved_boundary(axis, true, 2).unwrap().0.is_none());
            assert!(
                proof
                    .preserved_boundary(axis, false, 1)
                    .unwrap()
                    .0
                    .is_none()
            );
        }
        assert!(proof.preserved_boundary(2, false, 2).is_err());
        s.control_points[1] = vec![vec![1., 0., 0.]; 2];
        let proof = qualify_weight_profile(&s, [0, 0], 4).unwrap().0.unwrap();
        assert!(proof.preserved_boundary(0, true, 2).unwrap().0.is_some());
    }
}
