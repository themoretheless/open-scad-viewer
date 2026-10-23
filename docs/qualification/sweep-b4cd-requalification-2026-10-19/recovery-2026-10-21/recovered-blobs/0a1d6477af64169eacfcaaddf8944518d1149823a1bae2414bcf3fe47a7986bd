use geometry_bridge::dispatch;
use nurbs_core::curve::Curve;
use value_codec::json;
fn main() {
    let outer = Curve::from_polyline(vec![
        vec![0., 0.],
        vec![10., 0.],
        vec![10., 10.],
        vec![0., 10.],
        vec![0., 0.],
    ])
    .unwrap();
    let hole = Curve::from_polyline(vec![
        vec![4., 4.],
        vec![4., 6.],
        vec![6., 6.],
        vec![6., 4.],
        vec![4., 4.],
    ])
    .unwrap();
    let a = brep_core::prism::extrude(&[vec![outer], vec![hole]], -1., 0.).unwrap();
    let b = brep_core::cuboid([4.75, 4.75, 2.], [5.25, 5.25, 3.]).unwrap();
    let fa = a.faces.len() - 1;
    let fb = b
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == 2.)
        })
        .unwrap();
    let result=dispatch(json!({"op":"cad_face_distance","a":a,"b":b,"faceA":fa,"faceB":fb,"toleranceMm":0.001,"toleranceUv":1e-7,"maxCells":100000,"maxDomainCells":1000000})).unwrap();
    let ma = dispatch(json!({"op":"brep_nurbs_tessellate","model":a,"segments":4})).unwrap();
    let mb = dispatch(json!({"op":"brep_nurbs_tessellate","model":b,"segments":4})).unwrap();
    println!(
        "{}",
        json!({"document":{"version":1,"sketches":[],"bodies":[{"id":"plate","name":"Plate with hole","brep":a,"mesh":ma},{"id":"probe","name":"Probe","brep":b,"mesh":mb}]},"faceA":fa,"faceB":fb,"expectedMm":(4_f64+0.75*0.75).sqrt(),"result":result})
    );
}
