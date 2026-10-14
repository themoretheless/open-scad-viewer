//! Run with: cargo run --manifest-path crates/Cargo.toml -p nurbs-core --example retained_cap_contour
//! Contour identity is a premise; the filled cap region needs its own audit.
#[cfg(feature="transport")]
use nurbs_core::dispatch;
#[cfg(feature="transport")]
use value_codec::json;

#[cfg(feature="transport")]
fn main() {
    let curve=json!({"degree":2,"knots":[0.,0.,0.,1.,1.,1.],
        "controlPoints":[[0.,0.,0.],[1.,1.,0.],[2.,0.,0.]],
        "weights":[1.,0.5,1.],"periodic":false});
    let request=json!({"op":"sweep_retained_cap_contour_audit",
        "expected":[curve.clone()],"actual":[curve.clone()],"reversed":[false],"maxWork":3});
    let exact=dispatch(request.clone()).expect("native contour audit");
    assert_eq!(exact["contourIdentity"].as_bool(),Some(true));
    assert_eq!(exact["filledRegionCertified"].as_bool(),Some(false));
    let mut exhausted=request.clone();exhausted["maxWork"]=json!(2);
    assert_eq!(dispatch(exhausted).unwrap()["contourIdentity"].as_bool(),Some(false));
    let mut damaged=request;damaged["actual"][0]["weights"][1]=json!(0.5000000000000001);
    assert_eq!(dispatch(damaged).unwrap()["contourIdentity"].as_bool(),Some(false));
    println!("Exact contour accepted; damaged weights and exhausted budget refused.");
}

#[cfg(not(feature="transport"))]
fn main() { eprintln!("Enable the transport feature to run this example."); }
