//! Exact closed-contour face subdivision. Does not admit a mixed solid.
use super::*;
#[derive(Clone, Copy, Default)]
pub struct CensusArrangementLimits {
    /// Per ordered contour-pair obligation; at most 64*63 queries.
    pub winding: ss::OriginalSsWindingLimits,
    /// Per exact selector query.
    pub geometry: ss::OriginalSsCurveLimits,
}
pub struct CensusFaceArrangement {
    census: Arc<QualifiedInteriorSsCensus>,
    support: usize,
    limits: CensusArrangementLimits,
    parents: Vec<Option<usize>>,
    faces: Vec<usize>,
    model: MixedBrep,
}
impl CensusFaceArrangement {
    pub fn model(&self) -> &MixedBrep {
        &self.model
    }
    pub fn parents(&self) -> &[Option<usize>] {
        &self.parents
    }
    /// Exterior first, followed by each contour's immediate interior piece.
    pub fn region_faces(&self) -> &[usize] {
        &self.faces
    }
    pub fn geometry_admitted(&self) -> bool {
        false
    }
    pub fn reverify(&self) -> Result<()> {
        self.census.reverify()?;
        let fresh = self
            .census
            .build_face_arrangement(self.support, self.limits)?;
        if fresh.parents != self.parents || fresh.faces != self.faces {
            return Err(unproved("Original contour arrangement changed"));
        }
        Ok(())
    }
    /// Map an exhaustive side signature to a literal candidate face. This is
    /// ancestry ownership, not permission to skip geometric body admission.
    pub fn face_for_signature(&self, signature: &[bool]) -> Result<usize> {
        face_for_signature(&self.parents, &self.faces, signature)
    }
}

fn face_for_signature(parents: &[Option<usize>], faces: &[usize], signature: &[bool]) -> Result<usize> {
    if signature.len() != parents.len() {
        return Err(unproved("Foreign contour signature length"));
    }
    let mut deepest = None;
    for (i, inside) in signature.iter().enumerate() {
        if *inside {
            let mut expected = vec![false; signature.len()];
            let mut current = Some(i);
            while let Some(k) = current {
                expected[k] = true;
                current = parents[k];
            }
            if expected == signature {
                deepest = Some(i);
                break;
            }
        }
    }
    match deepest {
        Some(i) => Ok(faces[i + 1]),
        None if signature.iter().all(|v| !*v) => Ok(faces[0]),
        _ => Err(unproved(
            "Contour signature is inconsistent with proved nesting",
        )),
    }
}

fn nesting(
    cuts: &[(&ClosedCutCandidate, usize)],
    limits: CensusArrangementLimits,
) -> Result<Vec<Option<usize>>> {
    let n = cuts.len();
    let mut contains = vec![vec![false; n]; n];
    for (i, &(cut, support)) in cuts.iter().enumerate() {
        let point = cut.wire.edges()[0]
            .start()
            .point()
            .enclose(limits.geometry)?;
        let uv = [point.uv[2 * support], point.uv[2 * support + 1]];
        for (j, &(other, other_support)) in cuts.iter().enumerate() {
            if i != j {
                let w = other.wire.winding_box(other_support, uv, limits.winding)?.winding();
                if w != 0 && w != other.orientation {
                    return Err(unproved("Original contour nesting winding is unproved"));
                }
                contains[j][i] = w != 0;
            }
        }
    }
    let mut parents = vec![None; n];
    for i in 0..n {
        let immediate: Vec<_> = (0..n)
            .filter(|&j| contains[j][i] && !(0..n).any(|k| contains[k][i] && contains[j][k]))
            .collect();
        if immediate.len() > 1 {
            return Err(unproved("Contour nesting has multiple immediate parents"));
        }
        parents[i] = immediate.first().copied();
    }
    // Require exact agreement with a finite acyclic ancestor forest; no partial
    // order, mutual containment or guessed parent survives this check.
    for i in 0..n {
        let mut ancestors = vec![false; n];
        let mut current = parents[i];
        for _ in 0..n {
            if let Some(k) = current {
                if k == i || ancestors[k] {
                    return Err(unproved("Contour nesting cycle"));
                }
                ancestors[k] = true;
                current = parents[k];
            }
        }
        if current.is_some() || (0..n).any(|j| contains[j][i] != ancestors[j]) {
            return Err(unproved("Incomplete original contour nesting forest"));
        }
    }
    Ok(parents)
}
impl QualifiedInteriorSsCensus {
    /// Build all closed-contour pieces together, with common literal root edges
    /// in opposing loops and each original hole assigned exactly once.
    pub fn build_face_arrangement(
        self: &Arc<Self>,
        support: usize,
        limits: CensusArrangementLimits,
    ) -> Result<CensusFaceArrangement> {
        let base = self.parent(support)?;
        let mut topology = MixedBrep::from_closed_solid(&base.source)?.topology;
        let (parents, faces) = self.append_face_arrangement(&mut topology, support, limits)?;
        Ok(CensusFaceArrangement {
            census: self.clone(),
            support,
            limits,
            parents,
            faces,
            model: MixedBrep::from_topology(topology)?,
        })
    }

