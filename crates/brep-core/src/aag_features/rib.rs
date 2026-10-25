use super::*;
use std::collections::BTreeSet;

/// A rib must be narrow relative to both the whole part and its own side
/// faces: thickness < 10% of bbox diagonal and < 20% of sqrt(smaller side
/// area). These deliberately conservative limits reject ordinary block walls.
const RIB_BBOX_FRACTION: f64 = 0.10;
const RIB_FACE_FRACTION: f64 = 0.20;

#[derive(Clone, Debug, PartialEq)]
pub struct RibFeature {
    /// The two opposing planar side faces, ascending.
    pub faces: [usize; 2],
    /// Distinct supporting faces contacted through concave boundaries.
    pub support_faces: Vec<usize>,
    /// Mean plane-to-plane distance over the medial sample grid.
    pub thickness: f64,
    pub medial_samples: usize,
}

fn rib_error(message: impl Into<String>) -> Error {
    Error::new("BREP_AAG_RIB_INPUT", message)
}

fn bbox_diagonal(model: &Model) -> f64 {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for point in model.faces.iter().flat_map(|face| face.surface.control_points.iter().flatten()) {
        for axis in 0..3.min(point.len()) {
            lo[axis] = lo[axis].min(point[axis]);
            hi[axis] = hi[axis].max(point[axis]);
        }
    }
    norm([hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]])
}

fn bounds(model: &Model, face: usize) -> Result<Option<([f64; 3], [f64; 3])>> {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    let mut count = 0;
    for wire in std::iter::once(model.faces[face].outer).chain(model.faces[face].holes.iter().copied()) {
        for vertex in crate::operations::loop_vertices(model, wire)? {
            let point = model.vertices[vertex].point;
            count += 1;
            for axis in 0..3 {
                lo[axis] = lo[axis].min(point[axis]);
                hi[axis] = hi[axis].max(point[axis]);
            }
        }
    }
    Ok((count > 0).then_some((lo, hi)))
}

fn in_bounds(point: [f64; 3], bounds: ([f64; 3], [f64; 3]), tolerance: f64) -> bool {
    (0..3).all(|axis| point[axis] >= bounds.0[axis] - tolerance && point[axis] <= bounds.1[axis] + tolerance)
}

