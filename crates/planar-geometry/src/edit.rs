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
    Vertex,
    Corner,
    Center,
    Perpendicular,
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
            SnapKind::Vertex | SnapKind::Corner => 3,
            SnapKind::Center => 2,
            SnapKind::Perpendicular => 0,
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

/// Geometry metadata retained for Curvex snapping. Cubic paths contribute
/// actual anchors, while primitives retain centers, corners and edge midpoints.
#[derive(Debug, Clone, PartialEq)]
pub enum SnapGeometry {
    Line {
        start: [f64; 2],
        end: [f64; 2],
    },
    Rectangle {
        min: [f64; 2],
        max: [f64; 2],
        outline: Option<BezierPath>,
    },
    Ellipse {
        center: [f64; 2],
        radii: [f64; 2],
    },
    Polygon {
        points: Vec<[f64; 2]>,
        closed: bool,
        outline: Option<BezierPath>,
    },
    Polyline(Vec<[f64; 2]>),
    Path(BezierPath),
    Compound(Vec<BezierPath>),
}

/// Curvex semantic snapping: kind priority wins within the cursor tolerance;
/// distance breaks ties of the same kind. Grid is a fallback. Rounded primitive
/// outlines contribute their visible samples, never their unrounded corners.
pub fn find_geometry_snap(
    cursor: [f64; 2],
    threshold: f64,
    grid: Option<f64>,
    geometry: &[SnapGeometry],
) -> Result<Option<SnapHit>> {
    check(
        threshold > 0. && threshold.is_finite(),
        "Invalid snap threshold",
    )?;
    check(cursor.iter().all(|x| x.is_finite()), "Non-finite cursor")?;
    check(geometry.len() <= 4096, "Snap geometry budget exceeded")?;
    let priority = |kind: SnapKind| match kind {
        SnapKind::Midpoint => 5,
        SnapKind::Endpoint | SnapKind::Vertex | SnapKind::Corner => 4,
        SnapKind::Center => 3,
        SnapKind::Intersection => 2,
        SnapKind::Perpendicular => 1,
        SnapKind::Grid => 0,
    };
    let mut best: Option<SnapHit> = None;
    let mut consider = |point: [f64; 2], kind: SnapKind| {
        let distance = dist(cursor, point);
        if distance <= threshold
            && best.is_none_or(|b| {
                priority(kind) > priority(b.kind)
                    || (priority(kind) == priority(b.kind) && distance < b.distance)
            })
        {
            best = Some(SnapHit {
                point,
                kind,
                distance,
            });
        }
    };
    if let Some(spacing) = grid {
        consider(snap_to_grid(cursor, spacing)?, SnapKind::Grid);
    }
    let mut segments = Vec::new();
    for shape in geometry {
        let mut candidates = Vec::new();
        let mut outlines: Vec<(Vec<[f64; 2]>, bool, bool)> = Vec::new();
        let mut add_path = |path: &BezierPath| -> Result<()> {
            candidates.push((
                path.start,
                if path.closed {
                    SnapKind::Vertex
                } else {
                    SnapKind::Endpoint
                },
            ));
            for (i, segment) in path.segments.iter().enumerate() {
                candidates.push((
                    segment.end(),
                    if !path.closed && i + 1 == path.segments.len() {
                        SnapKind::Endpoint
                    } else {
                        SnapKind::Vertex
                    },
                ));
            }
            outlines.push((path.flatten()?, path.closed, false));
            Ok(())
        };
        match shape {
            SnapGeometry::Path(path) => add_path(path)?,
            SnapGeometry::Compound(paths) => {
                for path in paths {
                    add_path(path)?;
                }
            }
            SnapGeometry::Line { start, end } => {
                candidates.extend([(*start, SnapKind::Endpoint), (*end, SnapKind::Endpoint)]);
                outlines.push((vec![*start, *end], false, true));
            }
            SnapGeometry::Rectangle { min, max, outline } => {
                candidates.push((lerp(*min, *max, 0.5), SnapKind::Center));
                let points = if let Some(path) = outline {
                    path.flatten()?
                } else {
                    vec![*min, [max[0], min[1]], *max, [min[0], max[1]]]
                };
                candidates.extend(points.iter().map(|&p| (p, SnapKind::Corner)));
                outlines.push((points, true, true));
            }
            SnapGeometry::Ellipse { center, radii } => {
                check(
                    radii.iter().all(|r| r.is_finite() && *r >= 0.),
                    "Invalid snap ellipse radii",
                )?;
                candidates.push((*center, SnapKind::Center));
                candidates.extend(
                    [
                        [center[0] + radii[0], center[1]],
                        [center[0] - radii[0], center[1]],
                        [center[0], center[1] + radii[1]],
                        [center[0], center[1] - radii[1]],
                    ]
                    .map(|p| (p, SnapKind::Corner)),
                );
            }
            SnapGeometry::Polygon {
                points,
                closed,
                outline,
            } => {
                let points = if *closed && let Some(path) = outline {
                    path.flatten()?
                } else {
                    points.clone()
                };
                candidates.extend(points.iter().map(|&p| (p, SnapKind::Vertex)));
                outlines.push((points, *closed, true));
            }
            SnapGeometry::Polyline(points) => {
                if let Some(&p) = points.first() {
                    candidates.push((p, SnapKind::Endpoint));
                }
                if let Some(&p) = points.last() {
                    candidates.push((p, SnapKind::Endpoint));
                }
                outlines.push((points.clone(), false, false));
            }
        }
        for (point, kind) in candidates {
            check(point.iter().all(|x| x.is_finite()), "Non-finite snap point")?;
            consider(point, kind);
        }
        for (mut points, closed, midpoints) in outlines {
            check(points.len() <= 32768, "Snap outline budget exceeded")?;
            check(
                points.iter().flatten().all(|x| x.is_finite()),
                "Non-finite snap outline",
            )?;
            if closed && points.len() >= 2 && points.first() != points.last() {
                points.push(points[0]);
            }
            for edge in points.windows(2) {
                let (a, b) = (edge[0], edge[1]);
                if midpoints {
                    consider(lerp(a, b, 0.5), SnapKind::Midpoint);
                }
                let d = [b[0] - a[0], b[1] - a[1]];
                let len_sq = d[0] * d[0] + d[1] * d[1];
                if len_sq > 1e-20 {
                    let t = ((cursor[0] - a[0]) * d[0] + (cursor[1] - a[1]) * d[1]) / len_sq;
                    if (0.0..=1.0).contains(&t) {
                        consider(lerp(a, b, t), SnapKind::Perpendicular);
                    }
                    // An intersection in range requires both edge bounds to
                    // meet the cursor square; omit remote edges before pairing.
                    if (0..2).all(|i| {
                        a[i].min(b[i]) <= cursor[i] + threshold
                            && a[i].max(b[i]) >= cursor[i] - threshold
                    }) {
                        segments.push((a, b));
                    }
                }
            }
        }
    }
    check(segments.len() <= 4096, "Snap intersection budget exceeded")?;
    for i in 0..segments.len() {
        for j in i + 1..segments.len() {
            if let Some(p) =
                segment_intersection(segments[i].0, segments[i].1, segments[j].0, segments[j].1)
            {
                // Chord junctions are not intersections. Curvex excludes the
                // first/last 1e-4 of either segment from this candidate class.
                let interior = |(a, b): ([f64; 2], [f64; 2])| {
                    let axis = usize::from((b[1] - a[1]).abs() > (b[0] - a[0]).abs());
                    let t = (p[axis] - a[axis]) / (b[axis] - a[axis]);
                    t > 1e-4 && t < 1.0 - 1e-4
                };
                if interior(segments[i]) && interior(segments[j]) {
                    consider(p, SnapKind::Intersection);
                }
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

/// Optional Curvex angular snap: project onto the nearest rotated ray only
/// inside the angular tolerance. Displacements shorter than 2 units do not snap.
pub fn snap_to_rays(
    start: [f64; 2],
    cursor: [f64; 2],
    base: f64,
    count: usize,
    tolerance_degrees: f64,
) -> Result<Option<([f64; 2], f64)>> {
    check(
        start.iter().chain(cursor.iter()).all(|x| x.is_finite()) && base.is_finite(),
        "Non-finite ray snap geometry",
    )?;
    check(
        count > 0
            && count <= 4096
            && tolerance_degrees.is_finite()
            && (0.0..=180.0).contains(&tolerance_degrees),
        "Invalid ray snap options",
    )?;
    let d = [cursor[0] - start[0], cursor[1] - start[1]];
    if d[0] * d[0] + d[1] * d[1] < 4.0 {
        return Ok(None);
    }
    let step = std::f64::consts::TAU / count as f64;
    let relative = d[1].atan2(d[0]) - base;
    let snapped = (relative / step).round() * step;
    let difference = (relative - snapped)
        .sin()
        .atan2((relative - snapped).cos())
        .abs();
    if difference > tolerance_degrees.to_radians() {
        return Ok(None);
    }
    let angle = snapped + base;
    let direction = [angle.cos(), angle.sin()];
    let projection = d[0] * direction[0] + d[1] * direction[1];
    if projection <= 0.0 {
        return Ok(None);
    }
    Ok(Some((
        [
            start[0] + direction[0] * projection,
            start[1] + direction[1] * projection,
        ],
        angle,
    )))
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

fn usable_box(b: BBox) -> bool {
    b.min.iter().chain(b.max.iter()).all(|x| x.is_finite())
        && b.min[0] <= b.max[0]
        && b.min[1] <= b.max[1]
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DragSnapGuide {
    pub vertical: bool,
    pub position: f64,
    pub moving: BBox,
    pub target: BBox,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DragSnapResult {
    pub delta: [f64; 2],
    pub guides: Vec<DragSnapGuide>,
}

/// Curvex drag alignment: first nearest target wins ties, with at most one
/// guide per axis carrying both rectangles for guide extents.
pub fn find_drag_snap(moving: BBox, targets: &[BBox], threshold: f64) -> Result<DragSnapResult> {
    check(
        threshold > 0. && threshold.is_finite(),
        "Invalid guide threshold",
    )?;
    let mut result = DragSnapResult {
        delta: [0., 0.],
        guides: Vec::new(),
    };
    if !usable_box(moving) {
        return Ok(result);
    }
    let mut selected: [Option<BBox>; 2] = [None, None];
    for axis in 0..2 {
        let mut best_distance = f64::INFINITY;
        for edge in [moving.min[axis], moving.center()[axis], moving.max[axis]] {
            for &target in targets.iter().filter(|b| usable_box(**b)) {
                for position in [target.min[axis], target.center()[axis], target.max[axis]] {
                    let d = position - edge;
                    if d.abs() <= threshold && d.abs() < best_distance {
                        best_distance = d.abs();
                        result.delta[axis] = d;
                        selected[axis] = Some(target);
                    }
                }
            }
        }
    }
    let snapped = BBox {
        min: [
            moving.min[0] + result.delta[0],
            moving.min[1] + result.delta[1],
        ],
        max: [
            moving.max[0] + result.delta[0],
            moving.max[1] + result.delta[1],
        ],
    };
    for (axis, target) in selected.into_iter().enumerate() {
        if let Some(target) = target {
            let mut best = (f64::INFINITY, 0.);
            for edge in [snapped.min[axis], snapped.center()[axis], snapped.max[axis]] {
                for position in [target.min[axis], target.center()[axis], target.max[axis]] {
                    if (edge - position).abs() < best.0 {
                        best = ((edge - position).abs(), (edge + position) * 0.5);
                    }
                }
            }
            result.guides.push(DragSnapGuide {
                vertical: axis == 0,
                position: best.1,
                moving: snapped,
                target,
            });
        }
    }
    Ok(result)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DistanceMark {
    pub from: [f64; 2],
    pub to: [f64; 2],
    pub distance: f64,
}

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
