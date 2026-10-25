//! Transfer privately admitted full-source bodies without geometric welding.
use crate::source_boundary_fragment::{Endpoint, Fragment};
use std::collections::BTreeMap;
fn unsupported(message: &str) -> crate::Error {
    crate::Error::new("BREP_SOURCE_MODEL_UNSUPPORTED", message)
}
fn full(fragment: &Fragment) -> crate::Result<()> {
    let d = fragment.curve().domain();
    let expected = if fragment.reversed() { [d[1], d[0]] } else { d };
    if !(0..2).all(|i| matches!(fragment.endpoints()[i],Endpoint::Parameter(t) if t==expected[i])) {
        return Err(unsupported(
            "Root-valued and partial source trims need exact Model restriction storage",
        ));
    }
    Ok(())
}
fn directed(fragment: &Fragment) -> crate::Result<nurbs_core::curve::Curve> {
    full(fragment)?;
    let mut c = fragment.curve().clone();
    if fragment.reversed() {
        let d = c.domain();
        if c.control_points.len() != c.degree + 1
            || c.knots[..=c.degree].iter().any(|&k| k != d[0])
            || c.knots[c.control_points.len()..].iter().any(|&k| k != d[1])
        {
            return Err(unsupported(
                "Reverse multispan pcurves need exact source parameter transport",
            ));
        }
        // A clamped Bezier reflection only reverses coefficients: no knot arithmetic.
        c.control_points.reverse();
        c.weights.reverse();
    }
    Ok(c)
}
/// Transfer operations are implemented only by privately admitted native edges.
pub trait Transfer:crate::source_seam_tangency::Carrier {
    fn complete(&self)->bool;
    fn traversal_reversed(&self,side:usize)->bool;
    fn directed_pcurve(&self,side:usize)->crate::Result<nurbs_core::curve::Curve>;
}
impl Transfer for crate::source_shared_edge::SharedEdge {
    fn complete(&self)->bool {self.covers_complete_canonical_source()}
    fn traversal_reversed(&self,side:usize)->bool {self.reversed()[side]}
    fn directed_pcurve(&self,side:usize)->crate::Result<nurbs_core::curve::Curve> {directed(&self.uses()[side])}
}
impl Transfer for crate::source_projective_edge::Edge {
    fn complete(&self)->bool {
        crate::source_allowed_contact::Support::maps_complete_source(self)
    }
    fn traversal_reversed(&self,side:usize)->bool {self.uses()[side].reversed()}
    fn directed_pcurve(&self,side:usize)->crate::Result<nurbs_core::curve::Curve> {
        let source=&self.uses()[side];let fragment=source.fragment();
        let mut candidate=directed(fragment)?;
        let mut map=source.weights();if fragment.reversed() {map.reverse();}
        // Invert the positive projective parameter map in homogeneous Bezier
        // coefficients. Rounded proposals require fresh whole-curve identity.
        if map[0]!=map[1] {
            if candidate.control_points.len()!=candidate.degree+1 {return Err(unsupported("Projective multispan transport requires exact partitioned composition"));}
            for (i,w) in candidate.weights.iter_mut().enumerate() {
                *w*=map[1].powi((candidate.degree-i) as i32)*map[0].powi(i as i32);
                if !w.is_finite() || *w<=0. {return Err(unsupported("Projective UV transport exceeds finite positive coefficient range"));}
            }
        }
        let range=if source.reversed() {[[1.,1.],[0.,1.]]}else{[[0.,1.],[1.,1.]]};
        let proof=nurbs_core::curve_surface_affine::verify_algebraic(self.world(),&candidate,fragment.surface(),range,cad_predicates::MAX_WORK)?;
        if !proof.is_some_and(|p|p.outcome==cad_predicates::BezierIdentity::Equal) {return Err(unsupported("Exact projective UV transport is unproved"));}
        Ok(candidate)
    }
}
fn canonical_vertex_ids<E:Transfer>(shared:&E,side:usize,mut ids:[usize;2])->[usize;2] {
    if shared.traversal_reversed(side) {ids.reverse();}ids
}
/// Only a private native Body can authorize conversion. Entity addresses and
/// shell vertex identities are preserved; coordinates never merge vertices.
pub fn convert<E:Transfer>(
    body: &crate::source_volume::Body<E>,
    tolerance_mm: f64,
) -> crate::Result<crate::Model> {
    if !tolerance_mm.is_finite() || tolerance_mm <= 0. {
        return Err(unsupported("Choose a positive finite Model tolerance"));
    }
    let shell = body.geometry().shell();
    let mut points = BTreeMap::new();
    let mut uses = BTreeMap::new();
    let mut edges = Vec::new();
    let insert = |points: &mut BTreeMap<usize, [f64; 3]>, id, point| -> crate::Result<()> {
        if points.get(&id).is_some_and(|p| *p != point) {
            return Err(unsupported(
                "One source vertex has conflicting exact world endpoints",
            ));
        }
        points.insert(id, point);
        Ok(())
    };
    for (index, shared) in shell.edges().iter().enumerate() {
        if !shared.complete() {
            return Err(unsupported(
                "Partial canonical carriers need exact root restriction storage",
            ));
        }
        let c = shared.world();
        let d = c.domain();
        if c.knots[..=c.degree].iter().any(|&k| k != d[0])
            || c.knots[c.control_points.len()..].iter().any(|&k| k != d[1])
        {
            return Err(unsupported(
                "Canonical endpoints must be original clamped controls",
            ));
        }
        let mut canonical = None;
        for (slot, address) in shell.uses()[index].iter().enumerate() {
            let fragment = &shell.faces()[address.face][address.wire].edges()[address.edge];
            full(fragment)?;
            let reversed = shared.traversal_reversed(slot);
            let ids=canonical_vertex_ids(shared,slot,shell.vertices()[address.face][address.wire][address.edge]);
            if canonical.is_some_and(|v| v != ids) {
                return Err(unsupported("Shared canonical endpoint ownership conflicts"));
            }
            canonical = Some(ids);
            uses.insert(
                (address.face, address.wire, address.edge),
                (index, reversed, shared.directed_pcurve(slot)?),
            );
        }
        let ids = canonical.unwrap();
        for end in 0..2 {
            let p = &c.control_points[if end == 0 {
                0
            } else {
                c.control_points.len() - 1
            }];
            insert(&mut points, ids[end], [p[0], p[1], p[2]])?;
        }
        edges.push(crate::Edge {
            vertices: ids,
            curve: c.clone(),
            degenerate: false,
        });
    }
    for (address, pole) in shell.poles() {
        let ids = shell.vertices()[address.face][address.wire][address.edge];
        if ids[0] != ids[1] {
            return Err(unsupported("Collapsed source use must own one vertex"));
        }
        insert(&mut points, ids[0], pole.point())?;
        let curve = nurbs_core::curve::Curve::from_polyline(vec![pole.point().to_vec(); 2])
            .map_err(|e| crate::Error::new(e.code, e.message))?;
        uses.insert(
            (address.face, address.wire, address.edge),
            (edges.len(), false, directed(pole.source())?),
        );
        edges.push(crate::Edge {
            vertices: ids,
            curve,
            degenerate: true,
        });
    }
    let ids = points
        .keys()
        .enumerate()
        .map(|(i, &id)| (id, i))
        .collect::<BTreeMap<_, _>>();
    for edge in &mut edges {
        edge.vertices = edge.vertices.map(|id| ids[&id]);
    }
    let mut loops = Vec::new();
    let mut faces = Vec::new();
    for (face, wires) in shell.faces().iter().enumerate() {
        let first = loops.len();
        for (wire, source) in wires.iter().enumerate() {
            let mut coedges = Vec::new();
            for (edge, _fragment) in source.edges().iter().enumerate() {
                let (index, reversed, pcurve) = &uses[&(face, wire, edge)];
                coedges.push(crate::Coedge::new(
                    *index,
                    *reversed,
                    pcurve.clone(),
                ));
            }
            loops.push(crate::Loop { coedges });
        }
        faces.push(crate::Face {
            surface: wires[0].edges()[0].surface().clone(),
            outer: first,
            holes: (first + 1..loops.len()).collect(),
        });
    }
    let face_uses = (0..faces.len())
        .map(|face| crate::FaceUse {
            face,
            reversed: body.reverse_orientation()
                ^ (shell.regions().unwrap()[face].chart_winding() < 0),
        })
        .collect();
    let mut model = crate::Model(
        brep_topology::Model {
            vertices: points
                .values()
                .map(|&point| crate::Vertex { point })
                .collect(),
            edges,
            loops,
            faces,
            shells: vec![crate::Shell {
                faces: face_uses,
                closed: true,
            }],
            bodies: vec![crate::Body {
                outer_shell: 0,
                inner_shells: vec![],
            }],
            tolerance_mm,
        },
        Default::default(),
    );
    model.rebuild_topology_ids();
    model.validate()?;
    Ok(model)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn directed_fragment_reversal_is_counted_once_in_canonical_vertex_ownership() {
        let surface = nurbs_core::polynomial::graph([0., 1., 0., 1.], &[vec![0.]]).unwrap();
        let uv = nurbs_core::curve::Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
        let world =
            nurbs_core::curve::Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]])
                .unwrap();
        let forward = Fragment::new(
            &surface,
            &uv,
            Endpoint::Parameter(0.),
            Endpoint::Parameter(1.),
        )
        .unwrap();
        let backward = Fragment::new(
            &surface,
            &uv,
            Endpoint::Parameter(1.),
            Endpoint::Parameter(0.),
        )
        .unwrap();
        let edge = crate::source_shared_edge::qualify(
            &world,
            [&forward, &backward],
            [false, false],
            1000000,
        )
        .unwrap()
        .edge
        .unwrap();
        assert_eq!(
            canonical_vertex_ids(&edge, 0, [4, 5]),
            canonical_vertex_ids(&edge, 1, [5, 4])
        );
        let restored = crate::source_shared_edge_restore::restore(
            edge.definition(),
            crate::source_shared_edge_restore::Limits {
                mapping_cells_per_use: 100,
                exact_work: 1000000,
                driver_cells: 100,
            },
        )
        .unwrap()
        .edge
        .unwrap();
        assert_eq!(canonical_vertex_ids(&restored, 1, [5, 4]), [4, 5]);
    }
    #[test]
    fn partial_trims_refuse_and_bezier_reversal_preserves_original_coefficients() {
        let surface = nurbs_core::polynomial::graph([0., 1., 0., 1.], &[vec![0.]]).unwrap();
        let curve =
            nurbs_core::curve::Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
        let partial = Fragment::new(
            &surface,
            &curve,
            Endpoint::Parameter(0.25),
            Endpoint::Parameter(0.75),
        )
        .unwrap();
        assert_eq!(
            directed(&partial).unwrap_err().code,
            "BREP_SOURCE_MODEL_UNSUPPORTED"
        );
        let complete = Fragment::new(
            &surface,
            &curve,
            Endpoint::Parameter(1.),
            Endpoint::Parameter(0.),
        )
        .unwrap();
        let reverse = directed(&complete).unwrap();
        assert_eq!(reverse.knots, curve.knots);
        assert_eq!(
            reverse.control_points,
            curve.control_points.into_iter().rev().collect::<Vec<_>>()
        );
    }
}

