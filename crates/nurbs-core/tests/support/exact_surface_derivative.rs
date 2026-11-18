use nurbs_core::{
    editing::exact_surface_derivative::{self as d, Axis},
    surface::Surface,
};
pub fn domains(s: &Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}
pub fn accepted(s: Surface, axis: Axis) -> Vec<Surface> {
    let before = s.clone();
    let r = d::surface(&s, axis, 1000000).unwrap();
    assert!(r.refusal.is_none(), "{:?}", r.refusal);
    let out = r.patches.unwrap();
    assert_eq!(s, before);
    let us = (s.degree_u..s.control_points.len())
        .filter(|&i| s.knots_u[i] < s.knots_u[i + 1])
        .collect::<Vec<_>>();
    let vs = (s.degree_v..s.control_points[0].len())
        .filter(|&j| s.knots_v[j] < s.knots_v[j + 1])
        .collect::<Vec<_>>();
    assert_eq!(out.len(), us.len() * vs.len());
    for (index, p) in out.iter().enumerate() {
        p.validate().unwrap();
        assert!(!p.periodic_u && !p.periodic_v);
        let i = us[index / vs.len()];
        let j = vs[index % vs.len()];
        assert_eq!(
            domains(p),
            [
                [s.knots_u[i], s.knots_u[i + 1]],
                [s.knots_v[j], s.knots_v[j + 1]]
            ]
        );
        let [u, v] = domains(p);
        for a in 1..4 {
            for b in 1..4 {
                let u = u[0] + (u[1] - u[0]) * a as f64 / 4.;
                let v = v[0] + (v[1] - v[0]) * b as f64 / 4.;
                let source = s.evaluate(u, v).unwrap();
                let expected = source.first_derivatives().unwrap();
                let expected = match axis {
                    Axis::U => expected.0,
                    Axis::V => expected.1,
                };
                let actual = p.evaluate(u, v).unwrap().point;
                for k in 0..3 {
                    assert!((actual[k] - expected[k]).abs() < 1e-10);
                }
            }
        }
    }
    #[cfg(feature = "codec")]
    println!(
        "exact-surface-derivative-oracle={}",
        value_codec::json!({"axis":match axis{Axis::U=>"u",Axis::V=>"v"},"source":s,"patches":out})
    );
    out
}
