//! Strict subperiod charts use complete original UV pole-hull enclosures.
//! Full-period quotients use the literal outer rectangle; complete independent
//! hole winding/containment checks remain mandatory in final qualification.
use super::*;
mod full;
use num_rational::BigRational as Q;
use num_traits::ToPrimitive;
fn q(x: f64) -> Q {
    Q::from_float(x).unwrap()
}
fn outward(x: Q, upper: bool) -> Result<f64> {
    let mut v = x
        .to_f64()
        .ok_or_else(|| error("Periodic chart endpoint overflow"))?;
    if !v.is_finite() {
        return Err(error("Periodic chart endpoint overflow"));
    }
    if upper && q(v) < x {
        v = v.next_up();
    }
    if !upper && q(v) > x {
        v = v.next_down();
    }
    Ok(v)
}
pub(crate) fn axis(bounds: [f64; 2], natural: [f64; 2], periodic: bool) -> Result<Vec<[f64; 2]>> {
    if !periodic {
        if bounds[0] < natural[0] || bounds[1] > natural[1] || bounds[0] >= bounds[1] {
            return Err(error("Face UV enclosure leaves nonperiodic chart"));
        }
        return Ok(vec![bounds]);
    }
    let period = q(natural[1]) - q(natural[0]);
    if q(bounds[1]) - q(bounds[0]) >= period || bounds[0] >= bounds[1] {
        return Err(error(
            "Face UV enclosure must cover less than one exact period",
        ));
    }
    let lift = ((q(bounds[0]) - q(natural[0])) / &period)
        .floor()
        .to_integer();
    let k = lift
        .to_i64()
        .filter(|k| (-1000000..=1000000).contains(k))
        .ok_or_else(|| error("Face chart lift exceeds bounded original atlas"))?;
    let offset = period.clone() * Q::from_integer(k.into());
    let lo = q(bounds[0]) - &offset;
    let hi = q(bounds[1]) - &offset;
    let start = outward(lo, false)?;
    if hi <= q(natural[1]) {
        return Ok(vec![[start, outward(hi, true)?]]);
    }
    let end = outward(hi - period, true)?;
    if end >= start {
        return Err(error(
            "Outward periodic chart enclosure overlaps a complete lap",
        ));
    }
    Ok(vec![[start, natural[1]], [natural[0], end]])
}
fn charge(remaining: &mut usize, n: usize) -> Result<()> {
    if n > *remaining {
        return Err(error("Periodic face chart proof work exhausted"));
    }
    *remaining -= n;
    Ok(())
}
pub(crate) fn verify(
    model: &Model,
    face: usize,
    remaining: &mut usize,
    pairs: &mut usize,
    max_pairs: usize,
) -> Result<bool> {
    verify_impl(model, face, remaining, pairs, max_pairs, false)
}

/// Called after exact original lifts and active material loops have passed.
pub(crate) fn verify_active(
    model: &Model,
    face: usize,
    remaining: &mut usize,
    pairs: &mut usize,
    max_pairs: usize,
) -> Result<bool> {
    verify_impl(model, face, remaining, pairs, max_pairs, true)
}