    /// Subdivide distinct faces of one admitted operand in a shared topology.
    /// Returned face groups follow the order of cuts, with each exterior first.
    /// The subdivision preserves original edges and does not admit material.
    pub fn build_multi_face_arrangement(
        source: &crate::closed_solid::QualifiedClosedSolid,
        cuts: &[(&Self, usize, CensusArrangementLimits)],
    ) -> Result<(MixedBrep, Vec<Vec<usize>>)> {
        let (model, arrangements) = Self::build_multi_face_layout(source, cuts)?;
        Ok((model, arrangements.into_iter().map(|(_, faces)| faces).collect()))
    }

    pub(super) fn build_multi_face_layout(
        source: &crate::closed_solid::QualifiedClosedSolid,
        cuts: &[(&Self, usize, CensusArrangementLimits)],
    ) -> Result<(MixedBrep, Vec<(Vec<Option<usize>>, Vec<usize>)>)> {
        let mut cut_faces = vec![false; source.model().faces.len()];
        for &(census, support, _) in cuts {
            let base = census.parent(support)?;
            if !std::ptr::eq(base.source.model(), source.model()) {
                return Err(unproved("Multi-face arrangement needs one literal source body"));
            }
            if std::mem::replace(&mut cut_faces[base.face], true) {
                return Err(unproved("Multiple cuts on one face need a joint contour arrangement"));
            }
        }
        let mut topology = MixedBrep::from_closed_solid(source)?.topology;
        let mut groups = Vec::with_capacity(cuts.len());
        for &(census, support, limits) in cuts {
            groups.push(census.append_face_arrangement(&mut topology, support, limits)?);
        }
        Ok((MixedBrep::from_topology(topology)?, groups))
    }

    pub(super) fn build_network_face_layout(
        solid: &crate::closed_solid::QualifiedClosedSolid,
        groups: &std::collections::BTreeMap<usize, Vec<(&ClosedCutCandidate, usize)>>,
        limits: CensusArrangementLimits,
    ) -> Result<(MixedBrep, Vec<(Vec<Option<usize>>, Vec<usize>)>)> {
        let mut topology = MixedBrep::from_closed_solid(solid)?.topology;
        let mut layouts = Vec::with_capacity(groups.len());
        for (&face, cuts) in groups {
            if cuts.len() > 64 {
                return Err(unproved("Network face exceeds the supported contour work bound"));
            }
            layouts.push(append_face_contours(&mut topology, face, cuts, limits)?);
        }
        Ok((MixedBrep::from_topology(topology)?, layouts))
    }

    fn append_face_arrangement(
        &self,
        t: &mut MixedTopology,
        support: usize,
        limits: CensusArrangementLimits,
    ) -> Result<(Vec<Option<usize>>, Vec<usize>)> {
        let base = self.parent(support)?;
        let cuts: Vec<_> = self.loops.iter().map(|component| (&component.cuts[support], support)).collect();
        append_face_contours(t, base.face, &cuts, limits)
    }
}

