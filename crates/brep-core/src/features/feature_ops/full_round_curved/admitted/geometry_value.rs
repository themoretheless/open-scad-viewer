//! Lossless NURBS representation of the ACTUAL admitted rational topology.
use super::*;
use crate::profile_certificate::ExactRatio;
use value_codec::{json, Value};
fn ratio(q: &num_rational::BigRational) -> ExactRatio {
    [q.numer().to_string(), q.denom().to_string()]
}
fn knots(lo: ExactRatio, hi: ExactRatio, degree: usize) -> Vec<ExactRatio> {
    [vec![lo; degree + 1], vec![hi; degree + 1]].concat()
}
fn unit(degree: usize) -> Vec<ExactRatio> {
    knots(["0".into(), "1".into()], ["1".into(), "1".into()], degree)
}
pub(super) fn encode(body: &AdmittedFullRound) -> Result<Value> {
    let m = body.model();
    let vertices = m
        .vertices
        .iter()
        .map(|v| json!({"point":v.point.each_ref().map(ratio)}))
        .collect::<Vec<_>>();
    let edges=m.edges.iter().map(|e| {
        let controls=e.curve.homogeneous_controls();let degree=controls.len()-1;
        json!({"vertices":e.vertices,"degenerate":e.degenerate,"curve":{"degree":degree,"knots":unit(degree),"homogeneousControls":controls}})
    }).collect::<Vec<_>>();
    let mut loops = Vec::new();
    for w in &m.loops {
        let mut coedges = Vec::new();
        for c in &w.coedges {
            if c.trim().is_some() {
                return Err(error(
                    "Exact transport encountered an unsupported coedge interval",
                ));
            }
            let controls = c.pcurve.homogeneous_controls();
            let degree = controls.len() - 1;
            coedges.push(json!({"edge":c.edge,"reversed":c.reversed,"pcurve":{"degree":degree,"knots":unit(degree),"homogeneousControls":controls}}));
        }
        loops.push(json!({"coedges":coedges}));
    }
    let faces = m
        .faces
        .iter()
        .map(|f| {
            let controls = f.surface.homogeneous_controls();
            let u = controls.len() - 1;
            let v = controls[0].len() - 1;
            // parameter_bounds are low/high CORNERS, not the two axis intervals.
            let b = f.surface.parameter_bounds();
            json!({"outer":f.outer,"holes":f.holes,"surface":{"degreeU":u,"degreeV":v,
            "knotsU":knots(b[0][0].clone(),b[1][0].clone(),u),
            "knotsV":knots(b[0][1].clone(),b[1][1].clone(),v),"homogeneousControls":controls}})
        })
        .collect::<Vec<_>>();
    let shells = m
        .shells
        .iter()
        .map(|s| {
            json!({"closed":s.closed,"faces":s.faces.iter()
        .map(|u|json!({"face":u.face,"reversed":u.reversed})).collect::<Vec<_>>()})
        })
        .collect::<Vec<_>>();
    let bodies = m
        .bodies
        .iter()
        .map(|b| json!({"outerShell":b.outer_shell,"innerShells":b.inner_shells}))
        .collect::<Vec<_>>();
    let tolerance = num_rational::BigRational::from_float(m.tolerance_mm)
        .ok_or_else(|| error("Nonfinite exact transport tolerance"))?;
    let lineage = body.lineage()?;
    let origins=lineage.faces.iter().map(|f| {
        use super::lineage::FaceOrigin;
        let origin=match &f.origin {
            FaceOrigin::RetainedSide {face,rim,source_loop,coedge,source_piece,oriented_range,stored_range,reversed,full_piece} =>
                json!({"kind":"retained-side","face":face,"rim":rim,"sourceLoop":source_loop,"coedge":coedge,
                    "sourcePiece":source_piece,"orientedRange":oriented_range,"storedRange":stored_range,"reversed":reversed,"fullPiece":full_piece}),
            FaceOrigin::Blend {selected_faces,bridge,span}=>json!({"kind":"blend","selectedFaces":selected_faces,"bridge":bridge,"span":span}),
            FaceOrigin::Cap {face}=>json!({"kind":"cap","face":face}),
        };
        json!({"output":f.output,"id":f.id,"origin":origin})
    }).collect::<Vec<_>>();
    let ids = &lineage.identities;
    Ok(
        json!({"format":"exact-rational-nurbs-brep/1","tolerance":ratio(&tolerance),
        "vertices":vertices,"edges":edges,"loops":loops,"faces":faces,"shells":shells,"bodies":bodies,
        "wallSources":body.material().result().shell_geometry().wall_sources(),
        "topologyIds":{"vertices":ids.vertices,"edges":ids.edges,"loops":ids.loops,"faces":ids.faces,
            "shells":ids.shells,"bodies":ids.bodies,"changeSet":lineage.change_set},"faceOrigins":origins}),
    )
}
