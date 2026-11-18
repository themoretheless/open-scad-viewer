//! Oriented original-source quotient cells and manifold vertex links.
use super::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalQuotientCoedge {
    pub edge: usize,
    pub reversed: bool,
    pub halfedge: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalQuotientEdge {
    pub vertices: [usize; 2],
    pub halfedges: Vec<usize>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalQuotientVertexLink {
    pub vertex: usize,
    /// Separate edge-end darts even when a quotient edge is a loop.
    pub darts: Vec<[usize; 2]>,
    pub corners: Vec<[[usize; 2]; 2]>,
    pub boundary: bool,
}
#[derive(Clone, Debug)]
pub struct OwnedOriginalPeriodicTopology {
    seams: Arc<OwnedOriginalSurfaceSeams>,
    edges: Vec<OriginalQuotientEdge>,
    loops: Vec<Vec<OriginalQuotientCoedge>>,
    links: Vec<OriginalQuotientVertexLink>,
    components: Vec<Vec<usize>>,
    boundaries: Vec<Vec<usize>>,
    euler: Vec<i64>,
}
type Topology = (
    Vec<OriginalQuotientEdge>,
    Vec<Vec<OriginalQuotientCoedge>>,
    Vec<OriginalQuotientVertexLink>,
    Vec<Vec<usize>>,
    Vec<Vec<usize>>,
    Vec<i64>,
);
fn root(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}
/// An edge end: (edge, end index).
type Dart = [usize; 2];
/// Neighbouring darts of every dart around one vertex.
type VertexLink = BTreeMap<Dart, Vec<Dart>>;
/// Faces using an edge, each with the traversal direction.
type EdgeFaces = Vec<Vec<(usize, bool)>>;
/// Seam mate of every paired half-edge; mates must run in opposite directions.
fn seam_mates(
    pairs: impl Iterator<Item = [usize; 2]>,
    ends: impl Fn(usize) -> [usize; 2],
) -> Result<BTreeMap<usize, usize>> {
    let mut mates = BTreeMap::new();
    for [a, b] in pairs {
        if mates.insert(a, b).is_some() || mates.insert(b, a).is_some() {
            return Err(refused("Original seam half-edge is paired more than once"));
        }
        if ends(a) != [ends(b)[1], ends(b)[0]] {
            return Err(refused(
                "Original seam quotient endpoints do not reverse coherently",
            ));
        }
    }
    Ok(mates)
}
/// One quotient edge per retained half-edge or seam pair, and the coedge of
/// every retained half-edge.
fn quotient_edges(
    retained: &BTreeSet<usize>,
    mates: &BTreeMap<usize, usize>,
    ends: impl Fn(usize) -> [usize; 2],
) -> Result<(
    Vec<OriginalQuotientEdge>,
    BTreeMap<usize, OriginalQuotientCoedge>,
)> {
    let mut edges = Vec::new();
    let mut coedges = BTreeMap::new();
    for &h in retained {
        if coedges.contains_key(&h) {
            continue;
        }
        let edge = edges.len();
        let mut uses = vec![h];
        coedges.insert(
            h,
            OriginalQuotientCoedge {
                edge,
                reversed: false,
                halfedge: h,
            },
        );
        if let Some(&other) = mates.get(&h) {
            if !retained.contains(&other) {
                return Err(refused("Original seam mate is not a retained material use"));
            }
            coedges.insert(
                other,
                OriginalQuotientCoedge {
                    edge,
                    reversed: true,
                    halfedge: other,
                },
            );
            uses.push(other);
        }
        edges.push(OriginalQuotientEdge {
            vertices: ends(h),
            halfedges: uses,
        });
    }
    Ok((edges, coedges))
}
/// Face boundaries as quotient coedges; together they use every retained
/// half-edge exactly once.
fn quotient_loops(
    boundaries: &[Vec<usize>],
    coedges: &BTreeMap<usize, OriginalQuotientCoedge>,
    retained: &BTreeSet<usize>,
) -> Result<Vec<Vec<OriginalQuotientCoedge>>> {
    let mut loops = Vec::new();
    let mut seen = BTreeSet::new();
    for boundary in boundaries {
        let mut out = Vec::new();
        for &h in boundary {
            if !seen.insert(h) {
                return Err(refused(
                    "Original quotient boundary repeats a source half-edge",
                ));
            }
            out.push(
                coedges
                    .get(&h)
                    .ok_or_else(|| refused("Original quotient has an omitted material use"))?
                    .clone(),
            );
        }
        if out.is_empty() {
            return Err(refused("Original quotient face boundary is empty"));
        }
        loops.push(out);
    }
    if seen != *retained {
        return Err(refused(
            "Original quotient omits a retained source half-edge",
        ));
    }
    Ok(loops)
}
/// The face that owns every boundary loop; each loop has exactly one.
fn boundary_faces(face_boundaries: &[Vec<usize>], loops: usize) -> Result<Vec<usize>> {
    let mut face_of = vec![None; loops];
    for (face, boundaries) in face_boundaries.iter().enumerate() {
        for &boundary in boundaries {
            if boundary >= loops || face_of[boundary].replace(face).is_some() {
                return Err(refused(
                    "Original quotient boundary has invalid or repeated face ownership",
                ));
            }
        }
    }
    face_of
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| refused("Original quotient boundary has no material face owner"))
}
/// Face uses of every edge, and the dart adjacency and face corners around
/// every vertex.
struct Incidence {
    edge_faces: EdgeFaces,
    adjacency: BTreeMap<usize, VertexLink>,
    corners: BTreeMap<usize, Vec<[Dart; 2]>>,
}
fn incidence(
    loops: &[Vec<OriginalQuotientCoedge>],
    edges: &[OriginalQuotientEdge],
    face_of: &[usize],
) -> Result<Incidence> {
    let mut out = Incidence {
        edge_faces: vec![Vec::new(); edges.len()],
        adjacency: BTreeMap::new(),
        corners: BTreeMap::new(),
    };
    for (boundary, wire) in loops.iter().enumerate() {
        for (i, c) in wire.iter().enumerate() {
            out.edge_faces[c.edge].push((face_of[boundary], c.reversed));
            let next = &wire[(i + 1) % wire.len()];
            let incoming = [c.edge, usize::from(!c.reversed)];
            let outgoing = [next.edge, usize::from(next.reversed)];
            let vertex = edges[c.edge].vertices[incoming[1]];
            if vertex != edges[next.edge].vertices[outgoing[1]] {
                return Err(refused("Original quotient face coedges do not join"));
            }
            let link = out.adjacency.entry(vertex).or_default();
            link.entry(incoming).or_default().push(outgoing);
            link.entry(outgoing).or_default().push(incoming);
            out.corners
                .entry(vertex)
                .or_default()
                .push([incoming, outgoing]);
        }
    }
    for uses in &out.edge_faces {
        if uses.is_empty() || uses.len() > 2 || (uses.len() == 2 && uses[0].1 == uses[1].1) {
            return Err(refused(
                "Original quotient edge has nonmanifold or incoherent face uses",
            ));
        }
    }
    Ok(out)
}
/// The link of every vertex, which must be one circle or one interval.
fn vertex_links(
    adjacency: BTreeMap<usize, VertexLink>,
    mut corners: BTreeMap<usize, Vec<[Dart; 2]>>,
) -> Result<Vec<OriginalQuotientVertexLink>> {
    let mut links = Vec::new();
    for (vertex, link) in adjacency {
        let ends = link.values().filter(|n| n.len() == 1).count();
        if (ends != 0 && ends != 2) || link.values().any(|n| !(1..=2).contains(&n.len())) {
            return Err(refused(
                "Original quotient vertex link is not a circle or interval",
            ));
        }
        let mut stack = link.keys().next().copied().into_iter().collect::<Vec<_>>();
        let mut visited = BTreeSet::new();
        while let Some(dart) = stack.pop() {
            if visited.insert(dart) {
                stack.extend(link[&dart].iter().copied());
            }
        }
        if visited.len() != link.len() {
            return Err(refused(
                "Original quotient vertex has disconnected surface fans",
            ));
        }
        links.push(OriginalQuotientVertexLink {
            vertex,
            darts: link.keys().copied().collect(),
            corners: corners.remove(&vertex).unwrap_or_default(),
            boundary: ends == 2,
        });
    }
    Ok(links)
}
/// Groups of faces connected across shared edges.
fn face_components(edge_faces: &EdgeFaces, faces: usize) -> Vec<Vec<usize>> {
    let mut parent = (0..faces).collect::<Vec<_>>();
    for uses in edge_faces.iter().filter(|uses| uses.len() == 2) {
        let a = root(&mut parent, uses[0].0);
        let b = root(&mut parent, uses[1].0);
        parent[a.max(b)] = a.min(b);
    }
    let mut groups = BTreeMap::<usize, Vec<usize>>::new();
    for face in 0..faces {
        let r = root(&mut parent, face);
        groups.entry(r).or_default().push(face);
    }
    groups.into_values().collect()
}
/// Euler characteristic V - E + F of one component, counting each face
/// with its holes as 1 - holes.
fn euler_characteristic(
    component: &[usize],
    face_boundaries: &[Vec<usize>],
    loops: &[Vec<OriginalQuotientCoedge>],
    edges: &[OriginalQuotientEdge],
) -> i64 {
    let mut selected_edges = BTreeSet::new();
    let mut vertices = BTreeSet::new();
    let mut characteristic = 0i64;
    for &face in component {
        let boundaries = &face_boundaries[face];
        characteristic += 2 - boundaries.len() as i64;
        for c in boundaries.iter().flat_map(|&boundary| &loops[boundary]) {
            selected_edges.insert(c.edge);
            vertices.extend(edges[c.edge].vertices);
        }
    }
    characteristic + vertices.len() as i64 - selected_edges.len() as i64
}
/// Closed orbits of the unpaired half-edges, following face successors
/// across sewn seams.
fn external_boundaries(
    retained: &BTreeSet<usize>,
    mates: &BTreeMap<usize, usize>,
    face_successor: &[Option<usize>],
    ends: impl Fn(usize) -> [usize; 2],
) -> Result<Vec<Vec<usize>>> {
    let external = retained
        .iter()
        .copied()
        .filter(|h| !mates.contains_key(h))
        .collect::<BTreeSet<_>>();
    let mut successor = BTreeMap::new();
    for &h in &external {
        let mut next = face_successor[h]
            .ok_or_else(|| refused("Original quotient has a missing boundary successor"))?;
        let mut crossed = BTreeSet::new();
        while let Some(&mate) = mates.get(&next) {
            if !crossed.insert(next) {
                return Err(refused(
                    "Original external boundary is trapped in seam-only transitions",
                ));
            }
            next = face_successor[mate]
                .ok_or_else(|| refused("Original seam mate has a missing face successor"))?;
        }
        if !external.contains(&next) || ends(h)[1] != ends(next)[0] {
            return Err(refused(
                "Original quotient external boundary fails endpoint closure",
            ));
        }
        successor.insert(h, next);
    }
    if successor.values().copied().collect::<BTreeSet<_>>() != external {
        return Err(refused(
            "Original quotient external successor is not bijective",
        ));
    }
    let mut visited = BTreeSet::new();
    let mut boundaries = Vec::new();
    for &start in &external {
        if visited.contains(&start) {
            continue;
        }
        let mut h = start;
        let mut boundary = Vec::new();
        while visited.insert(h) {
            boundary.push(h);
            h = successor[&h];
        }
        if h != start {
            return Err(refused("Original external boundary joins another orbit"));
        }
        boundaries.push(boundary);
    }
    Ok(boundaries)
}
fn derive(seams: &OwnedOriginalSurfaceSeams) -> Result<Topology> {
    seams.reverify_source()?;
    let dictionary = seams.dictionary();
    let regions = dictionary.regions();
    let filled = regions.owner();
    let chart = dictionary.chart();
    let graph = chart.graph();
    let retained = filled.retained().iter().copied().collect::<BTreeSet<_>>();
    let ends = |h: usize| {
        let p = graph.ends()[h / 2];
        let p = if h.is_multiple_of(2) { p } else { [p[1], p[0]] };
        p.map(|v| chart.vertex_groups()[v])
    };
    let pairs = dictionary
        .pairs()
        .iter()
        .map(|pair| pair.map(|use_| dictionary.uses()[use_].halfedge));
    let mates = seam_mates(pairs, ends)?;
    let (edges, coedges) = quotient_edges(&retained, &mates, ends)?;
    let loops = quotient_loops(filled.boundaries(), &coedges, &retained)?;
    let face_boundaries = regions
        .regions()
        .iter()
        .map(|region| {
            std::iter::once(region.outer)
                .chain(region.holes.iter().copied())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let face_of = boundary_faces(&face_boundaries, loops.len())?;
    let Incidence {
        edge_faces,
        adjacency,
        corners,
    } = incidence(&loops, &edges, &face_of)?;
    let links = vertex_links(adjacency, corners)?;
    let components = face_components(&edge_faces, face_boundaries.len());
    let euler = components
        .iter()
        .map(|component| euler_characteristic(component, &face_boundaries, &loops, &edges))
        .collect();
    let boundaries = external_boundaries(&retained, &mates, filled.successor(), ends)?;
    Ok((edges, loops, links, components, boundaries, euler))
}
impl OwnedOriginalPeriodicTopology {
    pub fn seams(&self) -> &Arc<OwnedOriginalSurfaceSeams> {
        &self.seams
    }
    pub fn edges(&self) -> &[OriginalQuotientEdge] {
        &self.edges
    }
    pub fn loops(&self) -> &[Vec<OriginalQuotientCoedge>] {
        &self.loops
    }
    pub fn vertex_links(&self) -> &[OriginalQuotientVertexLink] {
        &self.links
    }
    pub fn components(&self) -> &[Vec<usize>] {
        &self.components
    }
    pub fn boundaries(&self) -> &[Vec<usize>] {
        &self.boundaries
    }
    /// Face patch contribution is 1 minus its owned hole count.
    pub fn euler_characteristics(&self) -> &[i64] {
        &self.euler
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (edges, loops, links, components, boundaries, euler) = derive(&self.seams)?;
        if edges != self.edges
            || loops != self.loops
            || links != self.links
            || components != self.components
            || boundaries != self.boundaries
            || euler != self.euler
        {
            return Err(refused(
                "Original periodic quotient topology or manifold link receipt changed",
            ));
        }
        Ok(())
    }
}
/// Actual source-surface seam identity plus complete original UV quotient cell
/// incidence proves oriented manifold links and external boundary closure.
/// All coedges retain original algebraic carrier half-edges, including loop
/// edge endpoint darts. This admits combinatorial topology only; source chart
/// coverage, surface regularity/injectivity and physical face construction are
/// separate obligations. No rounded curve Model is fabricated.
pub fn own_original_periodic_topology(
    seams: Arc<OwnedOriginalSurfaceSeams>,
) -> Result<OwnedOriginalPeriodicTopology> {
    let (edges, loops, links, components, boundaries, euler) = derive(&seams)?;
    Ok(OwnedOriginalPeriodicTopology {
        seams,
        edges,
        loops,
        links,
        components,
        boundaries,
        euler,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quotient_replay_rejects_edges_coedges_links_components_boundaries_and_euler() {
        let c = Curve::from_polyline(vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![1., 1., 0.],
            vec![0., 1., 0.],
            vec![0., 0., 0.],
        ])
        .unwrap();
        let surface = nurbs_core::surface::extrude(&c, [0., 0., 1.]).unwrap();
        let v = [[0., 0.], [4., 0.], [4., 1.], [0., 1.], [0., 0.]];
        let wire = v
            .windows(2)
            .map(|p| Curve::from_polyline(vec![p[0].to_vec(), p[1].to_vec()]).unwrap())
            .collect();
        let regions = Arc::new(
            assemble_original_filled_regions(
                &[wire],
                &[1],
                1e-12,
                Default::default(),
                Default::default(),
            )
            .unwrap(),
        );
        let chart = Arc::new(
            own_original_chart_vertices(
                regions.owner().cycles().graph().clone(),
                [[0., 4.], [0., 1.]],
                [true, false],
                Default::default(),
            )
            .unwrap(),
        );
        let dictionary =
            Arc::new(own_original_seam_dictionary(regions, chart, Default::default()).unwrap());
        let seams = Arc::new(bind_original_seams_to_surface(dictionary, &surface).unwrap());
        let t = own_original_periodic_topology(seams).unwrap();
        t.reverify_source().unwrap();
        let mut bad = t.clone();
        bad.edges[0].vertices[0] = 99;
        assert!(bad.reverify_source().is_err());
        let mut bad = t.clone();
        bad.loops[0][0].reversed = !bad.loops[0][0].reversed;
        assert!(bad.reverify_source().is_err());
        let mut bad = t.clone();
        bad.links[0].darts.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = t.clone();
        bad.components[0].push(0);
        assert!(bad.reverify_source().is_err());
        let mut bad = t.clone();
        bad.boundaries.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = t;
        bad.euler[0] = 99;
        assert!(bad.reverify_source().is_err());
    }
}
