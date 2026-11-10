//! profile section bridge operations: section.
use super::*;
impl RailConstrainedBridge<'_, '_, '_, '_, '_> {
/// Original section identity used to bind B-rep carrier ownership.
    pub fn section(&self, high: bool) -> &ProfileSection<'_, '_> {
        self.bridge.section(high)
    }
pub(super) fn source_rail_domain(&self, high: bool, across: [f64; 2]) -> Result<[[f64; 2]; 2]> {
        let a = self.bridge.section(false);
        let b = self.bridge.section(true);
        let t = I::new(across[0], across[1])?;
        let coordinate = I::point(a.parameter())
            .add(t.mul(I::point(b.parameter()).sub(I::point(a.parameter()))?)?)?;
        let mut domain = [[0.; 2]; 2];
        domain[a.axis()] = [coordinate.lo, coordinate.hi];
        domain[1 - a.axis()] = [a.range()[usize::from(high)]; 2];
        let surface = a.profile().source();
        let axis = a.axis();
        let chart = if axis == 0 {
            [
                surface.knots_u[surface.degree_u],
                surface.knots_u[surface.control_points.len()],
            ]
        } else {
            [
                surface.knots_v[surface.degree_v],
                surface.knots_v[surface.control_points[0].len()],
            ]
        };
        domain[axis][0] = domain[axis][0].max(chart[0]);
        domain[axis][1] = domain[axis][1].min(chart[1]);
        Ok(domain)
    }
pub(super) fn source_rail(
        &self,
        high: bool,
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        self.bridge
            .section(false)
            .profile()
            .enclose(self.source_rail_domain(high, across)?, budget)
    }
/// Position enclosure for the full side Hermite correction. This is a
    /// distinct expression from `enclose`: its normals and regularity must be
    /// qualified separately before assembly.
    pub fn enclose_with_transverse_correction(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        self.enclose_with_transverse_correction_cached(free,across,budget,None)
    }
pub(crate) fn enclose_with_transverse_correction_cached(
        &self, free:[f64;2], across:[f64;2], budget:usize,
        cache:Option<&SideRemainderCache>,
    ) -> Result<(Option<[[f64;2];3]>,usize)> {
        self.bridge.validate(free, across, budget)?;
        let range = self.bridge.section(false).range();
        if across == [0., 0.] || across == [1., 1.] {
            return self.bridge.enclose(free, across, budget);
        }
        for high in [false, true] {
            if free == [range[usize::from(high)]; 2] {
                return self.source_rail(high, across, budget);
            }
        }
        let (base, mut spent) = self.bridge.enclose_cached(free, across, budget, cache)?;
        let Some(base) = base else {
            return Ok((None, spent));
        };
        let width = I::point(range[1]).sub(I::point(range[0]))?;
        let t = I::new(free[0], free[1])?
            .sub(I::point(range[0]))?
            .div(width)?;
        let q = I::point(1.).sub(t)?;
        let position_weights = [
            q.mul(q)?.mul(I::point(1.).add(I::point(2.).mul(t)?)?)?,
            t.mul(t)?.mul(I::point(3.).sub(I::point(2.).mul(t)?)?)?,
        ];
        let tangent_weights = [
            t.mul(q)?.mul(q)?.mul(width)?,
            I::point(-1.).mul(t)?.mul(t)?.mul(q)?.mul(width)?,
        ];
        let mut result = [I::point(0.); 3];
        for k in 0..3 {
            result[k] = I::new(base[k][0], base[k][1])?;
        }
        for high in [false, true] {
            let section = self.bridge.section(false);
            let mut values = Vec::new();
            for query in 0..4 {
                if spent == budget {
                    return Ok((None, spent));
                }
                let rail_key=(self.bridge as *const _ as usize,high,across.map(f64::to_bits),query);
                let cached=if query<2 {cache.and_then(|c|c.rail_positions.get(&rail_key))} else {None};
                let (value, used) = if let Some(value)=cached {(Some(*value),0)} else {match query {
                    0 => self.source_rail(high, across, budget - spent)?,
                    1 => self.bridge.rail(high).enclose(across, budget - spent)?,
                    2 => {
                        let (jet, used) = section.profile().derivative_bounds(
                            self.source_rail_domain(high, across)?,
                            budget - spent,
                        )?;
                        (
                            jet.map(|j| j.map(|component| component[1 - section.axis()])),
                            used,
                        )
                    }
                    _ => {
                        let (jet, used) = self.bridge.derivative_bounds(
                            [range[usize::from(high)]; 2],
                            across,
                            budget - spent,
                        )?;
                        (jet.map(|j| j[0]), used)
                    }
                }};
                spent += used;
                let Some(value) = value else {
                    return Ok((None, spent));
                };
                values.push(value);
            }
            for k in 0..3 {
                let v = |i: usize| I::new(values[i][k][0], values[i][k][1]);
                result[k] = result[k]
                    .add(position_weights[usize::from(high)].mul(v(0)?.sub(v(1)?)?)?)?
                    .add(tangent_weights[usize::from(high)].mul(v(2)?.sub(v(3)?)?)?)?;
            }
        }
        Ok((Some(result.map(|v| [v.lo, v.hi])), spent))
    }
