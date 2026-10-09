//! Wire adapter for native miter resource and station layout; no admission policy.
use super::{Result, Value, field};
use brep_core::sweep_miter_layout;
use nurbs_core::curve::Curve;
use value_codec::json;

pub fn plan(v: Value) -> Result<Value> {
    let loops = field::<Vec<Vec<Curve>>>(&v, "loops")?;
    let p = sweep_miter_layout::plan(
        &loops,
        field(&v, "sites")?,
        field(&v, "closed")?,
        field(&v, "initialSteps")?,
        field(&v, "maxSteps")?,
    )?;
    Ok(json!({"edges":p.edges,"spans":p.spans,"maxSteps":p.max_steps}))
}
pub fn partition(v: Value) -> Result<Value> {
    super::encode(sweep_miter_layout::partition(
        &field::<Vec<Vec<Curve>>>(&v, "sections")?,
        &field::<Vec<usize>>(&v, "rings")?,
    )?)
}
pub fn sharp(v: Value) -> Result<Value> {
    super::encode(sweep_miter_layout::sharp_stations(
        field(&v, "stations")?,
        field(&v, "edges")?,
        field(&v, "steps")?,
        field(&v, "closed")?,
    )?)
}
pub fn preview(v: Value) -> Result<Value> {
    let p = sweep_miter_layout::preview(&field::<Vec<Vec<Curve>>>(&v, "sections")?)?;
    Ok(json!({"patches":p.patches,"profilePatchRanges":p.profile_patch_ranges}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transport_preserves_complete_layout_and_refusals() {
        let sections = (0..3)
            .map(|z| {
                vec![nurbs_core::primitives::circle([0., 0., z as f64], [0., 0., 1.], 0.5).unwrap()]
            })
            .collect::<Vec<_>>();
        let p = plan(
            json!({"loops":[sections[0]],"sites":2,"closed":false,"initialSteps":1,"maxSteps":64}),
        )
        .unwrap();
        assert_eq!(p["spans"], 4);
        assert_eq!(p["maxSteps"], 64);
        assert_eq!(
            partition(json!({"sections":sections,"rings":[1]}))
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            3
        );
        assert_eq!(
            sharp(json!({"stations":3,"edges":1,"steps":2,"closed":false}))
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            0
        );
        assert_eq!(
            preview(json!({"sections":sections})).unwrap()["patches"]
                .as_array()
                .unwrap()
                .len(),
            8
        );
        assert!(partition(json!({"sections":sections,"rings":[1,1]})).is_err());
        assert!(sharp(json!({"stations":2,"edges":1,"steps":2,"closed":false})).is_err());
    }
}

/// Atomic source/cap correspondence; no global Solid certificate.
pub fn reconstruct(v: Value) -> Result<Value> {
    let report = brep_core::sweep_station_reconstruction::reconstruct(
        &field::<brep_core::Model>(&v, "model")?,
        &field::<Vec<Vec<Vec<Curve>>>>(&v, "sections")?,
        &field::<Vec<usize>>(&v, "sharp")?,
        field(&v, "closed")?,
        field(&v, "quantum")?,
        field(&v, "tolerance")?,
        field(&v, "maxWork")?,
    )?;
    Ok(json!({"model":report.model,"candidate":{
        "sides":report.candidate.sides,
        "wallDisplacementUpper":report.candidate.wall_displacement_upper,
        "work":report.candidate.work,"reason":report.candidate.reason
    }}))
}

#[cfg(test)]
mod reconstruction_tests {
    use super::*;
    #[test]
    fn reconstruction_transport_rejects_unrelated_sections_and_zero_work() {
        let sections = (0..3)
            .map(|i| {
                vec![vec![
                    nurbs_core::primitives::circle([0., 0., i as f64 * 5.], [0., 0., 1.], 1.)
                        .unwrap(),
                ]]
            })
            .collect::<Vec<_>>();
        let model = brep_core::rational_section_loft(&sections).unwrap();
        let report = reconstruct(json!({"model":model,"sections":sections,"sharp":[],
            "closed":false,"quantum":0.125,"tolerance":10.,"maxWork":0}))
        .unwrap();
        assert!(report["model"].is_null());
        assert!(report["candidate"]["sides"].is_null());
        let mut altered = sections.clone();
        altered[1][0][0].control_points[0][0] += 0.125;
        assert!(
            reconstruct(json!({"model":model,"sections":altered,"sharp":[],
            "closed":false,"quantum":0.125,"tolerance":10.,"maxWork":10000}))
            .is_err()
        );
    }
}

/// Exact placement and full boundary error transport; provenance is a premise.
pub fn affine_boundary(v: Value) -> Result<Value> {
    let source: Value = field(&v, "sourceCertificate")?;
    let closed = field(&source, "closed")?;
    let budget: Option<f64> = field(&v, "budget")?;
    let premises = brep_core::sweep_affine_boundary::Premises {
        wall: field(&source, "wallErrorUpper")?,
        caps: field(&source, "filledCapErrorUpper")?,
        closed,
        source_budget: field(&source, "budget")?,
    };
    let report = brep_core::sweep_affine_boundary::place(
        &field(&v, "model")?,
        &premises,
        field(&v, "matrix")?,
        field(&v, "quantum")?,
        field(&v, "maxWork")?,
        budget,
    )?;
    Ok(affine_report_value(report, budget, closed))
}
pub(super) fn affine_report_value(
    report: brep_core::sweep_affine_boundary::Report,
    budget: Option<f64>,
    closed: bool,
) -> Value {
    json!({"reason":report.reason,
        "placement":report.placement.map(|p|json!({"model":p.model,
            "operatorNormUpper":p.operator_norm_upper,"arithmeticErrorUpper":p.arithmetic_error_upper,
            "work":p.work,"reason":p.reason})),
        "boundaryCertificate":report.boundary.map(|b|json!({
            "method":"retained-sweep-boundary-union","scope":"boundary-set-hausdorff",
            "continuousBound":b.continuous_bound,"withinBudget":b.within_budget,
            "errorUpper":b.error_upper,"budget":budget,"closed":closed,
            "wallErrorUpper":b.wall_error_upper,"filledCapErrorUpper":b.filled_cap_error_upper,
            "reason":b.reason}))})
}

#[cfg(test)]
mod affine_boundary_tests {
    use super::*;
    #[test]
    fn affine_boundary_wire_preserves_complete_caps_and_atomic_refusal() {
        let model = brep_core::cuboid([0., 0., 0.], [1., 1., 10.]).unwrap();
        let mut request = json!({"model":model,"sourceCertificate":{
            "closed":false,"wallErrorUpper":0.1,"filledCapErrorUpper":[0.2,0.3],"budget":0.4},
            "matrix":[[2.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.],[0.,0.,0.,1.]],
            "quantum":0.125,"maxWork":100000,"budget":1.});
        let result = affine_boundary(request.clone()).unwrap();
        assert!(!result["placement"]["model"].is_null());
        assert_eq!(result["boundaryCertificate"]["continuousBound"], true);
        request["budget"] = json!(0.5);
        let result = affine_boundary(request).unwrap();
        assert!(result["placement"]["model"].is_null());
        assert_eq!(result["boundaryCertificate"]["withinBudget"], false);
    }
}
