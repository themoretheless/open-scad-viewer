//! Isolate a body for local editing without regenerating untouched identities.
use crate::{Error, Model, Result};
use std::collections::{BTreeMap, BTreeSet};

fn index(ids: BTreeSet<usize>) -> BTreeMap<usize, usize> {
    ids.into_iter()
        .enumerate()
        .map(|(new, old)| (old, new))
        .collect()
}

fn extract(model: &Model, body: usize) -> (Model, BTreeMap<usize, usize>) {
    extract_range(model, body..body + 1)
}

fn extract_range(model: &Model, bodies: std::ops::Range<usize>) -> (Model, BTreeMap<usize, usize>) {
    let mut shell_ids: BTreeSet<usize> = bodies
        .clone()
        .flat_map(|i| {
            let b = &model.bodies[i];
            std::iter::once(b.outer_shell).chain(b.inner_shells.iter().copied())
        })
        .collect();
    // The empty range carries standalone open shells alongside the solids.
    if bodies.is_empty() {
        let owned: BTreeSet<_> = model
            .bodies
            .iter()
            .flat_map(|b| std::iter::once(b.outer_shell).chain(b.inner_shells.iter().copied()))
            .collect();
        shell_ids.extend((0..model.shells.len()).filter(|i| !owned.contains(i)));
    }
    let shells = index(shell_ids);
    let faces = index(
        shells
            .keys()
            .flat_map(|&s| model.shells[s].faces.iter().map(|f| f.face))
            .collect(),
    );
    let loops = index(
        faces
            .keys()
            .flat_map(|&f| {
                std::iter::once(model.faces[f].outer).chain(model.faces[f].holes.iter().copied())
            })
            .collect(),
    );
    let edges = index(
        loops
            .keys()
            .flat_map(|&l| model.loops[l].coedges.iter().map(|e| e.edge))
            .collect(),
    );
    let vertices = index(
        edges
            .keys()
            .flat_map(|&e| model.edges[e].vertices)
            .collect(),
    );
    let mut out = Model(
        brep_topology::Model {
            vertices: Vec::new(),
            edges: Vec::new(),
            loops: Vec::new(),
            faces: Vec::new(),
            shells: Vec::new(),
            bodies: Vec::new(),
            tolerance_mm: model.tolerance_mm,
        },
        crate::TopologyIds {
            lineage: model.1.lineage.clone(),
            ..Default::default()
        },
    );
    out.0.vertices = vertices
        .keys()
        .map(|&i| model.vertices[i].clone())
        .collect();
    out.0.edges = edges
        .keys()
        .map(|&i| {
            let mut e = model.edges[i].clone();
            e.vertices = e.vertices.map(|v| vertices[&v]);
            e
        })
        .collect();
    out.0.loops = loops
        .keys()
        .map(|&i| {
            let mut l = model.loops[i].clone();
            for e in &mut l.coedges {
                e.edge = edges[&e.edge];
            }
            l
        })
        .collect();
    out.0.faces = faces
        .keys()
        .map(|&i| {
            let mut f = model.faces[i].clone();
            f.outer = loops[&f.outer];
            for l in &mut f.holes {
                *l = loops[l];
            }
            f
        })
        .collect();
    out.0.shells = shells
        .keys()
        .map(|&i| {
            let mut s = model.shells[i].clone();
            for f in &mut s.faces {
                f.face = faces[&f.face];
            }
            s
        })
        .collect();
    out.0.bodies = bodies
        .clone()
        .map(|i| {
            let mut b = model.bodies[i].clone();
            b.outer_shell = shells[&b.outer_shell];
            for s in &mut b.inner_shells {
                *s = shells[s];
            }
            b
        })
        .collect();
    out.1.vertices = vertices.keys().map(|&i| model.1.vertices[i]).collect();
    out.1.edges = edges.keys().map(|&i| model.1.edges[i]).collect();
    out.1.loops = loops.keys().map(|&i| model.1.loops[i]).collect();
    out.1.faces = faces.keys().map(|&i| model.1.faces[i]).collect();
    out.1.shells = shells.keys().map(|&i| model.1.shells[i]).collect();
    out.1.bodies = bodies.map(|i| model.1.bodies[i]).collect();
    out.refresh_change_set(&[model]);
    (out, faces)
}