pub(super) fn side_remainders(
        &self, high: bool, across: [f64; 2], budget: usize,
    ) -> Result<(Option<[[[f64; 2]; 3]; 4]>, usize)> {
        self.side_remainders_cached(high, across, budget, &mut SideRemainderCache::default())
    }
pub(super) fn side_remainders_cached(
        &self,
        high: bool,
        across: [f64; 2],
        budget: usize,
        cache: &mut SideRemainderCache,
    ) -> Result<(Option<[[[f64; 2]; 3]; 4]>, usize)> {
        let a = self.bridge.section(false);
        let b = self.bridge.section(true);
        let mut domain = self.source_rail_domain(high, across)?;
        domain[a.axis()] = [
            a.parameter().min(b.parameter()),
            a.parameter().max(b.parameter()),
        ];
        let source = a.profile().source();
        let (knots, degree) = if a.axis() == 0 {
            (&source.knots_u, source.degree_u)
        } else {
            (&source.knots_v, source.degree_v)
        };
        if knots.iter().any(|&k| {
            k > domain[a.axis()][0]
                && k < domain[a.axis()][1]
                && knots.iter().filter(|&&v| v == k).count() + 3 > degree
        }) {
            return Ok((None, 0));
        }
        let t = I::new(across[0], across[1])?;
        let q = I::point(1.).sub(t)?;
        let h01 = t.mul(t)?.mul(I::point(3.).sub(I::point(2.).mul(t)?)?)?;
        let h11 = t.mul(t)?.mul(q)?;
        let value = t
            .mul(t)?
            .mul(t)?
            .mul(t)?
            .div(I::point(24.))?
            .add(h01.div(I::point(24.))?)?
            .add(h11.div(I::point(6.))?)?;
        let d11 = t.mul(I::point(3.).mul(t)?.sub(I::point(2.))?)?;
        let derivative = t
            .mul(t)?
            .mul(t)?
            .div(I::point(6.))?
            .add(I::point(6.).mul(t)?.mul(q)?.div(I::point(24.))?)?
            .add(I::point(d11.lo.abs().max(d11.hi.abs())).div(I::point(6.))?)?;
        let width = I::point(b.parameter()).sub(I::point(a.parameter()))?;
        let scale = width.mul(width)?.mul(width)?.mul(width)?;
        let mut out = [[[0.; 2]; 3]; 4];
        let mut spent = 0;
        for free_jet in [false, true] {
            if spent == budget {
                return Ok((None, spent));
            }
            let mut order = [0; 2];
            order[a.axis()] = 4;
            order[1 - a.axis()] = usize::from(free_jet);
            let key = (a.profile() as *const _ as usize,
                domain.map(|d| d.map(f64::to_bits)), order);
            let (fourth, used) = if let Some(value) = cache.entries.get(&key) {
                (Some(*value), 0)
            } else {
                let result = a.profile().higher_derivative_bounds(domain, order, budget - spent)?;
                if let Some(value) = result.0 {
                    if cache.entries.len() < 100000 { cache.entries.insert(key, value); }
                }
                result
            };
            spent += used;
            let Some(fourth) = fourth else {
                return Ok((None, spent));
            };
            for k in 0..3 {
                let magnitude = I::point(fourth[k][0].abs().max(fourth[k][1].abs())).mul(scale)?;
                for (deriv, factor) in [value, derivative].into_iter().enumerate() {
                    let bound = magnitude.mul(factor)?.hi.max(0.);
                    out[2 * usize::from(free_jet) + deriv][k] = [-bound, bound];
                }
            }
        }
        Ok((Some(out), spent))
    }
/// First jets of the full side Hermite expression, not of the linear
    /// position-only correction. Shares every source query's work allowance.
    pub fn derivatives_with_transverse_correction(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[[f64; 2]; 3]; 2]>, usize)> {
        self.derivatives_with_transverse_correction_cached(
            free, across, budget, &mut SideRemainderCache::default())
    }
