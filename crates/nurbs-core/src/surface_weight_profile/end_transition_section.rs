//! surface weight profile operations: section.
use super::*;
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
/// Derivative in the transition coordinate, keeping the exact section
    /// and transverse expressions. Source queries share the span allowance.
    pub fn across_derivative_bounds(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        max_spans: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        self.section.rectangle(free)?;
        check(
            max_spans <= 100000
                && across.iter().all(|x| x.is_finite())
                && across[0] >= 0.
                && across[1] <= 1.
                && across[0] <= across[1],
            "Bound transition derivative parameters and work",
        )?;
        if across == [1., 1.] {
            return Ok((Some([[0., 0.]; 3]), 0));
        }
        if across == [0., 0.] {
            let (d, spent) = self.section.transverse_bounds(free, max_spans)?;
            let mut bounds = [[0.; 2]; 3];
            let Some(d) = d else {
                return Ok((None, spent));
            };
            for k in 0..3 {
                let value = I::new(d[k][0], d[k][1])?.mul(I::point(self.transverse_scale))?;
                bounds[k] = [value.lo, value.hi];
            }
            return Ok((Some(bounds), spent));
        }
        let (source, spent) = self.section.enclose(free, max_spans)?;
        let Some(source) = source else {
            return Ok((None, spent));
        };
        if spent == max_spans {
            return Ok((None, spent));
        }
        let (d, more) = self.section.transverse_bounds(free, max_spans - spent)?;
        let Some(d) = d else {
            return Ok((None, spent + more));
        };
        let t = I::new(across[0], across[1])?;
        let one_minus = I::point(1.).sub(t)?;
        let h01 = I::point(6.).mul(t)?.mul(one_minus)?;
        let h10 = one_minus
            .mul(I::point(1.).sub(I::point(3.).mul(t)?)?)?
            .mul(I::point(self.transverse_scale))?;
        let mut bounds = [[0.; 2]; 3];
        for k in 0..3 {
            let value = h01
                .mul(I::point(self.target[k]).sub(I::new(source[k][0], source[k][1])?)?)?
                .add(h10.mul(I::new(d[k][0], d[k][1])?)?)?;
            bounds[k] = [value.lo, value.hi];
        }
        Ok((Some(bounds), spent + more))
    }
/// Derivative in the section coordinate. The mixed support derivative
    /// differentiates the actual transverse field rather than freezing it.
    pub fn free_derivative_bounds(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        max_spans: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        let rectangle = self.section.rectangle(free)?;
        check(
            max_spans <= 100000
                && across.iter().all(|x| x.is_finite())
                && across[0] >= 0.
                && across[1] <= 1.
                && across[0] <= across[1],
            "Bound transition derivative work",
        )?;
        if across == [1., 1.] {
            return Ok((Some([[0., 0.]; 3]), 0));
        }
        let (tangent, spent) = self.section.tangent_bounds(free, max_spans)?;
        if across == [0., 0.] {
            return Ok((tangent, spent));
        }
        let Some(tangent) = tangent else {
            return Ok((None, spent));
        };
        if spent == max_spans {
            return Ok((None, spent));
        }
        let (mixed, more) = self
            .section
            .profile
            .mixed_derivative_bounds(rectangle, max_spans - spent)?;
        let Some(mixed) = mixed else {
            return Ok((None, spent + more));
        };
        let t = I::new(across[0], across[1])?;
        let one_minus = I::point(1.).sub(t)?;
        let square = one_minus.mul(one_minus)?;
        let h00 = square.mul(I::point(1.).add(I::point(2.).mul(t)?)?)?;
        let h10 = t.mul(square)?.mul(I::point(self.transverse_scale))?;
        let mut bounds = [[0.; 2]; 3];
        for k in 0..3 {
            let value = h00
                .mul(I::new(tangent[k][0], tangent[k][1])?)?
                .add(h10.mul(I::new(mixed[k][0], mixed[k][1])?)?)?;
            bounds[k] = [value.lo, value.hi];
        }
        Ok((Some(bounds), spent + more))
    }
pub fn normal_bounds(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        max_spans: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        let (a, spent) = self.free_derivative_bounds(free, across, max_spans)?;
        let Some(a) = a else {
            return Ok((None, spent));
        };
        if spent == max_spans && across != [1., 1.] {
            return Ok((None, spent));
        }
        let (b, more) = self.across_derivative_bounds(free, across, max_spans - spent)?;
        let Some(b) = b else {
            return Ok((None, spent + more));
        };
        let mut bounds = [[0.; 2]; 3];
        for k in 0..3 {
            let i = (k + 1) % 3;
            let j = (k + 2) % 3;
            let n = I::new(a[i][0], a[i][1])?
                .mul(I::new(b[j][0], b[j][1])?)?
                .sub(I::new(a[j][0], a[j][1])?.mul(I::new(b[i][0], b[i][1])?)?)?;
            bounds[k] = [n.lo, n.hi];
        }
        Ok((Some(bounds), spent + more))
    }
