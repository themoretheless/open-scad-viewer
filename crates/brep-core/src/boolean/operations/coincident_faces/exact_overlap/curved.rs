use super::*;
use crate::Curve;
use crate::model::planar_pcurve_regions::{PlanarOperation, contact_census, reverse_exact};
use crate::profile_certificate::certify_point_winding;
use nurbs_core::curve_partition_agreement::exact_bezier_span_partition;
use std::collections::BTreeMap;

mod contact;

fn map_wire(wire: &Loop, source: &AffineChart, target: &AffineChart) -> Result<Vec<Curve>> {
    let mut mapped = Vec::new();
    for coedge in &wire.coedges {
        if coedge
            .trim()
            .is_some_and(|trim| trim.periodic_lift != [[0; 2]; 2])
        {
            return Err(unsupported(
                "Affine curved overlap requires zero periodic lift",
            ));
        }
        for mut curve in crate::profile::pcurve_trim::active_pcurves(coedge)? {
            let [lo, hi] = curve.domain();
            if curve.periodic
                || curve.knots[..=curve.degree].iter().any(|&v| v != lo)
                || curve.knots[curve.control_points.len()..]
                    .iter()
                    .any(|&v| v != hi)
            {
                return Err(unsupported("Curved overlap needs clamped active pcurves"));
            }
            for point in &mut curve.control_points {
                let exact = |i: usize| {
                    Q::from_float(point[i]).ok_or_else(|| invalid("Nonfinite pcurve pole"))
                };
                let world = source.lift(&[exact(0)?, exact(1)?]);
                let [u, v] = target.inverse(&world)?;
                *point = vec![representable(&u)?, representable(&v)?];
            }
            mapped.push(curve);
        }
    }
    Ok(mapped)
}

#[derive(Clone, Copy)]
struct RetainedWire {
    side: usize,
    wire: usize,
}

fn copy_patch(
    models: [&Model; 2],
    faces: [usize; 2],
    source: [&SourceFace; 2],
    outer: RetainedWire,
    holes: &[RetainedWire],
) -> Result<Model> {
    let owner = outer.side;
    let reversed = source[owner].reversed;
    let mut patch = Model(
        brep_topology::Model {
            vertices: vec![],
            edges: vec![],
            faces: vec![],
            loops: vec![],
            shells: vec![Shell {
                faces: vec![FaceUse { face: 0, reversed }],
                closed: false,
            }],
            bodies: vec![],
            tolerance_mm: models[0].tolerance_mm.min(models[1].tolerance_mm),
        },
        TopologyIds::default(),
    );
    let mut f = models[owner].faces[faces[owner]].clone();
    let mut vertices = BTreeMap::new();
    let mut edges = BTreeMap::new();
    for retained in std::iter::once(&outer).chain(holes) {
        let side = retained.side;
        let model = models[side];
        let mut wire = model.loops[retained.wire].clone();
        if side != owner {
            let mapped = map_wire(&wire, &source[side].chart, &source[owner].chart)?;
            for (coedge, pcurve) in wire.coedges.iter_mut().zip(mapped) {
                if source[side].reversed == reversed {
                    coedge.pcurve = pcurve;
                } else {
                    coedge.pcurve = reverse_exact(pcurve)?;
                    coedge.reversed = !coedge.reversed;
                }
            }
            if source[side].reversed != reversed {
                wire.coedges.reverse();
            }
        }
        for coedge in &mut wire.coedges {
            coedge.edge = *edges.entry((side, coedge.edge)).or_insert_with(|| {
                let mut edge = model.edges[coedge.edge].clone();
                edge.vertices = edge.vertices.map(|v| {
                    *vertices.entry((side, v)).or_insert_with(|| {
                        let id = patch.vertices.len();
                        patch.vertices.push(model.vertices[v].clone());
                        id
                    })
                });
                let id = patch.edges.len();
                patch.edges.push(edge);
                id
            });
        }
        patch.loops.push(wire);
    }
    f.outer = 0;
    f.holes = (1..patch.loops.len()).collect();
    patch.faces.push(f);
    patch.rebuild_topology_ids();
    patch.validate()?;
    Ok(patch)
}

pub(super) fn strict_intersection(
    models: [&Model; 2],
    faces: [usize; 2],
    source: [&SourceFace; 2],
    max_tests: usize,
) -> Result<Vec<Model>> {
    let wire_ids = [0, 1].map(|side| {
        let f = &models[side].faces[faces[side]];
        std::iter::once(f.outer)
            .chain(f.holes.iter().copied())
            .collect::<Vec<_>>()
    });
    let [first, second] = [0, 1].map(|side| {
        wire_ids[side]
            .iter()
            .map(|&id| {
                let mapped = map_wire(
                    &models[side].loops[id],
                    &source[side].chart,
                    &source[0].chart,
                )?;
                if source[side].reversed != source[0].reversed {
                    mapped.into_iter().rev().map(reverse_exact).collect()
                } else {
                    Ok(mapped)
                }
            })
            .collect::<Result<Vec<_>>>()
    });
    let partition = |wires: Vec<Vec<Curve>>| -> Result<Vec<Vec<Curve>>> {
        wires
            .into_iter()
            .map(|wire| {
                let mut spans = Vec::new();
                for curve in wire {
                    if let Some(parts) = exact_bezier_span_partition(&curve)? {
                        spans.extend(parts);
                    } else {
                        // Preserve exact contact census for general B-splines.
                        spans.push(curve);
                    }
                }
                Ok(spans)
            })
            .collect()
    };
    let wires = [partition(first?)?, partition(second?)?];
    // When the source uses restricted pcurves, build the output from the
    // active mapped fragments even if no cross-boundary event is found.
    let uses_trim = wire_ids.iter().enumerate().any(|(side, wires)| {
        wires.iter().any(|&i| {
            models[side].loops[i]
                .coedges
                .iter()
                .any(|c| c.trim().is_some())
        })
    });
    let (cuts, has_contacts, remaining) = contact_census(
        [&wires[0], &wires[1]],
        max_tests,
        PlanarOperation::Intersection,
    )?;
    if has_contacts || uses_trim {
        return contact::intersection(
            [&wires[0], &wires[1]],
            cuts,
            &source[0].chart,
            &models[0].faces[faces[0]].surface,
            source[0].reversed,
            models[0].tolerance_mm.min(models[1].tolerance_mm),
            remaining,
        );
    }
    // With no boundary contacts, each source loop lies wholly inside or
    // outside the other face. Intersection keeps exactly the inside loops.
    let mut outers = Vec::new();
    let mut holes = Vec::new();
    for side in [0, 1] {
        for (index, wire) in wires[side].iter().enumerate() {
            let point = [wire[0].control_points[0][0], wire[0].control_points[0][1]];
            let winding: i64 = certify_point_winding(&wires[1 - side], point)?
                .windings
                .iter()
                .sum();
            if winding == 0 {
                continue;
            }
            let retained = RetainedWire {
                side,
                wire: wire_ids[side][index],
            };
            if index == 0 {
                outers.push(retained);
            } else {
                holes.push(retained);
            }
        }
    }
    match outers.as_slice() {
        [] if holes.is_empty() => Ok(Vec::new()),
        [outer] => Ok(vec![copy_patch(models, faces, source, *outer, &holes)?]),
        _ => Err(unsupported(
            "Curved overlap has inconsistent retained loop roles",
        )),
    }
}
