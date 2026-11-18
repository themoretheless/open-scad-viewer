use super::*;

/// Conservative source-face region for one direct edit. `seed_faces` are the
/// requested faces; `result_expanded_faces` are additional faces whose source
/// bounds intersect the edited result envelope. The latter prevents a long
/// extension from being evaluated only against the pre-edit geometry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirtyRegion {
    pub seed_faces: BTreeSet<usize>,
    /// Conservative region as indices into the source model.
    pub faces: BTreeSet<usize>,
    pub result_expanded_faces: BTreeSet<usize>,
    /// Matching region plus changed/new faces as indices into the result model.
    pub result_faces: BTreeSet<usize>,
}

impl DirtyRegion {
    pub fn len(&self) -> usize {
        self.faces.len()
    }

    pub fn is_empty(&self) -> bool {
        self.faces.is_empty()
    }
}

#[derive(Clone, Copy, Debug)]
struct Bounds {
    min: [f64; 3],
    max: [f64; 3],
}

impl Bounds {
    fn empty() -> Self {
        Self {
            min: [f64::INFINITY; 3],
            max: [f64::NEG_INFINITY; 3],
        }
    }

    fn is_empty(&self) -> bool {
        self.min[0] > self.max[0]
    }

    fn include(&mut self, p: [f64; 3]) {
        for axis in 0..3 {
            self.min[axis] = self.min[axis].min(p[axis]);
            self.max[axis] = self.max[axis].max(p[axis]);
        }
    }

    fn union(&mut self, other: &Self) {
        if !other.is_empty() {
            self.include(other.min);
            self.include(other.max);
        }
    }

    fn intersects(&self, other: &Self, tolerance: f64) -> bool {
        !self.is_empty()
            && !other.is_empty()
            && (0..3).all(|axis| {
                self.min[axis] <= other.max[axis] + tolerance
                    && other.min[axis] <= self.max[axis] + tolerance
            })
    }
}

/// Control-hull bounds are conservative for the positive-weight NURBS used by
/// the admitted direct edits and also cover poles and curved rim edges.
fn face_bounds(model: &Model, face: usize) -> Bounds {
    let mut bounds = Bounds::empty();
    for point in model.faces[face].surface.control_points.iter().flatten() {
        if point.len() == 3 {
            bounds.include([point[0], point[1], point[2]]);
        }
    }
    for &ring in std::iter::once(&model.faces[face].outer).chain(&model.faces[face].holes) {
        for coedge in &model.loops[ring].coedges {
            let edge = &model.edges[coedge.edge];
            for &vertex in &edge.vertices {
                bounds.include(model.vertices[vertex].point);
            }
            for point in &edge.curve.control_points {
                if point.len() == 3 {
                    bounds.include([point[0], point[1], point[2]]);
                }
            }
        }
    }
    bounds
}

fn same_bounds(a: &Bounds, b: &Bounds) -> bool {
    a.is_empty() == b.is_empty() && (a.is_empty() || (a.min == b.min && a.max == b.max))
}

/// Faces whose surface or pcurves actually changed in an in-place edit. This
/// is deliberately separate from the conservative dirty region: cloning a
/// model is not counted as rebuilding every face.
pub(super) fn rebuilt_faces_in_place(source: &Model, staged: &Model) -> usize {
    debug_assert_eq!(source.faces.len(), staged.faces.len());
    (0..source.faces.len())
        .filter(|&face| {
            if source.faces[face].surface != staged.faces[face].surface {
                return true;
            }
            for &ring in std::iter::once(&source.faces[face].outer).chain(&source.faces[face].holes)
            {
                if source.loops[ring]
                    .coedges
                    .iter()
                    .zip(&staged.loops[ring].coedges)
                    .any(|(a, b)| a.pcurve != b.pcurve)
                {
                    return true;
                }
            }
            false
        })
        .count()
}

/// Seed + two AAG adjacency levels, conservatively expanded against the
/// *result* control-hull envelope. New or geometrically changed result faces
/// always contribute to that envelope, including faces with fresh persistent
/// ids after a polygonal rebuild.
pub fn dirty_region_for_result(
    source: &Model,
    result: &Model,
    seeds: &[usize],
    budget: &Budget,
) -> Result<DirtyRegion> {
    let mut guard = budget.guard("direct-edit-dirty-region");
    guard.check()?;
    let seed_faces: BTreeSet<usize> = seeds.iter().copied().collect();
    if let Some(&face) = seed_faces.iter().find(|&&face| face >= source.faces.len()) {
        return Err(error(
            DIRTY_INVALID,
            format!("Dirty-region seed face {face} is out of range"),
        ));
    }
    let aag = crate::aag::Aag::build(source, budget)?;
    let mut base = seed_faces.clone();
    let mut frontier = seed_faces.clone();
    for _ in 0..2 {
        let mut next = BTreeSet::new();
        for face in frontier {
            guard.tick()?;
            for &neighbor in &aag.nodes[face].neighbors {
                if base.insert(neighbor) {
                    next.insert(neighbor);
                }
            }
        }
        frontier = next;
    }

    let source_bounds: Vec<Bounds> = (0..source.faces.len())
        .map(|face| face_bounds(source, face))
        .collect();
    let result_bounds: Vec<Bounds> = (0..result.faces.len())
        .map(|face| face_bounds(result, face))
        .collect();
    let source_by_id: BTreeMap<_, _> = source
        .1
        .faces
        .iter()
        .enumerate()
        .map(|(face, &id)| (id, face))
        .collect();
    let result_to_source: Vec<Option<usize>> = result
        .1
        .faces
        .iter()
        .map(|id| source_by_id.get(id).copied())
        .collect();
    let always_include: Vec<bool> = (0..result.faces.len())
        .map(|face| {
            let Some(source_face) = result_to_source[face] else {
                return true;
            };
            source.faces[source_face].surface != result.faces[face].surface
                || !same_bounds(&source_bounds[source_face], &result_bounds[face])
        })
        .collect();

    let tolerance = source.tolerance_mm.max(result.tolerance_mm).max(1e-7) * 8.;
    let mut dirty = base.clone();
    loop {
        guard.check()?;
        let mut envelope = Bounds::empty();
        for face in 0..result.faces.len() {
            guard.tick()?;
            if always_include[face]
                || result_to_source[face].is_some_and(|source_face| dirty.contains(&source_face))
            {
                envelope.union(&result_bounds[face]);
            }
        }
        if envelope.is_empty() {
            break;
        }
        let mut changed = false;
        for face in 0..source.faces.len() {
            guard.tick()?;
            if !dirty.contains(&face) && source_bounds[face].intersects(&envelope, tolerance) {
                dirty.insert(face);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let result_expanded_faces = dirty.difference(&base).copied().collect();
    let result_faces = (0..result.faces.len())
        .filter(|&face| {
            always_include[face]
                || result_to_source[face].is_some_and(|source_face| dirty.contains(&source_face))
        })
        .collect();
    Ok(DirtyRegion {
        seed_faces,
        faces: dirty,
        result_expanded_faces,
        result_faces,
    })
}