/// N=(1-s)^3 F exactly. F remains finite at the collapsed tip, but
    /// nonzero F at s=1 does not make the actual zero Jacobian regular there.
    pub fn factored_normal_bounds(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        max_spans: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        let domain = self.section.rectangle(free)?;
        check(
            max_spans <= 100000
                && across.iter().all(|x| x.is_finite())
                && across[0] >= 0.
                && across[1] <= 1.
                && across[0] <= across[1],
            "Bound factored transition normal work",
        )?;
        let mut values = Vec::new();
        let mut spent = 0;
        for query in 0..4 {
            if spent == max_spans {
                return Ok((None, spent));
            }
            let (bounds, used) = match query {
                0 => self.section.enclose(free, max_spans - spent)?,
                1 => self.section.tangent_bounds(free, max_spans - spent)?,
                2 => self.section.transverse_bounds(free, max_spans - spent)?,
                _ => self
                    .section
                    .profile
                    .mixed_derivative_bounds(domain, max_spans - spent)?,
            };
            spent += used;
            let Some(bounds) = bounds else {
                return Ok((None, spent));
            };
            values.push(bounds);
        }
        let t = I::new(across[0], across[1])?;
        let scale = I::point(self.transverse_scale);
        let mut a = [I::point(0.); 3];
        let mut b = a;
        for k in 0..3 {
            let v = |i: usize| I::new(values[i][k][0], values[i][k][1]);
            a[k] = I::point(1.)
                .add(I::point(2.).mul(t)?)?
                .mul(v(1)?)?
                .add(t.mul(scale)?.mul(v(3)?)?)?;
            b[k] = I::point(6.)
                .mul(t)?
                .mul(I::point(self.target[k]).sub(v(0)?)?)?
                .add(
                    I::point(1.)
                        .sub(I::point(3.).mul(t)?)?
                        .mul(scale)?
                        .mul(v(2)?)?,
                )?;
        }
        let mut bounds = [[0.; 2]; 3];
        for k in 0..3 {
            let i = (k + 1) % 3;
            let j = (k + 2) % 3;
            let n = a[i].mul(b[j])?.sub(a[j].mul(b[i])?)?;
            bounds[k] = [n.lo, n.hi];
        }
        Ok((Some(bounds), spent))
    }
/// Compare two independently enclosed limiting planes at the collapsed
    /// target. Equal endpoint limits alone do not qualify the full tip fan.
    pub fn tip_limits(
        &self,
        threshold: f64,
        max_spans: usize,
    ) -> Result<(Option<ProfilePoleLimits>, usize)> {
        check(
            threshold.is_finite() && (0. ..1.).contains(&threshold) && max_spans <= 100000,
            "Bound transition tip angle and work",
        )?;
        let mut normals = [[[0.; 2]; 3]; 2];
        let mut spent = 0;
        for end in 0..2 {
            let (bounds, used) = self.factored_normal_bounds(
                [self.section.range[end]; 2],
                [1., 1.],
                max_spans - spent,
            )?;
            spent += used;
            let Some(bounds) = bounds else {
                return Ok((None, spent));
            };
            normals[end] = bounds;
        }
        let compared = crate::normal_alignment::inspect_pair_bounds(normals, threshold)?;
        Ok((
            Some(ProfilePoleLimits {
                point: self.target,
                components: normals,
                sine_squared_interval: compared.sine_squared_interval,
                distinct_tangent_limits: compared.aligned == Some(false),
            }),
            spent,
        ))
    }
/// Cover the entire requested rectangle with certified regular cells and
    /// explicit pending cells. Subdivision never discards unresolved domains.
    pub fn inspect_regularity(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        max_cells: usize,
        max_spans: usize,
    ) -> Result<TransitionRegularityReport> {
        self.section.rectangle(free)?;
        check(
            max_cells <= 100000
                && max_spans <= 100000
                && across.iter().all(|x| x.is_finite())
                && across[0] >= 0.
                && across[1] <= 1.
                && across[0] <= across[1],
            "Bound transition regularity domain and work",
        )?;
        let mut queue = std::collections::VecDeque::from([[free, across]]);
        let mut out = TransitionRegularityReport {
            regular: Vec::new(),
            pending: Vec::new(),
            cells: 0,
            spans: 0,
            complete_regular: false,
            reason: "transition-regularity-work-limit",
        };
        while let Some(rect) = queue.pop_front() {
            if out.cells == max_cells || out.spans == max_spans {
                out.pending.push(rect);
                out.pending.extend(queue);
                return Ok(out);
            }
            out.cells += 1;
            let (bounds, spent) =
                self.factored_normal_bounds(rect[0], rect[1], max_spans - out.spans)?;
            out.spans += spent;
            let Some(bounds) = bounds else {
                out.pending.push(rect);
                out.pending.extend(queue);
                return Ok(out);
            };
            let nonzero = bounds.iter().any(|b| b[0] > 0. || b[1] < 0.);
            if nonzero && rect[1][1] < 1. {
                out.regular.push(rect);
                continue;
            }
            let free_width = self.section.range[1] - self.section.range[0];
            let axis = if nonzero && rect[1][1] == 1. {
                1
            } else {
                usize::from((rect[0][1] - rect[0][0]) / free_width < rect[1][1] - rect[1][0])
            };
            let mid = rect[axis][0] * 0.5 + rect[axis][1] * 0.5;
            if mid <= rect[axis][0] || mid >= rect[axis][1] {
                out.pending.push(rect);
                continue;
            }
            let mut left = rect;
            left[axis][1] = mid;
            let mut right = rect;
            right[axis][0] = mid;
            queue.push_back(left);
            queue.push_back(right);
        }
        out.complete_regular = out.pending.is_empty();
        out.reason = if out.complete_regular {
            "transition-regularity-qualified"
        } else {
            "transition-regularity-resolution-limit"
        };
        Ok(out)
    }
pub fn rail(&self, high: bool) -> ProfileTransitionRail<'_, '_, '_> {
        ProfileTransitionRail {
            transition: self,
            free: self.section.range[usize::from(high)],
        }
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
