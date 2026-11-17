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