fn verify_impl(
    model: &Model,
    face: usize,
    remaining: &mut usize,
    pairs: &mut usize,
    max_pairs: usize,
    active: bool,
) -> Result<bool> {
    let f = &model.faces[face];
    let s = &f.surface;
    let natural = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    let mut bounds = [[f64::INFINITY, f64::NEG_INFINITY]; 2];
    let mut outer_bounds = bounds;
    let mut outer_segments = Vec::new();
    for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
        for c in &model.loops[wire].coedges {
            let endpoints = if active
                && c.trim()
                    .is_some_and(|t| t.pcurve_parameter != c.pcurve.domain())
            {
                let mut used = 0;
                let endpoints = crate::shared_boundary::chart_segments::natural_segment(
                    c, &mut used, *remaining,
                );
                charge(remaining, used)?;
                Some(endpoints.ok_or_else(|| error("Active periodic UV side is unproved"))?)
            } else {
                None
            };
            if wire == f.outer {
                outer_segments.push(endpoints);
            }
            charge(
                remaining,
                endpoints
                    .as_ref()
                    .map_or(c.pcurve.control_points.len(), |_| 2),
            )?;
            let points = endpoints.as_ref().map(|p| p.iter().map(|p| &p[..]));
            for p in points.into_iter().flatten().chain(
                c.pcurve
                    .control_points
                    .iter()
                    .filter(|_| endpoints.is_none())
                    .map(Vec::as_slice),
            ) {
                for k in 0..2 {
                    bounds[k][0] = bounds[k][0].min(p[k]);
                    bounds[k][1] = bounds[k][1].max(p[k]);
                    if wire == f.outer {
                        outer_bounds[k][0] = outer_bounds[k][0].min(p[k]);
                        outer_bounds[k][1] = outer_bounds[k][1].max(p[k]);
                    }
                }
            }
        }
    }
    for k in 0..2 {
        if [s.periodic_u, s.periodic_v][k] {
            charge(
                remaining,
                s.control_points.len() * s.control_points[0].len(),
            )?;
            if nurbs_core::periodic_trim::verify_surface_seam_identity(s, k)?.is_none() {
                return Err(error(
                    "Actual source periodic seam identity remains unproven",
                ));
            }
        }
    }
    // A proved global quotient remains injective after removing contained holes.
    // Hole pole hulls need not bound the outer material rectangle; the original
    // complete hole curves must still pass the later trim-region audit.
    if let Some(regular) = full::verify(
        model,
        face,
        outer_bounds,
        &outer_segments,
        natural,
        remaining,
        pairs,
        max_pairs,
    )? {
        return Ok(regular);
    }
    let axes = [
        axis(bounds[0], natural[0], s.periodic_u)?,
        axis(bounds[1], natural[1], s.periodic_v)?,
    ];
    if *remaining == 0 {
        return Err(error("Periodic face chart proof work exhausted"));
    }
    let split = axes.each_ref().map(|pieces| pieces.len() == 2);
    let r = nurbs_core::surface_injectivity::certify_seam_glued_rectangle(s, axes, *remaining, 4)?;
    charge(remaining, r.spans)?;
    if !r.proven && *remaining > 0 && *pairs < max_pairs {
        // A split chart may have no common planar projection. Restricting
        // a globally embedded periodic carrier still proves injectivity.
        for k in 0..2 {
            if !split[k] || [s.periodic_u, s.periodic_v][1 - k] {
                continue;
            }
            let proof = nurbs_core::surface_injectivity::certify_periodic_extrusion(
                s,
                k,
                max_pairs - *pairs,
                *remaining,
            )?;
            charge(remaining, proof.cells)?;
            *pairs += proof.pairs;
            if proof.proven
                && proof.regular_interior
                && proof
                    .collapsed_end
                    .is_none_or(|end| bounds[1 - k][end] != natural[1 - k][end])
            {
                return Ok(true);
            }
        }
    }
    if !r.proven {
        return Err(error(format!(
            "Complete periodic face enclosure injectivity remains unproven: {}",
            r.reason
        )));
    }
    // The common inverse-Jacobian contraction also proves rank two on the
    // complete actual enclosure, even when unused carrier boundaries collapse.
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_laps_and_unbounded_sheet_indices_cannot_gain_local_chart_authority() {
        assert!(axis([0., 5.], [0., 5.], true).is_err());
        assert!(axis([-0.1, 5.], [0., 5.], true).is_err());
        assert!(axis([-0.1, 0.5], [0., 1.], false).is_err());
        assert!(axis([1e6, 1e6 + 0.25], [0., 0.5], true).is_err());
        assert!(axis([-1., (-1f64).next_up()], [0., 2f64.powi(-63)], true).is_err());
    }
    #[test]
    fn opposite_shores_use_original_endpoints_and_preserve_negative_sheet_coverage() {
        assert_eq!(
            axis([4., 6.], [0., 5.], true).unwrap(),
            vec![[4., 5.], [0., 1.]]
        );
        assert_eq!(
            axis([-11., -9.], [0., 5.], true).unwrap(),
            vec![[4., 5.], [0., 1.]]
        );
        assert_eq!(axis([9., 10.], [0., 5.], true).unwrap(), vec![[4., 5.]]);
    }
}
