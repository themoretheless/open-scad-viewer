//! profile section bridge operations: inspect_transverse_corrected_injectivity_policy.
use super::*;
impl RailConstrainedBridge<'_, '_, '_, '_, '_> {
pub(super) fn inspect_transverse_corrected_injectivity_policy(
        &self, free:[f64;2], across:[f64;2], projection:[[f64;3];2],
        max_cells:usize, max_spans:usize, tighten:bool,
    ) -> Result<crate::surface_weight_profile::TransitionRegularityReport> {
        self.bridge.validate(free, across, max_spans)?;
        check(
            max_cells <= 100000 && projection.iter().flatten().all(|x| x.is_finite()),
            "Bound injectivity work and projection",
        )?;
        let mut out = crate::surface_weight_profile::TransitionRegularityReport {
            regular: Vec::new(),
            pending: Vec::new(),
            cells: 0,
            spans: 0,
            complete_regular: false,
            reason: "transverse-corrected-bridge-injectivity-work-limit",
        };
        // The transverse field must be single-valued and piecewise C1. Refuse
        // internal knots whose multiplicity would allow first-derivative jumps.
        for high in [false, true] {
            let surface = self.bridge.section(high).profile().source();
            for (knots, degree, count) in [
                (
                    &surface.knots_u,
                    surface.degree_u,
                    surface.control_points.len(),
                ),
                (
                    &surface.knots_v,
                    surface.degree_v,
                    surface.control_points[0].len(),
                ),
            ] {
                if knots.iter().any(|&k| {
                    k > knots[degree]
                        && k < knots[count]
                        && knots.iter().filter(|&&x| x == k).count() >= degree
                }) {
                    out.pending.push([free, across]);
                    out.reason = "transverse-corrected-bridge-injectivity-continuity-unproven";
                    return Ok(out);
                }
            }
        }
        let mut queue = std::collections::VecDeque::from([[free, across]]);
        while let Some(rect) = queue.pop_front() {
            if out.cells == max_cells || out.spans == max_spans {
                out.pending.push(rect);
                out.pending.extend(queue);
                return Ok(out);
            }
            out.cells += 1;
            let (jet, spent) = self.derivatives_with_transverse_correction(
                rect[0],
                rect[1],
                max_spans - out.spans,
            )?;
            out.spans += spent;
            let Some(mut jet) = jet else {
                out.pending.push(rect);
                out.pending.extend(queue);
                return Ok(out);
            };
            let mut j = [[I::point(0.); 2]; 2];
            for row in 0..2 {
                for axis in 0..2 {
                    for k in 0..3 {
                        j[row][axis] = j[row][axis].add(
                            I::point(projection[row][k])
                                .mul(I::new(jet[axis][k][0], jet[axis][k][1])?)?,
                        )?;
                    }
                }
            }
            if projected_principal_minors_positive(j)? {
                out.regular.push(rect);
                continue;
            }
            if tighten && out.spans < max_spans {
                let (parts, spent) = self.free_jet_from_rail_parts(
                    rect[0], rect[1], max_spans-out.spans)?;
                out.spans += spent;
                if let Some((anchor,second,delta)) = parts {
                    let mut anchored=[[0.;2];3];
                    for k in 0..3 {
                        let v=I::new(anchor[k][0],anchor[k][1])?.add(
                            I::new(second[k][0],second[k][1])?.mul(delta)?)?;
                        anchored[k]=[v.lo,v.hi];
                    }
                    for k in 0..3 {
                        let common = I::new(jet[0][k][0],jet[0][k][1])?
                            .intersect(anchored[k][0],anchored[k][1])?;
                        jet[0][k]=[common.lo,common.hi];
                    }
                    for row in 0..2 {
                        j[row][0]=I::point(0.);
                        for k in 0..3 {
                            j[row][0]=j[row][0].add(I::point(projection[row][k])
                                .mul(I::new(jet[0][k][0],jet[0][k][1])?)?)?;
                        }
                    }
                    let projected=second_free_jet::projected_rail_column(projection,anchor,second,delta)?;
                    for row in 0..2 {
                        j[row][0]=j[row][0].intersect(projected[row].lo,projected[row].hi)?;
                    }
                    if projected_principal_minors_positive(j)? ||
                        (j[0][0].lo>0. && j[1][1].lo>0. &&
                            second_free_jet::projected_rail_determinant(projection,anchor,second,
                                delta,[j[0][1],j[1][1]])?.lo>0.) {
                        out.regular.push(rect);
                        continue;
                    }
                }
            }
            let range = self.bridge.section(false).range();
            let axis = usize::from(
                (rect[0][1] - rect[0][0]) / (range[1] - range[0]) < rect[1][1] - rect[1][0],
            );
            let mid = rect[axis][0] * 0.5 + rect[axis][1] * 0.5;
            if mid <= rect[axis][0] || mid >= rect[axis][1] {
                out.pending.push(rect);
                continue;
            }
            let mut left = rect;
            left[axis][1] = mid;
            let mut right = rect;
            right[axis][0] = mid;
            if tighten {
                // Cover the whole domain before refining one difficult rail
                // corner to machine precision. Pending cells stay explicit.
                queue.push_back(left);
                queue.push_back(right);
            } else {
                queue.push_front(right);
                queue.push_front(left);
            }
        }
        out.complete_regular = out.pending.is_empty();
        out.reason = if out.complete_regular {
            "transverse-corrected-bridge-injectivity-qualified"
        } else {
            "transverse-corrected-bridge-injectivity-resolution-limit"
        };
        Ok(out)
    }
