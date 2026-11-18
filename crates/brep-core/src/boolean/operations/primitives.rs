use super::*;
pub fn extrude_polygon(profile: &[[f64; 2]], z_min: f64, z_max: f64) -> Result<Model> {
    extrude_polygon_with_holes(profile, &[], z_min, z_max)
}

pub(crate) fn ring_area(ring: &[[f64; 2]]) -> f64 {
    (0..ring.len())
        .map(|i| {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        / 2.
}

fn planar_segments_intersect(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
    let orient = |p: [f64; 2], q: [f64; 2], r: [f64; 2]| {
        (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])
    };
    let boxes_overlap = a[0].min(b[0]) <= c[0].max(d[0]) + 1e-7
        && c[0].min(d[0]) <= a[0].max(b[0]) + 1e-7
        && a[1].min(b[1]) <= c[1].max(d[1]) + 1e-7
        && c[1].min(d[1]) <= a[1].max(b[1]) + 1e-7;
    boxes_overlap
        && orient(a, b, c) * orient(a, b, d) <= 1e-14
        && orient(c, d, a) * orient(c, d, b) <= 1e-14
}

pub(crate) fn point_in_ring(point: [f64; 2], ring: &[[f64; 2]]) -> bool {
    let mut inside = false;
    for i in 0..ring.len() {
        let a = ring[i];
        let b = ring[(i + 1) % ring.len()];
        if (a[1] > point[1]) != (b[1] > point[1])
            && point[0] < (b[0] - a[0]) * (point[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
    }
    inside
}

fn rings_intersect(a: &[[f64; 2]], b: &[[f64; 2]]) -> bool {
    (0..a.len()).any(|i| {
        (0..b.len()).any(|j| {
            planar_segments_intersect(a[i], a[(i + 1) % a.len()], b[j], b[(j + 1) % b.len()])
        })
    })
}

pub(crate) fn validate_extrusion_ring(ring: &[[f64; 2]], ccw: bool, name: &str) -> Result<()> {
    if ring.len() < 3 || ring.len() > 128 {
        return Err(unsupported(format!("{name} must have 3..128 vertices")));
    }
    if ring
        .iter()
        .flatten()
        .any(|value| !value.is_finite() || value.abs() > 1e6)
    {
        return Err(Error::new(
            "BREP_INVALID_SIZE",
            "Extrusion coordinates must be finite and within 1000000 mm",
        ));
    }
    let area = ring_area(ring);
    if area.abs() <= 1e-12 || area.is_sign_positive() != ccw {
        return Err(unsupported(format!(
            "{name} must be {}",
            if ccw {
                "counter-clockwise"
            } else {
                "clockwise"
            }
        )));
    }
    for i in 0..ring.len() {
        let a = ring[i];
        let b = ring[(i + 1) % ring.len()];
        if (a[0] - b[0]).hypot(a[1] - b[1]) <= 1e-7 {
            return Err(unsupported(format!(
                "{name} has duplicate consecutive vertices"
            )));
        }
        for j in i + 1..ring.len() {
            if j == i || j == (i + 1) % ring.len() || i == (j + 1) % ring.len() {
                continue;
            }
            let c = ring[j];
            let d = ring[(j + 1) % ring.len()];
            if planar_segments_intersect(a, b, c, d) {
                return Err(unsupported(format!("{name} must be simple")));
            }
        }
    }
    Ok(())
}

/// Extrude a simple, possibly concave planar profile with optional clockwise
/// holes into a manifold B-rep with two genuinely trimmed cap faces.
pub fn extrude_polygon_with_holes(
    profile: &[[f64; 2]],
    holes: &[Vec<[f64; 2]>],
    z_min: f64,
    z_max: f64,
) -> Result<Model> {
    validate_extrusion_ring(profile, true, "Extrusion outer profile")?;
    if holes.len() > 16 || profile.len() + holes.iter().map(Vec::len).sum::<usize>() > 512 {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Extrusion supports at most 16 holes and 512 boundary vertices",
        ));
    }
    for (index, hole) in holes.iter().enumerate() {
        validate_extrusion_ring(hole, false, "Extrusion hole")?;
        if !point_in_ring(hole[0], profile)
            || rings_intersect(profile, hole)
            || holes[..index].iter().any(|other| {
                rings_intersect(other, hole)
                    || point_in_ring(hole[0], other)
                    || point_in_ring(other[0], hole)
            })
        {
            return Err(unsupported(
                "Extrusion holes must be disjoint and strictly inside the outer profile",
            ));
        }
    }
    if !z_min.is_finite() || !z_max.is_finite() || z_max <= z_min {
        return Err(Error::new(
            "BREP_INVALID_SIZE",
            "Extrusion requires finite zMax greater than zMin",
        ));
    }
    if z_min.abs() > 1e6 || z_max.abs() > 1e6 {
        return Err(Error::new(
            "BREP_INVALID_SIZE",
            "Extrusion coordinates must be finite and within 1000000 mm",
        ));
    }
    let tolerance = 1e-7;
    let lower: Vec<_> = profile.iter().rev().map(|p| [p[0], p[1], z_min]).collect();
    let upper: Vec<_> = profile.iter().map(|p| [p[0], p[1], z_max]).collect();
    let lower_holes: Vec<_> = holes
        .iter()
        .map(|hole| hole.iter().rev().map(|p| [p[0], p[1], z_min]).collect())
        .collect();
    let upper_holes: Vec<_> = holes
        .iter()
        .map(|hole| hole.iter().map(|p| [p[0], p[1], z_max]).collect())
        .collect();
    let mut polygons = vec![
        PlanarBoundary {
            outer: lower,
            holes: lower_holes,
        },
        PlanarBoundary {
            outer: upper,
            holes: upper_holes,
        },
    ];
    for ring in std::iter::once(profile).chain(holes.iter().map(Vec::as_slice)) {
        for i in 0..ring.len() {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            polygons.push(PlanarBoundary {
                outer: vec![
                    [a[0], a[1], z_min],
                    [b[0], b[1], z_min],
                    [b[0], b[1], z_max],
                    [a[0], a[1], z_max],
                ],
                holes: vec![],
            });
        }
    }
    model_from_trimmed_polygons(polygons, tolerance)
}

pub(crate) fn validate_convex_profile(profile: &[[f64; 2]], name: &str) -> Result<()> {
    if profile.len() < 3 || profile.len() > 128 {
        return Err(unsupported(format!(
            "{name} profile must have 3..128 vertices"
        )));
    }
    if profile
        .iter()
        .flatten()
        .any(|value| !value.is_finite() || value.abs() > 1e6)
    {
        return Err(Error::new(
            "BREP_INVALID_SIZE",
            format!("{name} coordinates must be finite and within 1000000 mm"),
        ));
    }
    validate_convex_boundary(profile, name)
}

// Use after dimensional and world-coordinate admission. Rigid local placement
// can exceed the world-coordinate bound without making the input inadmissible.
fn validate_convex_boundary(profile: &[[f64; 2]], name: &str) -> Result<()> {
    let tolerance = 1e-7;
    // Every nonincident vertex must lie strictly inside every oriented support.
    // Adjacent turn signs alone admit multiply wound stars and repeated loops.
    for i in 0..profile.len() {
        let next = (i + 1) % profile.len();
        let a = profile[i];
        let b = profile[next];
        for (j, p) in profile.iter().enumerate() {
            if j == i || j == next {
                continue;
            }
            let side = (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
            if !side.is_finite() || side <= tolerance {
                return Err(unsupported(format!(
                    "{name} currently requires a simple strictly convex CCW profile"
                )));
            }
        }
    }
    Ok(())
}

fn triangulated_strip(sections: &[Vec<[f64; 3]>]) -> Vec<Vec<[f64; 3]>> {
    let count = sections[0].len();
    let mut polygons = vec![
        sections[0].iter().rev().copied().collect(),
        sections.last().unwrap().clone(),
    ];
    for pair in sections.windows(2) {
        for i in 0..count {
            let next = (i + 1) % count;
            polygons.push(vec![pair[0][i], pair[0][next], pair[1][next]]);
            polygons.push(vec![pair[0][i], pair[1][next], pair[1][i]]);
        }
    }
    polygons
}

/// Loft strictly convex, consistently oriented parallel sections into a faceted B-rep.
///
/// Side quads are triangulated because arbitrary corresponding section edges
/// need not be coplanar. This is planar construction, not a smooth NURBS loft.
pub fn faceted_loft(sections: &[Vec<[f64; 3]>]) -> Result<Model> {
    admit_loft_sections(sections)?;
    model_from_polygons(triangulated_strip(sections), 1e-7)
}

pub(crate) fn admit_loft_sections(sections: &[Vec<[f64; 3]>]) -> Result<()> {
    if sections.len() < 2 || sections.len() > 64 {
        return Err(unsupported("Loft requires 2..64 sections"));
    }
    let count = sections[0].len();
    if !(3..=128).contains(&count) || sections.iter().any(|section| section.len() != count) {
        return Err(unsupported(
            "Loft sections must have the same 3..128 vertex count",
        ));
    }
    if sections
        .iter()
        .flatten()
        .flatten()
        .any(|value| !value.is_finite() || value.abs() > 1e6)
    {
        return Err(Error::new(
            "BREP_INVALID_SIZE",
            "Loft coordinates must be finite and within 1000000 mm",
        ));
    }
    let origin = sections[0][0];
    let u = unit(sub(sections[0][1], origin))?;
    let normal = unit(cross(u, sub(sections[0][2], origin)))?;
    let v = cross(normal, u);
    let mut previous_height = f64::NEG_INFINITY;
    for section in sections {
        let height = dot(sub(section[0], origin), normal);
        if section
            .iter()
            .any(|point| (dot(sub(*point, origin), normal) - height).abs() > 1e-7)
            || height <= previous_height + 1e-7
        {
            return Err(unsupported(
                "Loft sections must be parallel planar profiles ordered along their oriented normal",
            ));
        }
        let profile: Vec<_> = section
            .iter()
            .map(|point| {
                let p = sub(*point, origin);
                [dot(p, u), dot(p, v)]
            })
            .collect();
        validate_convex_boundary(&profile, "Loft")?;
        previous_height = height;
    }
    Ok(())
}

/// Sweep a strictly convex CCW profile along a polyline with transported frames.
///
/// Every side patch is triangulated and planar. The path is sampled exactly as
/// authored; no analytic pipe or smooth transition is claimed.
pub fn faceted_sweep(profile: &[[f64; 2]], path: &[[f64; 3]], up: [f64; 3]) -> Result<Model> {
    validate_convex_profile(profile, "Sweep")?;
    if path.len() < 2 || path.len() > 64 {
        return Err(unsupported("Sweep path must have 2..64 points"));
    }
    if path
        .iter()
        .flatten()
        .chain(up.iter())
        .any(|value| !value.is_finite() || value.abs() > 1e6)
    {
        return Err(Error::new(
            "BREP_INVALID_SIZE",
            "Sweep coordinates must be finite and within 1000000 mm",
        ));
    }
    let tolerance = 1e-7;
    let segments: Vec<_> = path
        .windows(2)
        .map(|pair| unit(sub(pair[1], pair[0])))
        .collect::<Result<_>>()?;
    let mut sections = Vec::with_capacity(path.len());
    let mut previous_u: Option<[f64; 3]> = None;
    for i in 0..path.len() {
        let tangent = if i == 0 {
            segments[0]
        } else if i + 1 == path.len() {
            segments[i - 1]
        } else {
            unit(add(segments[i - 1], segments[i]))
                .map_err(|_| unsupported("Sweep path contains a 180 degree reversal"))?
        };
        let projected_up = sub(up, mul(tangent, dot(up, tangent)));
        let mut v = unit(projected_up)
            .map_err(|_| unsupported("Sweep up vector must not be parallel to the path"))?;
        let mut u = unit(cross(v, tangent))?;
        if let Some(previous) = previous_u
            && dot(previous, u) < 0.
        {
            u = mul(u, -1.);
            v = mul(v, -1.);
        }
        previous_u = Some(u);
        sections.push(
            profile
                .iter()
                .map(|point| add(path[i], add(mul(u, point[0]), mul(v, point[1]))))
                .collect(),
        );
    }
    model_from_polygons(triangulated_strip(&sections), tolerance)
}

/// Revolve a closed `(radius, z)` profile around Z as a faceted planar B-rep.
///
/// A full turn is required. Radius-zero profile vertices are supported as
/// poles; negative radii and analytic cylindrical/spherical claims are not.
pub fn faceted_revolve(profile: &[[f64; 2]], segments: usize) -> Result<Model> {
    validate_convex_profile(profile, "Revolve")?;
    if !(3..=128).contains(&segments) {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Faceted revolve segments must be 3..128",
        ));
    }
    if profile.iter().any(|point| point[0] < 0.) || profile.iter().all(|point| point[0] <= 1e-7) {
        return Err(unsupported(
            "Revolve profile radii must be nonnegative with positive extent",
        ));
    }
    let rings: Vec<Vec<_>> = (0..segments)
        .map(|segment| {
            let angle = std::f64::consts::TAU * segment as f64 / segments as f64;
            profile
                .iter()
                .map(|point| [point[0] * angle.cos(), point[0] * angle.sin(), point[1]])
                .collect()
        })
        .collect();
    let mut polygons = vec![];
    for segment in 0..segments {
        let next_segment = (segment + 1) % segments;
        for i in 0..profile.len() {
            let next = (i + 1) % profile.len();
            let a = rings[segment][i];
            let b = rings[next_segment][i];
            let c = rings[next_segment][next];
            let d = rings[segment][next];
            if !close(a, b, 1e-7) && !close(b, c, 1e-7) {
                polygons.push(vec![a, b, c]);
            }
            if !close(a, c, 1e-7) && !close(c, d, 1e-7) {
                polygons.push(vec![a, c, d]);
            }
        }
    }
    model_from_polygons(polygons, 1e-7)
}

