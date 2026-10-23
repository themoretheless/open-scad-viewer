use brep_core::ray_parity::classify_ray;
use nurbs_core::curve::Curve;
use value_codec::json;
fn main() {
    let curve = |p: Vec<[f64; 2]>| {
        Curve::from_polyline(p.into_iter().map(|p| p.to_vec()).collect()).unwrap()
    };
    let outer = curve(vec![[0., 0.], [10., 0.], [10., 10.], [0., 10.], [0., 0.]]);
    let hole = curve(vec![[4., 4.], [4., 6.], [6., 6.], [6., 4.], [4., 4.]]);
    let models = [
        brep_core::cuboid([0.; 3], [1.; 3]).unwrap(),
        brep_core::analytic::sphere(2.).unwrap(),
        brep_core::prism::extrude(&[vec![outer], vec![hole]], 0., 1.).unwrap(),
    ];
    let mut queries = Vec::new();
    for (index, model) in models.iter().enumerate() {
        let coordinates = match index {
            0 => [
                vec![-0.3, 0.2, 0.7, 1.4],
                vec![-0.2, 0.3, 0.8, 1.3],
                vec![-0.1, 0.4, 0.6, 1.2],
            ],
            1 => [
                vec![-2.5, -0.7, 0.4, 2.3],
                vec![-2.4, -0.6, 0.3, 2.2],
                vec![-2.3, -0.5, 0.2, 2.1],
            ],
            _ => [
                vec![-1., 1., 3., 5., 7., 9., 11.],
                vec![-1., 1., 5., 9., 11.],
                vec![-0.5, 0.25, 0.75, 1.5],
            ],
        };
        for &x in &coordinates[0] {
            for &y in &coordinates[1] {
                for &z in &coordinates[2] {
                    let point = [x, y, z];
                    let direction = [1., 0.37, 0.19];
                    let r = classify_ray(model, point, direction, 1e-7, 30000, 1000000).unwrap();
                    let crossings = r
                        .crossings
                        .iter()
                        .map(|c| json!({"face":c.face,"uv":c.uv,"parameter":c.parameter}))
                        .collect::<Vec<_>>();
                    queries.push(json!({"model":index,"point":point,"direction":direction,"parity":r.parity,"crossings":crossings,"unresolved":r.unresolved.len(),"cells":r.cells,"domainCells":r.domain_cells}));
                }
            }
        }
    }
    println!("{}", json!({"models":models,"queries":queries}));
}
