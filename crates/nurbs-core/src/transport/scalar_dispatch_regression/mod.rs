use super::*;
#[test]
fn profile_dispatch_preserves_continuous_certificate_and_zero_cell_refusal() {
    let profile = json!({"degree":1,"knots":[0.,0.,1.,1.],"controlPoints":[[1.,0.,0.],[2.,0.,0.]],"weights":[1.,1.],"periodic":false});
    let path = json!({"degree":1,"knots":[0.,0.,1.,1.],"controlPoints":[[0.,0.,0.],[0.,0.,4.]],"weights":[1.,1.],"periodic":false});
    let scale = json!({"degree":1,"knots":[0.,0.,1.,1.],"controlPoints":[[1.,0.,0.],[1.,0.,0.]],"weights":[1.,1.],"periodic":false});
    let mut request = json!({"op":"surface_profile_sweep","profile":profile,"path":path,"scale":scale,"normal":[1.,0.,0.],"sections":5,"max_deviation":0.01});
    let result = dispatch(request.clone()).unwrap();
    assert_eq!(result["report"]["continuousCertificate"]["withinBudget"],true);
    request["maxCells"] = json!(0);
    let result = dispatch(request).unwrap();
    assert_eq!(result["report"]["accepted"],false);
    assert_eq!(result["report"]["continuousCertificate"]["maxCells"],0);
    assert!(result["surface"].is_null());
}