/// Construct a declared faceted cylindrical B-rep with planar side faces.
pub fn faceted_cylinder(radius: f64, height: f64, segments: usize) -> Result<Model> {
    if !radius.is_finite() || !height.is_finite() || radius < 0.01 || height < 0.01 {
        return Err(Error::new(
            "BREP_INVALID_SIZE",
            "Faceted cylinder radius and height must be at least 0.01 mm",
        ));
    }
    if !(3..=128).contains(&segments) {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Faceted cylinder segments must be 3..128",
        ));
    }
    let profile: Vec<_> = (0..segments)
        .map(|i| {
            let angle = std::f64::consts::TAU * i as f64 / segments as f64;
            [radius * angle.cos(), radius * angle.sin()]
        })
        .collect();
    extrude_polygon(&profile, 0., height)
}

/// Construct an honest faceted sphere: every patch is a planar B-rep face.
pub fn faceted_sphere(
    radius: f64,
    radial_segments: usize,
    latitude_segments: usize,
) -> Result<Model> {
    if !radius.is_finite() || radius < 0.01 {
        return Err(Error::new(
            "BREP_INVALID_SIZE",
            "Faceted sphere radius must be at least 0.01 mm",
        ));
    }
    if !(3..=32).contains(&radial_segments) || !(2..=16).contains(&latitude_segments) {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Faceted sphere segments must be radial 3..32 and latitude 2..16",
        ));
    }
    let ring = |latitude: usize, radial: usize| {
        let phi = std::f64::consts::PI * latitude as f64 / latitude_segments as f64;
        let theta = std::f64::consts::TAU * radial as f64 / radial_segments as f64;
        [
            radius * phi.sin() * theta.cos(),
            radius * phi.sin() * theta.sin(),
            radius * phi.cos(),
        ]
    };
    let mut polygons = vec![];
    let top = [0., 0., radius];
    let bottom = [0., 0., -radius];
    for radial in 0..radial_segments {
        let next = (radial + 1) % radial_segments;
        polygons.push(vec![top, ring(1, radial), ring(1, next)]);
        for latitude in 1..latitude_segments - 1 {
            let a = ring(latitude, radial);
            let b = ring(latitude + 1, radial);
            let c = ring(latitude + 1, next);
            let d = ring(latitude, next);
            polygons.push(vec![a, b, c]);
            polygons.push(vec![a, c, d]);
        }
        polygons.push(vec![
            bottom,
            ring(latitude_segments - 1, next),
            ring(latitude_segments - 1, radial),
        ]);
    }
    model_from_polygons(polygons, 1e-7)
}
