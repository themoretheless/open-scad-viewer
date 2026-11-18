//! Exact common partition of a natural edge on two clamped source charts.
use super::*;
use nurbs_core::{
    analysis::surface_continuity, editing::exact_surface_knot_insertion::tile_tensor_surface,
};

struct Layout {
    along_domain: [f64; 2],
    cuts: [Vec<f64>; 2],
    fixed: usize,
    high: bool,
}

fn layout(surface: &Surface, boundary: &str) -> Option<Layout> {
    if surface.periodic_u || surface.periodic_v {
        return None;
    }
    let axes = [
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
    ];
    let mut cuts = [Vec::new(), Vec::new()];
    for (axis, (knots, degree, count)) in axes.iter().copied().enumerate() {
        if !(1..=8).contains(&degree)
            || count > 33
            || knots[..=degree].iter().any(|&k| k != knots[degree])
            || knots[count..].iter().any(|&k| k != knots[count])
        {
            return None;
        }
        for block in knots.chunk_by(|a, b| a == b) {
            if knots[degree] < block[0] && block[0] < knots[count] {
                if block.len() > degree {
                    return None;
                }
                cuts[axis].push(block[0]);
            }
        }
    }
    let fixed = usize::from(boundary.starts_with('v'));
    let (knots, degree, count) = axes[1 - fixed];
    Some(Layout {
        along_domain: [knots[degree], knots[count]],
        cuts,
        fixed,
        high: boundary.ends_with("Max"),
    })
}

fn edge_tiles(
    surface: &Surface,
    mut layout: Layout,
    partition: &[f64],
    work: u64,
) -> Result<(Option<Vec<Surface>>, u64)> {
    layout.cuts[1 - layout.fixed] = partition.to_vec();
    let [u, v] = &layout.cuts;
    let budget = work.min(cad_predicates::MAX_WORK);
    if budget == 0 || (u.len() + 1) * (v.len() + 1) > 4096 {
        return Ok((None, 0));
    }
    if u.is_empty() && v.is_empty() {
        let cost = (surface.control_points.len() * surface.control_points[0].len()) as u64;
        return Ok(if cost <= budget {
            (Some(vec![surface.clone()]), cost)
        } else {
            (None, 0)
        });
    }
    let tiling = tile_tensor_surface(surface, u, v, budget)?;
    let Some(patches) = tiling.patches else {
        return Ok((None, tiling.work_used));
    };
    let columns = tiling.columns;
    let rows = tiling.rows;
    let tiles = patches
        .into_iter()
        .enumerate()
        .filter_map(|(i, patch)| {
            let along_side = if layout.fixed == 0 {
                i % columns == (if layout.high { columns - 1 } else { 0 })
            } else {
                i / columns == (if layout.high { rows - 1 } else { 0 })
            };
            along_side.then_some(patch)
        })
        .collect();
    Ok((Some(tiles), tiling.work_used))
}

pub(super) fn inspect(
    surfaces: [&Surface; 2],
    boundaries: [&str; 2],
    order: usize,
    work: u64,
) -> Result<(bool, u64)> {
    let [Some(a), Some(b)] = std::array::from_fn(|i| layout(surfaces[i], boundaries[i])) else {
        return Ok((false, 0));
    };
    // For now both authored clocks must already have the same active interval.
    // Exact reflection, if needed, has already been applied by the BRep owner.
    if a.along_domain != b.along_domain {
        return Ok((false, 0));
    }
    let mut partition = a.cuts[1 - a.fixed].clone();
    partition.extend_from_slice(&b.cuts[1 - b.fixed]);
    partition.sort_by(f64::total_cmp);
    partition.dedup();
    if partition.len() > 254
        || (partition.is_empty() && a.cuts[a.fixed].is_empty() && b.cuts[b.fixed].is_empty())
    {
        return Ok((false, 0));
    }
    let mut used = 0;
    let mut sides = Vec::with_capacity(2);
    for (surface, layout) in surfaces.into_iter().zip([a, b]) {
        // The original chart must remain smooth where two edge subspans
        // meet. Per-piece jets alone would miss a crease across a knot.
        let raw = surface_continuity::qualify(surface, order, 4096, work - used)?;
        used += raw.exact_work;
        if raw.certificate.is_none() {
            return Ok((false, used));
        }
        let (tiles, tile_work) = edge_tiles(surface, layout, &partition, work - used)?;
        used += tile_work;
        let Some(tiles) = tiles else {
            return Ok((false, used));
        };
        sides.push(tiles);
    }
    let right = sides.pop().unwrap();
    let left = sides.pop().unwrap();
    assert_eq!(left.len(), right.len());
    for (a, b) in left.into_iter().zip(right) {
        let patches = [a, b];
        let seam = ProjectiveSeam {
            patches: [0, 1],
            boundaries: boundaries.map(str::to_owned),
            order,
            normal_scale: 1.,
        };
        let report = continuity::inspect_projective_seam_collection_with_rational_scale_proposals(
            &patches,
            &[seam],
            (work - used).min(cad_predicates::MAX_WORK),
        )?;
        used += report.exact_work;
        if !report.seams[0].affine_parameter_identity {
            return Ok((false, used));
        }
        if report.certified {
            continue;
        }
        let (accepted, piece_work) =
            super::inspect([&patches[0], &patches[1]], boundaries, order, work - used)?;
        used += piece_work;
        if !accepted {
            return Ok((false, used));
        }
    }
    Ok((true, used))
}
