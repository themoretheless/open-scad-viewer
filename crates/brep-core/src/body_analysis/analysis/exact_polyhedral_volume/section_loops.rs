use super::{exact_polyhedral_transverse_section, refuse, spend};
use crate::{Result, closed_solid::QualifiedClosedSolid};
use num_rational::BigRational as Q;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolyhedralSectionLoop {
    /// Ordered vertices, without repeating the first vertex at the end.
    pub points_mm: Vec<[Q; 3]>,
    /// Source face traversed from vertex i to the next vertex in the cycle.
    pub faces: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct ExactPolyhedralSectionLoops {
    pub loops: Vec<PolyhedralSectionLoop>,
    pub exact_work: u64,
}

struct Node {
    point: [Q; 3],
    segments: Vec<usize>,
}

struct Segment {
    ends: [usize; 2],
    face: usize,
}

/// Recover all loops of a strict transverse section. Endpoints match by exact
/// rational coordinates. Only disjoint cycles are accepted, and source face
/// attribution is preserved for every segment. Loop winding is arbitrary.
pub fn exact_polyhedral_section_loops(
    solid: &QualifiedClosedSolid,
    normal: [f64; 3],
    offset: f64,
    max_exact_work: u64,
) -> Result<ExactPolyhedralSectionLoops> {
    let section = exact_polyhedral_transverse_section(solid, normal, offset, max_exact_work)?;
    let mut remaining = max_exact_work - section.exact_work;
    let mut nodes = Vec::<Node>::new();
    let mut indices = BTreeMap::new();
    let mut segments = Vec::new();
    for face in section.faces {
        for segment in face.segments_mm {
            let comparisons = (nodes.len() as u64 + 2).ilog2() + 1;
            spend(&mut remaining, 8 + 16 * u64::from(comparisons))?;
            let ends = segment.map(|point| {
                *indices.entry(point.clone()).or_insert_with(|| {
                    let index = nodes.len();
                    nodes.push(Node {
                        point,
                        segments: Vec::with_capacity(2),
                    });
                    index
                })
            });
            if ends[0] == ends[1] {
                return Err(refuse("Section loop contains a collapsed segment"));
            }
            for id in ends {
                let incident = &mut nodes[id].segments;
                if incident.len() == 2 {
                    return Err(refuse(
                        "Section endpoint has more than two incident segments",
                    ));
                }
                incident.push(segments.len());
            }
            segments.push(Segment {
                ends,
                face: face.face,
            });
        }
    }
    if nodes.iter().any(|node| node.segments.len() != 2) {
        return Err(refuse("Section segments contain an open chain"));
    }

    let mut loops = Vec::new();
    let mut visited = vec![false; segments.len()];
    for first in 0..segments.len() {
        if visited[first] {
            continue;
        }
        let start = segments[first].ends[0];
        let mut current = start;
        let mut next = first;
        let mut points_mm = Vec::new();
        let mut faces = Vec::new();
        loop {
            spend(&mut remaining, 8)?;
            if visited[next] {
                return Err(refuse("Section cycle revisits a segment"));
            }
            visited[next] = true;
            let segment = &segments[next];
            points_mm.push(nodes[current].point.clone());
            faces.push(segment.face);
            current = segment.ends[usize::from(segment.ends[0] == current)];
            if current == start {
                break;
            }
            let incident = &nodes[current].segments;
            next = incident[usize::from(incident[0] == next)];
        }
        if points_mm.len() < 3 {
            return Err(refuse("Section cycle needs at least three vertices"));
        }
        loops.push(PolyhedralSectionLoop { points_mm, faces });
    }
    Ok(ExactPolyhedralSectionLoops {
        loops,
        exact_work: max_exact_work - remaining,
    })
}

#[derive(Clone, Debug)]
pub struct ExactPolyhedralSectionArea {
    /// Squared material area in the original 3D plane.
    pub area_squared_mm4: Q,
    /// Nesting depths in the same order as the returned section loops.
    pub depths: Vec<usize>,
    pub loops: Vec<PolyhedralSectionLoop>,
    pub exact_work: u64,
}

/// Measure the material area of a strict transverse section, subtracting
/// holes and adding islands. The area is the nonnegative square root of the
/// returned rational value.
pub fn exact_polyhedral_section_area(
    solid: &QualifiedClosedSolid,
    normal: [f64; 3],
    offset: f64,
    max_exact_work: u64,
) -> Result<ExactPolyhedralSectionArea> {
    measure_section(solid, normal, offset, max_exact_work, false).map(|report| report.section)
}

#[derive(Clone, Debug)]
pub struct ExactPolyhedralSectionCentroid {
    pub section: ExactPolyhedralSectionArea,
    /// Center of the retained material area in world coordinates.
    /// An empty section has no centroid.
    pub centroid_mm: Option<[Q; 3]>,
}

/// Compute the exact area centroid of a strict transverse section. The
/// common metric scale of the planar projection cancels in the first moments.
pub fn exact_polyhedral_section_centroid(
    solid: &QualifiedClosedSolid,
    normal: [f64; 3],
    offset: f64,
    max_exact_work: u64,
) -> Result<ExactPolyhedralSectionCentroid> {
    measure_section(solid, normal, offset, max_exact_work, true)
}

fn measure_section(
    solid: &QualifiedClosedSolid,
    normal: [f64; 3],
    offset: f64,
    max_exact_work: u64,
    compute_centroid: bool,
) -> Result<ExactPolyhedralSectionCentroid> {
    use num_traits::{Signed, Zero};

    let section = exact_polyhedral_section_loops(solid, normal, offset, max_exact_work)?;
    let mut remaining = max_exact_work - section.exact_work;
    let axes = projected_axes(normal);
    let omitted = (axes[0] + 2) % 3;
    let mut polygons = Vec::with_capacity(section.loops.len());
    for ring in &section.loops {
        spend(&mut remaining, ring.points_mm.len() as u64)?;
        polygons.push(
            ring.points_mm
                .iter()
                .map(|p| axes.map(|i| p[i].clone()))
                .collect::<Vec<_>>(),
        );
    }

    let mut depths = Vec::with_capacity(polygons.len());
    let mut twice_area = Q::zero();
    let mut moments = [Q::zero(), Q::zero()];
    for (index, ring) in polygons.iter().enumerate() {
        let mut signed_area = Q::zero();
        let mut first = [Q::zero(), Q::zero()];
        for (a, b) in edges(ring) {
            spend(&mut remaining, if compute_centroid { 11 } else { 5 })?;
            let cross = &a[0] * &b[1] - &b[0] * &a[1];
            if compute_centroid {
                for axis in 0..2 {
                    first[axis] += &cross * (&a[axis] + &b[axis]);
                }
            }
            signed_area += cross;
        }
        if signed_area.is_zero() {
            return Err(refuse("Section ring has zero projected area"));
        }
        let mut depth = 0;
        for (other, boundary) in polygons.iter().enumerate() {
            if other != index && contains_point(boundary, &ring[0], &mut remaining)? {
                depth += 1;
            }
        }
        let material = depth % 2 == 0;
        if material {
            twice_area += signed_area.abs();
        } else {
            twice_area -= signed_area.abs();
        }
        if compute_centroid {
            spend(&mut remaining, 4)?;
            for (total, moment) in moments.iter_mut().zip(first) {
                if material != signed_area.is_negative() {
                    *total += moment;
                } else {
                    *total -= moment;
                }
            }
        }
        depths.push(depth);
    }

    spend(&mut remaining, 16)?;
    let normal = super::lift(&normal);
    let norm_squared: Q = normal.iter().map(|n| n * n).sum();
    let area_squared_mm4 =
        twice_area.pow(2) * norm_squared / (Q::from_integer(4.into()) * normal[omitted].pow(2));
    let centroid_mm = if compute_centroid && !polygons.is_empty() {
        if !twice_area.is_positive() {
            return Err(refuse("Section material area must be positive"));
        }
        spend(&mut remaining, 16)?;
        let denominator = twice_area * Q::from_integer(3.into());
        let mut point = std::array::from_fn(|_| Q::zero());
        for (axis, moment) in axes.into_iter().zip(moments) {
            point[axis] = moment / &denominator;
        }
        let retained = &normal[axes[0]] * &point[axes[0]] + &normal[axes[1]] * &point[axes[1]];
        point[omitted] = (Q::from_float(offset).unwrap() - retained) / &normal[omitted];
        Some(point)
    } else {
        None
    };
    Ok(ExactPolyhedralSectionCentroid {
        section: ExactPolyhedralSectionArea {
            area_squared_mm4,
            depths,
            loops: section.loops,
            exact_work: max_exact_work - remaining,
        },
        centroid_mm,
    })
}

fn edges(ring: &[[Q; 2]]) -> impl Iterator<Item = (&[Q; 2], &[Q; 2])> {
    ring.iter().zip(ring.iter().cycle().skip(1))
}

fn contains_point(ring: &[[Q; 2]], [x, y]: &[Q; 2], remaining: &mut u64) -> Result<bool> {
    let mut inside = false;
    for (a, b) in edges(ring) {
        spend(remaining, 10)?;
        if (&a[1] > y) != (&b[1] > y) {
            let intersection = &a[0] + (y - &a[1]) * (&b[0] - &a[0]) / (&b[1] - &a[1]);
            if &intersection > x {
                inside = !inside;
            }
        }
    }
    Ok(inside)
}

#[derive(Clone, Debug)]
pub struct BoundedPolyhedralSectionPerimeter {
    /// Outward rational bounds for the sum of all contour lengths.
    pub bounds_mm: [Q; 2],
    pub loops: Vec<PolyhedralSectionLoop>,
    pub exact_work: u64,
}

/// Bound the total perimeter of a strict transverse section. Both material
/// exteriors and hole boundaries contribute their full length.
pub fn bounded_polyhedral_section_perimeter(
    solid: &QualifiedClosedSolid,
    normal: [f64; 3],
    offset: f64,
    max_width_mm: f64,
    max_exact_work: u64,
) -> Result<BoundedPolyhedralSectionPerimeter> {
    let width = super::enclosure_width(max_width_mm)?;
    let section = exact_polyhedral_section_loops(solid, normal, offset, max_exact_work)?;
    let mut remaining = max_exact_work - section.exact_work;
    let mut squares = Vec::new();
    for ring in &section.loops {
        let points = &ring.points_mm;
        for (a, b) in points.iter().zip(points.iter().cycle().skip(1)) {
            spend(&mut remaining, 9)?;
            squares.push(a.iter().zip(b).map(|(a, b)| (a - b).pow(2)).sum());
        }
    }
    let bounds_mm = super::bound_root_sum(squares, width, &mut remaining)?;
    Ok(BoundedPolyhedralSectionPerimeter {
        bounds_mm,
        loops: section.loops,
        exact_work: max_exact_work - remaining,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolyhedralSectionPointLocation {
    OffPlane,
    Outside,
    Material,
    Boundary {
        ring: usize,
        segment: usize,
        face: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPolyhedralSectionPoint {
    pub location: PolyhedralSectionPointLocation,
    pub exact_work: u64,
}

/// Classify an exact world-space point against a strict transverse material
/// section. Interior and exterior use even-odd membership across all loops.
/// Boundary hits retain an incident section segment and its source face.
pub fn exact_polyhedral_section_point(
    solid: &QualifiedClosedSolid,
    normal: [f64; 3],
    offset: f64,
    point_mm: &[Q; 3],
    max_exact_work: u64,
) -> Result<ExactPolyhedralSectionPoint> {
    validate_query_point(point_mm)?;
    let section = exact_polyhedral_section_loops(solid, normal, offset, max_exact_work)?;
    let mut remaining = max_exact_work - section.exact_work;
    spend(&mut remaining, 16)?;
    let exact_normal = super::lift(&normal);
    let projected: Q = exact_normal.iter().zip(point_mm).map(|(n, p)| n * p).sum();
    let location = if projected == Q::from_float(offset).unwrap() {
        classify_section_loops(
            &section.loops,
            projected_axes(normal),
            point_mm,
            &mut remaining,
        )?
    } else {
        PolyhedralSectionPointLocation::OffPlane
    };
    Ok(ExactPolyhedralSectionPoint {
        location,
        exact_work: max_exact_work - remaining,
    })
}

fn projected_axes(normal: [f64; 3]) -> [usize; 2] {
    let omitted = (0..3)
        .max_by(|&i, &j| normal[i].abs().total_cmp(&normal[j].abs()))
        .unwrap();
    [(omitted + 1) % 3, (omitted + 2) % 3]
}

fn classify_section_loops(
    loops: &[PolyhedralSectionLoop],
    [u, v]: [usize; 2],
    point: &[Q; 3],
    remaining: &mut u64,
) -> Result<PolyhedralSectionPointLocation> {
    use num_traits::{Signed, Zero};

    let [x, y] = [&point[u], &point[v]];
    let mut inside = false;
    for (ring, contour) in loops.iter().enumerate() {
        let points = &contour.points_mm;
        for (segment, (a, b)) in points.iter().zip(points.iter().cycle().skip(1)).enumerate() {
            spend(remaining, 12)?;
            let side = (&b[u] - &a[u]) * (y - &a[v]) - (&b[v] - &a[v]) * (x - &a[u]);
            if side.is_zero()
                && (std::cmp::min(&a[u], &b[u]) <= x && x <= std::cmp::max(&a[u], &b[u]))
                && (std::cmp::min(&a[v], &b[v]) <= y && y <= std::cmp::max(&a[v], &b[v]))
            {
                return Ok(PolyhedralSectionPointLocation::Boundary {
                    ring,
                    segment,
                    face: contour.faces[segment],
                });
            }
            if (&a[v] > y) != (&b[v] > y) && side.is_positive() == (b[v] > a[v]) {
                inside = !inside;
            }
        }
    }
    Ok(if inside {
        PolyhedralSectionPointLocation::Material
    } else {
        PolyhedralSectionPointLocation::Outside
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolyhedralSectionNearestPoint {
    pub distance_squared_mm2: Q,
    pub point_mm: [Q; 3],
    pub ring: usize,
    pub segment: usize,
    pub face: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPolyhedralSectionDistance {
    /// Nearest contour witness, or no witness for an empty section.
    pub nearest: Option<PolyhedralSectionNearestPoint>,
    pub exact_work: u64,
}

/// Find the nearest point on any contour of a strict transverse section.
/// This measures Euclidean distance in world space, including for off-plane
/// queries. Exact ties keep the first segment in section traversal order.
pub fn exact_polyhedral_section_distance(
    solid: &QualifiedClosedSolid,
    normal: [f64; 3],
    offset: f64,
    point_mm: &[Q; 3],
    max_exact_work: u64,
) -> Result<ExactPolyhedralSectionDistance> {
    validate_query_point(point_mm)?;
    let section = exact_polyhedral_section_loops(solid, normal, offset, max_exact_work)?;
    let mut remaining = max_exact_work - section.exact_work;
    let nearest = nearest_contour(&section.loops, point_mm, &mut remaining)?;
    Ok(ExactPolyhedralSectionDistance {
        nearest,
        exact_work: max_exact_work - remaining,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolyhedralSectionMaterialWitness {
    Interior {
        point_mm: [Q; 3],
        distance_squared_mm2: Q,
    },
    Boundary(PolyhedralSectionNearestPoint),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPolyhedralSectionMaterialDistance {
    /// No witness when the section is empty.
    pub nearest: Option<PolyhedralSectionMaterialWitness>,
    pub exact_work: u64,
}

/// Find the nearest point in the closed material region of a transverse
/// section. For an off-plane query, exact orthogonal projection is tested
/// before resolving the nearest contour, including hole boundaries.
pub fn exact_polyhedral_section_material_distance(
    solid: &QualifiedClosedSolid,
    normal: [f64; 3],
    offset: f64,
    point_mm: &[Q; 3],
    max_exact_work: u64,
) -> Result<ExactPolyhedralSectionMaterialDistance> {
    validate_query_point(point_mm)?;
    let section = exact_polyhedral_section_loops(solid, normal, offset, max_exact_work)?;
    let mut remaining = max_exact_work - section.exact_work;
    let nearest = if section.loops.is_empty() {
        None
    } else {
        spend(&mut remaining, 32)?;
        let n = super::lift(&normal);
        let norm_squared: Q = n.iter().map(|value| value * value).sum();
        let residual = n
            .iter()
            .zip(point_mm)
            .map(|(axis, point)| axis * point)
            .sum::<Q>()
            - Q::from_float(offset).unwrap();
        let scale = &residual / &norm_squared;
        let projected = std::array::from_fn(|i| &point_mm[i] - &scale * &n[i]);
        let location = classify_section_loops(
            &section.loops,
            projected_axes(normal),
            &projected,
            &mut remaining,
        )?;
        let plane_distance = &residual * scale;
        match location {
            PolyhedralSectionPointLocation::Material => {
                Some(PolyhedralSectionMaterialWitness::Interior {
                    point_mm: projected,
                    distance_squared_mm2: plane_distance,
                })
            }
            PolyhedralSectionPointLocation::Boundary {
                ring,
                segment,
                face,
            } => Some(PolyhedralSectionMaterialWitness::Boundary(
                PolyhedralSectionNearestPoint {
                    distance_squared_mm2: plane_distance,
                    point_mm: projected,
                    ring,
                    segment,
                    face,
                },
            )),
            PolyhedralSectionPointLocation::Outside => {
                nearest_contour(&section.loops, point_mm, &mut remaining)?
                    .map(PolyhedralSectionMaterialWitness::Boundary)
            }
            PolyhedralSectionPointLocation::OffPlane => {
                unreachable!("classified in plane coordinates")
            }
        }
    };
    Ok(ExactPolyhedralSectionMaterialDistance {
        nearest,
        exact_work: max_exact_work - remaining,
    })
}

fn nearest_contour(
    loops: &[PolyhedralSectionLoop],
    point_mm: &[Q; 3],
    remaining: &mut u64,
) -> Result<Option<PolyhedralSectionNearestPoint>> {
    use num_traits::{One, Zero};

    let mut nearest: Option<PolyhedralSectionNearestPoint> = None;
    for (ring, contour) in loops.iter().enumerate() {
        let points = &contour.points_mm;
        for (segment, (a, b)) in points.iter().zip(points.iter().cycle().skip(1)).enumerate() {
            spend(remaining, 40)?;
            let delta: [Q; 3] = std::array::from_fn(|i| &b[i] - &a[i]);
            let length_squared: Q = delta.iter().map(|d| d * d).sum();
            let projection: Q = delta
                .iter()
                .enumerate()
                .map(|(i, d)| d * (&point_mm[i] - &a[i]))
                .sum();
            let t = (projection / length_squared).clamp(Q::zero(), Q::one());
            let closest: [Q; 3] = std::array::from_fn(|i| &a[i] + &t * &delta[i]);
            let distance_squared_mm2: Q = point_mm
                .iter()
                .zip(&closest)
                .map(|(p, c)| (p - c).pow(2))
                .sum();
            if nearest
                .as_ref()
                .is_none_or(|candidate| distance_squared_mm2 < candidate.distance_squared_mm2)
            {
                nearest = Some(PolyhedralSectionNearestPoint {
                    distance_squared_mm2,
                    point_mm: closest,
                    ring,
                    segment,
                    face: contour.faces[segment],
                });
            }
        }
    }
    Ok(nearest)
}

fn validate_query_point(point_mm: &[Q; 3]) -> Result<()> {
    if point_mm
        .iter()
        .any(|coordinate| coordinate.numer().bits().max(coordinate.denom().bits()) > 8192)
    {
        return Err(refuse(
            "Section query coordinates exceed the exact bit limit",
        ));
    }
    Ok(())
}
