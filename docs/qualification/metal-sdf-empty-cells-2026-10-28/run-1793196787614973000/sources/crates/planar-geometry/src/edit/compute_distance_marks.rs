use super::*;

/// Nearest cardinal bbox gaps plus margins to the smallest enclosing box.
/// The caller supplies visible, non-selected targets; invalid boxes emit no marks.
pub fn compute_distance_marks(moving: BBox, targets: &[BBox]) -> Vec<DistanceMark> {
    if !usable_box(moving) {
        return Vec::new();
    }
    let mut neighbours: [Option<(f64, BBox)>; 4] = [None; 4];
    let mut container: Option<(f64, BBox)> = None;
    for &target in targets.iter().filter(|b| usable_box(**b)) {
        for axis in 0..2 {
            let other = 1 - axis;
            if moving.min[other] < target.max[other] && target.min[other] < moving.max[other] {
                let candidate = if target.max[axis] <= moving.min[axis] {
                    Some((axis * 2, moving.min[axis] - target.max[axis]))
                } else if target.min[axis] >= moving.max[axis] {
                    Some((axis * 2 + 1, target.min[axis] - moving.max[axis]))
                } else {
                    None
                };
                if let Some((side, gap)) = candidate
                    && neighbours[side].is_none_or(|(best, _)| gap < best)
                {
                    neighbours[side] = Some((gap, target));
                }
            }
        }
        if (0..2).all(|axis| {
            target.min[axis] <= moving.min[axis] && target.max[axis] >= moving.max[axis]
        }) {
            let area = target.width() * target.height();
            if container.is_none_or(|(best, _)| area < best) {
                container = Some((area, target));
            }
        }
    }
    let mut marks = Vec::new();
    for (side, neighbour) in neighbours.into_iter().enumerate() {
        if let Some((distance, target)) = neighbour {
            let axis = side / 2;
            let other = 1 - axis;
            let center = (moving.min[other].max(target.min[other])
                + moving.max[other].min(target.max[other]))
                * 0.5;
            let mut from = [center; 2];
            let mut to = from;
            if side % 2 == 0 {
                from[axis] = target.max[axis];
                to[axis] = moving.min[axis];
            } else {
                from[axis] = moving.max[axis];
                to[axis] = target.min[axis];
            }
            marks.push(DistanceMark { from, to, distance });
        }
    }
    if let Some((_, target)) = container {
        for side in 0..4 {
            let axis = side / 2;
            let mut from = moving.center();
            let mut to = from;
            if side % 2 == 0 {
                from[axis] = target.min[axis];
                to[axis] = moving.min[axis];
            } else {
                from[axis] = moving.max[axis];
                to[axis] = target.max[axis];
            }
            let distance = to[axis] - from[axis];
            if distance > 0.01 {
                marks.push(DistanceMark { from, to, distance });
            }
        }
    }
    marks
}

/// Every chord crossing, in source-path order, with its actual direction.
pub fn intersecting_paths(
    paths: &[BezierPath],
    a: [f64; 2],
    b: [f64; 2],
) -> Result<Vec<(f64, [f64; 2])>> {
    check(
        a.iter().chain(b.iter()).all(|x| x.is_finite()),
        "Invalid intersection chord",
    )?;
    let mut hits = Vec::new();
    for path in paths {
        let points = path.flatten()?;
        for edge in points.windows(2) {
            if let Some(point) = segment_intersection(a, b, edge[0], edge[1]) {
                hits.push((
                    (edge[1][1] - edge[0][1]).atan2(edge[1][0] - edge[0][0]),
                    point,
                ));
            }
        }
    }
    Ok(hits)
}

