//! Editor-precision helpers: snap, align/distribute, cut, measure.
//! Pure geometry — no UI / document layer.
use crate::path::BezierPath;
use crate::{Result, check};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VAlign {
    Top,
    Center,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapKind {
    Grid,
    Endpoint,
    Midpoint,
    Intersection,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapHit {
    pub point: [f64; 2],
    pub kind: SnapKind,
    pub distance: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BBox {
    pub min: [f64; 2],
    pub max: [f64; 2],
}

impl BBox {
    pub fn from_points(points: &[[f64; 2]]) -> Result<Self> {
        check(!points.is_empty(), "Empty bbox points")?;
        let mut min = points[0];
        let mut max = points[0];
        for p in points {
            check(p.iter().all(|x| x.is_finite()), "Non-finite bbox point")?;
            min[0] = min[0].min(p[0]);
            min[1] = min[1].min(p[1]);
            max[0] = max[0].max(p[0]);
            max[1] = max[1].max(p[1]);
        }
        Ok(Self { min, max })
    }

    pub fn union(self, other: Self) -> Self {
        Self {
            min: [self.min[0].min(other.min[0]), self.min[1].min(other.min[1])],
            max: [self.max[0].max(other.max[0]), self.max[1].max(other.max[1])],
        }
    }

    pub fn width(self) -> f64 {
        self.max[0] - self.min[0]
    }
    pub fn height(self) -> f64 {
        self.max[1] - self.min[1]
    }
    pub fn center(self) -> [f64; 2] {
        [
            (self.min[0] + self.max[0]) * 0.5,
            (self.min[1] + self.max[1]) * 0.5,
        ]
    }
    pub fn left(self) -> f64 {
        self.min[0]
    }
    pub fn right(self) -> f64 {
        self.max[0]
    }
    pub fn top(self) -> f64 {
        self.min[1]
    }
    pub fn bottom(self) -> f64 {
        self.max[1]
    }
}

fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

fn lerp(a: [f64; 2], b: [f64; 2], t: f64) -> [f64; 2] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

// ----- Snap -----

pub fn snap_to_grid(p: [f64; 2], spacing: f64) -> Result<[f64; 2]> {
    check(
        spacing > 1e-12 && spacing.is_finite(),
        "Invalid grid spacing",
    )?;
    check(p.iter().all(|x| x.is_finite()), "Non-finite point")?;
    Ok([
        (p[0] / spacing).round() * spacing,
        (p[1] / spacing).round() * spacing,
    ])
}

/// Best snap among grid, endpoints, midpoints, and segment intersections.
pub fn find_snap(
    cursor: [f64; 2],
    threshold: f64,
    grid: Option<f64>,
    paths: &[BezierPath],
) -> Result<Option<SnapHit>> {
    check(
        threshold > 0. && threshold.is_finite(),
        "Invalid snap threshold",
    )?;
    check(cursor.iter().all(|x| x.is_finite()), "Non-finite cursor")?;
    let mut best: Option<SnapHit> = None;
    let priority = |k: SnapKind| -> u8 {
        match k {
            SnapKind::Intersection => 4,
            SnapKind::Endpoint => 3,
            SnapKind::Midpoint => 2,
            SnapKind::Grid => 1,
        }
    };
    let mut consider = |point: [f64; 2], kind: SnapKind| {
        let d = dist(cursor, point);
        if d > threshold {
            return;
        }
        let better = match &best {
            None => true,
            Some(b) => {
                d < b.distance - 1e-12
                    || ((d - b.distance).abs() <= 1e-12 && priority(kind) > priority(b.kind))
            }
        };
        if better {
            best = Some(SnapHit {
                point,
                kind,
                distance: d,
            });
        }
    };
    if let Some(g) = grid {
        consider(snap_to_grid(cursor, g)?, SnapKind::Grid);
    }
    let mut segments: Vec<([f64; 2], [f64; 2])> = Vec::new();
    for path in paths {
        let pts = path.flatten()?;
        for i in 0..pts.len() {
            consider(pts[i], SnapKind::Endpoint);
            if i + 1 < pts.len() {
                consider(lerp(pts[i], pts[i + 1], 0.5), SnapKind::Midpoint);
                segments.push((pts[i], pts[i + 1]));
            } else if path.closed && pts.len() > 1 {
                consider(lerp(pts[i], pts[0], 0.5), SnapKind::Midpoint);
                segments.push((pts[i], pts[0]));
            }
        }
    }
    check(segments.len() <= 4096, "Snap segment budget exceeded")?;
    for i in 0..segments.len() {
        for j in i + 1..segments.len() {
            if let Some(p) =
                segment_intersection(segments[i].0, segments[i].1, segments[j].0, segments[j].1)
            {
                consider(p, SnapKind::Intersection);
            }
        }
    }
    Ok(best)
}

/// Snap `end` to the nearest multiple of `step_deg` around `start`, keeping length.
pub fn snap_to_angle(start: [f64; 2], end: [f64; 2], step_deg: f64) -> Result<[f64; 2]> {
    check(
        start.iter().chain(end.iter()).all(|x| x.is_finite()),
        "Non-finite angle-snap points",
    )?;
    check(
        step_deg > 0. && step_deg.is_finite() && step_deg <= 180.,
        "Invalid angle step",
    )?;
    let dx = end[0] - start[0];
    let dy = end[1] - start[1];
    if dx.abs() < 1e-12 && dy.abs() < 1e-12 {
        return Ok(end);
    }
    let step = step_deg.to_radians();
    let snapped = (dy.atan2(dx) / step).round() * step;
    let len = dx.hypot(dy);
    Ok([
        start[0] + snapped.cos() * len,
        start[1] + snapped.sin() * len,
    ])
}

/// 45° constraint (Shift-drag).
pub fn snap_to_45(start: [f64; 2], end: [f64; 2]) -> Result<[f64; 2]> {
    snap_to_angle(start, end, 45.0)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GuideLine {
    /// `true` = vertical line at `position` (X), else horizontal at Y.
    pub vertical: bool,
    pub position: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DragAlign {
    pub delta: [f64; 2],
    pub guides: Vec<GuideLine>,
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
                    if ad < best_x - 1e-12 {
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
                    if ad < best_y - 1e-12 {
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

fn segment_intersection(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> Option<[f64; 2]> {
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
    Ok(crate::scissors::hit_test(path, click, max_dist)?.map(|h| PathHit {
        segment: h.segment_index,
        t: h.t,
        point: h.point,
        distance: h.distance,
    }))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snap_grid_and_intersection() {
        let g = snap_to_grid([1.2, 3.8], 1.).unwrap();
        assert_eq!(g, [1., 4.]);
        let a = BezierPath::from_polyline(&[[0., 0.], [4., 4.]], false).unwrap();
        let b = BezierPath::from_polyline(&[[0., 4.], [4., 0.]], false).unwrap();
        let hit = find_snap([2.05, 2.0], 0.5, None, &[a, b]).unwrap().unwrap();
        assert_eq!(hit.kind, SnapKind::Intersection);
        assert!(dist(hit.point, [2., 2.]) < 1e-6);
    }

    #[test]
    fn align_and_distribute() {
        let boxes = [
            BBox {
                min: [0., 0.],
                max: [1., 1.],
            },
            BBox {
                min: [5., 2.],
                max: [6., 3.],
            },
            BBox {
                min: [2., 4.],
                max: [3., 5.],
            },
        ];
        let ref_bb = boxes.iter().copied().reduce(BBox::union).unwrap();
        let d = align_boxes(&boxes, ref_bb, Some(HAlign::Left), None);
        assert!((boxes[0].min[0] + d[0][0] - ref_bb.min[0]).abs() < 1e-9);
        let dist = distribute_centers(&boxes, true).unwrap();
        assert_eq!(dist.len(), 3);
        let edges = distribute_objects(&boxes, true, DistributeAnchor::LeftOrTop).unwrap();
        assert_eq!(edges.len(), 3);
        let page = BBox {
            min: [0., 0.],
            max: [20., 20.],
        };
        let key = align_boxes_relative(
            &boxes,
            RelativeTo::KeyObject,
            Some(0),
            None,
            None,
            Some(HAlign::Left),
            None,
        )
        .unwrap();
        assert!((boxes[1].min[0] + key[1][0] - boxes[0].min[0]).abs() < 1e-9);
        let packed = distribute_spacing(&boxes, true, 1.).unwrap();
        assert_eq!(packed.len(), 3);
        let _ = page;
    }

    #[test]
    fn angle_snap_and_guides() {
        let p = snap_to_45([0., 0.], [10., 1.]).unwrap();
        assert!((p[1]).abs() < 1e-9);
        let moving = BBox {
            min: [0.1, 0.],
            max: [1.1, 1.],
        };
        let target = BBox {
            min: [0., 4.],
            max: [2., 5.],
        };
        let g = drag_align_guides(moving, &[target], 0.2).unwrap();
        assert!((g.delta[0] + 0.1).abs() < 1e-9);
        assert!(g.guides.iter().any(|l| l.vertical));
    }

    #[test]
    fn scissors_and_knife() {
        let path = BezierPath::from_polyline(&[[0., 0.], [10., 0.]], false).unwrap();
        let parts = scissors_cut(&path, [5., 0.1], 0.5).unwrap();
        assert_eq!(parts.len(), 2);
        let knife = knife_cut(
            &BezierPath::from_polyline(&[[0., 0.], [10., 0.], [10., 10.]], false).unwrap(),
            [5., -1.],
            [5., 1.],
        )
        .unwrap();
        assert!(knife.len() >= 2);
    }

    #[test]
    fn measure_segment() {
        let m = measure([0., 0.], [3., 4.]).unwrap();
        assert!((m.distance - 5.).abs() < 1e-12);
        assert!((m.angle_deg - 53.130102).abs() < 1e-4);
    }
}
