//! Transport for original-support G2 runout surfaces; no body admission.
use super::{Result, Value, field};
use nurbs_core::{
    curve::Curve,
    surface::Surface,
    surface_blend::{self, Boundary, Options, Trace},
    surface_join::Decision,
};
use value_codec::json;
#[path = "cad_runout/inputs.rs"]
mod inputs;
pub fn create(v: Value) -> Result<Value> {
    let boundary = match field::<String>(&v, "sourceBoundary")?.as_str() {
        "uMin" => Boundary::UStart,
        "uMax" => Boundary::UEnd,
        "vMin" => Boundary::VStart,
        "vMax" => Boundary::VEnd,
        _ => {
            return Err(super::input(
                "Runout source boundary must be uMin/uMax/vMin/vMax",
            ));
        }
    };
    let max_cells = field::<usize>(&v, "maxCells")?;
    let max_exact = field::<u64>(&v, "maxExactWork")?;
    let tolerances = field::<[f64; 3]>(&v, "tolerances")?;
    let prepared = inputs::prepare(&v, boundary, tolerances, max_cells)?;
    let mut r = surface_blend::g2_to_plane_trace(&prepared.source, prepared.trace, surface_blend::PlaneTarget { plane: &prepared.plane, target: &prepared.target, transverse: prepared.transverse }, Options {
            boundaries: [boundary, Boundary::VStart],
            gains: prepared.gains,
            reverse_u: field(&v, "reverseU")?,
            max_patches: field(&v, "maxPatches")?,
        }, tolerances, max_cells - prepared.cells, max_exact)?;
    r.cells += prepared.cells;
    let fields = brep_core::region_replacement::inspect_runout_panels(
        &r.construction.patches,
        tolerances[0],
        1e-8,
        brep_core::region_replacement::ContactQualificationLimits {
            cells: max_cells - r.cells,
            exact_work: 0,
            domain_cells: 1_000_000,
            shape_cells: 1_000_000,
        },
    )?;
    let complete = r.complete && fields.complete;
    let within = complete && r.within_tolerance && fields.within_tolerance;
    let field_reports=fields.panels.iter().map(|p| {
        let c=p.curvature.as_ref();
        json!({"patch":p.patch,"injectivityCertified":p.injectivity.certified,
            "curvatureDecision":match c.map(|c|c.decision) {Some(brep_core::face_curvature::Decision::Within)=>"within",Some(brep_core::face_curvature::Decision::Exceeds)=>"exceeds",Some(brep_core::face_curvature::Decision::Unresolved)=>"unresolved",None=>"unvisited"},
            "absoluteCurvatureUpper":c.and_then(|c|c.absolute_curvature_upper),
            "witnessUv":c.and_then(|c|c.witness_uv),"witnessCurvature":c.and_then(|c|c.witness_curvature),
            "unresolvedRegion":c.and_then(|c|c.unresolved_region)})
    }).collect::<Vec<_>>();
    let seam = |r: &nurbs_core::surface_g2::Report| {
        json!({"positionBounds":r.position_bounds,
        "regularityProven":r.regularity_proven,"cells":r.cells,
        "decision":match r.decision {Decision::WithinTolerance=>"within-tolerance",Decision::CorrespondenceExceedsTolerance=>"exceeds-tolerance",Decision::Unresolved=>"unresolved"},
        "orientation":r.orientation.map(|o|match o {nurbs_core::surface_g2::Orientation::Same=>"same",nurbs_core::surface_g2::Orientation::Opposite=>"opposite"}),
        "branches":r.branches.iter().map(|b|json!({"normalLower":b.normal_lower,"normalUpper":b.normal_upper,
            "curvatureLower":b.tensor_lower,"curvatureUpper":b.tensor_upper})).collect::<Vec<_>>()})
    };
    Ok(
        json!({"patches":r.construction.patches,"report":{"method":"native-original-support-G2-runout/2",
        "patchCount":r.construction.patches.len(),"complete":complete,"withinTolerance":within,"cells":r.cells+fields.cells,"exactWork":r.exact_work,
        "seamComplete":r.complete,"seamWithinTolerance":r.within_tolerance,"fieldComplete":fields.complete,
        "curvatureLimitPerMm":10.,"domainCells":fields.domain_cells,"shapeCells":fields.shape_cells,
        "maxDomainCells":1000000,"maxShapeCells":1000000,"fields":field_reports,
        "maxCells":max_cells,"maxExactWork":max_exact,"tolerances":tolerances,
        "solidGeometryCertified":false,"radiusLawCertified":false,"interiorEmbeddingCertified":false,
        "supportSeams":r.support_seams.iter().map(|(patch,source,p)|json!({"patch":patch,"source":source,
            "positionValidated":p.positions.proven,"positionUpper":p.positions.upper,"inspection":seam(&p.geometric)})).collect::<Vec<_>>(),
        "internalSeams":r.internal_seams.iter().map(seam).collect::<Vec<_>>()}}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> Value {
        let floor = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let mut cap = floor.clone();
        for row in &mut cap.control_points {
            for p in row {
                p[2] = p[1];
                p[1] = 1.;
            }
        }
        json!({"op":"brep_g2_runout_surface","source":floor,"plane":cap,"sourceBoundary":"vMax",
            "sourceTrace":{"at":0.9,"along":[0.,1.]},"targetPcurve":Curve::from_polyline(vec![vec![0.,0.1],vec![1.,0.1]]).unwrap(),
            "transverseUv":[0.,1.],"gains":[0.1,0.1],"reverseU":[false,false],"maxPatches":256,
            "maxCells":100000,"maxExactWork":1000000,"tolerances":[1e-7,1e-5,1e-4]})
    }
    #[test]
    fn admitted_runout_requires_all_saved_panel_fields_below_ten() {
        let mut v = request();
        v["sourceTrace"]["at"] = json!(0.5);
        v["targetPcurve"]["controlPoints"] = json!([[0., 0.5], [1., 0.5]]);
        v["gains"] = json!([0.5, 0.5]);
        let r = crate::dispatch(v).unwrap();
        assert_eq!(r["report"]["method"], "native-original-support-G2-runout/2");
        assert_eq!(r["report"]["withinTolerance"], true);
        assert_eq!(r["report"]["fieldComplete"], true);
        for p in r["report"]["fields"].as_array().unwrap() {
            assert_eq!(p["injectivityCertified"], true);
            assert!(p["absoluteCurvatureUpper"].as_f64().unwrap() <= 10.);
        }
    }
    #[test]
    fn world_curve_inputs_preserve_original_projection_and_physical_tangent_length() {
        let mut v = request();
        let plane = field::<Surface>(&v, "plane").unwrap();
        v["targetCurve"] = json!(plane.iso(nurbs_core::surface::Axis::V, 0.1).unwrap());
        v["sourcePosition"] = json!(0.9);
        v["tangentLengthMm"] = json!(0.1);
        let r = crate::dispatch(v.clone()).unwrap();
        assert_eq!(r["report"]["seamWithinTolerance"], true);
        assert_eq!(r["report"]["withinTolerance"], false);
        assert!(
            r["report"]["fields"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f["curvatureDecision"] == "exceeds")
        );
        assert!(r["report"]["cells"].as_u64().unwrap() <= 100000);
        v["targetCurve"]["controlPoints"][0][1] = json!(0.5);
        assert!(crate::dispatch(v).is_err());
    }
    #[test]
    fn dispatched_runout_returns_original_support_receipts_without_a_solid_claim() {
        let r = crate::dispatch(request()).unwrap();
        assert_eq!(r["report"]["seamWithinTolerance"], true);
        assert_eq!(r["report"]["withinTolerance"], false);
        assert!(
            r["report"]["fields"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f["curvatureDecision"] == "exceeds")
        );
        assert_eq!(r["report"]["solidGeometryCertified"], false);
        assert_eq!(r["report"]["radiusLawCertified"], false);
        assert_eq!(r["report"]["supportSeams"].as_array().unwrap().len(), 2);
        let mut v = request();
        v["maxCells"] = json!(0);
        v["maxExactWork"] = json!(0);
        let r = crate::dispatch(v).unwrap();
        assert_eq!(r["report"]["complete"], false);
        assert_eq!(r["report"]["withinTolerance"], false);
        let mut v = request();
        v["sourceBoundary"] = json!("unknown");
        assert!(crate::dispatch(v).is_err());
        let mut v = request();
        v["maxExactWork"] = json!(1000001);
        assert!(crate::dispatch(v).is_err());
    }
}

#[path = "cad_runout/graph_preview.rs"]
mod graph_preview;
pub(crate) fn source_graph_preview(v: Value) -> Result<Value> { graph_preview::create(v) }

#[path="cad_runout/graph_body.rs"]
mod graph_body;
pub(crate) fn restore_graph_body(v: Value) -> Result<Value> {graph_body::restore(v)}

pub(crate) fn display_graph_body(v: Value) -> Result<Value> {graph_body::display(v)}
