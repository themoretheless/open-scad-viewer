//! Refine an original uniform root band, including singleton point selectors.
use super::*;
pub(super) struct RootBand {
    pub fiber: QualifiedFiber,
    pub cells: usize,
}
pub(super) fn refine_band(f: &QualifiedFiber, interval: [f64; 2], max: usize) -> Result<RootBand> {
    check(max > 0, "No original root band work")?;
    let mut d = f.domain();
    d[f.free_axis().index()] = interval;
    let r = f
        .source()
        .certify_contracted(d, f.free_axis(), max.min(100000))?;
    let mut used = r.rectangles();
    let mut point = r
        .into_fiber()
        .ok_or_else(|| crate::input("Original root band remains unproved"))?;
    for _ in 0..12 {
        let bounds = super::cycle::point_bounds(&point)?;
        let mut next = point.domain();
        let mut improved = false;
        for k in 0..4 {
            if k != point.free_axis().index() {
                let mid = bounds[k][0] * 0.5 + bounds[k][1] * 0.5;
                let padding =
                    (0.1 * (bounds[k][1] - bounds[k][0])).max(16. * (mid.next_up() - mid).abs());
                next[k] = [
                    (bounds[k][0] - padding).max(next[k][0]),
                    (bounds[k][1] + padding).min(next[k][1]),
                ];
                improved |=
                    next[k][1] - next[k][0] < 0.8 * (point.domain()[k][1] - point.domain()[k][0]);
            }
        }
        if !improved
            || next
                .iter()
                .enumerate()
                .any(|(k, d)| k != point.free_axis().index() && d[0] >= d[1])
        {
            break;
        }
        if used == max {
            return Err(crate::resource("Original root band refinement exhausted"));
        }
        let mut refined = None;
        for &extra in if interval[0] == interval[1] {
            &[0.0, 0.25, 0.5][..]
        } else {
            &[0.0][..]
        } {
            let mut proposal = next;
            for axis in 0..4 {
                if axis != f.free_axis().index() {
                    let old = point.domain()[axis];
                    proposal[axis][0] += (old[0] - proposal[axis][0]) * extra;
                    proposal[axis][1] += (old[1] - proposal[axis][1]) * extra;
                }
            }
            if used == max {
                break;
            }
            let result =
                f.source()
                    .certify_contracted(proposal, f.free_axis(), (max - used).min(100000))?;
            used += result.rectangles();
            if let Some(candidate) = result.into_fiber() {
                refined = Some(candidate);
                break;
            }
        }
        let Some(candidate) = refined else { break };
        point = candidate;
    }
    Ok(RootBand {
        fiber: point,
        cells: used,
    })
}