pub(crate) fn derivatives_with_transverse_correction_cached(
        &self, free: [f64; 2], across: [f64; 2], budget: usize,
        cache: &mut SideRemainderCache,
    ) -> Result<(Option<[[[f64; 2]; 3]; 2]>, usize)> {
        self.bridge.validate(free, across, budget)?;
        if across == [0., 0.] || across == [1., 1.] {
            return self.bridge.derivative_bounds(free, across, budget);
        }
        let section = self.bridge.section(false);
        let range = section.range();
        for high in [false, true] {
            if free == [range[usize::from(high)]; 2] {
                let (jet, used) = section
                    .profile()
                    .derivative_bounds(self.source_rail_domain(high, across)?, budget)?;
                let Some(jet) = jet else {
                    return Ok((None, used));
                };
                let mut out = [[[0.; 2]; 3]; 2];
                for k in 0..3 {
                    out[0][k] = jet[k][1 - section.axis()];
                    let v = I::new(jet[k][section.axis()][0], jet[k][section.axis()][1])?
                        .mul(I::point(self.bridge.scales[0]))?;
                    out[1][k] = [v.lo, v.hi];
                }
                return Ok((Some(out), used));
            }
        }
        let (base, mut spent) = self
            .bridge
            .derivative_bounds_cached(free, across, budget, true, Some(cache))?;
        let Some(base) = base else {
            return Ok((None, spent));
        };
        let mut out = [[I::point(0.); 3]; 2];
        for axis in 0..2 {
            for k in 0..3 {
                out[axis][k] = I::new(base[axis][k][0], base[axis][k][1])?;
            }
        }
        let width = I::point(range[1]).sub(I::point(range[0]))?;
        let t = I::new(free[0], free[1])?
            .sub(I::point(range[0]))?
            .div(width)?;
        let q = I::point(1.).sub(t)?;
        let h = [
            q.mul(q)?.mul(I::point(1.).add(I::point(2.).mul(t)?)?)?,
            t.mul(t)?.mul(I::point(3.).sub(I::point(2.).mul(t)?)?)?,
        ];
        let g = [
            t.mul(q)?.mul(q)?.mul(width)?,
            I::point(-1.).mul(t)?.mul(t)?.mul(q)?.mul(width)?,
        ];
        let dh = [
            I::point(-6.).mul(t)?.mul(q)?.div(width)?,
            I::point(6.).mul(t)?.mul(q)?.div(width)?,
        ];
        let dg = [
            q.mul(I::point(1.).sub(I::point(3.).mul(t)?)?)?,
            t.mul(I::point(3.).mul(t)?.sub(I::point(2.))?)?,
        ];
        for high in [false, true] {
            let domain = self.source_rail_domain(high, across)?;
            let rail_free = [range[usize::from(high)]; 2];
            let mut values = Vec::new();
            let mut source_jet: Option<[[[f64; 2]; 2]; 3]> = None;
            let mut bridge_jet: Option<[[[f64; 2]; 3]; 2]> = None;
            for query in 0..8 {
                if spent == budget && query != 4 && query != 5 {
                    return Ok((None, spent));
                }
                let rail_key=(self.bridge as *const _ as usize,high,across.map(f64::to_bits),query);
                let cached=if query<2 {cache.rail_positions.get(&rail_key)} else {None};
                let (value, used) = if let Some(value)=cached {(Some(*value),0)} else {match query {
                    0 => self.source_rail(high, across, budget - spent)?,
                    1 => self.bridge.rail(high).enclose(across, budget - spent)?,
                    2 | 4 => {
                        let (j, used) = if query == 4 {
                            (source_jet, 0)
                        } else {
                            let (j, used) = section
                                .profile()
                                .derivative_bounds(domain, budget - spent)?;
                            source_jet = j;
                            (j, used)
                        };
                        let mut v = [[0.; 2]; 3];
                        if let Some(j) = j {
                            for k in 0..3 {
                                let axis = if query == 2 {
                                    1 - section.axis()
                                } else {
                                    section.axis()
                                };
                                let scale = if query == 2 {
                                    1.
                                } else {
                                    self.bridge.scales[0]
                                };
                                let x =
                                    I::new(j[k][axis][0], j[k][axis][1])?.mul(I::point(scale))?;
                                v[k] = [x.lo, x.hi];
                            }
                            (Some(v), used)
                        } else {
                            (None, used)
                        }
                    }
                    3 | 5 => {
                        let (j, used) = if query == 5 {
                            (bridge_jet, 0)
                        } else {
                            let (j, used) = self.bridge.derivative_bounds_impl(
                                rail_free,
                                across,
                                budget - spent,
                                true,
                            )?;
                            bridge_jet = j;
                            (j, used)
                        };
                        (j.map(|j| j[usize::from(query == 5)]), used)
                    }
                    6 => {
                        let (j, used) = section
                            .profile()
                            .mixed_derivative_bounds(domain, budget - spent)?;
                        let mut v = [[0.; 2]; 3];
                        if let Some(j) = j {
                            for k in 0..3 {
                                let x = I::new(j[k][0], j[k][1])?
                                    .mul(I::point(self.bridge.scales[0]))?;
                                v[k] = [x.lo, x.hi];
                            }
                            (Some(v), used)
                        } else {
                            (None, used)
                        }
                    }
                    _ => self
                        .bridge
                        .mixed_derivative_bounds(rail_free, across, budget - spent)?,
                }};
                // Only complete position hulls are shared across free domains.
                // The exact owner, rail, across interval and query stay bound.
                if query<2 {if let Some(value)=value {
                    if cache.rail_positions.len()<100000 {cache.rail_positions.insert(rail_key,value);}
                }}
                spent += used;
                let Some(value) = value else {
                    return Ok((None, spent));
                };
                values.push(value);
            }
            let (remainders, used) = self.side_remainders_cached(high, across, budget - spent, cache)?;
            spent += used;
            let i = usize::from(high);
            for k in 0..3 {
                let v = |i: usize| I::new(values[i][k][0], values[i][k][1]);
                let mut errors = [
                    v(0)?.sub(v(1)?)?,
                    v(4)?.sub(v(5)?)?,
                    v(2)?.sub(v(3)?)?,
                    v(6)?.sub(v(7)?)?,
                ];
                if let Some(remainders) = remainders {
                    for j in 0..4 {
                        errors[j] =
                            errors[j].intersect(remainders[j][k][0], remainders[j][k][1])?;
                    }
                } else if spent == budget {
                    return Ok((None, spent));
                }
                out[0][k] = out[0][k]
                    .add(dh[i].mul(errors[0])?)?
                    .add(dg[i].mul(errors[2])?)?;
                out[1][k] = out[1][k]
                    .add(h[i].mul(errors[1])?)?
                    .add(g[i].mul(errors[3])?)?;
            }
        }
        Ok((Some(out.map(|axis| axis.map(|v| [v.lo, v.hi]))), spent))
    }
