use nurbs_core::{surface::Surface,offset::fillet_surfaces::{self as f,FilletOptions},
    surface_shape_operator,curve_differential::Side};
fn support(side:f64,h:f64)->Surface {
    Surface {degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:(0..2).map(|u|(0..2).map(|v|vec![2.*u as f64,side*v as f64,-h*v as f64]).collect()).collect(),
        weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false}
}
#[test]
fn shallow_g2_sections_do_not_amplify_midpoint_curvature_by_twenty_two() {
    for h in [1./128.,1./32.,1./8.] {for radius in [0.25,1.,2.] {
        let a=support(-1.,h);let b=support(1.,h);
        let built=f::g2_fillet(&a,&b,radius,&FilletOptions{side_a:1.,side_b:-1.,max_sections:5,..Default::default()}).unwrap();
        assert!(built.failure.is_none(),"{:?}",built.failure);
        let s=built.surface.unwrap();
        let uv=[(s.knots_u[s.degree_u]+s.knots_u[s.control_points.len()])*0.5,
            (s.knots_v[s.degree_v]+s.knots_v[s.control_points[0].len()])*0.5];
        let shape=surface_shape_operator::at(&s,uv,[Side::Right;2]).unwrap();
        let t=shape.tensor.unwrap_or_else(||panic!("h={h} radius={radius} uv={uv:?} status={:?}",shape.status));
        // Extrusion has one zero principal curvature. Trace encloses the
        // signed curvature of its section, using the actual returned surface.
        let trace=[(0..3).map(|k|t[k][k][0]).sum::<f64>(),(0..3).map(|k|t[k][k][1]).sum::<f64>()];
        let lower=if trace[0]<=0. && trace[1]>=0. {0.} else {trace[0].abs().min(trace[1].abs())};
        let upper=trace[0].abs().max(trace[1].abs());
        assert!(lower*radius>1.3 && upper*radius<2.,"h={h} radius={radius} trace={trace:?}");
    }}
}