/// Exact side first-jet identity is established by the full Hermite
    /// expression. Only regular source support normals remain to be enclosed.
    /// This checks the original profile support, not neighboring Body ownership.
    pub fn inspect_transverse_corrected_section_join(&self,high:bool,free:[f64;2],
        max_cells:usize,max_spans:usize
    )->Result<crate::surface_weight_profile::TransitionRegularityReport> {
        let across=[if high {1.}else{0.};2];
        self.bridge.validate(free,across,max_spans)?;
        let section=self.bridge.section(high);
        // The full correction vanishes with its first jet at across=0/1;
        // exact section position and the nonzero scaled transverse derivative
        // are inherited. Coverage below proves source support regularity.
        let mut domain=[[section.parameter();2];2];
        domain[1-section.axis()]=free;
        let mut report=section.profile().inspect_regularity(domain,max_cells,max_spans)?;
        report.reason=if report.complete_regular {"full-section-join-qualified"}else{"full-section-join-regularity-unproven"};
        Ok(report)
    }
/// Cover a free-end rail of the exact full corrected expression. Body
    /// ownership and neighbor angle remain separate qualification steps.
    pub fn inspect_transverse_corrected_side_join(
        &self,
        high: bool,
        across: [f64; 2],
        max_cells: usize,
        max_spans: usize,
    ) -> Result<crate::surface_weight_profile::TransitionRegularityReport> {
        let free = self.bridge.section(false).range()[usize::from(high)];
        self.bridge.validate([free; 2], across, max_spans)?;
        check(max_cells <= 100000, "Bound full side join work")?;
        let mut out = crate::surface_weight_profile::TransitionRegularityReport {
            regular: Vec::new(),
            pending: Vec::new(),
            cells: 0,
            spans: 0,
            complete_regular: false,
            reason: "full-side-join-work-limit",
        };
        let mut queue = std::collections::VecDeque::from([[[free; 2], across]]);
        while let Some(rect) = queue.pop_front() {
            if out.cells == max_cells || out.spans == max_spans {
                out.pending.push(rect);
                out.pending.extend(queue);
                return Ok(out);
            }
            out.cells += 1;
            let (normal, used) = self.bridge.section(false).profile().normal_bounds(
                self.source_rail_domain(high, rect[1])?,
                max_spans - out.spans,
            )?;
            out.spans += used;
            let Some(normal) = normal else {
                out.pending.push(rect);
                out.pending.extend(queue);
                return Ok(out);
            };
            if normal.iter().any(|r| r[0] > 0. || r[1] < 0.) {
                out.regular.push(rect);
                continue;
            }
            let mid = rect[1][0] * 0.5 + rect[1][1] * 0.5;
            if mid <= rect[1][0] || mid >= rect[1][1] {
                out.pending.push(rect);
                continue;
            }
            let mut left = rect;
            left[1][1] = mid;
            let mut right = rect;
            right[1][0] = mid;
            queue.push_back(left);
            queue.push_back(right);
        }
        out.complete_regular = out.pending.is_empty();
        out.reason = if out.complete_regular {
            "full-side-join-qualified"
        } else {
            "full-side-join-resolution-limit"
        };
        Ok(out)
    }
