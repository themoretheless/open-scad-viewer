//! lib operations: euler_characteristic.
use super::*;
impl<C, S, P, V> Model<C, S, P, V> {
/// Euler-Poincaré characteristic $\chi = V - E + F - H$ of the B-rep model.
    pub fn euler_characteristic(&self) -> i64 {
        explorer::euler_characteristic(self)
    }
/// Euler-Poincaré characteristic $\chi = V - E + F - H$ of `shell_id`.
    pub fn shell_euler_characteristic(&self, shell_id: usize) -> Option<i64> {
        explorer::shell_euler_characteristic(self, shell_id)
    }
/// Topological genus $g = (2 - \chi) / 2$ of closed shell `shell_id`.
    pub fn shell_genus(&self, shell_id: usize) -> Option<i64> {
        explorer::shell_genus(self, shell_id)
    }
/// Topological genus summed over all closed shells in the model.
    pub fn genus(&self) -> i64 {
        explorer::genus(self)
    }
/// Fast adjacency query: non-degenerate edges incident to `vertex_id`.
    pub fn vertex_edges(&self, vertex_id: usize) -> Vec<usize> {
        explorer::vertex_edges(self, vertex_id)
    }
/// Fast adjacency query: edges bounding `face_id`.
    pub fn face_edges(&self, face_id: usize) -> Vec<usize> {
        explorer::face_edges(self, face_id)
    }
/// Fast adjacency query: faces sharing at least one non-degenerate edge with `face_id`.
    pub fn adjacent_faces(&self, face_id: usize) -> Vec<usize> {
        explorer::adjacent_faces(self, face_id)
    }
/// Sorted indices of non-degenerate boundary edges (used by exactly 1 face coedge).
    pub fn boundary_edges(&self) -> Vec<EdgeId> {
        explorer::boundary_edges(self)
    }
/// Sorted indices of non-degenerate non-manifold edges (used by 3 or more face coedges).
    pub fn non_manifold_edges(&self) -> Vec<EdgeId> {
        explorer::non_manifold_edges(self)
    }
/// Number of connected components of faces across non-degenerate shared edges.
    pub fn connected_face_components(&self) -> usize {
        explorer::connected_face_components(self)
    }
/// Sorted indices of isolated vertices (vertices not referenced by any edge).
    pub fn isolated_vertices(&self) -> Vec<VertexId> {
        explorer::isolated_vertices(self)
    }
/// Total loop count (`1 + holes.len()`) for each face `(FaceId, loop_count)`.
    pub fn face_loop_counts(&self) -> Vec<(FaceId, usize)> {
        explorer::face_loop_counts(self)
    }
/// Canonical immutable predicate context derived from this model's legacy
    /// scalar. New serialized models also carry its versioned specification.
    pub fn tolerance_context(&self) -> Result<cad_predicates::ToleranceContext> {
        cad_predicates::ToleranceContext::from_brep_tolerance_mm(self.tolerance_mm)
            .map_err(|_| invalid("Invalid B-rep tolerance context"))
    }
/// Validate indexed incidence with application-owned vertex admission.
    /// The callback must validate geometry/provenance in its owning context;
    /// this method alone does not certify geometric or solid validity.
    pub fn validate_topology_with_vertices(
        &self,
        validate_vertex: impl FnMut(&V) -> Result<()>,
    ) -> Result<()> {
        self.validate_topology_scoped(validate_vertex, &ValidationPlan::full())
    }
/// Incremental revalidation: checks that only read unchanged entities are
    /// skipped. Every skipped check is provably unchanged for a valid pre-edit
    /// model and an honest [`EditScope`], so the accepted/rejected outcome and
    /// the first error (code, message) match full validation exactly. All
    /// cheap global invariants (resource ceilings, tolerance, edge/vertex
    /// reference and usage bitmaps, loop reuse, face ownership, body checks)
    /// always run in full and in the original order.
    pub(super) fn validate_topology_scoped(
        &self,
        mut validate_vertex: impl FnMut(&V) -> Result<()>,
        plan: &ValidationPlan,
    ) -> Result<()> {
        let count = self.vertices.len()
            + self.edges.len()
            + self.loops.len()
            + self.faces.len()
            + self.shells.len()
            + self.bodies.len();
        let uses = self.loops.iter().map(|l| l.coedges.len()).sum::<usize>();
        // Gear bodies carry a few hundred faces (six per tooth, twice for a
        // herringbone); the ceiling stays a guard against runaway authoring.
        if count > MAX_ENTITIES || uses > MAX_COEDGES || self.faces.len() > MAX_FACES {
            return Err(Error {
                code: "BREP_RESOURCE_LIMIT",
                message: format!(
                    "B-rep exceeds {MAX_ENTITIES} entities, {MAX_FACES} faces or {MAX_COEDGES} coedges"
                ),
            });
        }
        require(
            self.tolerance_mm.is_finite() && (1e-10..=1e-2).contains(&self.tolerance_mm),
            "Invalid B-rep tolerance",
        )?;
        let mut vertices = vec![false; self.vertices.len()];
        if plan.full {
            for v in &self.vertices {
                validate_vertex(&v.point)?;
            }
        } else {
            // Unchanged vertices were admitted before the edit and cannot
            // newly fail; check affected ones in ascending index order so the
            // first failure matches the full pass.
            let mut affected: Vec<usize> = plan.vertices.iter().copied().collect();
            affected.sort_unstable();
            for i in affected {
                validate_vertex(&self.vertices[i].point)?;
            }
        }
        for e in &self.edges {
            require(
                !e.degenerate || e.vertices[0] == e.vertices[1],
                "Collapsed boundary must reference one pole vertex",
            )?;
            for &v in &e.vertices {
                require(v < vertices.len(), "Unknown edge vertex")?;
                vertices[v] = true;
            }
        }
        require(vertices.iter().all(|v| *v), "Unused vertex")?;
        let mut loops = vec![false; self.loops.len()];
        let mut edges = vec![false; self.edges.len()];
        for (fi, f) in self.faces.iter().enumerate() {
            // The chain check only reads this face's loops, their coedges and
            // the referenced edges; skip it when none of those changed. The
            // bounds/reuse/emptiness checks and usage marking stay global so
            // error selection and the "Unused loop or edge" bitmap are exact.
            let check_chain = plan.full || plan.faces.contains(&fi);
            for &l in std::iter::once(&f.outer).chain(&f.holes) {
                require(l < loops.len(), "Unknown face loop")?;
                require(!loops[l], "Loop reused by faces")?;
                loops[l] = true;
                let cs = &self.loops[l].coedges;
                require(!cs.is_empty(), "Empty loop")?;
                for (i, c) in cs.iter().enumerate() {
                    let e = self
                        .edges
                        .get(c.edge)
                        .ok_or_else(|| invalid("Unknown coedge edge"))?;
                    edges[c.edge] = true;
                    if check_chain {
                        let next = &cs[(i + 1) % cs.len()];
                        let next_edge = self
                            .edges
                            .get(next.edge)
                            .ok_or_else(|| invalid("Unknown coedge edge"))?;
                        require(
                            e.vertices[usize::from(!c.reversed)]
                                == next_edge.vertices[usize::from(next.reversed)],
                            "Disconnected loop vertex chain",
                        )?;
                    }
                }
            }
        }
        require(
            loops.iter().all(|v| *v) && edges.iter().all(|v| *v),
            "Unused loop or edge",
        )?;
        let mut face_owner = vec![None; self.faces.len()];
        for (si, s) in self.shells.iter().enumerate() {
            require(!s.faces.is_empty(), "Empty shell")?;
            if !plan.full && !plan.shells.contains(&si) {
                // Unchanged shell: its pole/incidence/fan checks only read its
                // own faces, loops, coedges and edges, none of which changed,
                // so they cannot newly fail. Face ownership is global, though:
                // keep the bounds and duplicate checks so a repeated face use
                // still fails here, exactly where the full pass would fail.
                for u in &s.faces {
                    self.faces
                        .get(u.face)
                        .ok_or_else(|| invalid("Shell references unknown face"))?;
                    require(
                        face_owner[u.face].replace(si).is_none(),
                        "Face belongs to multiple shells or is repeated",
                    )?;
                }
                continue;
            }
            let mut incidence = FxHashMap::<usize, Vec<(usize, bool)>>::with_capacity_and_hasher(
                s.faces.len() * 4,
                Default::default(),
            );
            let mut links = FxHashMap::<usize, Vec<(usize, usize)>>::with_capacity_and_hasher(
                s.faces.len() * 4,
                Default::default(),
            );
            let mut pole_uses = FxHashMap::<usize, FxHashSet<usize>>::default();
            let mut cs = Vec::new();
            for u in &s.faces {
                let f = self
                    .faces
                    .get(u.face)
                    .ok_or_else(|| invalid("Shell references unknown face"))?;
                require(
                    face_owner[u.face].replace(si).is_none(),
                    "Face belongs to multiple shells or is repeated",
                )?;
                for l in std::iter::once(&f.outer).chain(&f.holes) {
                    let all = &self.loops[*l].coedges;
                    for c in all.iter().filter(|c| self.edges[c.edge].degenerate) {
                        require(*l == f.outer, "Collapsed boundary cannot belong to a hole")?;
                        require(
                            pole_uses.entry(c.edge).or_default().insert(u.face),
                            "Collapsed boundary is repeated on one face",
                        )?;
                    }
                    cs.clear();
                    cs.extend(all.iter().filter(|c| !self.edges[c.edge].degenerate));
                    require(
                        !cs.is_empty() && (cs.len() == all.len() || cs.len() >= 2),
                        "Pole face needs at least two ordinary boundary edges",
                    )?;
                    for (i, c) in cs.iter().enumerate() {
                        incidence
                            .entry(c.edge)
                            .or_insert_with(|| Vec::with_capacity(2))
                            .push((u.face, c.reversed ^ u.reversed));
                        let next = &cs[(i + 1) % cs.len()];
                        let vertex = self.edges[c.edge].vertices[usize::from(!c.reversed)];
                        links.entry(vertex).or_default().push((c.edge, next.edge));
                    }
                }
            }
            let mut adjacency = FxHashMap::<usize, Vec<usize>>::with_capacity_and_hasher(
                s.faces.len(),
                Default::default(),
            );
            for (edge_index, edges) in &incidence {
                require(
                    edges.len() <= 2,
                    "Non-manifold edge has more than two face uses",
                )?;
                if edges.len() == 1 {
                    require(!s.closed, "Closed shell has a boundary edge")?;
                } else {
                    require(
                        edges[0].1 != edges[1].1,
                        &format!(
                            "Adjacent face uses traverse edge {edge_index} in the same direction ({:?})",
                            edges
                        ),
                    )?;
                    adjacency.entry(edges[0].0).or_default().push(edges[1].0);
                    adjacency.entry(edges[1].0).or_default().push(edges[0].0);
                }
            }
            let mut seen = FxHashSet::with_capacity_and_hasher(s.faces.len(), Default::default());
            let mut queue = Vec::with_capacity(s.faces.len());
            queue.push(s.faces[0].face);
            while let Some(f) = queue.pop() {
                if seen.insert(f) {
                    if let Some(neighbors) = adjacency.get(&f) {
                        queue.extend_from_slice(neighbors);
                    }
                }
            }
            require(
                seen.len() == s.faces.len(),
                "Shell has disconnected face components",
            )?;
            for fan in links.values() {
                let mut graph = FxHashMap::<usize, Vec<usize>>::default();
                for (a, b) in fan {
                    graph.entry(*a).or_default().push(*b);
                    graph.entry(*b).or_default().push(*a);
                }
                require(
                    graph
                        .values()
                        .all(|n| n.len() <= 2 && (!s.closed || n.len() == 2)),
                    "Non-manifold vertex link",
                )?;
                let mut seen = FxHashSet::default();
                let mut q = vec![*graph.keys().next().unwrap()];
                while let Some(e) = q.pop() {
                    if seen.insert(e) {
                        q.extend(&graph[&e]);
                    }
                }
                require(
                    seen.len() == graph.len(),
                    "Vertex has disconnected face fans",
                )?;
            }
        }
        require(face_owner.iter().all(Option::is_some), "Unused face")?;
        let mut owned = BTreeSet::new();
        for b in &self.bodies {
            for s in std::iter::once(&b.outer_shell).chain(&b.inner_shells) {
                require(
                    self.shells
                        .get(*s)
                        .ok_or_else(|| invalid("Body references unknown shell"))?
                        .closed,
                    "Body requires closed shells",
                )?;
                require(owned.insert(*s), "Shell reused by multiple body boundaries")?;
            }
        }
        Ok(())
    }
}
