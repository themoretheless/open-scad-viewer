//! Exact Cartesian splitting from a single globally refined tensor net.
use super::{split::SplitRefusal, *};
use crate::check;

#[derive(Clone, Debug)]
pub struct TensorTiling {
    /// Complete V-major grid, with U varying fastest.
    pub patches: Option<Vec<Surface>>,
    pub columns: usize,
    pub rows: usize,
    pub work_used: u64,
    pub refusal: Option<SplitRefusal>,
}

/// Split at ordered absolute interior UV cuts. Refine the whole source
/// before slicing so that every part inherits the same rational gauge and
/// the literal shared seam controls. Both axes must be clamped and
/// nonperiodic. Any refusal withholds the complete tile set.
pub fn tile_tensor_surface(
    source: &Surface,
    cuts_u: &[f64],
    cuts_v: &[f64],
    max_work: u64,
) -> Result<TensorTiling> {
    source.validate()?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact tensor tiling work must be in [1,1000000]",
    )?;
    check(
        !cuts_u.is_empty() || !cuts_v.is_empty(),
        "Tensor tiling needs an interior cut",
    )?;
    for (knots, degree, count, periodic, cuts) in [
        (
            &source.knots_u,
            source.degree_u,
            source.control_points.len(),
            source.periodic_u,
            cuts_u,
        ),
        (
            &source.knots_v,
            source.degree_v,
            source.control_points[0].len(),
            source.periodic_v,
            cuts_v,
        ),
    ] {
        let [a, b] = [knots[degree], knots[count]];
        check(
            !periodic
                && knots[..=degree].iter().all(|&k| k == a)
                && knots[count..].iter().all(|&k| k == b),
            "Exact tensor tiling needs clamped nonperiodic axes",
        )?;
        check(
            cuts.len() <= 254
                && cuts.iter().all(|&k| a < k && k < b)
                && cuts.windows(2).all(|w| w[0] < w[1]),
            "Tensor tiling cuts must be strictly increasing interior parameters",
        )?;
    }
    let mut report = TensorTiling {
        patches: None,
        columns: cuts_u.len() + 1,
        rows: cuts_v.len() + 1,
        work_used: 0,
        refusal: None,
    };
    let result = tile(source, cuts_u, cuts_v, max_work, &mut report.work_used)?;
    match result {
        Ok(patches) => report.patches = Some(patches),
        Err(reason) => report.refusal = Some(reason),
    }
    Ok(report)
}

fn tile(
    source: &Surface,
    cuts_u: &[f64],
    cuts_v: &[f64],
    max_work: u64,
    used: &mut u64,
) -> Result<std::result::Result<Vec<Surface>, SplitRefusal>> {
    let mut refined: Option<Surface> = None;
    for (axis, cuts) in [(Axis::U, cuts_u), (Axis::V, cuts_v)] {
        for &cut in cuts {
            let s = refined.as_ref().unwrap_or(source);
            let (p, knots) = match axis {
                Axis::U => (s.degree_u, &s.knots_u),
                Axis::V => (s.degree_v, &s.knots_v),
            };
            let needed = p - knots.iter().filter(|&&k| k == cut).count();
            if needed == 0 {
                continue;
            }
            if *used == max_work {
                return Ok(Err(SplitRefusal::ResourceLimit));
            }
            let result = insert(s, axis, cut, needed, max_work - *used)?;
            *used += result.work_used;
            if let Some(reason) = result.refusal {
                return Ok(Err(SplitRefusal::Insertion(reason)));
            }
            refined = Some(result.surface);
        }
    }
    let s = refined.as_ref().unwrap_or(source);
    let nu = s.control_points.len();
    let nv = s.control_points[0].len();
    // Each internal break duplicates one pole row or column.
    let copy_work = ((nu + cuts_u.len()) * (nv + cuts_v.len())) as u64;
    if copy_work > max_work - *used {
        return Ok(Err(SplitRefusal::ResourceLimit));
    }
    *used += copy_work;
    let Some(u) = slices(&s.knots_u, s.degree_u, nu, cuts_u) else {
        return Ok(Err(SplitRefusal::InvalidOutput));
    };
    let Some(v) = slices(&s.knots_v, s.degree_v, nv, cuts_v) else {
        return Ok(Err(SplitRefusal::InvalidOutput));
    };
    let mut patches = Vec::with_capacity(u.len() * v.len());
    for (v_range, knots_v) in &v {
        for (u_range, knots_u) in &u {
            let patch = Surface {
                degree_u: s.degree_u,
                degree_v: s.degree_v,
                knots_u: knots_u.clone(),
                knots_v: knots_v.clone(),
                control_points: s.control_points[u_range.clone()]
                    .iter()
                    .map(|row| row[v_range.clone()].to_vec())
                    .collect(),
                weights: s.weights[u_range.clone()]
                    .iter()
                    .map(|row| row[v_range.clone()].to_vec())
                    .collect(),
                periodic_u: false,
                periodic_v: false,
            };
            if patch.validate().is_err() {
                return Ok(Err(SplitRefusal::InvalidOutput));
            }
            patches.push(patch);
        }
    }
    Ok(Ok(patches))
}

type AxisSlice = (std::ops::Range<usize>, Vec<f64>);

fn slices(knots: &[f64], degree: usize, count: usize, cuts: &[f64]) -> Option<Vec<AxisSlice>> {
    let mut out = Vec::with_capacity(cuts.len() + 1);
    let mut first = 0;
    for &cut in cuts {
        let end = knots.partition_point(|&k| k < cut);
        if knots.get(end..end + degree)?.iter().any(|&k| k != cut) || end <= first {
            return None;
        }
        // The first pole of every later slice is shared with its predecessor.
        let mut tile_knots = knots[first..end + degree].to_vec();
        if first != 0 {
            tile_knots[0] = knots[first + 1];
        }
        tile_knots.push(cut);
        out.push((first..end, tile_knots));
        first = end - 1;
    }
    let mut last_knots = knots[first..].to_vec();
    if first != 0 {
        last_knots[0] = knots[first + 1];
    }
    out.push((first..count, last_knots));
    Some(out)
}