/// Find thin planar rib pairs. Thickness approximates the subgraph medial
/// axis: points sampled on one side are projected onto the opposite plane,
/// checked against its trim bounds, and their distances are averaged.
pub fn find_ribs(model: &Model, graph: &Aag, budget: &Budget) -> Result<Vec<RibFeature>> {
    require_finite_f64(model.tolerance_mm, "model.tolerance_mm")?;
    if graph.nodes.len() != model.faces.len() {
        return Err(rib_error("AAG node count does not match model faces"));
    }
    if !graph.stale_face_attrs(model).is_empty() {
        return Err(rib_error("AAG face attributes missing or stale; run attach_face_attrs first"));
    }
    let mut guard = budget.guard("aag-find-ribs");
    guard.check()?;
    let diagonal = bbox_diagonal(model);
    if !(diagonal.is_finite() && diagonal > 0.) {
        return Err(rib_error("Model bbox diagonal is degenerate"));
    }
    let tolerance = (model.tolerance_mm * 100.).max(1e-7);
    let mut planes = Vec::new();
    for face in 0..model.faces.len() {
        guard.tick()?;
        if graph.nodes[face].attrs.as_ref().expect("fresh attrs checked").class != SurfaceClass::Plane {
            continue;
        }
        let Some(normal) = super::pockets::outward_normal(model, face, 0.5, 0.5)? else { continue };
        let center = super::pockets::face_point(model, face, 0.5, 0.5)?;
        let Some(bounds) = bounds(model, face)? else { continue };
        planes.push((face, normal, dot(normal, center), bounds));
    }

    let mut ribs = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, &(a, na, oa, ba)) in planes.iter().enumerate() {
        for &(b, nb, ob, bb) in planes.iter().skip(index + 1) {
            guard.tick()?;
            if dot(na, nb) >= -1. + AXIS_DOT_TOLERANCE {
                continue;
            }
            let area_a = graph.nodes[a].attrs.as_ref().expect("fresh attrs checked").area;
            let area_b = graph.nodes[b].attrs.as_ref().expect("fresh attrs checked").area;
            let samples = face_samples(model, a, &mut guard)?;
            let mut distance = 0.;
            let mut used = 0;
            for point in samples {
                let projected = crate::aag_features::sub(point, crate_scale(nb, dot(nb, point) - ob));
                if !in_bounds(projected, bb, tolerance) {
                    continue;
                }
                distance += (dot(nb, point) - ob).abs();
                used += 1;
            }
            if used < 4 {
                continue;
            }
            let thickness = distance / used as f64;
            if !(thickness > 10. * model.tolerance_mm
                && thickness < RIB_BBOX_FRACTION * diagonal
                && thickness < RIB_FACE_FRACTION * area_a.min(area_b).sqrt())
            {
                continue;
            }
            // Also require the reverse medial projection, preventing one large
            // plane with only a small overlapping strip from becoming a rib.
            let reverse = face_samples(model, b, &mut guard)?;
            if reverse.iter().filter(|&&point| {
                let projected = crate::aag_features::sub(point, crate_scale(na, dot(na, point) - oa));
                in_bounds(projected, ba, tolerance)
            }).count() < 4 {
                continue;
            }
            let pair = [a.min(b), a.max(b)];
            let mut supports = BTreeSet::new();
            for edge in &graph.edges {
                guard.tick()?;
                if edge.class != DihedralClass::Concave || edge.uses.len() != 2 {
                    continue;
                }
                let inside = edge.uses.iter().filter(|usage| pair.contains(&usage.face)).count();
                if inside == 1 {
                    supports.extend(edge.uses.iter().filter(|usage| !pair.contains(&usage.face)).map(|usage| usage.face));
                }
            }
            // Two independent support contacts are the rib signature. A plain
            // block has convex boundaries and therefore produces none.
            if supports.len() < 2 || !seen.insert((pair, supports.clone())) {
                continue;
            }
            ribs.push(RibFeature {
                faces: pair,
                support_faces: supports.into_iter().collect(),
                thickness,
                medial_samples: used,
            });
        }
    }
    ribs.sort_by_key(|rib| (rib.faces, rib.thickness.to_bits()));
    guard.check()?;
    Ok(ribs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn budget() -> Budget {
        Budget::new(10_000_000, 8, 60_000).unwrap()
    }

    fn graph(model: &Model) -> Aag {
        let mut graph = Aag::build(model, &budget()).unwrap();
        graph.attach_face_attrs(model, &budget()).unwrap();
        graph
    }

    #[test]
    fn rib_between_walls_has_medial_thickness() {
        let base = crate::cuboid([0.; 3], [30., 20., 2.]).unwrap();
        let left = crate::cuboid([0., 0., 2.], [2., 20., 12.]).unwrap();
        let right = crate::cuboid([28., 0., 2.], [30., 20., 12.]).unwrap();
        let rib = crate::cuboid([2., 9., 2.], [28., 11., 10.]).unwrap();
        let body = crate::boolean(&base, &left, "union").unwrap();
        let body = crate::boolean(&body, &right, "union").unwrap();
        let body = crate::boolean(&body, &rib, "union").unwrap();
        let ribs = find_ribs(&body, &graph(&body), &budget()).unwrap();
        assert!(!ribs.is_empty(), "no rib recognized");
        assert!(ribs.iter().any(|rib| (rib.thickness - 2.).abs() < 1e-6), "{ribs:?}");
        assert!(ribs.iter().all(|rib| rib.support_faces.len() >= 2));
    }

    #[test]
    fn rib_rejects_block_walls_and_stale_or_exhausted_input() {
        let block = crate::cuboid([0.; 3], [10., 10., 10.]).unwrap();
        assert!(find_ribs(&block, &graph(&block), &budget()).unwrap().is_empty());
        let plain = Aag::build(&block, &budget()).unwrap();
        assert_eq!(find_ribs(&block, &plain, &budget()).unwrap_err().code, "BREP_AAG_RIB_INPUT");
        let tiny = Budget::new(1, 8, 60_000).unwrap();
        assert_eq!(find_ribs(&block, &graph(&block), &tiny).unwrap_err().code, "NURBS_RESOURCE_LIMIT");
    }
}