pub fn derivative_bounds(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[[f64; 2]; 3]; 2]>, usize)> {
        self.bridge.validate(free, across, budget)?;
        if across == [0., 0.] || across == [1., 1.] {
            return self.bridge.derivative_bounds(free, across, budget);
        }
        let (base, mut spent) = self.bridge.derivative_bounds(free, across, budget)?;
        let Some(base) = base else {
            return Ok((None, spent));
        };
        let mut result = [[I::point(0.); 3]; 2];
        for axis in 0..2 {
            for k in 0..3 {
                result[axis][k] = I::new(base[axis][k][0], base[axis][k][1])?;
            }
        }
        let range = self.bridge.section(false).range();
        let width = I::point(range[1]).sub(I::point(range[0]))?;
        let t = I::new(free[0], free[1])?
            .sub(I::point(range[0]))?
            .div(width)?;
        for high in [false, true] {
            let mut values = Vec::new();
            for query in 0..4 {
                if spent == budget {
                    return Ok((None, spent));
                }
                let (bounds, used) = match query {
                    0 => self.source_rail(high, across, budget - spent)?,
                    1 => self.bridge.rail(high).enclose(across, budget - spent)?,
                    2 => {
                        let section = self.bridge.section(false);
                        let (j, used) = section.profile().derivative_bounds(
                            self.source_rail_domain(high, across)?,
                            budget - spent,
                        )?;
                        let mut bounds = [[0.; 2]; 3];
                        if let Some(j) = j {
                            for k in 0..3 {
                                let value =
                                    I::new(j[k][section.axis()][0], j[k][section.axis()][1])?
                                        .mul(I::point(self.bridge.scales[0]))?;
                                bounds[k] = [value.lo, value.hi];
                            }
                            (Some(bounds), used)
                        } else {
                            (None, used)
                        }
                    }
                    _ => {
                        let (j, used) = self.bridge.derivative_bounds(
                            [range[usize::from(high)]; 2],
                            across,
                            budget - spent,
                        )?;
                        (j.map(|j| j[1]), used)
                    }
                };
                spent += used;
                let Some(bounds) = bounds else {
                    return Ok((None, spent));
                };
                values.push(bounds);
            }
            let weight = if high { t } else { I::point(1.).sub(t)? };
            let sign = I::point(if high { 1. } else { -1. }).div(width)?;
            for k in 0..3 {
                let v = |q: usize| I::new(values[q][k][0], values[q][k][1]);
                result[0][k] = result[0][k].add(sign.mul(v(0)?.sub(v(1)?)?)?)?;
                result[1][k] = result[1][k].add(weight.mul(v(2)?.sub(v(3)?)?)?)?;
            }
        }
        Ok((Some(result.map(|axis| axis.map(|r| [r.lo, r.hi]))), spent))
    }
pub fn normal_bounds(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        let (j, spent) = self.derivative_bounds(free, across, budget)?;
        let Some([a, b]) = j else {
            return Ok((None, spent));
        };
        let mut bounds = [[0.; 2]; 3];
        for k in 0..3 {
            let i = (k + 1) % 3;
            let l = (k + 2) % 3;
            let n = I::new(a[i][0], a[i][1])?
                .mul(I::new(b[l][0], b[l][1])?)?
                .sub(I::new(a[l][0], a[l][1])?.mul(I::new(b[i][0], b[i][1])?)?)?;
            bounds[k] = [n.lo, n.hi];
        }
        Ok((Some(bounds), spent))
    }
pub fn inspect_support_rail_normals(
        &self,
        high: bool,
        across: [f64; 2],
        threshold: f64,
        budget: usize,
    ) -> Result<crate::normal_alignment::PairReport> {
        check(
            threshold.is_finite() && (0. ..1.).contains(&threshold),
            "Bound constrained rail angle",
        )?;
        let free = self.bridge.section(false).range()[usize::from(high)];
        let (a, mut spent) = self.normal_bounds([free; 2], across, budget)?;
        let mut out = crate::normal_alignment::PairReport {
            aligned: None,
            sine_squared_interval: None,
            normal_components: [a, None],
            spans: spent,
            reason: "constrained-rail-normal-work-limit",
        };
        let Some(a) = a else {
            return Ok(out);
        };
        if spent == budget {
            return Ok(out);
        }
        let (b, used) = self
            .bridge
            .section(false)
            .profile()
            .normal_bounds(self.source_rail_domain(high, across)?, budget - spent)?;
        spent += used;
        let Some(b) = b else {
            out.spans = spent;
            return Ok(out);
        };
        let mut compared = crate::normal_alignment::inspect_pair_bounds([a, b], threshold)?;
        compared.spans = spent;
        Ok(compared)
    }
pub fn enclose(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        self.bridge.validate(free, across, budget)?;
        if across == [0., 0.] || across == [1., 1.] {
            return self.bridge.enclose(free, across, budget);
        }
        let range = self.bridge.section(false).range();
        for high in [false, true] {
            if free == [range[usize::from(high)]; 2] {
                return self.source_rail(high, across, budget);
            }
        }
        let (base, mut spent) = self.bridge.enclose(free, across, budget)?;
        let Some(base) = base else {
            return Ok((None, spent));
        };
        let mut result = base
            .map(|r| I::new(r[0], r[1]))
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        let t = I::new(free[0], free[1])?
            .sub(I::point(range[0]))?
            .div(I::point(range[1]).sub(I::point(range[0]))?)?;
        for high in [false, true] {
            if spent == budget {
                return Ok((None, spent));
            }
            let (rail, used) = self.source_rail(high, across, budget - spent)?;
            spent += used;
            let Some(rail) = rail else {
                return Ok((None, spent));
            };
            if spent == budget {
                return Ok((None, spent));
            }
            let (old, used) = self.bridge.rail(high).enclose(across, budget - spent)?;
            spent += used;
            let Some(old) = old else {
                return Ok((None, spent));
            };
            let weight = if high { t } else { I::point(1.).sub(t)? };
            for k in 0..3 {
                let error = I::new(rail[k][0], rail[k][1])?.sub(I::new(old[k][0], old[k][1])?)?;
                result[k] = result[k].add(weight.mul(error)?)?;
            }
        }
        Ok((
            Some(std::array::from_fn(|k| [result[k].lo, result[k].hi])),
            spent,
        ))
    }
}