fn append(out: &mut Model, part: &Model) {
    let (v, e, l, f, s) = (
        out.vertices.len(),
        out.edges.len(),
        out.loops.len(),
        out.faces.len(),
        out.shells.len(),
    );
    out.0.vertices.extend(part.vertices.iter().cloned());
    out.0
        .edges
        .extend(part.edges.iter().cloned().map(|mut item| {
            item.vertices = item.vertices.map(|i| i + v);
            item
        }));
    out.0
        .loops
        .extend(part.loops.iter().cloned().map(|mut item| {
            for u in &mut item.coedges {
                u.edge += e;
            }
            item
        }));
    out.0
        .faces
        .extend(part.faces.iter().cloned().map(|mut item| {
            item.outer += l;
            for u in &mut item.holes {
                *u += l;
            }
            item
        }));
    out.0
        .shells
        .extend(part.shells.iter().cloned().map(|mut item| {
            for u in &mut item.faces {
                u.face += f;
            }
            item
        }));
    out.0
        .bodies
        .extend(part.bodies.iter().cloned().map(|mut item| {
            item.outer_shell += s;
            for u in &mut item.inner_shells {
                *u += s;
            }
            item
        }));
    out.1.vertices.extend(&part.1.vertices);
    out.1.edges.extend(&part.1.edges);
    out.1.loops.extend(&part.1.loops);
    out.1.faces.extend(&part.1.faces);
    out.1.shells.extend(&part.1.shells);
    out.1.bodies.extend(&part.1.bodies);
    for record in &part.1.lineage {
        if !out.1.lineage.contains(record) {
            out.1.lineage.push(record.clone());
        }
    }
}