pub fn normals_with_transverse_correction(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        let (jet, spent) = self.derivatives_with_transverse_correction(free, across, budget)?;
        let Some([a, b]) = jet else {
            return Ok((None, spent));
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
        Ok((Some(bounds), spent))
    }
pub fn inspect_transverse_corrected_regularity(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        max_cells: usize,
        max_spans: usize,
    ) -> Result<crate::surface_weight_profile::TransitionRegularityReport> {
        self.bridge.validate(free, across, max_spans)?;
        check(max_cells <= 100000, "Bound bridge cell work")?;
        let mut out = crate::surface_weight_profile::TransitionRegularityReport {
            regular: Vec::new(),
            pending: Vec::new(),
            cells: 0,
            spans: 0,
            complete_regular: false,
            reason: "transverse-corrected-bridge-regularity-work-limit",
        };
        let mut queue = std::collections::VecDeque::from([[free, across]]);
        while let Some(rect) = queue.pop_front() {
            if out.cells == max_cells || out.spans == max_spans {
                out.pending.push(rect);
                out.pending.extend(queue);
                return Ok(out);
            }
            out.cells += 1;
            let (bounds, spent) =
                self.normals_with_transverse_correction(rect[0], rect[1], max_spans - out.spans)?;
            out.spans += spent;
            let Some(bounds) = bounds else {
                out.pending.push(rect);
                out.pending.extend(queue);
                return Ok(out);
            };
            if bounds.iter().any(|r| r[0] > 0. || r[1] < 0.) {
                out.regular.push(rect);
                continue;
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
            // Finish refining a local rectangle before distributing work over
            // every unresolved breadth level. The untouched sibling remains
            // in the queue, preserving complete requested-domain coverage.
            queue.push_front(right);
            queue.push_front(left);
        }
        out.complete_regular = out.pending.is_empty();
        out.reason = if out.complete_regular {
            "transverse-corrected-bridge-regularity-qualified"
        } else {
            "transverse-corrected-bridge-regularity-resolution-limit"
        };
        Ok(out)
    }
/// Prove injectivity only with one fixed projection and complete coverage
    /// of the corrected expression's projected Jacobian principal minors.
    pub fn inspect_transverse_corrected_injectivity(
        &self, free:[f64;2], across:[f64;2], projection:[[f64;3];2],
        max_cells:usize, max_spans:usize,
    ) -> Result<crate::surface_weight_profile::TransitionRegularityReport> {
        self.inspect_transverse_corrected_injectivity_policy(
            free,across,projection,max_cells,max_spans,false)
    }
/// Spend additional shared work on first jets integrated from source rails.
    pub fn inspect_transverse_corrected_injectivity_from_rails(
        &self, free:[f64;2], across:[f64;2], projection:[[f64;3];2],
        max_cells:usize, max_spans:usize,
    ) -> Result<crate::surface_weight_profile::TransitionRegularityReport> {
        self.inspect_transverse_corrected_injectivity_policy(
            free,across,projection,max_cells,max_spans,true)
    }
}
