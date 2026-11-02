#![allow(dead_code)]
use value_codec::{Value, json};
pub fn requests() -> Vec<Value> {
    vec![
        json!({"op":"curve_exact_ellipse","center":[0.,0.,0.],"axisU":[1.,0.,0.],"axisV":[0.,1.,0.],"maxWork":1000000u64}),
        json!({"op":"surface_exact_ellipsoid","center":[0.,0.,0.],"axes":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]],"maxWork":1000000u64}),
        json!({"op":"surface_exact_ring_torus","center":[0.,0.,0.],"axes":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]],"majorRadius":2.,"minorRadius":1.,"maxWork":1000000u64}),
        json!({"op":"surface_exact_frustum_side","center":[0.,0.,0.],"axes":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]],"radii":[2.,1.],"maxWork":1000000u64}),
        json!({"op":"surface_exact_planar_region","center":[0.,0.,0.],"axes":[[1.,0.,0.],[0.,1.,0.]],"radii":[0.,1.],"maxWork":1000000u64}),
    ]
}
pub fn envelope(request: &Value) -> Value {
    let unchanged = request.clone();
    let direct = nurbs_core::dispatch(request.clone());
    assert_eq!(*request, unchanged);
    let wire: Value = value_codec::from_str(&nurbs_core::execute(&request.to_string())).unwrap();
    match direct {
        Ok(v) => {
            assert_eq!(wire["ok"], true);
            let normalized: Value = value_codec::from_str(&v.to_string()).unwrap();
            assert_eq!(wire["value"], normalized);
        }
        Err(e) => {
            assert_eq!(wire["ok"], false);
            assert_eq!(wire["error"]["code"].as_str(), Some(e.code));
        }
    }
    wire
}
pub fn constructed(request: &Value) -> Value {
    let r = envelope(request);
    assert_eq!(r["ok"], true);
    let v = r["value"].clone();
    assert_eq!(v["status"], "constructed");
    assert!(v["refusal"].is_null());
    assert!(!v["geometry"].is_null());
    assert_eq!(v["topologyAuthority"], false);
    assert!(v["workUsed"].as_u64().unwrap() <= request["maxWork"].as_u64().unwrap());
    println!(
        "exact-transport-oracle={}",
        json!({"request":request,"response":v})
    );
    v
}
