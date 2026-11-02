//! Ordered same-UV interpolation; independent dense-grid acceptance measurement.
use super::{NormalOffset, Result, refused};
use crate::{offset::surface_offset::OffsetReport, surface::Surface};

fn measure(offset: &NormalOffset, surface: &Surface) -> Result<f64> {
    let domain = offset.domain();
    let mut maximum = 0f64;
    // These 65x65 validation sites are independent of the 5/9/11 fit grids.
    for i in 0..=64 {
        for j in 0..=64 {
            let uv = [
                domain[0][0] + (domain[0][1] - domain[0][0]) * i as f64 / 64.,
                domain[1][0] + (domain[1][1] - domain[1][0]) * j as f64 / 64.,
            ];
            let actual = offset.evaluate(uv)?.point;
            let fitted = surface.evaluate(uv[0], uv[1])?.point;
            let error = (0..3).fold(0f64, |length, k| length.hypot(actual[k] - fitted[k]));
            if !error.is_finite() {
                return Err(refused("Offset fit residual overflow"));
            }
            maximum = maximum.max(error);
        }
    }
    Ok(maximum.next_up())
}
pub(super) fn approximate(offset: &NormalOffset, tolerance: f64) -> Result<OffsetReport> {
    if !tolerance.is_finite() || tolerance <= 0. {
        return Err(refused("Offset fit needs a positive finite tolerance"));
    }
    if offset.distance() == 0. {
        return Ok(OffsetReport {
            surface: offset.source().clone(),
            max_deviation: 0.,
            degenerate_regions: vec![],
            exceeds_fold_free_bound: false,
            exact: true,
        });
    }
    let domain = offset.domain();
    let mut best = None::<(Surface, f64)>;
    for count in [5, 9, 11] {
        let parameters: [Vec<f64>; 2] = std::array::from_fn(|axis| {
            (0..count)
                .map(|i| {
                    domain[axis][0]
                        + (domain[axis][1] - domain[axis][0]) * i as f64 / (count - 1) as f64
                })
                .collect()
        });
        let points = parameters[0]
            .iter()
            .map(|&u| {
                parameters[1]
                    .iter()
                    .map(|&v| offset.evaluate([u, v]).map(|e| e.point))
                    .collect::<Result<Vec<_>>>()
            })
            .collect::<Result<Vec<_>>>()?;
        let mut surface = crate::grid_spline::interpolate(&points, &parameters[0], &parameters[1])?;
        for k in &mut surface.knots_u {
            *k = domain[0][0] + (domain[0][1] - domain[0][0]) * *k;
        }
        for k in &mut surface.knots_v {
            *k = domain[1][0] + (domain[1][1] - domain[1][0]) * *k;
        }
        surface.validate()?;
        let measured = measure(offset, &surface)?;
        if best.as_ref().is_none_or(|(_, old)| measured < *old) {
            best = Some((surface, measured));
        }
        if measured <= tolerance {
            break;
        }
    }
    let (surface, max_deviation) = best.ok_or_else(|| refused("No offset fit candidate"))?;
    let validity = crate::surface_offset::offset_validity(offset.source(), offset.distance())?;
    Ok(OffsetReport {
        surface,
        max_deviation,
        degenerate_regions: vec![],
        exceeds_fold_free_bound: !validity.requested_ok,
        exact: false,
    })
}
