//! surface self chord algorithm: rational_pole_pair_report_cached.
use super::*;
pub fn rational_pole_pair_report_cached(
    s: &Surface,
    pair: [[[f64; 2]; 2]; 2],
    differential: &crate::rational_pole_differential::Certificate<'_>,
    normals: &crate::factored_surface_normals::Certificate<'_>,
    sine: f64,
    max_spans: usize,
    cached: Option<&crate::factored_surface_normals::PairBounds<'_>>,
) -> Result<PairExclusion> {
    check(
        std::ptr::eq(s, differential.surface())
            && std::ptr::eq(s, normals.surface())
            && differential.end() == normals.end(),
        "Use original pole facts for this pair",
    )?;
    check(
        sine.is_finite() && (0. ..1.).contains(&sine),
        "Valid pole pair sine threshold",
    )?;
    let components = cached.map(|cache| cache.components(normals, pair)).transpose()?;
    if max_spans < 5 {
        return Ok(PairExclusion::default());
    }
    let union = std::array::from_fn(|a| {
        [
            pair[0][a][0].min(pair[1][a][0]),
            pair[0][a][1].max(pair[1][a][1]),
        ]
    });
    let threshold = I::point(1.).sub(I::point(sine))?.lo;
    let mut used = 0;
    let mut attempted = false;
    let mut available = false;
    let mut local_attempted = false;
    let mut local_available = false;
    // Scoped to this invocation: source, pole orientation and rectangles have
    // already been validated and remain immutable throughout all three modes.
    // Charge a restriction only when its endpoint bound is first computed.
    let mut endpoint_bounds = components.map(|values| values.map(Some)).unwrap_or([None; 2]);
    // Start with the tightest independently confined chart; retain wider
    // alternatives when the endpoint path cannot be confined there.
    // Small budgets retain the original broad-chart route, which may be
    // the only confined path when there is no budget for alternatives.
    let modes = if max_spans >= 25 { [0, 4, 3, 1, 2] } else { [0, 1, 3, 4, 2] };
    for mode in modes {
        let local_path = matches!(mode, 1 | 3 | 4);
        let cost = if local_path { 6 } else { 5 };
        let coordinate_path = mode == 0;
        if max_spans - used < cost {
            break;
        }
        if coordinate_path {
            attempted = true;
        }
        if local_path {
            local_attempted = true;
        }
        let (matrix, spent) = match mode {
            0 => differential.coordinate_path_report(union)?,
            1 => differential.local_coordinate_path_report(union, 3.)?,
            3 => differential.local_coordinate_path_report(union, 1.)?,
            4 => differential.local_coordinate_path_report(union, 0.25)?,
            _ => differential.bounds_report(union)?,
        };
        used += spent;
        if local_path && matrix.is_some() {
            local_available = true;
        }
        if coordinate_path && matrix.is_some() {
            available = true;
        }
        let Some(mut j) = matrix else {
            continue;
        };
        for a in 0..2 {
            let scale = j
                .iter()
                .map(|r| r[a].lo.abs().max(r[a].hi.abs()))
                .fold(0_f64, f64::max);
            if scale >= f64::MIN_POSITIVE && scale.is_finite() {
                for row in &mut j {
                    row[a] = row[a].div(I::point(scale))?;
                }
            }
        }
        if mean_matrix_lower(j)?<=0. {continue;}
        for (index, endpoint) in pair.into_iter().enumerate() {
            let b = if let Some(bounds) = endpoint_bounds[index] {
                bounds
            } else {
                if used == max_spans {
                    break;
                }
                let bounds = normals.bounds(endpoint)?;
                used += 1;
                endpoint_bounds[index] = Some(bounds);
                bounds
            };
            let scale = b.iter().flatten().map(|x| x.abs()).fold(0_f64, f64::max);
            if scale < f64::MIN_POSITIVE {
                continue;
            }
            let mut n = [I::point(0.); 3];
            for k in 0..3 {
                n[k] = I::new(b[k][0], b[k][1])?.div(I::point(scale))?;
            }
            if endpoint_matrix_cosine(j, n)? < threshold {
                return Ok(PairExclusion {
                    excluded: true,
                    spans: used,
                    coordinate_path_excluded: coordinate_path,
                    coordinate_path_attempted: attempted,
                    coordinate_path_available: available,
                    local_path_excluded: local_path,
                    local_path_margin: match mode { 1 => Some(3.), 3 => Some(1.),
                        4 => Some(0.25), _ => None },
                    local_path_attempted: local_attempted,
                    local_path_available: local_available,
                });
            }
        }
    }
    Ok(PairExclusion {
        excluded: false,
        spans: used,
        coordinate_path_excluded: false,
        coordinate_path_attempted: attempted,
        coordinate_path_available: available,
        local_path_excluded: false,
        local_path_margin: None,
        local_path_attempted: local_attempted,
        local_path_available: local_available,
    })
}
