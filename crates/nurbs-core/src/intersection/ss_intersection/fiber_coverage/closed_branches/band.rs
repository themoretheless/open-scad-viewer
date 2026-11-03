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
        let r = f
            .source()
            .certify_contracted(next, f.free_axis(), (max - used).min(100000))?;
        used += r.rectangles();
        if let Some(next) = r.into_fiber() {
            point = next;
        } else {
            break;
        }
    }
    Ok(RootBand {
        fiber: point,
        cells: used,
    })
}