fn append_face_contours(
    t: &mut MixedTopology,
    face: usize,
    cuts: &[(&ClosedCutCandidate, usize)],
    limits: CensusArrangementLimits,
) -> Result<(Vec<Option<usize>>, Vec<usize>)> {
    let parents = nesting(cuts, limits)?;
    let n = cuts.len();
    let surface = t.faces[face].surface.clone();
    let old_holes = t.faces[face].holes.clone();
    let mut holes = Vec::new();
    let mut outers = Vec::new();
    for &(cut, support) in cuts {
        let mut ids = Vec::new();
        for edge in cut.wire.edges() {
            let mut vertices = [0; 2];
            for (i, port) in [edge.start(), edge.end()].into_iter().enumerate() {
                vertices[i]=if let Some(id)=t.vertices.iter().position(|v|matches!(&v.point,MixedVertex::Root(p) if p.same_selector(port.point()))) {id}
                else {let id=t.vertices.len();t.vertices.push(brep_topology::Vertex{point:MixedVertex::Root(port.point().clone())});id};
            }
            let id = t.edges.len();
            t.edges.push(brep_topology::Edge {
                degenerate: false,
                vertices,
                curve: MixedEdge::Root(edge.clone()),
            });
            ids.push(id);
        }
        holes.push(append_loop(t, &ids, support, cut.orientation > 0));
        outers.push(append_loop(t, &ids, support, cut.orientation < 0));
    }
    let mut faces = vec![face];
    t.faces[face].holes.clear();
    for i in 0..n {
        let id = t.faces.len();
        faces.push(id);
        t.faces.push(brep_topology::Face {
            surface: surface.clone(),
            outer: outers[i],
            holes: Vec::new(),
        });
    }
    for i in 0..n {
        let owner = parents[i].map(|p| faces[p + 1]).unwrap_or(face);
        t.faces[owner].holes.push(holes[i]);
    }
    for hole in old_holes {
        let signature: Vec<_> = cuts.iter()
            .map(|(cut, _)| cut.inherited_holes.contains(&hole))
            .collect();
        let mut owner = None;
        for i in 0..n {
            if signature[i] {
                let mut expected = vec![false; n];
                let mut current = Some(i);
                while let Some(k) = current {
                    expected[k] = true;
                    current = parents[k];
                }
                if expected == signature {
                    owner = Some(faces[i + 1]);
                    break;
                }
            }
        }
        if owner.is_none() && signature.iter().any(|x| *x) {
            return Err(unproved("Original hole has inconsistent contour ancestry"));
        }
        t.faces[owner.unwrap_or(face)].holes.push(hole);
    }
    for shell in &mut t.shells {
        if let Some(use_) = shell.faces.iter().find(|u| u.face == face).cloned() {
            for &face in &faces[1..] {
                shell.faces.push(brep_topology::FaceUse {
                    face,
                    reversed: use_.reversed,
                });
            }
        }
    }
    Ok((parents, faces))
}

pub struct QualifiedArrangementMaterialRectangle {
    arrangement: Arc<CensusFaceArrangement>,
    face: usize,
    material: QualifiedCensusMaterialRectangle,
}
impl QualifiedArrangementMaterialRectangle {
    pub fn arrangement(&self) -> &Arc<CensusFaceArrangement> {
        &self.arrangement
    }
    pub fn face(&self) -> usize {
        self.face
    }
    pub fn material(&self) -> &QualifiedCensusMaterialRectangle {
        &self.material
    }
    pub fn reverify(&self) -> Result<()> {
        self.arrangement.reverify()?;
        self.material.reverify()?;
        let face = self.arrangement.face_for_signature(
            self.material
                .region()
                .signature()
                .ok_or_else(|| unproved("Arrangement material lost its contour signature"))?,
        )?;
        if face != self.face {
            return Err(unproved("Arrangement material face ownership changed"));
        }
        Ok(())
    }
}
impl CensusFaceArrangement {
    /// Search natural UV space breadth-first so smaller regions receive probes
    /// even when nearby cells already have witnesses for larger regions.
    pub(super) fn discover_boolean_regions(
        self: &Arc<Self>,
        operation: crate::closed_solid::BooleanMaterialOp,
        directions: &[[f64; 3]],
        limits: CutMaterialLimits,
        max_cells: usize,
    ) -> Result<Vec<QualifiedArrangementMaterialRectangle>> {
        Ok(self.census.discover_arrangement_material(
            self.support, &self.parents, &self.faces, operation, directions, limits, max_cells,
        )?.into_iter().map(|(face, material)| QualifiedArrangementMaterialRectangle {
            arrangement: self.clone(), face, material,
        }).collect())
    }

    /// Material ownership is bound to this exact immutable topology and its
    /// literal region face, rather than to an unattached bit vector.
    pub fn qualify_boolean_rectangle(
        self: &Arc<Self>,
        operation: crate::closed_solid::BooleanMaterialOp,
        rectangle: [[f64; 2]; 2],
        directions: &[[f64; 3]],
        limits: CutMaterialLimits,
    ) -> Result<QualifiedArrangementMaterialRectangle> {
        let side = if self.support == 0 {
            crate::closed_solid::BooleanSourceSide::First
        } else {
            crate::closed_solid::BooleanSourceSide::Second
        };
        let material = self
            .census
            .qualify_boolean_rectangle(operation, side, rectangle, directions, limits)?;
        let face =
            self.face_for_signature(material.region().signature().ok_or_else(|| {
                unproved("Arrangement rectangle has no exhaustive side signature")
            })?)?;
        Ok(QualifiedArrangementMaterialRectangle {
            arrangement: self.clone(),
            face,
            material,
        })
    }
}


