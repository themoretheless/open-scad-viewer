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
    use num_traits::{Signed, Zero};

    let section = exact_polyhedral_section_loops(solid, normal, offset, max_exact_work)?;
    let mut remaining = max_exact_work - section.exact_work;
    let omitted = (0..3)
        .max_by(|&i, &j| normal[i].abs().total_cmp(&normal[j].abs()))
        .unwrap();
    let axes = [(omitted + 1) % 3, (omitted + 2) % 3];
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
    for (index, ring) in polygons.iter().enumerate() {
        spend(&mut remaining, 5 * ring.len() as u64)?;
        let unsigned_area = edges(ring)
            .map(|(a, b)| &a[0] * &b[1] - &b[0] * &a[1])
            .sum::<Q>()
            .abs();
        if unsigned_area.is_zero() {
            return Err(refuse("Section ring has zero projected area"));
        }
        let mut depth = 0;
        for (other, boundary) in polygons.iter().enumerate() {
            if other != index && contains_point(boundary, &ring[0], &mut remaining)? {
                depth += 1;
            }
        }
        if depth % 2 == 0 {
            twice_area += unsigned_area;
        } else {
            twice_area -= unsigned_area;
        }
        depths.push(depth);
    }

    spend(&mut remaining, 16)?;
    let normal = super::lift(&normal);
    let norm_squared: Q = normal.iter().map(|n| n * n).sum();
    let area_squared_mm4 =
        twice_area.pow(2) * norm_squared / (Q::from_integer(4.into()) * normal[omitted].pow(2));
    Ok(ExactPolyhedralSectionArea {
        area_squared_mm4,
        depths,
        loops: section.loops,
        exact_work: max_exact_work - remaining,
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
