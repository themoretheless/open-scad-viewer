use crate::{Result, Sketch, profile_error};

/// Validated polygonal profile. Loop zero is the outer boundary; remaining loops are holes.
/// Indices refer to the original sketch and no coordinates are snapped or modified.
#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedProfile {
    pub loops: Vec<Vec<usize>>,
    pub signed_outer_area: f64,
    pub signed_hole_areas: Vec<f64>,
}

fn signed_area(points: &[[f64; 2]]) -> f64 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(a, b)| a[0] * b[1] - a[1] * b[0])
        .sum::<f64>()
        * 0.5
}

fn orientation(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

fn on_segment(a: [f64; 2], b: [f64; 2], p: [f64; 2], tolerance: f64) -> bool {
    orientation(a, b, p).abs() <= tolerance * crate::length(crate::subtract(b, a)).max(1.0)
        && p[0] >= a[0].min(b[0]) - tolerance
        && p[0] <= a[0].max(b[0]) + tolerance
        && p[1] >= a[1].min(b[1]) - tolerance
        && p[1] <= a[1].max(b[1]) + tolerance
}

fn segments_intersect(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2], tolerance: f64) -> bool {
    let o1 = orientation(a, b, c);
    let o2 = orientation(a, b, d);
    let o3 = orientation(c, d, a);
    let o4 = orientation(c, d, b);
    if ((o1 > 0.0 && o2 < 0.0) || (o1 < 0.0 && o2 > 0.0))
        && ((o3 > 0.0 && o4 < 0.0) || (o3 < 0.0 && o4 > 0.0))
    {
        return true;
    }
    on_segment(a, b, c, tolerance)
        || on_segment(a, b, d, tolerance)
        || on_segment(c, d, a, tolerance)
        || on_segment(c, d, b, tolerance)
}

fn point_inside(point: [f64; 2], polygon: &[[f64; 2]]) -> bool {
    let mut inside = false;
    for (a, b) in polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .take(polygon.len())
    {
        if (a[1] > point[1]) != (b[1] > point[1]) {
            let crossing_x = a[0] + (point[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1]);
            if crossing_x > point[0] {
                inside = !inside;
            }
        }
    }
    inside
}

/// Validates closure, non-degeneracy, self-intersection, loop intersections and hole containment
/// before a caller creates an extrusion. This function never modifies the sketch.
pub fn validate_profile(
    sketch: &Sketch,
    loops: &[Vec<usize>],
    tolerance: f64,
) -> Result<ValidatedProfile> {
    crate::validate_tolerance(tolerance)?;
    if loops.is_empty() || loops.len() > 256 {
        return Err(profile_error(
            "Profile requires one outer loop and at most 255 holes",
        ));
    }
    if loops.iter().map(Vec::len).sum::<usize>() > 513 {
        return Err(profile_error("Profile point budget exceeded"));
    }
    let mut normalized = Vec::with_capacity(loops.len());
    let mut coordinates = Vec::with_capacity(loops.len());
    for (loop_index, indices) in loops.iter().enumerate() {
        let mut indices = indices.clone();
        if indices.len() >= 2 && indices.first() == indices.last() {
            indices.pop();
        }
        if indices.len() < 3 {
            return Err(profile_error(format!(
                "Profile loop {loop_index} has fewer than 3 vertices"
            )));
        }
        let mut points = Vec::with_capacity(indices.len());
        for &index in &indices {
            let Some(&point) = sketch.points.get(index) else {
                return Err(profile_error(format!(
                    "Profile loop {loop_index} has invalid point {index}"
                )));
            };
            if !point.iter().all(|v| v.is_finite() && v.abs() <= 1e6) {
                return Err(profile_error(format!(
                    "Profile loop {loop_index} has non-finite coordinates"
                )));
            }
            points.push(point);
        }
        for (index, (a, b)) in points
            .iter()
            .zip(points.iter().cycle().skip(1))
            .take(points.len())
            .enumerate()
        {
            let length = crate::length(crate::subtract(*a, *b));
            let is_closure = index + 1 == points.len();
            if length <= 1e-12 || (!is_closure && length <= tolerance * 1e-6) {
                return Err(profile_error(format!(
                    "Profile loop {loop_index} has a zero-length edge"
                )));
            }
        }
        for i in 0..points.len() {
            for j in (i + 1)..points.len() {
                if crate::length(crate::subtract(points[i], points[j])) <= tolerance {
                    return Err(profile_error(format!(
                        "Profile loop {loop_index} repeats a vertex"
                    )));
                }
                let adjacent = j == i + 1 || (i == 0 && j + 1 == points.len());
                if !adjacent
                    && segments_intersect(
                        points[i],
                        points[(i + 1) % points.len()],
                        points[j],
                        points[(j + 1) % points.len()],
                        tolerance,
                    )
                {
                    return Err(profile_error(format!(
                        "Profile loop {loop_index} self-intersects"
                    )));
                }
            }
        }
        let area = signed_area(&points);
        if !area.is_finite() || area.abs() <= (tolerance * tolerance).max(1e-24) {
            return Err(profile_error(format!(
                "Profile loop {loop_index} has zero area"
            )));
        }
        normalized.push(indices);
        coordinates.push(points);
    }

    for hole in 1..coordinates.len() {
        for (a, b) in coordinates[hole]
            .iter()
            .zip(coordinates[hole].iter().cycle().skip(1))
            .take(coordinates[hole].len())
        {
            for (outer, outer_points) in coordinates.iter().enumerate().take(hole) {
                for (c, d) in outer_points
                    .iter()
                    .zip(outer_points.iter().cycle().skip(1))
                    .take(outer_points.len())
                {
                    if segments_intersect(*a, *b, *c, *d, tolerance) {
                        return Err(profile_error(format!(
                            "Profile loops {outer} and {hole} intersect"
                        )));
                    }
                }
            }
        }
        if !point_inside(coordinates[hole][0], &coordinates[0]) {
            return Err(profile_error(format!(
                "Profile hole {hole} lies outside the outer loop"
            )));
        }
        for previous_hole in 1..hole {
            if point_inside(coordinates[hole][0], &coordinates[previous_hole])
                || point_inside(coordinates[previous_hole][0], &coordinates[hole])
            {
                return Err(profile_error(
                    "Nested or overlapping profile holes are not supported",
                ));
            }
        }
    }

    Ok(ValidatedProfile {
        loops: normalized,
        signed_outer_area: signed_area(&coordinates[0]),
        signed_hole_areas: coordinates[1..]
            .iter()
            .map(|points| signed_area(points))
            .collect(),
    })
}
