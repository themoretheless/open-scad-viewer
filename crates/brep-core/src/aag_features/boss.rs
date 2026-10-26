use super::*;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BossKind {
    Cylindrical,
    Prismatic,
    /// Drafted, conical and otherwise non-prismatic wall subgraphs.
    Freeform,
}

/// Material protrusion mirrored from the pocket template: convex top/wall
/// edges, concave base ring and material immediately behind every wall.
#[derive(Clone, Debug, PartialEq)]
pub struct BossFeature {
    /// Top and wall faces, ascending.
    pub faces: Vec<usize>,
    pub top_faces: Vec<usize>,
    pub wall_faces: Vec<usize>,
    /// Supporting faces outside the feature at its concave base ring.
    pub base_faces: Vec<usize>,
    pub kind: BossKind,
    pub height: f64,
    pub top_area: f64,
    pub top_normal: [f64; 3],
}

fn boss_error(message: impl Into<String>) -> Error {
    Error::new("BREP_AAG_BOSS_INPUT", message)
}

/// Recognize bosses over fresh 865 attributes. The graph candidate matcher is
/// shared with small-feature scanning; this API only removes its volume cutoff
/// and enriches the result with roles, type and support faces.
pub fn find_bosses(model: &Model, graph: &Aag, budget: &Budget) -> Result<Vec<BossFeature>> {
    require_finite_f64(model.tolerance_mm, "model.tolerance_mm")?;
    if graph.nodes.len() != model.faces.len() {
        return Err(boss_error("AAG node count does not match model faces"));
    }
    if !graph.stale_face_attrs(model).is_empty() {
        return Err(boss_error("AAG face attributes missing or stale; run attach_face_attrs first"));
    }
    let candidates = super::small::find_boss_candidates(model, graph, budget, f64::INFINITY)?;
    let linear_tol = (model.tolerance_mm * 100.).max(1e-7);
    let mut guard = budget.guard("aag-find-bosses-enrich");
    let mut result = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        guard.tick()?;
        let top_offset = dot(candidate.top_normal, candidate.top_center);
        let top_faces = candidate
            .faces
            .iter()
            .copied()
            .filter(|&face| {
                let attrs = graph.nodes[face]
                    .attrs
                    .as_ref()
                    .expect("fresh attrs checked");
                if attrs.class != SurfaceClass::Plane {
                    return false;
                }
                super::pockets::outward_normal(model, face, 0.5, 0.5)
                    .ok()
                    .flatten()
                    .is_some_and(|normal| {
                        dot(normal, candidate.top_normal) > 1. - AXIS_DOT_TOLERANCE
                            && (dot(normal, candidate.top_center) - top_offset).abs() <= linear_tol
                    })
            })
            .collect::<Vec<_>>();
        let wall_faces = candidate
            .faces
            .iter()
            .copied()
            .filter(|face| !top_faces.contains(face))
            .collect::<Vec<_>>();
        let top_area = top_faces
            .iter()
            .map(|&face| {
                graph.nodes[face]
                    .attrs
                    .as_ref()
                    .expect("fresh attrs checked")
                    .area
            })
            .sum::<f64>();
        let kind = if wall_faces.iter().all(|&face| {
            graph.nodes[face]
                .attrs
                .as_ref()
                .expect("fresh attrs checked")
                .class
                == SurfaceClass::Cylinder
        }) {
            BossKind::Cylindrical
        } else if wall_faces.iter().all(|&face| {
            graph.nodes[face]
                .attrs
                .as_ref()
                .expect("fresh attrs checked")
                .class
                == SurfaceClass::Plane
        }) {
            BossKind::Prismatic
        } else {
            BossKind::Freeform
        };
        let member = candidate.faces.iter().copied().collect::<BTreeSet<_>>();
        let mut base_faces = BTreeSet::new();
        for edge in &graph.edges {
            guard.tick()?;
            if edge.class != DihedralClass::Concave || edge.uses.len() != 2 {
                continue;
            }
            let inside = edge.uses.iter().filter(|usage| member.contains(&usage.face)).count();
            if inside == 1 {
                base_faces.extend(edge.uses.iter().filter(|usage| !member.contains(&usage.face)).map(|usage| usage.face));
            }
        }
        result.push(BossFeature {
            faces: candidate.faces,
            top_faces,
            wall_faces,
            base_faces: base_faces.into_iter().collect(),
            kind,
            height: candidate.height,
            top_area,
            top_normal: candidate.top_normal,
        });
    }
    Ok(result)
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

    fn translated(model: &Model, delta: [f64; 3]) -> Model {
        crate::transform::affine(model, [
            [1., 0., 0., delta[0]], [0., 1., 0., delta[1]],
            [0., 0., 1., delta[2]], [0., 0., 0., 1.],
        ]).unwrap()
    }

    fn cylindrical_boss() -> Model {
        let plate = crate::cuboid([0.; 3], [30., 30., 2.]).unwrap();
        let pin = translated(&crate::cylinder(4., 6.).unwrap(), [15., 15., 2.]);
        crate::boolean(&plate, &pin, "union").unwrap()
    }

    #[test]
    fn boss_cylindrical_and_prismatic_parameters() {
        let model = cylindrical_boss();
        let bosses = find_bosses(&model, &graph(&model), &budget()).unwrap();
        assert_eq!(bosses.len(), 1, "{bosses:?}");
        let boss = &bosses[0];
        assert_eq!(boss.kind, BossKind::Cylindrical);
        assert!((boss.height - 6.).abs() < 1e-5, "{}", boss.height);
        assert!((boss.top_area - 16. * std::f64::consts::PI).abs() < 1e-4, "{}", boss.top_area);
        assert!(!boss.base_faces.is_empty());
        assert!(find_pockets(&model, &graph(&model), &budget()).unwrap().is_empty());

        let plate = crate::cuboid([0.; 3], [30., 30., 2.]).unwrap();
        let prism = crate::cuboid([12., 12., 2.], [18., 18., 8.]).unwrap();
        let model = crate::boolean(&plate, &prism, "union").unwrap();
        let bosses = find_bosses(&model, &graph(&model), &budget()).unwrap();
        assert_eq!(bosses.len(), 1, "{bosses:?}");
        assert_eq!(bosses[0].kind, BossKind::Prismatic);
        assert!((bosses[0].height - 6.).abs() < 1e-6);
        assert!((bosses[0].top_area - 36.).abs() < 1e-6);
    }

    #[test]
    fn boss_rejects_pocket_and_validates_input_and_budget() {
        let plate = crate::cuboid([0.; 3], [30., 30., 8.]).unwrap();
        let tool = crate::cuboid([12., 12., 2.], [18., 18., 10.]).unwrap();
        let pocket = crate::boolean(&plate, &tool, "difference").unwrap();
        assert!(find_bosses(&pocket, &graph(&pocket), &budget()).unwrap().is_empty());
        assert!(!find_pockets(&pocket, &graph(&pocket), &budget()).unwrap().is_empty());

        let plain = Aag::build(&pocket, &budget()).unwrap();
        let error = find_bosses(&pocket, &plain, &budget()).unwrap_err();
        assert_eq!(error.code, "BREP_AAG_BOSS_INPUT");
        let tiny = Budget::new(1, 8, 60_000).unwrap();
        let error = find_bosses(&pocket, &graph(&pocket), &tiny).unwrap_err();
        assert_eq!(error.code, "NURBS_RESOURCE_LIMIT");
    }
}