/// Chord-crossing directions modulo a half-turn, deduplicated within 1 degree.
pub fn intersecting_path_directions(
    paths: &[BezierPath],
    a: [f64; 2],
    b: [f64; 2],
) -> Result<Vec<f64>> {
    let mut directions: Vec<f64> = Vec::new();
    for (angle, _) in intersecting_paths(paths, a, b)? {
        let mut normalized = angle;
        while normalized > std::f64::consts::FRAC_PI_2 {
            normalized -= std::f64::consts::PI;
        }
        while normalized <= -std::f64::consts::FRAC_PI_2 {
            normalized += std::f64::consts::PI;
        }
        if !directions
            .iter()
            .any(|d| (d - normalized).abs() < 1f64.to_radians())
        {
            directions.push(normalized);
        }
    }
    Ok(directions)
}

/// Live align-guides: snap a moving bbox to left/center/right and top/mid/bottom of targets.
pub fn drag_align_guides(moving: BBox, targets: &[BBox], threshold: f64) -> Result<DragAlign> {
    check(
        threshold > 0. && threshold.is_finite(),
        "Invalid guide threshold",
    )?;
    let mut dx = 0.0_f64;
    let mut dy = 0.0_f64;
    let mut best_x = threshold;
    let mut best_y = threshold;
    let mut guides = Vec::new();
    let moving_x = [moving.left(), moving.center()[0], moving.right()];
    let moving_y = [moving.top(), moving.center()[1], moving.bottom()];
    for t in targets {
        let tx = [t.left(), t.center()[0], t.right()];
        let ty = [t.top(), t.center()[1], t.bottom()];
        for &m in &moving_x {
            for &target in &tx {
                let d = target - m;
                let ad = d.abs();
                if ad <= best_x + 1e-12 && ad <= threshold {
                    if ad < best_x - 1e-12 || (d - dx).abs() > 1e-12 {
                        guides.retain(|g: &GuideLine| !g.vertical);
                    }
                    best_x = ad;
                    dx = d;
                    if !guides
                        .iter()
                        .any(|g| g.vertical && (g.position - target).abs() < 1e-12)
                    {
                        guides.push(GuideLine {
                            vertical: true,
                            position: target,
                        });
                    }
                }
            }
        }
        for &m in &moving_y {
            for &target in &ty {
                let d = target - m;
                let ad = d.abs();
                if ad <= best_y + 1e-12 && ad <= threshold {
                    if ad < best_y - 1e-12 || (d - dy).abs() > 1e-12 {
                        guides.retain(|g: &GuideLine| g.vertical);
                    }
                    best_y = ad;
                    dy = d;
                    if !guides
                        .iter()
                        .any(|g| !g.vertical && (g.position - target).abs() < 1e-12)
                    {
                        guides.push(GuideLine {
                            vertical: false,
                            position: target,
                        });
                    }
                }
            }
        }
    }
    if best_x > threshold {
        dx = 0.0;
    }
    if best_y > threshold {
        dy = 0.0;
    }
    Ok(DragAlign {
        delta: [dx, dy],
        guides,
    })
}

pub(super) fn segment_intersection(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> Option<[f64; 2]> {
    let r = [b[0] - a[0], b[1] - a[1]];
    let s = [d[0] - c[0], d[1] - c[1]];
    let den = r[0] * s[1] - r[1] * s[0];
    if den.abs() < 1e-15 {
        return None;
    }
    let qp = [c[0] - a[0], c[1] - a[1]];
    let t = (qp[0] * s[1] - qp[1] * s[0]) / den;
    let u = (qp[0] * r[1] - qp[1] * r[0]) / den;
    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        Some([a[0] + t * r[0], a[1] + t * r[1]])
    } else {
        None
    }
}

// ----- Align / distribute -----

