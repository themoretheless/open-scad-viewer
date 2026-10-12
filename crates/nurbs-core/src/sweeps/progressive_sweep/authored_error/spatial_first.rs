//! Retained linear wall error from joint original first images.
use super::*;

/// Original position and unit tangent are continuous by the transport's exact
/// premises. On each regular source span Bishop's ODE supplies the derivative.
/// A first-defect integral remains valid across source speed/curvature jumps;
/// it never promotes those joins to retained C1/G1/G2.
pub(super) fn section_interpolation(
    sweep: &Sweep<'_>,
    count: usize,
    transport: &super::super::rmf_transport::OriginalRmfTransportReport,
    qs: &[[[f64; 2]; 3]],
    sections: &[Curve],
    max_cells: usize,
    mut out: SectionInterpolationReport,
    mut shared_pose: Option<&mut SharedArcPose>,
) -> Result<SectionInterpolationReport> {
    let (work, owner) = super::super::rmf_first_images::Owner::new(sweep, max_cells - out.cells)?;
    out.cells += work;
    let Some(owner) = owner else {
        return Ok(out);
    };
    let station = |i: usize| -> Result<[f64; 2]> {
        let q = I::point(i as f64).div(I::point((count - 1) as f64))?;
        Ok([q.lo.max(0.), q.hi.min(1.)])
    };
    let mut displacements = vec![vec![0.; qs.len()]; count];
    let mut endpoint = 0_f64;
    for i in 0..count {
        let at = station(i)?;
        let image = if let Some(pose) = shared_pose.as_deref_mut() {
            pose.rmf_first_images(at, qs, |basis| {
                owner.query(transport, basis, at, max_cells - out.cells)
            })?
        } else {
            owner.query(transport, qs, at, max_cells - out.cells)?
        };
        out.cells += image.cells;
        let Some(images) = image.images else {
            out.reason = Some("spatial-rmf-joint-endpoint-unproved");
            return Ok(out);
        };
        for (j, image) in images.into_iter().enumerate() {
            let retained = std::array::from_fn(|k| I::point(sections[i].control_points[j][k]));
            let upper = crate::numerics::interval_vec3::norm(crate::numerics::interval_vec3::sub(
                image.value,
                retained,
            )?)?
            .hi;
            displacements[i][j] = upper;
            endpoint = endpoint.max(upper);
        }
    }
    let mut segment_bounds = vec![endpoint; count - 1];
    let mut refinable = vec![false; count - 1];
    for i in 0..count - 1 {
        let interval = [station(i)?[0], station(i + 1)?[1]];
        let image = if let Some(pose) = shared_pose.as_deref_mut() {
            pose.rmf_first_images(interval, qs, |basis| {
                owner.query(transport, basis, interval, max_cells - out.cells)
            })?
        } else {
            owner.query(transport, qs, interval, max_cells - out.cells)?
        };
        out.cells += image.cells;
        let Some(images) = image.images else {
            out.reason = Some("spatial-rmf-joint-first-image-unproved");
            return Ok(out);
        };
        refinable[i] = image.continuous_laws;
        let width = I::point(1.).div(I::point((count - 1) as f64))?;
        for (j, jet) in images.into_iter().enumerate() {
            let mut delta = [I::point(0.); 3];
            for k in 0..3 {
                let a = sections[i].control_points[j][k];
                let b = sections[i + 1].control_points[j][k];
                delta[k] = if image.continuous_laws {
                    jet.first[k].sub(I::point(b).sub(I::point(a))?.div(width)?)?
                } else {
                    jet.value[k].sub(I::new(a.min(b), a.max(b))?)?
                };
            }
            let magnitude = crate::numerics::interval_vec3::norm(delta)?;
            let upper = if image.continuous_laws {
                magnitude
                    .mul(width)?
                    .div(I::point(2.))?
                    .add(I::point(displacements[i][j].max(displacements[i + 1][j])))?
                    .hi
            } else {
                magnitude.hi
            };
            segment_bounds[i] = segment_bounds[i].max(upper);
        }
    }
    // Complete the coarse cover before spending optional refinement work.
    // Integrating separately restricted first-defect images reduces interval
    // dependency near source knots. From the nearer retained endpoint the
    // displacement is at most half the whole defect integral plus endpoint
    // uncertainty; no interior retained/source equality is presumed.
    let mut refinement_order = (0..count - 1).collect::<Vec<_>>();
    refinement_order.sort_by(|&a, &b| segment_bounds[b].total_cmp(&segment_bounds[a]));
    for i in refinement_order {
        if !refinable[i] || segment_bounds[i] <= sweep.options.max_deviation {
            continue;
        }
        let width = I::point(1.).div(I::point((count - 1) as f64))?;
        let mut integral = vec![I::point(0.); qs.len()];
        let mut complete = true;
        for part in 0..4 {
            let at = |j: usize| -> Result<I> {
                I::point((i * 4 + j) as f64).div(I::point(((count - 1) * 4) as f64))
            };
            let a = at(part)?;
            let b = at(part + 1)?;
            let interval = [a.lo.max(0.), b.hi.min(1.)];
            let image = if let Some(pose) = shared_pose.as_deref_mut() {
                pose.rmf_first_images(interval, qs, |basis| {
                    owner.query(transport, basis, interval, max_cells - out.cells)
                })?
            } else {
                owner.query(transport, qs, interval, max_cells - out.cells)?
            };
            out.cells += image.cells;
            let Some(images) = image.images.filter(|_| image.continuous_laws) else {
                complete = false;
                break;
            };
            for (j, image) in images.into_iter().enumerate() {
                let mut defect = [I::point(0.); 3];
                for k in 0..3 {
                    let slope = I::point(sections[i + 1].control_points[j][k])
                        .sub(I::point(sections[i].control_points[j][k]))?
                        .div(width)?;
                    defect[k] = image.first[k].sub(slope)?;
                }
                integral[j] = integral[j].add(
                    crate::numerics::interval_vec3::norm(defect)?.mul(width.div(I::point(4.))?)?,
                )?;
            }
        }
        if complete {
            let refined = integral
                .into_iter()
                .enumerate()
                .map(|(j, total)| {
                    total
                        .div(I::point(2.))?
                        .add(I::point(displacements[i][j].max(displacements[i + 1][j])))
                        .map(|x| x.hi)
                })
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .fold(endpoint, f64::max);
            segment_bounds[i] = segment_bounds[i].min(refined);
        }
    }
    let error = segment_bounds.into_iter().fold(endpoint, f64::max);
    out.status = Status::Certified;
    out.error_upper = Some(error);
    out.endpoint_displacement_upper = Some(endpoint);
    out.cap_endpoint_displacement_upper = Some([
        displacements[0].iter().copied().fold(0., f64::max),
        displacements[count - 1].iter().copied().fold(0., f64::max),
    ]);
    out.reason = None;
    Ok(out)
}
