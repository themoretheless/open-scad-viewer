use super::*;

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

pub(super) use crate::dist;
use crate::lerp;

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

pub(super) fn usable_box(b: BBox) -> bool {
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