/// Per-box translation `(dx, dy)` to align against a reference bbox.
pub fn align_boxes(
    boxes: &[BBox],
    reference: BBox,
    h: Option<HAlign>,
    v: Option<VAlign>,
) -> Vec<[f64; 2]> {
    boxes
        .iter()
        .map(|b| {
            let mut dx = 0.0;
            let mut dy = 0.0;
            if let Some(h) = h {
                dx = match h {
                    HAlign::Left => reference.min[0] - b.min[0],
                    HAlign::Center => reference.center()[0] - b.center()[0],
                    HAlign::Right => reference.max[0] - b.max[0],
                };
            }
            if let Some(v) = v {
                dy = match v {
                    VAlign::Top => reference.min[1] - b.min[1],
                    VAlign::Center => reference.center()[1] - b.center()[1],
                    VAlign::Bottom => reference.max[1] - b.max[1],
                };
            }
            [dx, dy]
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelativeTo {
    Selection,
    KeyObject,
    Page,
    LastSelected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistributeAnchor {
    LeftOrTop,
    Center,
    RightOrBottom,
}

/// Resolve the align reference bbox. `KeyObject` falls back to the selection union.
pub fn resolve_align_reference(
    boxes: &[BBox],
    relative: RelativeTo,
    key: Option<usize>,
    page: Option<BBox>,
    last: Option<usize>,
) -> Result<BBox> {
    check(!boxes.is_empty(), "Align needs at least one box")?;
    let union = boxes.iter().copied().reduce(BBox::union).unwrap();
    match relative {
        RelativeTo::Selection => Ok(union),
        RelativeTo::Page => page.ok_or_else(|| crate::error("Align to page needs a page bbox")),
        RelativeTo::KeyObject => match key.filter(|&i| i < boxes.len()) {
            Some(i) => Ok(boxes[i]),
            None => Ok(union),
        },
        RelativeTo::LastSelected => {
            let i = last.unwrap_or(boxes.len() - 1);
            check(i < boxes.len(), "Last-selected index out of range")?;
            Ok(boxes[i])
        }
    }
}

/// Align against a relative reference (selection / key / page / last selected).
pub fn align_boxes_relative(
    boxes: &[BBox],
    relative: RelativeTo,
    key: Option<usize>,
    page: Option<BBox>,
    last: Option<usize>,
    h: Option<HAlign>,
    v: Option<VAlign>,
) -> Result<Vec<[f64; 2]>> {
    let reference = resolve_align_reference(boxes, relative, key, page, last)?;
    Ok(align_boxes(boxes, reference, h, v))
}

/// Evenly space box centers along X (`horizontal`) or Y.
pub fn distribute_centers(boxes: &[BBox], horizontal: bool) -> Result<Vec<[f64; 2]>> {
    check(boxes.len() >= 3, "Distribute needs at least 3 boxes")?;
    let mut order: Vec<usize> = (0..boxes.len()).collect();
    order.sort_by(|&i, &j| {
        if horizontal {
            boxes[i].center()[0].total_cmp(&boxes[j].center()[0])
        } else {
            boxes[i].center()[1].total_cmp(&boxes[j].center()[1])
        }
    });
    let first = boxes[order[0]].center();
    let last = boxes[order[order.len() - 1]].center();
    let n = order.len() as f64;
    let mut deltas = vec![[0.0, 0.0]; boxes.len()];
    for (k, &i) in order.iter().enumerate() {
        let t = k as f64 / (n - 1.0);
        let target = if horizontal {
            [first[0] + (last[0] - first[0]) * t, boxes[i].center()[1]]
        } else {
            [boxes[i].center()[0], first[1] + (last[1] - first[1]) * t]
        };
        let c = boxes[i].center();
        deltas[i] = [target[0] - c[0], target[1] - c[1]];
    }
    Ok(deltas)
}

fn sample_box(b: BBox, horizontal: bool, anchor: DistributeAnchor) -> f64 {
    match (horizontal, anchor) {
        (true, DistributeAnchor::LeftOrTop) => b.left(),
        (true, DistributeAnchor::Center) => b.center()[0],
        (true, DistributeAnchor::RightOrBottom) => b.right(),
        (false, DistributeAnchor::LeftOrTop) => b.top(),
        (false, DistributeAnchor::Center) => b.center()[1],
        (false, DistributeAnchor::RightOrBottom) => b.bottom(),
    }
}

/// Equal-spacing distribute on an edge (or center). Outermost two stay put.
pub fn distribute_objects(
    boxes: &[BBox],
    horizontal: bool,
    anchor: DistributeAnchor,
) -> Result<Vec<[f64; 2]>> {
    check(boxes.len() >= 3, "Distribute needs at least 3 boxes")?;
    let mut order: Vec<usize> = (0..boxes.len()).collect();
    order.sort_by(|&i, &j| {
        sample_box(boxes[i], horizontal, anchor)
            .total_cmp(&sample_box(boxes[j], horizontal, anchor))
    });
    let first = sample_box(boxes[order[0]], horizontal, anchor);
    let last = sample_box(boxes[order[order.len() - 1]], horizontal, anchor);
    let step = (last - first) / (order.len() - 1) as f64;
    let mut deltas = vec![[0.0, 0.0]; boxes.len()];
    for (k, &i) in order.iter().enumerate() {
        let target = first + step * k as f64;
        let d = target - sample_box(boxes[i], horizontal, anchor);
        deltas[i] = if horizontal { [d, 0.0] } else { [0.0, d] };
    }
    Ok(deltas)
}

/// Pack boxes so consecutive gaps equal `gap`. First (lowest) box stays put.
pub fn distribute_spacing(boxes: &[BBox], horizontal: bool, gap: f64) -> Result<Vec<[f64; 2]>> {
    check(
        boxes.len() >= 2,
        "Distribute spacing needs at least 2 boxes",
    )?;
    check(
        gap.is_finite() && gap.abs() <= 1e6,
        "Invalid distribute gap",
    )?;
    let lo = |b: BBox| if horizontal { b.left() } else { b.top() };
    let size = |b: BBox| if horizontal { b.width() } else { b.height() };
    let mut order: Vec<usize> = (0..boxes.len()).collect();
    order.sort_by(|&i, &j| lo(boxes[i]).total_cmp(&lo(boxes[j])));
    let mut deltas = vec![[0.0, 0.0]; boxes.len()];
    let mut cursor = lo(boxes[order[0]]) + size(boxes[order[0]]);
    for &i in order.iter().skip(1) {
        let target = cursor + gap;
        let d = target - lo(boxes[i]);
        deltas[i] = if horizontal { [d, 0.0] } else { [0.0, d] };
        cursor = target + size(boxes[i]);
    }
    Ok(deltas)
}

// ----- Scissors / knife (cubic-aware; see scissors.rs) -----

pub use crate::scissors::{
    CutHit, cut_at, hit_test as hit_test_cut, knife_cut, knife_hits, knife_split, scissors_cut,
    split_segment_at,
};

/// Closest hit for snap/hover: prefers cubic-aware scissors hit, falls back to polyline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathHit {
    pub segment: usize,
    pub t: f64,
    pub point: [f64; 2],
    pub distance: f64,
}

pub fn hit_test_path(path: &BezierPath, click: [f64; 2], max_dist: f64) -> Result<Option<PathHit>> {
    Ok(
        crate::scissors::hit_test(path, click, max_dist)?.map(|h| PathHit {
            segment: h.segment_index,
            t: h.t,
            point: h.point,
            distance: h.distance,
        }),
    )
}

// ----- Measure -----

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Measurement {
    pub a: [f64; 2],
    pub b: [f64; 2],
    pub distance: f64,
    pub angle_deg: f64,
    pub delta: [f64; 2],
}

pub fn measure(a: [f64; 2], b: [f64; 2]) -> Result<Measurement> {
    check(
        a.iter().chain(b.iter()).all(|x| x.is_finite()),
        "Non-finite measure points",
    )?;
    let delta = [b[0] - a[0], b[1] - a[1]];
    Ok(Measurement {
        a,
        b,
        distance: delta[0].hypot(delta[1]),
        angle_deg: delta[1].atan2(delta[0]).to_degrees(),
        delta,
    })
}