pub(crate) fn edit_face(
    model: &Model,
    face: usize,
    edit: impl FnOnce(&Model, usize) -> Result<Model>,
) -> Result<Model> {
    model.validate()?;
    let owners: Vec<_> = model
        .bodies
        .iter()
        .enumerate()
        .filter(|(_, b)| {
            std::iter::once(b.outer_shell)
                .chain(b.inner_shells.iter().copied())
                .any(|s| model.shells[s].faces.iter().any(|u| u.face == face))
        })
        .map(|(i, _)| i)
        .collect();
    if owners.len() != 1 {
        return Err(Error::new(
            "BREP_INVALID_SELECTION",
            "Face must belong to exactly one body",
        ));
    }
    let owner = owners[0];
    let (part, faces) = extract(model, owner);
    part.validate()?;
    let edited = edit(&part, faces[&face])?;
    edited.validate()?;
    // Copy untouched topology in at most two ranges. Extracting each body
    // separately cloned the entire assembly and naming history N times.
    let mut out = if owner == 0 {
        edited.clone()
    } else {
        let mut prefix = extract_range(model, 0..owner).0;
        append(&mut prefix, &edited);
        prefix
    };
    if owner + 1 < model.bodies.len() {
        append(
            &mut out,
            &extract_range(model, owner + 1..model.bodies.len()).0,
        );
    }
    let owned_shells: usize = model.bodies.iter().map(|b| 1 + b.inner_shells.len()).sum();
    if owned_shells < model.shells.len() {
        append(&mut out, &extract_range(model, 0..0).0);
    }
    out.refresh_change_set(&[model, &edited]);
    out.validate()?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_standalone_shells_beside_edited_components() {
        let text = include_str!("../../../tests/fixtures/step-v6/self-authored-mixed-unit-product-assembly.step");
        let (mut model, _, _) = crate::step_interchange_v3::import_step_v9(text).unwrap();
        let source = model.clone();
        let mut surface = crate::cuboid([500., 0., 0.], [502., 2., 2.]).unwrap();
        surface.0.bodies.clear(); surface.1.bodies.clear();
        surface.0.shells[0].closed = false;
        surface.refresh_change_set(&[]);
        surface.validate().unwrap();
        append(&mut model, &surface);
        model.refresh_change_set(&[&source, &surface]);
        let face = model.faces.iter().position(|f| f.surface.control_points.iter().flatten().all(|p| (p[2] - 101.6).abs() < 1e-8)).unwrap();
        let result = crate::operations::push_planar_face(&model, face, 1.).unwrap();
        assert_eq!(result.faces.len(), model.faces.len());
        let untouched = extract_range(&result, 0..0).0;
        assert_eq!(value_codec::to_string(&untouched.0).unwrap(), value_codec::to_string(&surface.0).unwrap());
        assert_eq!(untouched.1.faces, surface.1.faces);
        assert_eq!(untouched.1.shells, surface.1.shells);
    }
    #[test]
    #[ignore = "explicit assembly edit timing"]
    fn assembly_edit_timing() {
        let count: usize = std::env::var("CAD_BODY_EDIT_COUNT")
            .unwrap_or_else(|_| "64".into())
            .parse()
            .unwrap();
        let parts: Vec<_> = (0..count)
            .map(|i| crate::cuboid([i as f64 * 3., 0., 0.], [i as f64 * 3. + 2., 2., 2.]).unwrap())
            .collect();
        let mut model = parts[0].clone();
        for part in &parts[1..] {
            append(&mut model, part);
        }
        model.refresh_change_set(&parts.iter().collect::<Vec<_>>());
        model.validate().unwrap();
        let owner = count / 2;
        let face = model.shells[model.bodies[owner].outer_shell]
            .faces
            .iter()
            .find(|f| {
                model.faces[f.face]
                    .surface
                    .control_points
                    .iter()
                    .flatten()
                    .all(|p| p[2] == 2.)
            })
            .unwrap()
            .face;
        let start = std::time::Instant::now();
        let result = crate::operations::push_planar_face(&model, face, 1.).unwrap();
        eprintln!(
            "assembly bodies={count} edit_ms={:.3}",
            start.elapsed().as_secs_f64() * 1000.
        );
        assert_eq!(result.1.bodies, model.1.bodies);
        for i in 0..count {
            let got = extract(&result, i).0;
            if i == owner {
                assert_eq!(
                    got.vertices.iter().map(|v| v.point[2]).fold(0., f64::max),
                    3.
                );
            } else {
                assert_eq!(
                    value_codec::to_string(&got.0).unwrap(),
                    value_codec::to_string(&parts[i].0).unwrap()
                );
                assert_eq!(got.1.faces, parts[i].1.faces);
            }
        }
    }
    #[test]
    fn cylinder_edit_preserves_separate_body_with_cavity() {
        let cavity = crate::operations::boolean(
            &crate::cuboid([10., 10., 10.], [20., 20., 20.]).unwrap(),
            &crate::cuboid([12., 12., 12.], [18., 18., 18.]).unwrap(),
            "difference",
        )
        .unwrap();
        assert_eq!(cavity.bodies[0].inner_shells.len(), 1);
        let cylinder = crate::cylinder(2., 5.).unwrap();
        let mut model = cylinder.clone();
        append(&mut model, &cavity);
        model.refresh_change_set(&[&cylinder, &cavity]);
        model.validate().unwrap();
        let cap = cylinder
            .faces
            .iter()
            .position(|f| {
                f.surface
                    .control_points
                    .iter()
                    .flatten()
                    .all(|p| (p[2] - 5.).abs() < 1e-8)
            })
            .unwrap();
        let edited = crate::operations::push_planar_face(&model, cap, 2.).unwrap();
        let other = extract(&edited, 1).0;
        assert_eq!(
            value_codec::to_string(&other.0).unwrap(),
            value_codec::to_string(&cavity.0).unwrap()
        );
        assert_eq!(other.1.faces, cavity.1.faces);
        assert_eq!(other.1.shells, cavity.1.shells);
        let changed = extract(&edited, 0).0;
        let expected = crate::operations::push_planar_face(&cylinder, cap, 2.).unwrap();
        assert_eq!(
            value_codec::to_string(&changed.0).unwrap(),
            value_codec::to_string(&expected.0).unwrap()
        );
    }
    #[test]
    fn mixed_unit_component_push_preserves_other_body_and_identities() {
        let text = include_str!(
            "../../../tests/fixtures/step-v6/self-authored-mixed-unit-product-assembly.step"
        );
        let (model, _, _) = crate::step_interchange_v3::import_step_v9(text).unwrap();
        let original = value_codec::to_string(&model).unwrap();
        for owner in 0..2 {
            let (part, faces) = extract(&model, owner);
            let z = part
                .vertices
                .iter()
                .map(|v| v.point[2])
                .fold(f64::NEG_INFINITY, f64::max);
            let face = *faces
                .keys()
                .find(|&&f| {
                    model.faces[f]
                        .surface
                        .control_points
                        .iter()
                        .flatten()
                        .all(|p| (p[2] - z).abs() < 1e-8)
                })
                .unwrap();
            for distance in [1., -1.] {
                let edited = crate::operations::push_planar_face(&model, face, distance).unwrap();
                assert_eq!(edited.bodies.len(), 2);
                assert_eq!(edited.1.bodies, model.1.bodies);
                assert!(edited.persistent_naming_complete());
                let before_other = extract(&model, 1 - owner).0;
                let after_other = extract(&edited, 1 - owner).0;
                assert_eq!(
                    value_codec::to_string(&before_other.0).unwrap(),
                    value_codec::to_string(&after_other.0).unwrap()
                );
                assert_eq!(before_other.1.vertices, after_other.1.vertices);
                assert_eq!(before_other.1.edges, after_other.1.edges);
                assert_eq!(before_other.1.faces, after_other.1.faces);
                let result = extract(&edited, owner).0;
                let max = result
                    .vertices
                    .iter()
                    .map(|v| v.point[2])
                    .fold(f64::NEG_INFINITY, f64::max);
                assert!((max - z - distance).abs() < 1e-8);
                let expected =
                    crate::operations::push_planar_face(&part, faces[&face], distance).unwrap();
                assert_eq!(
                    value_codec::to_string(&expected.0).unwrap(),
                    value_codec::to_string(&result.0).unwrap()
                );
            }
            assert!(crate::operations::push_planar_face(&model, face, -2. * z).is_err());
        }
        assert!(crate::operations::push_planar_face(&model, usize::MAX, 1.).is_err());
        assert_eq!(value_codec::to_string(&model).unwrap(), original);
    }
}
