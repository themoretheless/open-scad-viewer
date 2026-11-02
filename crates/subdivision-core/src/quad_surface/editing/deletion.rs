//! Source-owned vertex deletion and bounded quad retopology. Retained control
//! coordinates remain exact; topology changes do NOT preserve the old limit.
use super::*;
mod insertion;
mod projection;
mod region;
mod tiling;
#[derive(Clone, Copy, Debug)]
pub struct DeletionOptions {
    /// Chosen orthogonal projection normal, or bounded automatic proposals.
    /// This defines a control-polygon fill test, not a surface normal field.
    pub projection: Option<[f64; 3]>,
    /// Even retained boundary size, 4..=512. At most 256 retained interior
    /// controls and 512 total reconstruction controls are supported.
    pub max_boundary_vertices: usize,
    /// Shared finite allowance for boundary tilings and interior quad splits.
    pub max_candidates: usize,
    /// Shared exact-predicate work across every attempted projection and split.
    pub predicate_work: u64,
}
impl Default for DeletionOptions {
    fn default() -> Self {
        Self {
            projection: None,
            max_boundary_vertices: 64,
            max_candidates: 1000000,
            predicate_work: 5000000,
        }
    }
}
pub struct VertexDeletion {
    source: QuadSurface,
    surface: QuadSurface,
    vertex_map: Vec<Option<usize>>,
    face_map: Vec<Option<usize>>,
    edge_map: BTreeMap<(usize, usize), Option<(usize, usize)>>,
    deleted: Vec<usize>,
    removed: Vec<usize>,
    boundary: Vec<usize>,
    controls: Vec<usize>,
    new_faces: Vec<usize>,
    quads: Vec<[usize; 4]>,
    projection: [f64; 3],
    work: (usize, u64),
}
pub struct DeletionVerification {
    /// Same Euler characteristic, connected components and boundary loops;
    /// source/result manifold validation and original-map binding also passed.
    pub topology_preserved: bool,
    /// A simple projected control disk, without edge crossings or hidden
    /// original controls. This grants no limit-surface embedding authority.
    pub projected_fill_proven: bool,
    pub predicate_work: u64,
}
impl VertexDeletion {
    pub fn source(&self) -> &QuadSurface {
        &self.source
    }
    pub fn surface(&self) -> &QuadSurface {
        &self.surface
    }
    pub fn into_surface(self) -> QuadSurface {
        self.surface
    }
    pub fn vertex_map(&self) -> &[Option<usize>] {
        &self.vertex_map
    }
    pub fn face_map(&self) -> &[Option<usize>] {
        &self.face_map
    }
    pub fn edge_map(&self) -> &BTreeMap<(usize, usize), Option<(usize, usize)>> {
        &self.edge_map
    }
    pub fn deleted_vertices(&self) -> &[usize] {
        &self.deleted
    }
    pub fn removed_faces(&self) -> &[usize] {
        &self.removed
    }
    pub fn boundary_original_vertices(&self) -> &[usize] {
        &self.boundary
    }
    /// Original retained boundary controls followed by retained interior controls.
    pub fn reconstruction_controls(&self) -> &[usize] {
        &self.controls
    }
    pub fn replacement_faces(&self) -> &[usize] {
        &self.new_faces
    }
    pub fn projection_normal(&self) -> [f64; 3] {
        self.projection
    }
    pub fn work(&self) -> (usize, u64) {
        self.work
    }
    pub fn reverify(&self, predicate_work: u64) -> Result<DeletionVerification> {
        if !(1..=50000000).contains(&predicate_work) {
            return Err(error("Choose bounded original retile replay work"));
        }
        self.source.cage.validate()?;
        self.surface.cage.validate()?;
        let r = region::extract(&self.source, &self.deleted, 512)?;
        if r.boundary != self.boundary
            || r.boundary
                .iter()
                .chain(&r.interior)
                .copied()
                .collect::<Vec<_>>()
                != self.controls
        {
            return Err(error("Stored original deletion boundary changed"));
        }
        if r.removed.iter().copied().collect::<Vec<_>>() != self.removed {
            return Err(error("Stored original deleted-face group changed"));
        }
        for (i, m) in self.vertex_map.iter().enumerate() {
            if let Some(j) = m {
                if self.surface.cage.vertices[*j].map(f64::to_bits)
                    != self.source.cage.vertices[i].map(f64::to_bits)
                {
                    return Err(error("Retiled source control position changed"));
                }
            }
        }
        for (i, j) in self.face_map.iter().enumerate() {
            if let Some(j) = j {
                let expected = self.source.cage.faces[i]
                    .iter()
                    .map(|v| self.vertex_map[*v].unwrap())
                    .collect::<Vec<_>>();
                if self.surface.cage.faces[*j] != expected {
                    return Err(error("Retained original face connectivity changed"));
                }
            }
        }
        let edges = self.surface.cage.build_edges();
        for (&(a, b), m) in &self.edge_map {
            let expected = self.vertex_map[a]
                .zip(self.vertex_map[b])
                .map(|(a, b)| edge(a, b))
                .filter(|e| edges.contains_key(e));
            if *m != expected {
                return Err(error("Retained original edge connectivity changed"));
            }
        }
        for (q, &f) in self.quads.iter().zip(&self.new_faces) {
            let vertices = q.map(|i| self.vertex_map[self.controls[i]].unwrap());
            if self.surface.cage.faces[f] != vertices {
                return Err(error(
                    "Actual saved replacement face differs from the original projection proof",
                ));
            }
        }
        let points = self
            .controls
            .iter()
            .map(|v| self.source.cage.vertices[*v])
            .collect::<Vec<_>>();
        let forbidden = forbidden(&self.source, &r);
        let mut p = projection::Projector::new(
            &points,
            self.boundary.len(),
            self.projection,
            predicate_work,
        )?;
        p.boundary()?;
        tiling::verify(&mut p, &points, &self.quads, &forbidden)?;
        let topology_preserved =
            region::counts(&self.source.cage) == region::counts(&self.surface.cage);
        if !topology_preserved {
            return Err(error(
                "Deletion changed original Euler/component/boundary topology",
            ));
        }
        Ok(DeletionVerification {
            topology_preserved,
            projected_fill_proven: true,
            predicate_work: p.used,
        })
    }
}
fn forbidden(s: &QuadSurface, r: &region::Region) -> BTreeSet<(usize, usize)> {
    let ids = r
        .boundary
        .iter()
        .chain(&r.interior)
        .enumerate()
        .map(|(i, v)| (*v, i))
        .collect::<BTreeMap<_, _>>();
    let mut out = BTreeSet::new();
    for (f, face) in s.cage.faces.iter().enumerate() {
        if r.removed.contains(&f) {
            continue;
        }
        for i in 0..4 {
            let a = face[i];
            let b = face[(i + 1) % 4];
            if r.old_boundary_edges.contains(&edge(a, b)) {
                continue;
            }
            if let (Some(a), Some(b)) = (ids.get(&a), ids.get(&b)) {
                out.insert(edge(*a, *b));
            }
        }
    }
    out
}
fn directions(points: &[[f64; 3]], o: DeletionOptions) -> Vec<[f64; 3]> {
    if let Some(n) = o.projection {
        return vec![n];
    }
    let mut normal = [0.; 3];
    let origin = points[0];
    for i in 1..points.len() - 1 {
        let a = std::array::from_fn::<_, 3, _>(|k| points[i][k] - origin[k]);
        let b = std::array::from_fn::<_, 3, _>(|k| points[i + 1][k] - origin[k]);
        for k in 0..3 {
            normal[k] += a[(k + 1) % 3] * b[(k + 2) % 3] - a[(k + 2) % 3] * b[(k + 1) % 3];
        }
    }
    let scale = normal.iter().map(|n| n.abs()).fold(0f64, f64::max);
    let mut out = Vec::new();
    if scale > 0. && scale.is_finite() {
        out.push(normal.map(|n| n / scale));
    }
    out.extend([
        [0., 0., 1.],
        [0., 1., 0.],
        [1., 0., 0.],
        [1., 1., 1.],
        [1., 1., -1.],
        [1., -1., 1.],
        [-1., 1., 1.],
        [1., 2., 3.],
        [3., 1., 2.],
        [2., 3., 1.],
    ]);
    out
}
impl QuadSurface {
    /// Remove exactly one original interior control, retessellating its complete
    /// star into quads without new/moved controls. Boundary parity may require
    /// delete_vertices with a paired boundary edit instead. Whole cage manifold
    /// validation and an exact projected polygon fill gate publication. These
    /// prove control topology/fill, NOT limit-surface embedding or old G2 shape.
    pub fn delete_vertex(&self, vertex: usize, options: DeletionOptions) -> Result<VertexDeletion> {
        self.delete_vertices(&[vertex], options)
    }
    /// Remove a connected disk of original vertex stars. Old face/edge/vertex
    /// maps, original boundary, deleted stars and new-face group survive. No
    /// unselected interior control or boundary parity is silently discarded.
    pub fn delete_vertices(
        &self,
        vertices: &[usize],
        options: DeletionOptions,
    ) -> Result<VertexDeletion> {
        if !(4..=512).contains(&options.max_boundary_vertices)
            || !(1..=50000000).contains(&options.max_candidates)
            || !(1..=50000000).contains(&options.predicate_work)
        {
            return Err(error("Choose bounded original quad deletion allowances"));
        }
        let r = region::extract(self, vertices, options.max_boundary_vertices)?;
        let controls = r
            .boundary
            .iter()
            .chain(&r.interior)
            .copied()
            .collect::<Vec<_>>();
        let points = controls
            .iter()
            .map(|v| self.cage.vertices[*v])
            .collect::<Vec<_>>();
        let forbidden = forbidden(self, &r);
        let mut candidates = 0;
        let mut used = 0;
        let mut reason = String::new();
        for direction in directions(&points[..r.boundary.len()], options) {
            if used == options.predicate_work || candidates == options.max_candidates {
                break;
            }
            let mut p = projection::Projector::new(
                &points,
                r.boundary.len(),
                direction,
                options.predicate_work - used,
            )?;
            let result = (|| {
                p.boundary()?;
                let mut faces = tiling::solve(
                    &mut p,
                    &points[..r.boundary.len()],
                    &forbidden,
                    &mut candidates,
                    options.max_candidates,
                )?;
                insertion::insert(
                    &mut p,
                    &points,
                    &mut faces,
                    &forbidden,
                    &mut candidates,
                    options.max_candidates,
                )?;
                tiling::verify(&mut p, &points, &faces, &forbidden)?;
                Ok::<_, crate::Error>(faces)
            })();
            used += p.used;
            let quads = match result {
                Ok(q) => q,
                Err(e) => {
                    reason = e.to_string();
                    continue;
                }
            };
            let mut vertex_map = vec![None; self.cage.vertices.len()];
            let mut retained = Vec::new();
            for (i, v) in self.cage.vertices.iter().enumerate() {
                if !r.selected.contains(&i) {
                    vertex_map[i] = Some(retained.len());
                    retained.push(*v);
                }
            }
            let mut face_map = vec![None; self.cage.faces.len()];
            let mut faces = Vec::new();
            for (i, f) in self.cage.faces.iter().enumerate() {
                if !r.removed.contains(&i) {
                    face_map[i] = Some(faces.len());
                    faces.push(f.iter().map(|v| vertex_map[*v].unwrap()).collect());
                }
            }
            let mut new_faces = Vec::new();
            for q in &quads {
                new_faces.push(faces.len());
                faces.push(
                    q.iter()
                        .map(|i| vertex_map[controls[*i]].unwrap())
                        .collect(),
                );
            }
            let surface = QuadSurface::new(Cage {
                vertices: retained,
                faces,
            })?;
            if region::counts(&self.cage) != region::counts(&surface.cage) {
                return Err(error("Retiling changes original surface topology"));
            }
            let new_edges = surface.cage.build_edges();
            let edge_map = self
                .cage
                .build_edges()
                .keys()
                .map(|&(a, b)| {
                    let mapped = vertex_map[a]
                        .zip(vertex_map[b])
                        .map(|(a, b)| edge(a, b))
                        .filter(|e| new_edges.contains_key(e));
                    ((a, b), mapped)
                })
                .collect();
            return Ok(VertexDeletion {
                source: self.clone(),
                surface,
                vertex_map,
                face_map,
                edge_map,
                deleted: r.selected.into_iter().collect(),
                removed: r.removed.into_iter().collect(),
                boundary: r.boundary,
                controls,
                new_faces,
                quads,
                projection: p.direction(),
                work: (candidates, used),
            });
        }
        Err(error(format!(
            "No bounded original quad retile admitted: {reason}"
        )))
    }
}
