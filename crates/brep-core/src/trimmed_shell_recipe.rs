//! Closed-shell incidence from immutable face fragments and explicit edge keys.
//! Keys declare sharing; coordinate coincidence never welds independent vertices.
//! This does not certify embedding, material volume or wall thickness.
use crate::trim_sew::RationalCurveDefinition;
use crate::{Edge, FaceUse, Model, Vertex, imprint_pipeline, trimmed_face_recipe::QualifiedFace};
use cad_predicates::ToleranceContext;
use nurbs_core::{Error, Result};
use std::collections::BTreeMap;

pub struct Placement<'a> {
    pub face: &'a QualifiedFace,
    pub reversed: bool,
    /// One explicit shared-edge identity for every local canonical edge.
    pub edge_keys: &'a [u64],
}
pub struct ClosedBoundary {
    model: Model,
}
impl ClosedBoundary {
    pub fn model(&self) -> &Model {
        &self.model
    }
}
fn refuse(code: &'static str, message: &str) -> Error {
    Error::new(code, message)
}
fn root(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}
struct Shared {
    definition: RationalCurveDefinition,
    ends: [usize; 2],
    first_face: usize,
    direction: bool,
    count: usize,
    curve: nurbs_core::curve::Curve,
}
/// Assemble one connected closed shell. Other bodies/cavities and final solid
/// authority require explicit material classification and geometric audits.
pub fn assemble(
    context: &ToleranceContext,
    placements: &[Placement<'_>],
    sources: &[&Model],
) -> Result<ClosedBoundary> {
    let tolerance = context.spatial_bounds().on_mm;
    if placements.len() < 2 || placements.len() > crate::MAX_FACES {
        return Err(refuse(
            "BREP_SHELL_RECIPE_LIMIT",
            "Shell needs 2..MAX_FACES qualified faces",
        ));
    }
    for source in sources {
        source.validate()?;
        if source.tolerance_mm.to_bits() != tolerance.to_bits() {
            return Err(refuse(
                "BREP_SHELL_RECIPE_CONTEXT",
                "Source tolerance differs from the shell context",
            ));
        }
    }
    let mut offsets = vec![];
    let mut points = vec![];
    let mut entities = placements.len() + 2;
    let mut uses = 0usize;
    for p in placements {
        if p.face.context() != &context.spec_identity() || p.edge_keys.len() != p.face.edges().len()
        {
            return Err(refuse(
                "BREP_SHELL_RECIPE_CONTEXT",
                "Face context or edge-key cardinality differs",
            ));
        }
        entities += p.face.vertices().len() + p.face.edges().len() + p.face.loops().len();
        uses += p
            .face
            .loops()
            .iter()
            .map(|l| l.coedges.len())
            .sum::<usize>();
        if entities > crate::MAX_ENTITIES || uses > crate::MAX_COEDGES {
            return Err(refuse(
                "BREP_SHELL_RECIPE_LIMIT",
                "Shell fragment work exceeds topology limits",
            ));
        }
        offsets.push(points.len());
        points.extend(p.face.vertices().iter().map(|v| v.point));
    }
    let mut parent = (0..points.len()).collect::<Vec<_>>();
    let mut shared = BTreeMap::<u64, Shared>::new();
    for (fi, p) in placements.iter().enumerate() {
        // Each local edge belongs to exactly one boundary use in a fragment.
        let mut directions = vec![None; p.face.edges().len()];
        for c in p.face.loops().iter().flat_map(|l| &l.coedges) {
            if directions[c.edge]
                .replace(c.reversed ^ p.reversed)
                .is_some()
            {
                return Err(refuse(
                    "BREP_SHELL_RECIPE_USE",
                    "Local edge occurs more than once",
                ));
            }
        }
        for (i, e) in p.face.edges().iter().enumerate() {
            let key = p.edge_keys[i];
            let ends = e.vertices.map(|v| offsets[fi] + v);
            let definition = RationalCurveDefinition::from_curve(&e.curve)?;
            let direction = directions[i]
                .ok_or_else(|| refuse("BREP_SHELL_RECIPE_USE", "Local edge has no face use"))?;
            if let Some(old) = shared.get_mut(&key) {
                if old.count != 1 || old.first_face == fi {
                    return Err(refuse(
                        "BREP_SHELL_RECIPE_INCIDENCE",
                        "Shared edge requires exactly two distinct face uses",
                    ));
                }
                if old.definition != definition {
                    return Err(refuse(
                        "BREP_SHELL_RECIPE_CURVE",
                        "Shared key has different complete rational definitions",
                    ));
                }
                if old.direction == direction {
                    return Err(refuse(
                        "BREP_SHELL_RECIPE_ORIENTATION",
                        "Shared edge traversals agree after face orientation",
                    ));
                }
                for k in 0..2 {
                    if points[old.ends[k]].map(f64::to_bits) != points[ends[k]].map(f64::to_bits) {
                        return Err(refuse(
                            "BREP_SHELL_RECIPE_ENDPOINT",
                            "Shared curve endpoints have different vertex definitions",
                        ));
                    }
                    let a = root(&mut parent, old.ends[k]);
                    let b = root(&mut parent, ends[k]);
                    parent[b] = a;
                }
                old.count += 1;
            } else {
                shared.insert(
                    key,
                    Shared {
                        definition,
                        ends,
                        first_face: fi,
                        direction,
                        count: 1,
                        curve: e.curve.clone(),
                    },
                );
            }
        }
    }
    if shared.values().any(|e| e.count != 2) {
        return Err(refuse(
            "BREP_SHELL_RECIPE_OPEN",
            "At least one explicit edge has only one face use",
        ));
    }
    let mut vertex_map = BTreeMap::new();
    let mut vertices = vec![];
    let mut local_to_vertex = vec![];
    for i in 0..points.len() {
        let r = root(&mut parent, i);
        let v = *vertex_map.entry(r).or_insert_with(|| {
            let v = vertices.len();
            vertices.push(Vertex { point: points[r] });
            v
        });
        local_to_vertex.push(v);
    }
    let mut edge_map = BTreeMap::new();
    let mut edges = vec![];
    for (key, e) in shared {
        edge_map.insert(key, edges.len());
        edges.push(Edge {
            degenerate: false,
            vertices: e.ends.map(|v| local_to_vertex[v]),
            curve: e.curve,
        });
    }
    let mut loops = vec![];
    let mut faces = vec![];
    let mut shell_faces = vec![];
    for p in placements {
        let loop_offset = loops.len();
        for l in p.face.loops() {
            let mut l = l.clone();
            for c in &mut l.coedges {
                c.edge = edge_map[&p.edge_keys[c.edge]];
            }
            loops.push(l);
        }
        let mut f = p.face.face().clone();
        f.outer += loop_offset;
        for h in &mut f.holes {
            *h += loop_offset;
        }
        shell_faces.push(FaceUse {
            face: faces.len(),
            reversed: p.reversed,
        });
        faces.push(f);
    }
    let model = imprint_pipeline::assemble_imprint_solid(
        vertices,
        edges,
        loops,
        faces,
        shell_faces,
        tolerance,
        sources,
    )?;
    Ok(ClosedBoundary { model })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trimmed_face_recipe::{self, Boundary, Limits};
    fn fixture() -> (Model, ToleranceContext, Vec<QualifiedFace>, Vec<Vec<u64>>) {
        let source = crate::cuboid([0., 0., 0.], [2., 3., 4.]).unwrap();
        from_source(source)
    }
    fn from_source(source: Model) -> (Model, ToleranceContext, Vec<QualifiedFace>, Vec<Vec<u64>>) {
        let context = ToleranceContext::from_brep_tolerance_mm(source.tolerance_mm).unwrap();
        let mut fragments = vec![];
        let mut keys = vec![];
        for face in &source.faces {
            let mut wires = vec![];
            let mut face_keys = vec![];
            for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                wires.push(
                    source.loops[wire]
                        .coedges
                        .iter()
                        .map(|c| {
                            face_keys.push(c.edge as u64);
                            Boundary {
                                curve: source.edges[c.edge].curve.clone(),
                                pcurve: c.pcurve.clone(),
                                reversed: c.reversed,
                            }
                        })
                        .collect(),
                );
            }
            let report = trimmed_face_recipe::assemble(
                &context,
                &face.surface,
                &wires,
                1e-8,
                Limits {
                    pairs: 10000,
                    region_cells: 100000,
                    domain_cells: 100000,
                    agreement_cells: 100000,
                },
            )
            .unwrap();
            fragments.push(report.face.expect(report.reason));
            keys.push(face_keys);
        }
        (source, context, fragments, keys)
    }
    #[test]
    fn shared_face_fragments_reconstruct_closed_shell_and_original_identities() {
        let (source, context, fragments, keys) = fixture();
        let placements = fragments
            .iter()
            .enumerate()
            .map(|(i, face)| Placement {
                face,
                reversed: source.shells[0].faces[i].reversed,
                edge_keys: &keys[i],
            })
            .collect::<Vec<_>>();
        let boundary = assemble(&context, &placements, &[&source]).unwrap();
        let model = boundary.model();
        assert_eq!(model.vertices.len(), 8);
        assert_eq!(model.edges.len(), 12);
        assert_eq!(model.faces.len(), 6);
        assert!(model.shells[0].closed);
        model.validate().unwrap();
        let old = source
            .1
            .edges
            .iter()
            .collect::<std::collections::BTreeSet<_>>();
        let new = model
            .1
            .edges
            .iter()
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(old, new);
        for (old, new) in [
            (&source.1.vertices, &model.1.vertices),
            (&source.1.loops, &model.1.loops),
            (&source.1.faces, &model.1.faces),
            (&source.1.shells, &model.1.shells),
            (&source.1.bodies, &model.1.bodies),
        ] {
            assert_eq!(
                old.iter().collect::<std::collections::BTreeSet<_>>(),
                new.iter().collect::<std::collections::BTreeSet<_>>()
            );
        }

        assert!(
            crate::boundary_agreement::verify(model, 100000)
                .unwrap()
                .complete
        );
    }
    #[test]
    fn coordinate_coincidence_does_not_replace_explicit_edge_sharing() {
        let (source, context, fragments, mut keys) = fixture();
        keys[0][0] = 1000;
        let placements = fragments
            .iter()
            .enumerate()
            .map(|(i, face)| Placement {
                face,
                reversed: source.shells[0].faces[i].reversed,
                edge_keys: &keys[i],
            })
            .collect::<Vec<_>>();
        assert_eq!(
            assemble(&context, &placements, &[&source])
                .err()
                .unwrap()
                .code,
            "BREP_SHELL_RECIPE_OPEN"
        );
    }
    #[test]
    fn key_collision_wrong_face_orientation_and_foreign_context_refuse() {
        let (source, context, fragments, mut keys) = fixture();
        keys[0][0] = keys[0][1];
        fn make<'a>(
            source: &Model,
            fragments: &'a [QualifiedFace],
            keys: &'a [Vec<u64>],
            flip: bool,
        ) -> Vec<Placement<'a>> {
            fragments
                .iter()
                .enumerate()
                .map(|(i, face)| Placement {
                    face,
                    reversed: source.shells[0].faces[i].reversed ^ (flip && i == 0),
                    edge_keys: &keys[i],
                })
                .collect()
        }
        assert!(
            assemble(
                &context,
                &make(&source, &fragments, &keys, false),
                &[&source]
            )
            .is_err()
        );
        let (_, _, _, keys) = fixture();
        assert_eq!(
            assemble(
                &context,
                &make(&source, &fragments, &keys, true),
                &[&source]
            )
            .err()
            .unwrap()
            .code,
            "BREP_SHELL_RECIPE_ORIENTATION"
        );
        assert!(
            assemble(
                &ToleranceContext::default_valid(),
                &make(&source, &fragments, &keys, false),
                &[&source]
            )
            .is_err()
        );
    }
    #[test]
    fn rational_curved_faces_and_end_caps_reconstruct_one_canonical_closed_boundary() {
        use nurbs_core::curve::Curve;
        let arc = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]],
            weights: vec![1., 0.5f64.sqrt(), 1.],
            periodic: false,
        };
        let wire = vec![
            arc,
            Curve::from_polyline(vec![vec![0., 1.], vec![0., 0.]]).unwrap(),
            Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap(),
        ];
        let (source, context, fragments, keys) =
            from_source(crate::prism::extrude(&[wire], 0., 2.).unwrap());
        let placements = fragments
            .iter()
            .enumerate()
            .map(|(i, face)| Placement {
                face,
                reversed: source.shells[0].faces[i].reversed,
                edge_keys: &keys[i],
            })
            .collect::<Vec<_>>();
        let result = assemble(&context, &placements, &[&source]).unwrap();
        let model = result.model();
        assert_eq!(model.edges.len(), source.edges.len());
        assert_eq!(model.vertices.len(), source.vertices.len());
        assert!(
            model
                .edges
                .iter()
                .any(|e| e.curve.weights.windows(2).any(|w| w[0] != w[1]))
        );
        assert!(
            crate::boundary_agreement::verify(model, 100000)
                .unwrap()
                .complete
        );
        for edge in &source.edges {
            assert!(model.edges.iter().any(|e| e.curve == edge.curve));
        }
    }
    #[test]
    fn shared_key_with_a_different_rational_definition_is_rejected() {
        let (source, context, fragments, mut keys) = fixture();
        let alias = *keys[0].iter().find(|k| **k != keys[1][0]).unwrap();
        keys[1][0] = alias;
        let placements = fragments
            .iter()
            .enumerate()
            .map(|(i, face)| Placement {
                face,
                reversed: source.shells[0].faces[i].reversed,
                edge_keys: &keys[i],
            })
            .collect::<Vec<_>>();
        assert_eq!(
            assemble(&context, &placements, &[&source])
                .err()
                .unwrap()
                .code,
            "BREP_SHELL_RECIPE_CURVE"
        );
    }
}
