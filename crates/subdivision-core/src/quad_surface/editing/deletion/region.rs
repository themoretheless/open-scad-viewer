//! Original vertex-star region extraction. Never silently delete an unselected
//! interior control, change boundary parity, or replace a nondisk source region.
use super::*;
pub(super) struct Region {
    pub removed: BTreeSet<usize>,
    pub boundary: Vec<usize>,
    pub selected: BTreeSet<usize>,
    pub interior: Vec<usize>,
    pub old_boundary_edges: BTreeSet<(usize, usize)>,
}
pub(super) fn extract(source: &QuadSurface, selected: &[usize], cap: usize) -> Result<Region> {
    let cage = &source.cage;
    if selected.is_empty()
        || selected.len() > 256
        || selected.iter().any(|v| *v >= cage.vertices.len())
        || selected.iter().copied().collect::<BTreeSet<_>>().len() != selected.len()
    {
        return Err(error("Choose 1..256 distinct original control vertices"));
    }
    let selected = selected.iter().copied().collect::<BTreeSet<_>>();
    let removed = cage
        .faces
        .iter()
        .enumerate()
        .filter(|(_, f)| f.iter().any(|v| selected.contains(v)))
        .map(|(i, _)| i)
        .collect::<BTreeSet<_>>();
    let mut uses = BTreeMap::<(usize, usize), Vec<(usize, usize)>>::new();
    let mut vertices = BTreeSet::new();
    for &fi in &removed {
        let f = &cage.faces[fi];
        for i in 0..4 {
            uses.entry(edge(f[i], f[(i + 1) % 4]))
                .or_default()
                .push((f[i], f[(i + 1) % 4]));
            vertices.insert(f[i]);
        }
    }
    if vertices.len() as isize - uses.len() as isize + removed.len() as isize != 1 {
        return Err(error("Deleted original vertex stars must form one disk"));
    }
    let mut next = BTreeMap::new();
    for owners in uses.values() {
        if owners.len() == 1 {
            let (a, b) = owners[0];
            if next.insert(a, b).is_some() {
                return Err(error("Original deleted region has a pinched boundary"));
            }
        }
    }
    let Some(&start) = next.keys().next() else {
        return Err(error("Deleted region has no disk boundary"));
    };
    let mut boundary = vec![start];
    let mut v = next[&start];
    while v != start {
        if boundary.contains(&v) || boundary.len() > next.len() {
            return Err(error("Original region boundary is not simple"));
        }
        boundary.push(v);
        v = *next
            .get(&v)
            .ok_or_else(|| error("Original region boundary is open"))?;
    }
    if boundary.len() != next.len() {
        return Err(error(
            "Deleted original region has multiple boundary cycles",
        ));
    }
    // Validate the complete source subcomplex, including all vertex fans.
    let local = vertices
        .iter()
        .enumerate()
        .map(|(i, v)| (*v, i))
        .collect::<BTreeMap<_, _>>();
    Cage {
        vertices: vertices.iter().map(|v| cage.vertices[*v]).collect(),
        faces: removed
            .iter()
            .map(|f| cage.faces[*f].iter().map(|v| local[v]).collect())
            .collect(),
    }
    .validate()?;
    let used_elsewhere = cage
        .faces
        .iter()
        .enumerate()
        .filter(|(i, _)| !removed.contains(i))
        .flat_map(|(_, f)| f.iter().copied())
        .collect::<BTreeSet<_>>();
    let boundary_set = boundary.iter().copied().collect::<BTreeSet<_>>();
    let interior = vertices
        .iter()
        .filter(|v| {
            !selected.contains(v) && !boundary_set.contains(v) && !used_elsewhere.contains(v)
        })
        .copied()
        .collect::<Vec<_>>();
    let old_boundary_edges = (0..boundary.len())
        .map(|i| edge(boundary[i], boundary[(i + 1) % boundary.len()]))
        .collect();
    boundary.retain(|v| !selected.contains(v));
    if boundary.len() < 4 || boundary.len() > cap || boundary.len() % 2 != 0 {
        return Err(error(
            "Quad retile requires an even original boundary with 4..limit vertices; a lone boundary deletion needs a paired edit",
        ));
    }
    if interior.len() > 256 || interior.len() + boundary.len() > 512 {
        return Err(error(
            "Retained interior controls exceed bounded quad reconstruction limits",
        ));
    }
    Ok(Region {
        removed,
        boundary,
        selected,
        old_boundary_edges,
        interior,
    })
}
pub(super) fn counts(c: &Cage) -> (isize, usize, usize) {
    let edges = c.build_edges();
    let chi = c.vertices.len() as isize - edges.len() as isize + c.faces.len() as isize;
    let components = |boundary: bool| {
        let mut adj = BTreeMap::<usize, Vec<usize>>::new();
        for (&(a, b), u) in &edges {
            if !boundary || u.len() == 1 {
                adj.entry(a).or_default().push(b);
                adj.entry(b).or_default().push(a);
            }
        }
        let mut seen = BTreeSet::new();
        let mut n = 0;
        for &a in adj.keys() {
            if seen.contains(&a) {
                continue;
            }
            n += 1;
            let mut todo = vec![a];
            while let Some(v) = todo.pop() {
                if seen.insert(v) {
                    todo.extend(&adj[&v]);
                }
            }
        }
        n
    };
    (chi, components(false), components(true))
}
