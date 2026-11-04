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
        if signature.len() != self.parents.len() {
            return Err(unproved("Foreign contour signature length"));
        }
        let mut deepest = None;
        for (i, inside) in signature.iter().enumerate() {
            if *inside {
                let mut expected = vec![false; signature.len()];
                let mut current = Some(i);
                while let Some(k) = current {
                    expected[k] = true;
                    current = self.parents[k];
                }
                if expected == signature {
                    deepest = Some(i);
                    break;
                }
            }
        }
        match deepest {
            Some(i) => Ok(self.faces[i + 1]),
            None if signature.iter().all(|v| !*v) => Ok(self.faces[0]),
            _ => Err(unproved(
                "Contour signature is inconsistent with proved nesting",
            )),
        }
    }
}
fn nesting(
    census: &QualifiedInteriorSsCensus,
    support: usize,
    limits: CensusArrangementLimits,
) -> Result<Vec<Option<usize>>> {
    let n = census.loops.len();
    let mut contains = vec![vec![false; n]; n];
    for i in 0..n {
        let point = census.loops[i].cuts[support].wire.edges()[0]
            .start()
            .point()
            .enclose(limits.geometry)?;
        let uv = [point.uv[2 * support], point.uv[2 * support + 1]];
        for j in 0..n {
            if i != j {
                let cut = &census.loops[j].cuts[support];
                let w = cut.wire.winding_box(support, uv, limits.winding)?.winding();
                if w != 0 && w != cut.orientation {
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
        let parents = nesting(self, support, limits)?;
        let n = self.loops.len();
        let mut t = MixedBrep::from_closed_solid(&base.source)?.topology;
        let surface = t.faces[base.face].surface.clone();
        let old_holes = t.faces[base.face].holes.clone();
        let mut holes = Vec::new();
        let mut outers = Vec::new();
        for component in &self.loops {
            let cut = &component.cuts[support];
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
            holes.push(append_loop(&mut t, &ids, support, cut.orientation > 0));
            outers.push(append_loop(&mut t, &ids, support, cut.orientation < 0));
        }
        let mut faces = vec![base.face];
        t.faces[base.face].holes.clear();
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
            let owner = parents[i].map(|p| faces[p + 1]).unwrap_or(base.face);
            t.faces[owner].holes.push(holes[i]);
        }
        for hole in old_holes {
            let signature: Vec<_> = self
                .loops
                .iter()
                .map(|c| c.cuts[support].inherited_holes.contains(&hole))
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
            t.faces[owner.unwrap_or(base.face)].holes.push(hole);
        }
        for shell in &mut t.shells {
            if let Some(use_) = shell.faces.iter().find(|u| u.face == base.face).cloned() {
                for &face in &faces[1..] {
                    shell.faces.push(brep_topology::FaceUse {
                        face,
                        reversed: use_.reversed,
                    });
                }
            }
        }
        let model = MixedBrep::from_topology(t)?;
        Ok(CensusFaceArrangement {
            census: self.clone(),
            support,
            limits,
            parents,
            faces,
            model,
        })
    }
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
