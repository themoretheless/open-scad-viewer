//! Exact common partition of a natural edge on two clamped source charts.
use super::*;
use num_rational::BigRational;
use num_traits::ToPrimitive;
use nurbs_core::{
    analysis::surface_continuity, editing::exact_surface_knot_insertion::tile_tensor_surface,
};

struct Layout {
    along_domain: [f64; 2],
    cuts: [Vec<f64>; 2],
    fixed: usize,
    high: bool,
}

fn common_partition(
    domains: [[f64; 2]; 2],
    cuts: [&[f64]; 2],
    work: u64,
) -> (Option<[Vec<f64>; 2]>, u64) {
    if domains[0] == domains[1] {
        let mut partition = [cuts[0], cuts[1]].concat();
        partition.sort_by(f64::total_cmp);
        partition.dedup();
        return (Some([partition.clone(), partition]), 0);
    }
    // Each axis has at most 33 controls. Reserve work for rational
    // normalization, sorting, and the two exact pullbacks before starting.
    let count = (cuts[0].len() + cuts[1].len()) as u64;
    let cost = 8 + count * (count + 8);
    if cost > work {
        return (None, 0);
    }
    let map = || {
        let clock = |[start, end]: [f64; 2]| {
            let origin = BigRational::from_float(start)?;
            let width = BigRational::from_float(end)? - &origin;
            (start < end).then_some((origin, width))
        };
        let clocks = [clock(domains[0])?, clock(domains[1])?];
        let mut partition = Vec::with_capacity(count as usize);
        for ((origin, width), knots) in clocks.iter().zip(cuts) {
            for &knot in knots {
                partition.push((BigRational::from_float(knot)? - origin) / width);
            }
        }
        partition.sort_unstable();
        partition.dedup();
        let pullback = |(origin, width): &(BigRational, BigRational)| {
            partition
                .iter()
                .map(|fraction| {
                    let exact = origin + width * fraction;
                    let knot = exact.to_f64()?;
                    (BigRational::from_float(knot).as_ref() == Some(&exact)).then_some(knot)
                })
                .collect::<Option<Vec<_>>>()
        };
        Some([pullback(&clocks[0])?, pullback(&clocks[1])?])
    };
    (map(), cost)
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
    // Exact reflection, if needed, has already been applied by the BRep owner.
    let (partitions, mut used) = common_partition(
        [a.along_domain, b.along_domain],
        [&a.cuts[1 - a.fixed], &b.cuts[1 - b.fixed]],
        work,
    );
    let Some(partitions) = partitions else {
        return Ok((false, used));
    };
    if partitions[0].is_empty() && a.cuts[a.fixed].is_empty() && b.cuts[b.fixed].is_empty() {
        return Ok((false, used));
    }
    let mut sides = Vec::with_capacity(2);
    for ((surface, layout), partition) in surfaces.into_iter().zip([a, b]).zip(partitions) {
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
    if left.len() != right.len() || left.is_empty() {
        return Ok((false, used));
    }
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

#[cfg(test)]
mod tests {
    use super::common_partition;

    #[test]
    fn pulls_back_exact_fractions_without_rounding_authored_knots() {
        let domains = [[0., 3.], [10., 16.]];
        let (partition, used) = common_partition(domains, [&[1.], &[14.]], 1_000);
        assert_eq!(partition, Some([vec![1., 2.], vec![12., 14.]]));
        assert_eq!(
            common_partition(domains, [&[1.], &[14.]], used - 1),
            (None, 0)
        );
        let (partition, used) = common_partition([[0., 3.], [10., 11.]], [&[1.], &[]], 1_000);
        assert!(partition.is_none() && used > 0);
    }
}
