//! Full original spatial charts in ONE fixed exact physical reference frame.
//! Whole source rate cells are interval covers, never sampled frame proxies.
use super::*;
struct RateCell {
    domain: [f64; 2],
    omega: f64,
    bend: f64,
    coefficients: affine::FullCoefficients,
    path_velocity: V3,
}
impl RateCell {
    /// Fresh bounds on the complete requested time interval, under one
    /// source-paid primitive budget. No cached prefix is assumed here.
    fn enclose(
        c: &cell::Context<'_, '_>,
        domain: [f64; 2],
        out: &mut IdealVolumeReport,
        max: usize,
        path_aligned: bool,
    ) -> Result<Option<Self>> {
        let stage = max.min(out.cells.saturating_add(100000));
        let Some(phase) = c
            .arc
            .as_ref()
            .unwrap()
            .phase(domain, &mut out.cells, stage)?
        else {
            return Ok(None);
        };
        let invariant = if path_aligned {
            frame_bounds::frenet_rates(c, domain, phase, out, max)?
        } else {
            None
        };
        let (omega, bend, path_velocity) = if let Some((omega, bend)) = invariant {
            (
                omega,
                bend,
                [I::point(0.), I::point(0.), c.arc.as_ref().unwrap().length()],
            )
        } else {
            let Some(jets) = phase.position_jets else {
                return Ok(None);
            };
            let Some(frame) = frame_bounds::jets(c, domain, phase, out, max)? else {
                return Ok(None);
            };
            let omega = frame_bounds::angular(&frame)?;
            let (bend, path_velocity) = if path_aligned {
                (
                    iv::norm(jets[1])?.div(c.arc.as_ref().unwrap().length())?.hi,
                    [I::point(0.), I::point(0.), c.arc.as_ref().unwrap().length()],
                )
            } else {
                // General authored/fixed frames need the COMPLETE R^T P_s.
                // No alignment of their third axis with the path is assumed.
                let mut velocity = [I::point(0.); 3];
                for (k, axis) in [&frame.transverse, &frame.binormal, &frame.longitudinal]
                    .into_iter()
                    .enumerate()
                {
                    velocity[k] = iv::dot_tight(decode(axis.as_ref().unwrap().value)?, jets[0])?;
                }
                (omega, velocity)
            };
            (omega, bend, path_velocity)
        };
        let Some(coefficients) = affine::full_coefficients(c.s, domain, out, max)? else {
            return Ok(None);
        };
        Ok(Some(RateCell {
            domain,
            omega,
            bend,
            coefficients,
            path_velocity,
        }))
    }
}
// Every spatial chart uses the same full UV carrier in the same initial basis.
struct Carrier {
    value: V3,
    derivatives: [V3; 2],
}
impl Carrier {
    fn enclose(
        c: &cell::Context<'_, '_>,
        out: &mut IdealVolumeReport,
        max: usize,
    ) -> Result<Option<Self>> {
        let Some(cap) = cap_jets(c.caps[0], [[0., 1.]; 2], &mut out.cells, max)? else {
            return Ok(None);
        };
        Ok(Some(Self {
            value: local(iv::sub(cap[0], c.start)?, c.basis)?,
            derivatives: [local(cap[1], c.basis)?, local(cap[2], c.basis)?],
        }))
    }
}
pub(super) struct Cover {
    carrier: Carrier,
    cells: Vec<RateCell>,
    // A cache may cover only a prefix after its bounded build trial stops.
    // This is NEVER a whole-field/global certificate. Every lookup must
    // contain its complete requested domain within this verified prefix.
    end: f64,
}
impl Cover {
    fn push(&mut self, cell: RateCell) -> bool {
        if !cell.domain.iter().all(|x| x.is_finite())
            || cell.domain[0] != self.end
            || cell.domain[1] <= self.end
            || cell.domain[1] > 1.
        {
            return false;
        }
        self.end = cell.domain[1];
        self.cells.push(cell);
        true
    }
    pub(super) fn covers(&self, pieces: &[Box3]) -> bool {
        !self.cells.is_empty()
            && pieces
                .iter()
                .all(|b| 0. <= b[2][0] && b[2][0] <= b[2][1] && b[2][1] <= self.end)
    }
    fn available(self) -> Option<Self> {
        if self.cells.is_empty() {
            None
        } else {
            Some(self)
        }
    }
    pub(super) fn build(
        c: &cell::Context<'_, '_>,
        out: &mut IdealVolumeReport,
        max: usize,
        path_aligned: bool,
    ) -> Result<Option<Self>> {
        let mut pending = (0..256)
            .rev()
            .map(|i| [i as f64 / 256., (i + 1) as f64 / 256.])
            .collect::<Vec<_>>();
        let Some(carrier) = Carrier::enclose(c, out, max)? else {
            return Ok(None);
        };
        let mut cover = Self {
            carrier,
            cells: Vec::new(),
            end: 0.,
        };
        while let Some(domain) = pending.pop() {
            if out.cells == max {
                return Ok(cover.available());
            }
            out.cells += 1;
            let source = RateCell::enclose(c, domain, out, max, path_aligned)?;
            let width = I::point(domain[1]).sub(I::point(domain[0]))?;
            // Subdivision proposal only: each accepted cell still encloses
            // the complete original field, never a sampled frame value.
            let refine = match source.as_ref() {
                None => true,
                Some(s) => I::point(s.omega).mul(width)?.hi > 1. / 16.,
            };
            if refine {
                let m = crate::distance_bounds::midpoint(domain[0], domain[1]);
                if !(domain[0] < m && m < domain[1]) {
                    return Ok(cover.available());
                }
                pending.extend([[m, domain[1]], [domain[0], m]]);
            } else {
                if !cover.push(source.unwrap()) {
                    return Ok(None);
                }
            }
        }
        Ok(cover.available())
    }
}
/// In a fixed exact frame R_ref, actual columns are U*C, ||U-I||<=angle.
/// Outward column balls enclose EVERY such matrix, including every average.
fn rotating_chart(mut columns: [V3; 3], angle: f64) -> Result<bool> {
    if !angle.is_finite() || angle < 0. {
        return Ok(false);
    }
    for column in &mut columns {
        let error = I::point(angle).mul(I::point(iv::norm(*column)?.hi))?.hi;
        for value in column {
            *value = value.add(I::new(-error, error)?)?;
        }
    }
    super::super::atlas::regular(columns, [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]])
}
pub(super) fn certify(
    tube: &Tube,
    c: &cell::Context<'_, '_>,
    pieces: &[Box3],
    out: &mut IdealVolumeReport,
    max: usize,
) -> Result<bool> {
    let connected = match pieces {
        [_] => true,
        [tail, head] => tube.closed && tail[2][1] == 1. && head[2][0] == 0.,
        _ => false,
    };
    if !connected || !c.same_cap() || out.cells == max {
        return Ok(false);
    }
    let Some(cover) = &tube.spatial_cover else {
        return Ok(false);
    };
    out.cells += 1;
    let q = cover.carrier.value;
    let derivatives = cover.carrier.derivatives;
    let mut columns: Option<[V3; 3]> = None;
    let mut angle = I::point(0.);
    for piece in pieces {
        // An incomplete cache supplies NO credit outside its verified
        // prefix. Enclose the ENTIRE requested piece freshly instead.
        let fresh = if cover.covers(std::slice::from_ref(piece)) {
            None
        } else {
            if out.cells == max {
                return Ok(false);
            }
            out.cells += 1;
            let stage = max.min(out.cells.saturating_add(100000));
            let Some(rate) = RateCell::enclose(c, piece[2], out, stage, tube.path_aligned)? else {
                return Ok(false);
            };
            Some(rate)
        };
        let rates = if let Some(rate) = &fresh {
            std::slice::from_ref(rate)
        } else {
            let charge = 2 * (usize::BITS - cover.cells.len().leading_zeros()) as usize;
            if charge > max - out.cells {
                return Ok(false);
            }
            out.cells += charge;
            let lo = cover.cells.partition_point(|s| s.domain[1] < piece[2][0]);
            let hi = cover.cells.partition_point(|s| s.domain[0] <= piece[2][1]);
            &cover.cells[lo..hi]
        };
        if rates.is_empty() {
            return Ok(false);
        }
        for rate in rates {
            if out.cells == max {
                return Ok(false);
            }
            out.cells += 1;
            let overlap = [
                rate.domain[0].max(piece[2][0]),
                rate.domain[1].min(piece[2][1]),
            ];
            angle = angle
                .add(I::point(rate.omega).mul(I::point(overlap[1]).sub(I::point(overlap[0]))?)?)?;
            let coeff = &rate.coefficients;
            let mut value = q;
            let mut velocity = q;
            for k in 0..3 {
                value[k] = coeff.value[k].mul(q[k])?.add(coeff.center[k])?;
                velocity[k] = coeff.first[k].mul(q[k])?.add(coeff.center_first[k])?;
            }
            // W=R^T R' is skew. The path-aligned case uses |T_s| for
            // transverse angular components; general frames use full Omega.
            // Every source translation component and axial carrier term remains.
            let spin = I::new(-rate.omega, rate.omega)?;
            let bend = I::new(-rate.bend, rate.bend)?;
            velocity[0] = velocity[0]
                .add(spin.mul(value[1])?)?
                .add(bend.mul(value[2])?)?;
            velocity[1] = velocity[1]
                .add(spin.mul(value[0])?)?
                .add(bend.mul(value[2])?)?;
            // The axial row of the skew matrix has one Euclidean
            // bound, ||R'_3||. Its two entries cannot each attain that
            // bound simultaneously.
            let transverse_radius = iv::norm([value[0], value[1], I::point(0.)])?;
            let axial_error = I::point(rate.bend).mul(transverse_radius)?.hi;
            velocity[2] = velocity[2].add(I::new(-axial_error, axial_error)?)?;
            for k in 0..3 {
                let path = rate.path_velocity[k];
                if path.lo != 0. || path.hi != 0. {
                    velocity[k] = velocity[k].add(path)?;
                }
            }
            let mut current = [[I::point(0.); 3]; 3];
            for j in 0..2 {
                for k in 0..3 {
                    current[j][k] = coeff.value[k].mul(derivatives[j][k])?;
                }
            }
            current[2] = velocity;
            if let Some(old) = &mut columns {
                for j in 0..3 {
                    for k in 0..3 {
                        old[j][k] = I::new(
                            old[j][k].lo.min(current[j][k].lo),
                            old[j][k].hi.max(current[j][k].hi),
                        )?;
                    }
                }
            } else {
                columns = Some(current);
            }
        }
    }
    let Some(columns) = columns else {
        return Ok(false);
    };
    // Use the frame at the midpoint of accumulated angular variation.
    // Such a station exists by continuity of the integral on this lift
    // (also across the periodic join). Both sides have variation at most
    // half the total. The proof only uses coordinates in that exact frame,
    // so neither the station nor the basis needs floating-point evaluation.
    let radius = angle.mul(I::point(0.5))?.hi;
    rotating_chart(columns, radius)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stopped_rate_cache_requires_complete_query_coverage_and_rejects_gaps() {
        let cell = |domain| RateCell {
            domain,
            omega: 0.,
            bend: 0.,
            coefficients: affine::FullCoefficients {
                value: [I::point(1.); 3],
                first: [I::point(0.); 3],
                center_first: [I::point(0.); 3],
                center: [I::point(0.); 3],
            },
            path_velocity: [I::point(0.), I::point(0.), I::point(1.)],
        };
        let box_for = |t| [[0., 1.], [0., 1.], t];
        let mut c = Cover {
            carrier: Carrier {
                value: [I::point(0.); 3],
                derivatives: [[I::point(0.); 3]; 2],
            },
            cells: Vec::new(),
            end: 0.,
        };
        assert!(!c.covers(&[box_for([0., 0.])]));
        assert!(c.push(cell([0., 0.25])));
        assert!(c.covers(&[box_for([0., 0.25])]));
        assert!(!c.covers(&[box_for([0., 1.])]));
        assert!(!c.push(cell([0.5, 1.])));
        assert!(!c.push(cell([0., 0.5])));
        assert_eq!(c.end, 0.25);
        assert!(c.push(cell([0.25, 0.75])));
        assert!(!c.covers(&[box_for([0.5, 1.]), box_for([0., 0.25])]));
        assert!(!c.covers(&[box_for([0.5, 0.5_f64.next_down()])]));
        assert!(c.push(cell([0.75, 1.])));
        assert!(c.covers(&[box_for([0., 1.])]));
    }
    #[test]
    fn common_rotating_metric_checks_averages_and_bounded_frame_error() {
        let identity = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]].map(|r| r.map(I::point));
        assert!(rotating_chart(identity, 0.1).unwrap());
        assert!(!rotating_chart(identity, 1.).unwrap());
        let mut shear = identity;
        shear[2][0] = I::new(-1000., 1000.).unwrap();
        shear[2][1] = shear[2][0];
        assert!(rotating_chart(shear, 0.).unwrap());
        shear[2][2] = I::new(-1., 1.).unwrap();
        assert!(!rotating_chart(shear, 0.).unwrap());
        let a = [[1_f64, -1., 0.], [10., 1., 0.], [0., 0., 1.]];
        let b = [[1_f64, 10., 0.], [-1., 1., 0.], [0., 0., 1.]];
        let family = std::array::from_fn(|j| {
            std::array::from_fn(|k| I::new(a[j][k].min(b[j][k]), a[j][k].max(b[j][k])).unwrap())
        });
        assert!(!rotating_chart(family, 0.).unwrap());
    }
}
