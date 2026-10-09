//! Original source carriers with bounded endpoint trims in an AP242 candidate.
//! Independent reader qualification is required; no Model/editing certificate.
use crate::step_interchange_v3::{Writer, emit_curve_source, emit_surface_source, refs_text};
use brep_core::{source_exchange_endpoints as endpoints, source_exchange_trims as trims};
use nurbs_core::{Error, Result};
use std::collections::BTreeMap;
pub struct Candidate {
    pub text: String,
    pub endpoint_error_upper: f64,
    pub vertices: usize,
    pub edges: usize,
    pub faces: usize,
}
fn invalid(message: &str) -> Error {
    Error::new("BREP_SOURCE_EXCHANGE_STEP", message)
}
fn trim(w: &mut Writer, curve: usize, t: [f64; 2]) -> usize {
    w.emit(format!("TRIMMED_CURVE('',#{curve},(PARAMETER_VALUE({:.17E})),(PARAMETER_VALUE({:.17E})),.{sense}.,.PARAMETER.)",t[0],t[1],sense=if t[0]<t[1] {"T"} else {"F"}))
}
/// Native authority comes only from a private Body, original recipes and fresh
/// endpoint/trim gates. Original curves and surfaces are emitted unchanged.
pub fn prepare(
    body: &brep_core::source_volume::Body,
    tolerance_mm: f64,
    limits: endpoints::Limits,
    max_trim_work: usize,
) -> Result<Option<Candidate>> {
    if !(1..=10_000_000).contains(&max_trim_work) {
        return Err(invalid("Choose bounded positive trim work"));
    }
    let shell = body.geometry().shell();
    let regions = shell
        .regions()
        .ok_or_else(|| invalid("Original source regions required"))?;
    let report = endpoints::prepare(body, tolerance_mm, limits)?;
    let Some(points) = report.prepared else {
        return Ok(None);
    };
    let Some(pole_work) = shell.poles().len().checked_mul(2) else {
        return Err(invalid("Pole trim count overflow"));
    };
    let Some(trim_work) = max_trim_work.checked_sub(pole_work).filter(|w| *w > 0) else {
        return Ok(None);
    };
    let Some(trims) = trims::prepare(body, &points, tolerance_mm, trim_work)? else {
        return Ok(None);
    };
    let mut w = Writer::new();
    let mut vertices = BTreeMap::new();
    for v in points.vertices() {
        let p = w.point(&v.point);
        vertices.insert(v.source_id, w.emit(format!("VERTEX_POINT('',#{p})")));
    }
    let surfaces = shell
        .faces()
        .iter()
        .map(|f| emit_surface_source(&mut w, f[0].edges()[0].surface()))
        .collect::<Vec<_>>();
    let context2 =
        w.emit("(GEOMETRIC_REPRESENTATION_CONTEXT(2)PARAMETRIC_REPRESENTATION_CONTEXT()REPRESENTATION_CONTEXT('','2D'))".into());
    let mut usages = BTreeMap::new();
    let mut edge_entities = Vec::new();
    for (index, restriction) in points.restrictions().iter().enumerate() {
        let edge = restriction.edge();
        let numeric = &trims.edges()[index];
        let carrier = emit_curve_source(&mut w, edge.world());
        let curve = trim(&mut w, carrier, numeric.canonical());
        let mut pcurves = Vec::new();
        for slot in 0..2 {
            let address = shell.uses()[index][slot];
            usages.insert(
                (address.face, address.wire, address.edge),
                (index, edge.reversed()[slot]),
            );
            let source = emit_curve_source(&mut w, edge.uses()[slot].curve());
            let mut t = numeric.uses()[slot];
            if edge.reversed()[slot] {
                t.reverse();
            }
            let pc = trim(&mut w, source, t);
            let rep = w.emit(format!(
                "DEFINITIONAL_REPRESENTATION('',(#{pc}),#{context2})"
            ));
            pcurves.push(w.emit(format!("PCURVE('',#{},#{rep})", surfaces[address.face])));
        }
        let sc = w.emit(format!(
            "SURFACE_CURVE('',#{curve},({}),.CURVE_3D.)",
            refs_text(&pcurves)
        ));
        let address = shell.uses()[index][0];
        let mut ids = shell.vertices()[address.face][address.wire][address.edge];
        if edge.reversed()[0] {
            ids.reverse();
        }
        edge_entities.push(w.emit(format!(
            "EDGE_CURVE('',#{},#{},#{sc},.T.)",
            vertices[&ids[0]], vertices[&ids[1]]
        )));
    }
    // A PCURVE preserves the nonconstant original UV image while its model
    // image is the privately proved pole. No surrogate 3D curve is authored.
    for (address, pole) in shell.poles() {
        let source = pole.source();
        let ids = shell.vertices()[address.face][address.wire][address.edge];
        if ids[0] != ids[1] {
            return Err(invalid("Pole vertex ownership differs"));
        }
        let parameters = source.endpoints().each_ref().map(|e| match e {
            brep_core::source_boundary_fragment::Endpoint::Parameter(t) => Some(*t),
            _ => None,
        });
        let [Some(a), Some(b)] = parameters else {
            return Err(invalid(
                "Root-valued pole trims need independent parameter representatives",
            ));
        };
        let curve = emit_curve_source(&mut w, source.curve());
        let trimmed = trim(&mut w, curve, [a, b]);
        let rep = w.emit(format!(
            "DEFINITIONAL_REPRESENTATION('',(#{trimmed}),#{context2})"
        ));
        let pc = w.emit(format!("PCURVE('',#{},#{rep})", surfaces[address.face]));
        let vertex = *vertices
            .get(&ids[0])
            .ok_or_else(|| invalid("Missing original pole vertex"))?;
        let index = edge_entities.len();
        edge_entities.push(w.emit(format!("EDGE_CURVE('',#{vertex},#{vertex},#{pc},.T.)")));
        usages.insert((address.face, address.wire, address.edge), (index, false));
    }
    let mut faces = Vec::new();
    for (face, wires) in shell.faces().iter().enumerate() {
        let mut bounds = Vec::new();
        for (wire, source) in wires.iter().enumerate() {
            let mut oriented = Vec::new();
            for edge in 0..source.edges().len() {
                let (index, reversed) = usages[&(face, wire, edge)];
                oriented.push(w.emit(format!(
                    "ORIENTED_EDGE('',*,*,#{},.{sense}.)",
                    edge_entities[index],
                    sense = if reversed { "F" } else { "T" }
                )));
            }
            let lp = w.emit(format!("EDGE_LOOP('',({}))", refs_text(&oriented)));
            bounds.push(w.emit(format!(
                "{}('',#{lp},.T.)",
                if wire == 0 {
                    "FACE_OUTER_BOUND"
                } else {
                    "FACE_BOUND"
                }
            )));
        }
        let reverse = body.reverse_orientation() ^ (regions[face].chart_winding() < 0);
        faces.push(w.emit(format!(
            "ADVANCED_FACE('',({}),#{},.{sense}.)",
            refs_text(&bounds),
            surfaces[face],
            sense = if reverse { "F" } else { "T" }
        )));
    }
    let closed = w.emit(format!("CLOSED_SHELL('',({}))", refs_text(&faces)));
    let solid = w.emit(format!("MANIFOLD_SOLID_BREP('',#{closed})"));
    let unit = w.emit("(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.))".into());
    let uncertainty=w.emit(format!("UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE({tolerance_mm:.17E}),#{unit},'distance_accuracy_value','')"));
    let context=w.emit(format!("(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#{uncertainty}))GLOBAL_UNIT_ASSIGNED_CONTEXT((#{unit}))REPRESENTATION_CONTEXT('','3D'))"));
    let rep = w.emit(format!(
        "ADVANCED_BREP_SHAPE_REPRESENTATION('',(#{solid}),#{context})"
    ));
    let app = w.emit("APPLICATION_CONTEXT('managed model based 3d engineering')".into());
    let pc = w.emit(format!("PRODUCT_CONTEXT('',#{app},'mechanical')"));
    let product = w.emit(format!("PRODUCT('source','source','',(#{pc}))"));
    let form = w.emit(format!("PRODUCT_DEFINITION_FORMATION('1','',#{product})"));
    let dc = w.emit(format!(
        "PRODUCT_DEFINITION_CONTEXT('part definition',#{app},'design')"
    ));
    let def = w.emit(format!("PRODUCT_DEFINITION('design','',#{form},#{dc})"));
    let shape = w.emit(format!("PRODUCT_DEFINITION_SHAPE('','',#{def})"));
    w.emit(format!("SHAPE_DEFINITION_REPRESENTATION(#{shape},#{rep})"));
    let mut text = String::from(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('Original source carrier exchange candidate'),'2;1');\nFILE_NAME('source.step','',(''),(''),'OpenSCAD Viewer','OpenSCAD Viewer','');\nFILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF'));\nENDSEC;\nDATA;\n",
    );
    for (id, row) in w.rows {
        text.push_str(&format!("#{id}={row};\n"));
    }
    text.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    if text.len() > 32 * 1024 * 1024 {
        return Err(invalid("Source STEP output limit"));
    }
    let error = trims
        .edges()
        .iter()
        .flat_map(|e| e.error_upper().into_iter().flatten())
        .chain(points.vertices().iter().map(|v| v.error_upper))
        .fold(0., f64::max);
    Ok(Some(Candidate {
        text,
        endpoint_error_upper: error,
        vertices: vertices.len(),
        edges: edge_entities.len(),
        faces: faces.len(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_serialization_preserves_near_unit_weights_and_distinct_knots() {
        let curve = nurbs_core::curve::Curve {
            degree: 1,
            periodic: false,
            knots: vec![0., 0., 2f64.powi(-54), 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![1., 0., 0.], vec![1., 1., 0.]],
            weights: vec![1., 1. + f64::EPSILON, 1.],
        };
        curve.validate().unwrap();
        let mut w = Writer::new();
        let id = emit_curve_source(&mut w, &curve);
        let text = &w.rows[&id];
        assert!(text.contains("RATIONAL_B_SPLINE_CURVE"));
        assert!(text.contains("B_SPLINE_CURVE_WITH_KNOTS((2,1,2)"));
        assert!(text.contains(&format!("{:.17E}", 1. + f64::EPSILON)));
        assert!(text.contains(&format!("{:.17E}", 2f64.powi(-54))));
    }
}
