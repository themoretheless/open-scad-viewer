use super::*;

/// Complete-domain exclusion stage for two signed normal-offset carriers.
/// Candidate boxes are possible center locations, never certified roots.
/// Pending boxes retain every unvisited, singular or precision-limited region.
/// UV trim admission and continuation/branch uniqueness belong to later stages.
pub fn intersection_candidates(
    surfaces: [&Surface; 2],
    domains: PairDomain,
    distances: [f64; 2],
    parameter_tolerance: f64,
    max_boxes: usize,
    max_spans_per_box: usize,
) -> Result<Candidates> {
    check(
        parameter_tolerance.is_finite() && parameter_tolerance > 0. && parameter_tolerance <= 1.,
        "Offset search parameter tolerance must lie in (0,1]",
    )?;
    check(
        (1..=100000).contains(&max_boxes),
        "Offset search needs 1..100000 boxes",
    )?;
    // Evaluate both full rectangles before an early exclusion can hide an
    // invalid second operand. Reuse these reports for the first visited box.
    let initial = [
        bounds(surfaces[0], domains[0], distances[0], max_spans_per_box)?,
        bounds(surfaces[1], domains[1], distances[1], max_spans_per_box)?,
    ];
    let widths = domains.map(|d| d.map(|r| r[1] - r[0]));
    check(
        widths.iter().flatten().all(|w| w.is_finite()),
        "Offset search parameter widths exceeded numeric range",
    )?;
    let mut queue = vec![domains];
    let mut out = Candidates {
        boxes: Vec::new(),
        pending: Vec::new(),
        visited_boxes: 0,
        excluded_boxes: 0,
        normal_span_visits: 0,
        reason: "candidate-boxes",
    };
    while let Some(cell) = queue.pop() {
        if out.visited_boxes == max_boxes {
            out.pending.push(cell);
            out.pending.extend(queue);
            out.reason = "work-limit";
            return Ok(out);
        }
        out.visited_boxes += 1;
        let mut images = [None, None];
        for side in 0..2 {
            let bound = if out.visited_boxes == 1 {
                initial[side]
            } else {
                bounds(
                    surfaces[side],
                    cell[side],
                    distances[side],
                    max_spans_per_box,
                )?
            };
            out.normal_span_visits += bound.spans;
            images[side] = bound.image;
        }
        if let [Some(a), Some(b)] = images {
            if crate::surface_distance::enclosure_distance(&a, &b)?.0 > 0. {
                out.excluded_boxes += 1;
                continue;
            }
        }
        let (side, axis, width) = (0..2)
            .flat_map(|side| (0..2).map(move |axis| (side, axis)))
            .map(|(side, axis)| {
                let width = if widths[side][axis] == 0. {
                    0.
                } else {
                    (cell[side][axis][1] - cell[side][axis][0]) / widths[side][axis]
                };
                (side, axis, width)
            })
            .max_by(|a, b| a.2.total_cmp(&b.2))
            .unwrap();
        if width <= parameter_tolerance {
            if images.iter().all(Option::is_some) {
                out.boxes.push(cell);
            } else {
                out.pending.push(cell);
                out.reason = "source-normal-unresolved";
            }
            continue;
        }
        let [lo, hi] = cell[side][axis];
        let mid = lo * 0.5 + hi * 0.5;
        if mid <= lo || mid >= hi {
            out.pending.push(cell);
            out.reason = "precision-limit";
            continue;
        }
        for range in [[lo, mid], [mid, hi]] {
            let mut child = cell;
            child[side][axis] = range;
            queue.push(child);
        }
    }
    if out.pending.is_empty() && out.boxes.is_empty() {
        out.reason = "all-excluded";
    }
    Ok(out)
}