impl QualifiedInteriorSsCensus {
    pub(super) fn discover_arrangement_material(
        self: &Arc<Self>,
        support: usize,
        parents: &[Option<usize>],
        faces: &[usize],
        operation: crate::closed_solid::BooleanMaterialOp,
        directions: &[[f64; 3]],
        limits: CutMaterialLimits,
        max_cells: usize,
    ) -> Result<Vec<(usize, QualifiedCensusMaterialRectangle)>> {
        let cut = self.parent(support)?;
        let bounds = finite_face_cover(cut.source.model(), cut.face)?;
        let side = [crate::closed_solid::BooleanSourceSide::First,
            crate::closed_solid::BooleanSourceSide::Second][support];
        discover_material_regions(bounds, faces.len(), support, max_cells, |probe| {
            let proof = self.qualify_boolean_rectangle(operation, side, probe, directions, limits)?;
            let signature = proof.region().signature()
                .ok_or_else(|| unproved("Arrangement rectangle has no exhaustive side signature"))?;
            let face = face_for_signature(parents, faces, signature)?;
            Ok((face, proof.boolean_material().retain(), proof))
        })
    }
}

pub(super) fn discover_network_face_material(
    query: crate::closed_solid::BooleanFace<'_>,
    support: usize,
    cuts: &[(&ClosedCutCandidate, usize)],
    parents: &[Option<usize>],
    faces: &[usize],
    directions: &[[f64; 3]],
    limits: CutMaterialLimits,
    max_cells: usize,
) -> Result<Vec<brep_topology::FaceUse>> {
    let bounds = finite_face_cover(query.solids[support].model(), query.face)?;
    let retained = discover_material_regions(bounds, faces.len(), support, max_cells, |probe| {
        let signature = cuts.iter().map(|(cut, _)| {
            let report = cut.classify_rectangle(probe, limits.region)?;
            match (report.inside_piece(), report.outside_piece()) {
                (Location::Inside, Location::Outside) => Ok(true),
                (Location::Outside, Location::Inside) => Ok(false),
                _ => Err(unproved("Network material probe has no complete contour signature")),
            }
        }).collect::<Result<Vec<_>>>()?;
        let face = face_for_signature(parents, faces, &signature)?;
        let proof = crate::closed_solid::qualify_boolean_face_box(
            crate::closed_solid::BooleanFace {
                solids: query.solids, operation: query.operation, side: query.side, face: query.face,
            },
            probe, directions, limits.boolean,
        )?;
        Ok((face, proof.retain(), proof.reversed_surface()))
    })?;
    Ok(retained.into_iter().map(|(face, reversed)| brep_topology::FaceUse { face, reversed }).collect())
}

fn discover_material_regions<T>(
    bounds: [[f64; 2]; 2],
    face_count: usize,
    support: usize,
    max_cells: usize,
    mut qualify: impl FnMut([[f64; 2]; 2]) -> Result<(usize, bool, T)>,
) -> Result<Vec<(usize, T)>> {
    // Include the split-axis choice in the queue so both normalized
    // parameters are refined regardless of their physical units.
    let mut pending = std::collections::VecDeque::from([(bounds, 0)]);
    let mut witnessed = std::collections::BTreeSet::new();
    let mut retained = Vec::new();
    for _ in 0..max_cells {
        let Some((mut cell, axis)) = pending.pop_front() else { break };
        let probe = cell.map(|[lo, hi]| {
            let mid = lo * 0.5 + hi * 0.5;
            [mid * 0.96875 + lo * 0.03125, mid * 0.96875 + hi * 0.03125]
        });
        if probe.iter().all(|[lo, hi]| lo < hi) {
            if let Ok((face, retain, proof)) = qualify(probe) {
                if witnessed.insert(face) && retain {
                    retained.push((face, proof));
                }
                if witnessed.len() == face_count {
                    return Ok(retained);
                }
            }
        }
        let [lo, hi] = cell[axis];
        let mid = lo * 0.5 + hi * 0.5;
        if lo < mid && mid < hi {
            let mut right = cell;
            cell[axis][1] = mid;
            right[axis][0] = mid;
            pending.extend([(cell, 1 - axis), (right, 1 - axis)]);
        }
    }
    Err(unproved(&format!(
        "Boolean region witness search found {} of {} regions on support {}",
        witnessed.len(), face_count, support,
    )))
}
