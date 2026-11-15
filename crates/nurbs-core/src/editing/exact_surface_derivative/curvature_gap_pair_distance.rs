//! Minimum Euclidean distance between two certified curvature contour branches.
use super::{
    curvature_gap_distance::{
        difference, dot, lift, segment_distance_squared, squared_distance_range,
    },
    curvature_gap_world::{
        CurvatureGapWorldPolyline, certify_curvature_gap_world_polyline, chord_box,
    },
    *,
};
use crate::distance_bounds::{Interval as I, box_distance};
use num_traits::Signed;

#[derive(Clone, Copy)]
pub struct CurvatureGapContourSpec<'a> {
    pub surface: &'a Surface,
    pub span: [usize; 2],
    pub axis: Axis,
    pub level: f64,
    pub transverse: &'a [f64],
    pub root_depth: u8,
}

#[derive(Clone, Debug)]
pub struct CurvatureGapPairDistance {
    pub distance_bounds: Option<[f64; 2]>,
    /// Pair of transverse segments providing the minimum upper bound.
    pub upper_segments: Option<[usize; 2]>,
    pub compared_pairs: usize,
    pub pruned_pairs: usize,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}

/// Enclose the minimum model-space distance between two proven branches.
/// Both contour proofs and exact segment projections share the work limit.
/// A positive lower bound certifies disjointness in the supplied cells.
pub fn distance_between_curvature_gap_contours(
    a: CurvatureGapContourSpec<'_>,
    b: CurvatureGapContourSpec<'_>,
    max_work: u64,
) -> Result<CurvatureGapPairDistance> {
    let mut report = CurvatureGapPairDistance {
        distance_bounds: None,
        upper_segments: None,
        compared_pairs: 0,
        pruned_pairs: 0,
        work_used: 0,
        refusal: None,
    };
    let a = world(a, max_work)?;
    report.work_used = a.uv.work_used;
    report.refusal = a.uv.refusal;
    if a.points.is_none() {
        return Ok(report);
    }
    let remaining = max_work - report.work_used;
    if remaining == 0 {
        report.refusal = Some(Refusal::ResourceLimit);
        return Ok(report);
    }
    let b = world(b, remaining)?;
    report.work_used += b.uv.work_used;
    report.refusal = b.uv.refusal;
    let (Some(ap), Some(ae), Some(bp), Some(be)) = (
        &a.points,
        &a.world_error_upper,
        &b.points,
        &b.world_error_upper,
    ) else {
        return Ok(report);
    };
    let boxes_a = boxes(ap, ae)?;
    let boxes_b = boxes(bp, be)?;
    let mut work = Work {
        used: report.work_used,
        limit: max_work,
    };
    let out = (|| {
        let mut best = [f64::INFINITY; 2];
        let mut witness = [0, 0];
        let a_points = ap
            .iter()
            .map(|&p| lift(p, &mut work))
            .collect::<PResult<Vec<_>>>()?;
        let b_points = bp
            .iter()
            .map(|&p| lift(p, &mut work))
            .collect::<PResult<Vec<_>>>()?;
        for (i, a_box) in boxes_a.iter().enumerate() {
            for (j, b_box) in boxes_b.iter().enumerate() {
                work.charge(1)?;
                let (lower, _) = box_distance(a_box, b_box).map_err(|_| Refusal::InvalidOutput)?;
                if lower >= best[1] {
                    report.pruned_pairs += 1;
                    continue;
                }
                let squared =
                    segment_pair_squared(&a_points[i..i + 2], &b_points[j..j + 2], &mut work)?;
                let error = I::point(ae[i])
                    .add(I::point(be[j]))
                    .map_err(|_| Refusal::InvalidOutput)?
                    .hi;
                let Some([lo, hi]) = squared_distance_range(&squared, error, &mut work)? else {
                    return Ok(None);
                };
                report.compared_pairs += 1;
                best[0] = best[0].min(lo.max(lower));
                if hi < best[1] {
                    best[1] = hi;
                    witness = [i, j];
                }
            }
        }
        Ok(Some((best, witness)))
    })();
    report.work_used = work.used;
    match out {
        Ok(Some((range, witness))) => {
            report.distance_bounds = Some(range);
            report.upper_segments = Some(witness);
        }
        Ok(None) => {}
        Err(reason) => report.refusal = Some(reason),
    }
    Ok(report)
}

fn world(spec: CurvatureGapContourSpec<'_>, max_work: u64) -> Result<CurvatureGapWorldPolyline> {
    certify_curvature_gap_world_polyline(
        spec.surface,
        spec.span,
        spec.axis,
        spec.level,
        spec.transverse,
        spec.root_depth,
        max_work,
    )
}

fn boxes(points: &[[f64; 3]], errors: &[f64]) -> Result<Vec<[I; 3]>> {
    points
        .windows(2)
        .zip(errors)
        .map(|(pair, &error)| chord_box(pair[0], pair[1], error))
        .collect()
}

fn segment_pair_squared(a: &[[Q; 3]], b: &[[Q; 3]], work: &mut Work) -> PResult<Q> {
    let mut best = segment_distance_squared(&a[0], &b[0], &b[1], work)?;
    best = best.min(segment_distance_squared(&a[1], &b[0], &b[1], work)?);
    best = best.min(segment_distance_squared(&b[0], &a[0], &a[1], work)?);
    best = best.min(segment_distance_squared(&b[1], &a[0], &a[1], work)?);

    let d = difference(&a[1], &a[0], work)?;
    let e = difference(&b[1], &b[0], work)?;
    let w = difference(&a[0], &b[0], work)?;
    let dd = dot(&d, &d, work)?;
    let ee = dot(&e, &e, work)?;
    let de = dot(&d, &e, work)?;
    let dw = dot(&d, &w, work)?;
    let ew = dot(&e, &w, work)?;
    let determinant = product_difference(&dd, &ee, &de, &de, work)?;
    // Parallel and degenerate segment pairs attain a minimum on a boundary.
    if determinant.is_positive() {
        let t = product_difference(&de, &ew, &ee, &dw, work)?;
        let u = product_difference(&dd, &ew, &de, &dw, work)?;
        if !t.is_negative() && t <= determinant && !u.is_negative() && u <= determinant {
            let t = work.op(&t, &determinant, |x, y| x / y)?;
            let u = work.op(&u, &determinant, |x, y| x / y)?;
            let mut delta = w;
            for i in 0..3 {
                let av = work.op(&t, &d[i], |x, y| x * y)?;
                let bv = work.op(&u, &e[i], |x, y| x * y)?;
                let diff = work.op(&av, &bv, |x, y| x - y)?;
                delta[i] = work.op(&delta[i], &diff, |x, y| x + y)?;
            }
            best = best.min(dot(&delta, &delta, work)?);
        }
    }
    Ok(best)
}

fn product_difference(a: &Q, b: &Q, c: &Q, d: &Q, work: &mut Work) -> PResult<Q> {
    let ab = work.op(a, b, |x, y| x * y)?;
    let cd = work.op(c, d, |x, y| x * y)?;
    Ok(work.op(&ab, &cd, |x, y| x - y)?)
}
