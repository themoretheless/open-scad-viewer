//! Interval connection integral for a Bishop frame in one regular tangent chart.
//! For a fixed unit axis A, q=A.T, N=(A-qT)/sqrt(1-q²), B=T×N.
//! The Bishop phase satisfies theta'=q (T×A).T'/(1-q²).
//! A closed exact tangent loop with |theta_total|<3 has an unambiguous principal
//! correction -theta_total, distributed by exact source arc length. Poles,
//! unresolved speed, branches and unfinished partitions never enable a chart.
use super::super::progressive_miter::trigonometric_certificate as trig;
use super::{V, curve_jets};
use crate::sweep_support::interval_vec3::{add, cross, div, dot_tight as dot, norm, scale, sub};
use crate::{Result, curve::Curve, distance_bounds::Interval as I};

struct Cell {
    lo: f64,
    hi: f64,
    phase: I,
    length: I,
    rate: I,
    speed: I,
}
pub(super) struct Chart {
    axis: V,
    cells: Vec<Cell>,
    phase: I,
    length: I,
    alpha: I,
    beta: I,
    closed: bool,
}
fn basis(axis: V, t: V) -> Result<(V, V)> {
    let q = dot(axis, t)?;
    let denominator = I::point(1.).sub(q.mul(q)?)?;
    if denominator.lo <= 0. {
        return Err(crate::Error::new(
            "NURBS_INVALID_INPUT",
            "Bishop chart pole unresolved",
        ));
    }
    let n = div(
        sub(axis, scale(t, q)?)?,
        I::new(
            denominator.lo.sqrt().next_down(),
            denominator.hi.sqrt().next_up(),
        )?,
    )?;
    Ok((n, cross(t, n)?))
}
fn rates(axis: V, jets: [V; 2], width: I) -> Result<Option<(I, I)>> {
    let d = scale(jets[0], width)?;
    let dd = scale(jets[1], width.mul(width)?)?;
    let speed = norm(d)?;
    if speed.lo <= 0. {
        return Ok(None);
    }
    // Eliminate parallel acceleration before interval evaluation. For unit A,
    // (T×A).T' = (D×A).D'' / |D|², and 1-q²=|D×A|²/|D|².
    // This exact identity avoids dependency widening from normalizing D twice.
    let horizontal = cross(d, axis)?;
    let horizontal_length = norm(horizontal)?;
    if horizontal_length.lo <= 0. {
        return Ok(None);
    }
    let denominator = speed.mul(horizontal_length.mul(horizontal_length)?)?;
    Ok(Some((
        dot(axis, d)?.mul(dot(horizontal, dd)?)?.div(denominator)?,
        speed,
    )))
}
impl Chart {
    pub(super) fn build(
        path: &Curve,
        t0: V,
        n0: V,
        closed: bool,
        goal: f64,
        remaining: usize,
        used: &mut usize,
    ) -> Result<Option<Self>> {
        if remaining == 0 {
            return Ok(None);
        }
        let domain = path.domain();
        let width = I::point(domain[1]).sub(I::point(domain[0]))?;
        // Choose a well-conditioned global chart. This ordering is only a
        // proposal; complete rate/branch enclosures below still prove it.
        let mut quality = [0_f64; 3];
        for i in 0..64 {
            let Some(jets) = curve_jets(
                path,
                [i as f64 / 64., (i + 1) as f64 / 64.],
                remaining,
                used,
            )?
            else {
                return Ok(None);
            };
            let speed = norm(jets[0])?;
            if speed.lo <= 0. {
                return Ok(None);
            }
            let t = div(jets[0], speed)?;
            for k in 0..3 {
                quality[k] = quality[k].max(t[k].lo.abs().max(t[k].hi.abs()));
            }
        }
        let mut axes = [0usize, 1, 2];
        axes.sort_by(|&a, &b| quality[a].total_cmp(&quality[b]));
        for axis_index in axes {
            let axis = std::array::from_fn(|k| I::point(if k == axis_index { 1. } else { 0. }));
            let Ok((n, b)) = basis(axis, t0) else {
                continue;
            };
            let alpha = dot(n0, n)?;
            let beta = dot(n0, b)?;
            let mut last = None;
            for divisions in [64usize, 128, 256, 512, 1024, 2048] {
                let mut cells = Vec::new();
                let mut phase = I::point(0.);
                let mut length = I::point(0.);
                let mut complete = true;
                for i in 0..divisions {
                    let lo = i as f64 / divisions as f64;
                    let hi = (i + 1) as f64 / divisions as f64;
                    let Some(jets) = curve_jets(path, [lo, hi], remaining, used)? else {
                        complete = false;
                        break;
                    };
                    let Some((rate, speed)) = rates(axis, jets, width)? else {
                        complete = false;
                        break;
                    };
                    cells.push(Cell {
                        lo,
                        hi,
                        phase,
                        length,
                        rate,
                        speed,
                    });
                    let h = I::point(hi).sub(I::point(lo))?;
                    phase = phase.add(rate.mul(h)?)?;
                    length = length.add(speed.mul(h)?)?;
                }
                if !complete {
                    break;
                }
                if closed && (phase.lo <= -3. || phase.hi >= 3.) {
                    continue;
                }
                if length.lo <= 0. {
                    break;
                }
                let uncertainty = I::point(phase.hi).sub(I::point(phase.lo))?.hi
                    + phase.lo.abs().max(phase.hi.abs()) * (length.hi - length.lo) / length.lo;
                last = Some(Self {
                    axis,
                    cells,
                    phase,
                    length,
                    alpha,
                    beta,
                    closed,
                });
                if uncertainty <= goal || *used >= remaining {
                    break;
                }
            }
            if last.is_some() {
                return Ok(last);
            }
            if *used >= remaining {
                break;
            }
        }
        Ok(None)
    }
    pub(super) fn frame(
        &self,
        range: [f64; 2],
        tangent: V,
        remaining: usize,
        used: &mut usize,
    ) -> Result<Option<(V, V)>> {
        let mut phase: Option<I> = None;
        for cell in &self.cells {
            if range[1] < cell.lo || range[0] > cell.hi {
                continue;
            }
            if *used >= remaining {
                return Ok(None);
            }
            *used += 1;
            let lo = I::point(range[0].max(cell.lo)).sub(I::point(cell.lo))?;
            let hi = I::point(range[1].min(cell.hi)).sub(I::point(cell.lo))?;
            let local = I::new(lo.lo.max(0.), hi.hi.min(cell.hi - cell.lo))?;
            let mut value = cell.phase.add(cell.rate.mul(local)?)?;
            if self.closed {
                let fraction = cell
                    .length
                    .add(cell.speed.mul(local)?)?
                    .div(self.length)?
                    .intersect(0., 1.)?;
                value = value.sub(self.phase.mul(fraction)?)?;
            }
            phase = Some(match phase {
                None => value,
                Some(p) => I::new(p.lo.min(value.lo), p.hi.max(value.hi))?,
            });
        }
        // Exact initial phase and exact closed tangent agreement make the
        // principal arc-length correction vanish at both closing endpoints.
        if range == [0., 0.] || self.closed && range == [1., 1.] {
            phase = Some(I::point(0.));
        }
        let phase = phase.ok_or_else(|| {
            crate::Error::new(
                "NURBS_INVALID_INPUT",
                "Bishop chart range is outside the complete partition",
            )
        })?;
        let tr = trig::certify([phase.lo, phase.hi])?;
        let sin = I::new(tr.sin[0], tr.sin[1])?;
        let cos = I::new(tr.cos[0], tr.cos[1])?;
        let (n, b) = basis(self.axis, tangent)?;
        let normal = add(
            scale(n, self.alpha.mul(cos)?.sub(self.beta.mul(sin)?)?)?,
            scale(b, self.alpha.mul(sin)?.add(self.beta.mul(cos)?)?)?,
        )?;
        let clip = |v: V| -> Result<V> {
            Ok([
                v[0].intersect(-1., 1.)?,
                v[1].intersect(-1., 1.)?,
                v[2].intersect(-1., 1.)?,
            ])
        };
        let normal = clip(normal)?;
        Ok(Some((normal, clip(cross(tangent, normal)?)?)))
    }
}
