use brep_core::Model;
pub fn source() -> Model {
    let mut a = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    let old = a.faces[1].surface.clone();
    let ranges = [
        [
            old.knots_u[old.degree_u],
            old.knots_u[old.control_points.len()],
        ],
        [
            old.knots_v[old.degree_v],
            old.knots_v[old.control_points[0].len()],
        ],
    ];
    let mut top = old.clone();
    top.degree_u = 2;
    top.degree_v = 2;
    top.knots_u = vec![0., 0., 0., 1., 1., 1.];
    top.knots_v = top.knots_u.clone();
    top.weights = vec![vec![1.; 3]; 3];
    top.control_points = (0..3)
        .map(|i| {
            (0..3)
                .map(|j| {
                    let mut p = (0..3)
                        .map(|k| {
                            old.control_points[0][0][k]
                                + (old.control_points[1][0][k] - old.control_points[0][0][k])
                                    * i as f64
                                    / 2.
                                + (old.control_points[0][1][k] - old.control_points[0][0][k])
                                    * j as f64
                                    / 2.
                        })
                        .collect::<Vec<_>>();
                    if i == 1 && j == 1 {
                        p[2] += 0.375;
                    }
                    p
                })
                .collect()
        })
        .collect();
    let wire = a.faces[1].outer;
    for c in &mut a.loops[wire].coedges {
        for p in &mut c.pcurve.control_points {
            for k in 0..2 {
                p[k] = (p[k] - ranges[k][0]) / (ranges[k][1] - ranges[k][0]);
            }
        }
    }
    a.faces[1].surface = top;
    a.rebuild_topology_ids();
    a.validate().unwrap();
    a
}