/// Assert that a STEP export/import round trip preserves the model identity:
/// same entity counts, topology indices and geometry within 1e-6.
#[cfg(test)]
pub fn assert_step_identity(source: &crate::Model, imported: &crate::Model) {
    fn close(a: f64, b: f64, what: &str) {
        assert!((a - b).abs() <= 1e-6, "{what}: {a} vs {b}");
    }
    assert_eq!(source.vertices.len(), imported.vertices.len(), "vertex count");
    assert_eq!(source.edges.len(), imported.edges.len(), "edge count");
    assert_eq!(source.loops.len(), imported.loops.len(), "loop count");
    assert_eq!(source.faces.len(), imported.faces.len(), "face count");
    assert_eq!(source.shells.len(), imported.shells.len(), "shell count");
    assert_eq!(source.bodies.len(), imported.bodies.len(), "body count");
    for (i, (a, b)) in source.vertices.iter().zip(&imported.vertices).enumerate() {
        for d in 0..3 {
            close(a.point[d], b.point[d], &format!("vertex {i} coordinate {d}"));
        }
    }
    for (i, (a, b)) in source.edges.iter().zip(&imported.edges).enumerate() {
        assert_eq!(a.degenerate, b.degenerate, "edge {i} degenerate");
        assert_eq!(a.vertices, b.vertices, "edge {i} vertices");
    }
    for (i, (a, b)) in source.faces.iter().zip(&imported.faces).enumerate() {
        let (sa, sb) = (&a.surface, &b.surface);
        assert_eq!(sa.degree_u, sb.degree_u, "face {i} degree_u");
        assert_eq!(sa.degree_v, sb.degree_v, "face {i} degree_v");
        assert_eq!(sa.control_points.len(), sb.control_points.len(), "face {i} rows");
        for (j, (ra, rb)) in sa.control_points.iter().zip(&sb.control_points).enumerate() {
            assert_eq!(ra.len(), rb.len(), "face {i} row {j} columns");
            for (k, (pa, pb)) in ra.iter().zip(rb).enumerate() {
                for d in 0..3 {
                    close(pa[d], pb[d], &format!("face {i} control point {j}/{k} coordinate {d}"));
                }
            }
        }
        for (j, (ra, rb)) in sa.weights.iter().zip(&sb.weights).enumerate() {
            for (k, (wa, wb)) in ra.iter().zip(rb).enumerate() {
                close(*wa, *wb, &format!("face {i} weight {j}/{k}"));
            }
        }
        for (ka, kb) in sa.knots_u.iter().zip(&sb.knots_u) {
            close(*ka, *kb, &format!("face {i} knot_u"));
        }
        for (ka, kb) in sa.knots_v.iter().zip(&sb.knots_v) {
            close(*ka, *kb, &format!("face {i} knot_v"));
        }
    }
}
