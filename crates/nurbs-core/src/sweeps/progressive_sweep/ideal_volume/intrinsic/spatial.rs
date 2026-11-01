//! Full original spatial charts in ONE fixed exact physical reference frame.
//! Whole source rate cells are interval covers, never sampled frame proxies.
use super::*;
struct RateCell {
    domain: [f64; 2],
    omega: f64,
    bend: f64,
    coefficients: affine::FullCoefficients,
}
pub(super) struct Cover {
    cells: Vec<RateCell>,
}
impl Cover {
    pub(super) fn build(
        c: &cell::Context<'_, '_>,
        out: &mut IdealVolumeReport,
        max: usize,
    ) -> Result<Option<Self>> {
        let mut pending = (0..256)
            .rev()
            .map(|i| [i as f64 / 256., (i + 1) as f64 / 256.])
            .collect::<Vec<_>>();
        let mut cells = Vec::new();
        while let Some(domain) = pending.pop() {
            if out.cells == max {
                return Ok(None);
            }
            out.cells += 1;
            let source = (|| -> Result<Option<RateCell>> {
                let stage = max.min(out.cells.saturating_add(100000));
                let Some(phase) = c
                    .arc
                    .as_ref()
                    .unwrap()
                    .phase(domain, &mut out.cells, stage)?
                else {
                    return Ok(None);
                };
                let Some(jets) = phase.position_jets else {
                    return Ok(None);
                };
                let Some(frame) = frame_bounds::jets(c, domain, phase, out, max)? else {
                    return Ok(None);
                };
                let omega = frame_bounds::angular(&frame)?;
                let bend = iv::norm(jets[1])?.div(c.arc.as_ref().unwrap().length())?.hi;
                let Some(coefficients) = affine::full_coefficients(c.s, domain, out, max)? else {
                    return Ok(None);
                };
                Ok(Some(RateCell {
                    domain,
                    omega,
                    bend,
                    coefficients,
                }))
            })()?;
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
                    return Ok(None);
                }
                pending.extend([[m, domain[1]], [domain[0], m]]);
            } else {
                cells.push(source.unwrap());
            }
        }
        Ok(Some(Self { cells }))
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
    if !connected || !tube.path_aligned || !c.same_cap() || out.cells == max {
        return Ok(false);
    }
    let Some(cover) = &tube.spatial_cover else {
        return Ok(false);
    };
    out.cells += 1;
    let stage = max.min(out.cells.saturating_add(100000));
    let Some(cap) = cap_jets(c.caps[0], [[0., 1.]; 2], &mut out.cells, stage)? else {
        return Ok(false);
    };
    let q = local(iv::sub(cap[0], c.start)?, c.basis)?;
    let derivatives = [local(cap[1], c.basis)?, local(cap[2], c.basis)?];
    let length = c.arc.as_ref().unwrap().length();
    let mut columns: Option<[V3; 3]> = None;
    let mut angle = I::point(0.);
    for piece in pieces {
        let charge = 2 * (usize::BITS - cover.cells.len().leading_zeros()) as usize;
        if charge > max - out.cells {
            return Ok(false);
        }
        out.cells += charge;
        let lo = cover.cells.partition_point(|s| s.domain[1] <= piece[2][0]);
        let hi = cover.cells.partition_point(|s| s.domain[0] < piece[2][1]);
        for rate in &cover.cells[lo..hi] {
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
            // W=R^T R' is skew. Since R e_z=T and P_s=L*T, its two
            // transverse angular components are bounded by |T_s|, independently
            // of potentially fast spin about T. All axial carrier terms remain.
            let spin = I::new(-rate.omega, rate.omega)?;
            let bend = I::new(-rate.bend, rate.bend)?;
            velocity[0] = velocity[0]
                .add(spin.mul(value[1])?)?
                .add(bend.mul(value[2])?)?;
            velocity[1] = velocity[1]
                .add(spin.mul(value[0])?)?
                .add(bend.mul(value[2])?)?;
            velocity[2] = velocity[2]
                .add(bend.mul(value[0])?)?
                .add(bend.mul(value[1])?)?
                .add(length)?;
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
    rotating_chart(columns, angle.hi)
}
#[cfg(test)]
mod tests {
    use super::*;
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
