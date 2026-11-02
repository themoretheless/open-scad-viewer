//! Exact binary64 inputs captured from the failing real BRep constructor.
//! This is the complete carrier-volume stage, not a simplified source fixture.
use super::*;
use value_codec::Value;
fn array(v: &Value) -> &[Value] {v.as_array().unwrap()}
fn scalar(v: &Value) -> f64 {
    let s=v.as_str().unwrap();assert_eq!(s.len(),16);
    let bits=u64::from_str_radix(s,16).unwrap();
    let x=f64::from_bits(bits);assert!(x.is_finite());assert_eq!(x.to_bits(),bits);x
}
fn values(v: &Value) -> Vec<f64> {array(v).iter().map(scalar).collect()}
fn vector(v: &Value) -> [f64;3] {values(v).try_into().unwrap()}
fn surface(v: &Value) -> Surface {
    Surface {
        degree_u:v["degrees"][0].as_u64().unwrap() as usize,
        degree_v:v["degrees"][1].as_u64().unwrap() as usize,
        knots_u:values(&v["knots_u"]),knots_v:values(&v["knots_v"]),
        control_points:array(&v["control_points"]).iter().map(|r|array(r).iter().map(values).collect()).collect(),
        weights:array(&v["weights"]).iter().map(values).collect(),
        periodic_u:v["periodic"][0].as_bool().unwrap(),
        periodic_v:v["periodic"][1].as_bool().unwrap(),
    }
}
#[test]
fn actual_hollow_quarter_turn_complete_field_has_a_global_certificate() {
    let packet:Value=value_codec::from_str(include_str!("fixtures/actual-quarter-turn-bits.json")).unwrap();
    assert_eq!(packet["schema"].as_str(),Some("binary64-hex-affine-caps/1"));
    let caps=[surface(&packet["caps"][0]),surface(&packet["caps"][1])];
    for c in &caps {c.validate().unwrap();}
    let m=&packet["motion"];let origin=vector(&m["origin"]);
    let projection: [[f64;3];3]=array(&m["projection"]).iter().map(vector).collect::<Vec<_>>().try_into().unwrap();
    let pieces=array(&m["controls"]).iter().zip(array(&m["weights"])).map(|(p,w)|RationalPiece {
        controls:array(p).iter().map(|pose|array(pose).iter().map(vector).collect::<Vec<_>>().try_into().unwrap()).collect(),
        weights:values(w),
    }).collect::<Vec<_>>();
    let cells=m["max_cells"].as_u64().unwrap() as usize;
    let work=m["max_exact_work"].as_u64().unwrap();
    assert_eq!(cells,99800);assert_eq!(work,746681);
    let full=certify_rational_caps([&caps[0],&caps[1]],origin,&pieces,projection,cells,work).unwrap();
    eprintln!("actual captured carrier field: {full:?}");
    if !full.embedding_certified {
        // Conditional original WORLD jet signs only diagnose a refusal.
        // Positive finite probes supply NO global admission credit.
        let mut signs=[0usize;3];
        for u in [0.,0.25,0.5,0.75,1.] {for v in [0.,0.25,0.5,0.75,1.] {for t in [0.,0.25,0.5,0.75,1.] {
            let r=jacobian_orientation_rational_caps([&caps[0],&caps[1]],origin,&pieces,[u,v,t],10000).unwrap();
            match r.sign {Some(1)=>signs[0]+=1,Some(-1)=>{signs[1]+=1;eprintln!("negative original jet at {:?}",[u,v,t]);},_=>signs[2]+=1}
        }}}
        eprintln!("diagnostic original WORLD signs [positive,negative,unproved]={signs:?}");
    }
    assert!(full.embedding_certified && full.regularity_certified
        && full.parameter_topology_preserved,"{full:?}");
    let run=|cells,work|certify_rational_caps([&caps[0],&caps[1]],origin,&pieces,projection,cells,work).unwrap();
    assert!(run(full.cells,full.exact_work).embedding_certified);
    let short=run(full.cells-1,full.exact_work);
    assert!(!short.embedding_certified && short.cells<full.cells);
    let short=run(full.cells,full.exact_work-1);
    assert!(!short.embedding_certified && short.exact_work<full.exact_work);
}
