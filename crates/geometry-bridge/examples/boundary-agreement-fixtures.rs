use value_codec::json;
fn main() {
    let normal = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    let mut defect = normal.clone();
    let mut power = vec![1.];
    for j in 0..=8 {
        let mut next = vec![0.; power.len() + 1];
        for (i, &x) in power.iter().enumerate() {
            next[i] -= j as f64 / 8. * x;
            next[i + 1] += x;
        }
        power = next;
    }
    fn choose(n: usize, k: usize) -> f64 {
        (0..k).fold(1., |v, i| v * (n - i) as f64 / (i + 1) as f64)
    }
    let edge = &mut defect.edges[0];
    let a = edge.curve.control_points[0].clone();
    let b = edge.curve.control_points[1].clone();
    let axis = (0..3).find(|&i| a[i] == b[i]).unwrap();
    edge.curve.degree = 9;
    edge.curve.knots = [vec![0.; 10], vec![1.; 10]].concat();
    edge.curve.weights = vec![1.; 10];
    edge.curve.control_points = (0..=9)
        .map(|i| {
            let mut p: Vec<_> = (0..3)
                .map(|k| a[k] + (b[k] - a[k]) * i as f64 / 9.)
                .collect();
            p[axis] += (0..=i)
                .map(|k| power[k] * choose(i, k) / choose(9, k))
                .sum::<f64>();
            p
        })
        .collect();
    defect.validate().unwrap();
    let mut cases = Vec::new();
    for (name, model) in [("normal", normal), ("defect", defect)] {
        let result = geometry_bridge::dispatch(
            json!({"op":"cad_boundary_agreement","model":model,"maxCells":10000}),
        )
        .unwrap();
        let partial = geometry_bridge::dispatch(
            json!({"op":"cad_boundary_agreement","model":model,"maxCells":1}),
        )
        .unwrap();
        let document = json!({"version":1,"sketches":[],"bodies":[{"id":"boundary-body","name":"Boundary fixture","brep":model,"mesh":{
            "positions":[0,0,0,1,0,0,1,1,0,0,1,0,0,0,1,1,0,1,1,1,1,0,1,1],
            "indices":[0,2,1,0,3,2,4,5,6,4,6,7,0,1,5,0,5,4,1,2,6,1,6,5,2,3,7,2,7,6,3,0,4,3,4,7]}}]});
        cases.push(json!({"name":name,"model":model,"result":result,"partial":partial,"document":document}));
    }
    println!("{}", json!({"cases":cases}));
}
