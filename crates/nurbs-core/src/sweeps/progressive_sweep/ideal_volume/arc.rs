//! Original normalized arc-length inverse and its first two derivatives.
//! Whole source speed cells enclose lengths; no fitted or sampled length enters
//! the phase. Positive speed is mandatory on every original closed cell.
use super::*;
use crate::sweeps::progressive_miter::vector_certificate;
struct Cell {
    domain: [f64; 2],
    speed: I,
    speed_rate: I,
    length: I,
    tangent: V3,
    tangent_rate: V3,
}
pub(super) struct ArcMap {
    // Immutable original coefficients, never a fitted inverse/proxy curve.
    path: Curve,
    cells: Vec<Cell>,
    stations: Vec<[f64; 2]>,
    nodes: Vec<f64>,
    total: I,
    prefixes: Vec<I>,
}
impl ArcMap {
    pub(super) fn build(path: &Curve, spent: &mut usize, max: usize) -> Result<Option<Self>> {
        let max = max.min(spent.saturating_add(100000));
        let [a, b] = path.domain();
        let width = I::point(b).sub(I::point(a))?;
        let mut pending = (0..64)
            .rev()
            .map(|i| [i as f64 / 64., (i + 1) as f64 / 64.])
            .collect::<Vec<_>>();
        let mut cells = Vec::new();
        while let Some(domain) = pending.pop() {
            if *spent == max {
                return Ok(None);
            }
            *spent += 1;
            let source = vector_certificate::certify_third_traversal(path, domain, max - *spent)?;
            let p = &source.base;
            *spent += p.cells;
            if p.status != Status::Certified {
                return Ok(None);
            }
            let v = iv::scale(decode(p.first.unwrap())?, width)?;
            let acc = iv::scale(decode(p.second.unwrap())?, width.mul(width)?)?;
            let speed = iv::norm(v)?;
            if speed.lo <= 0. {
                let mid = crate::distance_bounds::midpoint(domain[0], domain[1]);
                if !(domain[0] < mid && mid < domain[1]) {
                    return Ok(None);
                }
                pending.push([mid, domain[1]]);
                pending.push([domain[0], mid]);
                continue;
            }
            let speed_rate = iv::dot_tight(v, acc)?.div(speed)?;
            let h = I::point(domain[1]).sub(I::point(domain[0]))?;
            let mut length = speed.mul(h)?;
            if p.single_span {
                let jerk = iv::scale(
                    decode(source.third.unwrap())?,
                    width.mul(width)?.mul(width)?,
                )?;
                let speed_second = iv::dot_tight(acc, acc)?
                    .add(iv::dot_tight(v, jerk)?)?
                    .sub(speed_rate.mul(speed_rate)?)?
                    .div(speed)?;
                let mut ends = [I::point(0.); 2];
                for (i, t) in domain.into_iter().enumerate() {
                    let endpoint = vector_certificate::certify_first_point(path, t, max - *spent)?;
                    *spent += endpoint.cells;
                    let Some(first) = endpoint.first else {
                        return Ok(None);
                    };
                    ends[i] = iv::norm(iv::scale(decode(first)?, width)?)?;
                }
                let error = I::point(speed_second.lo.abs().max(speed_second.hi.abs()))
                    .mul(h)?
                    .mul(h)?
                    .mul(h)?
                    .div(I::point(12.))?;
                let trapezoid = ends[0].add(ends[1])?.mul(h)?.div(I::point(2.))?;
                length = I::new(trapezoid.lo, trapezoid.hi)?
                    .add(I::new(-error.hi, error.hi)?)?
                    .intersect(length.lo.max(0.), length.hi)?;
            }
            // Fixed coarse length boxes leave an inverse-clock uncertainty
            // floor: shrinking station/UV boxes then cannot tighten DF.
            // Refine the ORIGINAL integral enclosure under the same ledger.
            // These thresholds only propose subdivision, never supply proof.
            // A tiny cell crossing an original speed jump keeps its honest
            // hull integral; relative accuracy there need not be attainable.
            if length.hi-length.lo > length.lo*2_f64.powi(-16)
                && domain[1]-domain[0] > 2_f64.powi(-24) {
                let mid=crate::distance_bounds::midpoint(domain[0],domain[1]);
                if !(domain[0]<mid && mid<domain[1]) {return Ok(None);}
                pending.push([mid,domain[1]]);pending.push([domain[0],mid]);
                continue;
            }
            let tangent = iv::div(v, speed)?;
            let tangent_rate = iv::div(iv::sub(acc, iv::scale(tangent, speed_rate)?)?, speed)?;
            cells.push(Cell {
                domain,
                speed,
                speed_rate,
                length,
                tangent,
                tangent_rate,
            });
        }
        if cells.is_empty() {
            return Ok(None);
        }
        let mut prefixes = vec![I::point(0.)];
        for c in &cells {
            prefixes.push(prefixes.last().unwrap().add(c.length)?);
        }
        let mut suffixes = vec![I::point(0.); cells.len() + 1];
        for i in (0..cells.len()).rev() {
            suffixes[i] = suffixes[i + 1].add(cells[i].length)?;
        }
        let total = *prefixes.last().unwrap();
        if total.lo <= 0. {
            return Ok(None);
        }
        let mut stations = vec![[0., 0.]];
        let mut nodes = vec![0.];
        for i in 1..cells.len() {
            let p = prefixes[i];
            let q = suffixes[i];
            // p/(p+q) increases in p and decreases in q; this preserves the
            // prefix/total correlation instead of dividing unrelated length boxes.
            let lower = I::point(p.lo)
                .div(I::point(p.lo).add(I::point(q.hi))?)?
                .lo
                .max(0.);
            let upper = I::point(p.hi)
                .div(I::point(p.hi).add(I::point(q.lo))?)?
                .hi
                .min(1.);
            stations.push([lower, upper]);
            nodes.push(cells[i].domain[0]);
        }
        stations.push([1., 1.]);
        nodes.push(1.);
        Ok(Some(Self {
            path: path.clone(),
            cells,
            stations,
            nodes,
            total,
            prefixes,
        }))
    }
    pub(super) fn phase(
        &self,
        station: [f64; 2],
        spent: &mut usize,
        max: usize,
    ) -> Result<Option<frame::PathPhase>> {
        check(
            station.iter().all(|x| x.is_finite())
                && 0. <= station[0]
                && station[0] <= station[1]
                && station[1] <= 1.,
            "Invalid original arc station interval",
        )?;
        // Two bounded binary searches. Charge their worst case before executing.
        let charge = 2 * (usize::BITS - self.stations.len().leading_zeros()) as usize;
        if charge > max - *spent {
            return Ok(None);
        }
        *spent += charge;
        let lower = self
            .stations
            .partition_point(|p| p[1] <= station[0])
            .saturating_sub(1);
        let upper = self
            .stations
            .partition_point(|p| p[0] < station[1])
            .min(self.nodes.len() - 1);
        let coarse = [self.nodes[lower], self.nodes[upper]];
        let target = I::new(station[0], station[1])?.mul(self.total)?;
        let mut traversal = [f64::INFINITY, f64::NEG_INFINITY];
        let mut speed: Option<I> = None;
        let mut rate: Option<I> = None;
        let mut position: Option<[V3; 2]> = None;
        for (index, c) in self.cells.iter().enumerate() {
            if c.domain[1] < coarse[0] || c.domain[0] > coarse[1] {
                continue;
            }
            if *spent == max {
                return Ok(None);
            }
            *spent += 1;
            // The integral mean speed belongs to the original whole-cell
            // speed box. Solve its positive interval inequality, not a fitted
            // piecewise-linear inverse; all possible parameters are retained.
            let delta = target.sub(self.prefixes[index])?.div(c.speed)?;
            let h = I::point(c.domain[1]).sub(I::point(c.domain[0]))?;
            if delta.hi < 0. || delta.lo > h.hi {
                continue;
            }
            let dt = I::new(delta.lo.max(0.), delta.hi.min(h.hi))?;
            let at = I::point(c.domain[0])
                .add(dt)?
                .intersect(c.domain[0], c.domain[1])?;
            traversal[0] = traversal[0].min(at.lo);
            traversal[1] = traversal[1].max(at.hi);
            let d = iv::scale(c.tangent, self.total)?;
            let dd = iv::scale(c.tangent_rate, self.total.mul(self.total)?.div(c.speed)?)?;
            let jets = [d, dd];
            position = Some(if let Some(mut old) = position {
                for a in 0..2 {
                    for k in 0..3 {
                        old[a][k] = I::new(
                            old[a][k].lo.min(jets[a][k].lo),
                            old[a][k].hi.max(jets[a][k].hi),
                        )?;
                    }
                }
                old
            } else {
                jets
            });
            speed = Some(if let Some(old) = speed {
                I::new(old.lo.min(c.speed.lo), old.hi.max(c.speed.hi))?
            } else {
                c.speed
            });
            rate = Some(if let Some(old) = rate {
                I::new(old.lo.min(c.speed_rate.lo), old.hi.max(c.speed_rate.hi))?
            } else {
                c.speed_rate
            });
        }
        let (Some(speed), Some(rate)) = (speed, rate) else {
            return Ok(None);
        };
        if station == [0., 0.] {
            traversal = [0., 0.];
        } else if station == [1., 1.] {
            traversal = [1., 1.];
        }
        let mut first = self.total.div(speed)?;
        let mut second = I::point(0.).sub(
            self.total
                .mul(self.total)?
                .mul(rate)?
                .div(speed.mul(speed)?.mul(speed)?)?,
        )?;
        if traversal[0] < traversal[1] {
            // Narrowed inverse parameters must get their OWN original jets;
            // keeping full-cell tangents/rates leaves a permanent derivative
            // uncertainty floor even as station boxes are subdivided.
            let fresh =
                vector_certificate::certify_traversal(&self.path, traversal, max - *spent, false)?;
            *spent += fresh.cells;
            if fresh.status != Status::Certified {
                return Ok(None);
            }
            let [a, b] = self.path.domain();
            let width = I::point(b).sub(I::point(a))?;
            let v = iv::scale(decode(fresh.first.unwrap())?, width)?;
            let acc = iv::scale(decode(fresh.second.unwrap())?, width.mul(width)?)?;
            let w = iv::norm(v)?;
            if w.lo > 0. {
                let wt = iv::dot_tight(v, acc)?.div(w)?;
                let next_first = self.total.div(w)?;
                let next_second = I::point(0.).sub(
                    self.total
                        .mul(self.total)?
                        .mul(wt)?
                        .div(w.mul(w)?.mul(w)?)?,
                )?;
                first = first.intersect(next_first.lo, next_first.hi)?;
                second = second.intersect(next_second.lo, next_second.hi)?;
                let tangent = iv::div(v, w)?;
                let omega = iv::div(iv::cross(v, acc)?, w.mul(w)?)?;
                let tangent_rate = iv::cross(omega, tangent)?;
                let jets = [
                    iv::scale(tangent, self.total)?,
                    iv::scale(tangent_rate, self.total.mul(self.total)?.div(w)?)?,
                ];
                if let Some(old) = &mut position {
                    for a in 0..2 {
                        for k in 0..3 {
                            old[a][k] = old[a][k].intersect(jets[a][k].lo, jets[a][k].hi)?;
                        }
                    }
                }
            }
        }
        Ok(Some(frame::PathPhase {
            traversal,
            first,
            second,
            position_jets: position,
        }))
    }
}

#[cfg(test)]
mod tests;
