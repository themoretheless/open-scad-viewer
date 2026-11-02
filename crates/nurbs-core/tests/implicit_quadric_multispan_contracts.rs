//! Exact source knots and homogeneous tensors, including non-dyadic extraction ratios.
use nurbs_core::{analysis::implicit_quadric::{inspect,inspect_sphere},surface::Surface};
fn graph(ku:Vec<f64>,kv:Vec<f64>)->Surface {
    let nu=ku.len()-3;let nv=kv.len()-3;
    Surface{degree_u:2,degree_v:2,control_points:(0..nu).map(|i|(0..nv).map(|j|vec![(ku[i+1]+ku[i+2])*0.5,(kv[j+1]+kv[j+2])*0.5,ku[i+1]*ku[i+2]+kv[j+1]*kv[j+2]]).collect()).collect(),weights:vec![vec![1.;nv];nu],knots_u:ku,knots_v:kv,periodic_u:false,periodic_v:false}
}
fn paraboloid()->[[f64;4];4] {[[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,0.,-0.5],[0.,0.,-0.5,0.]]}
fn sphere(radius:f64)->Surface {
    // Exact two-quarter-circle chains. Shared endpoint gauge is 2; the
    // second span retains weights [2,2,4], never independently normalized.
    let x=[1.,1.,0.,-1.,-1.];let y=[0.,1.,1.,1.,0.];let w=[1.,1.,2.,2.,4.];
    Surface{degree_u:2,degree_v:2,knots_u:vec![0.,0.,0.,1.,1.,2.,2.,2.],knots_v:vec![0.,0.,0.,1.,1.,2.,2.,2.],control_points:(0..5).map(|i|(0..5).map(|j|vec![radius*x[i]*x[j],radius*x[i]*y[j],radius*y[i]]).collect()).collect(),weights:(0..5).map(|i|(0..5).map(|j|w[i]*w[j]).collect()).collect(),periodic_u:false,periodic_v:false}
}
#[test]
fn original_nonuniform_two_axis_graph_has_exact_membership() {
    let s=graph(vec![0.,0.,0.,0.25,1.,1.,1.],vec![0.,0.,0.,0.75,1.,1.,1.]);let saved=s.clone();
    let r=inspect(&s,paraboloid(),1_000_000,4096).unwrap();assert!(r.exact_membership&&r.regular_implicit_support,"{r:?}");assert!(r.gradient_lower.unwrap()>0.9);assert_eq!(s,saved);
    for u in [0.,0.125,0.25,0.5,1.] {for v in [0.,0.5,0.75,0.875,1.] {let p=s.evaluate(u,v).unwrap().point;assert!((p[0]-u).abs()<1e-14);assert!((p[1]-v).abs()<1e-14);assert!((p[2]-u*u-v*v).abs()<1e-13);}}
}
#[test]
fn rational_multispan_sphere_keeps_both_tensor_weight_gauges() {
    let s=sphere(2.);let r=inspect_sphere(&s,[0.;3],2.,1_000_000,4096).unwrap();assert!(r.exact_membership&&r.regular_implicit_support,"{r:?}");
    for u in [0.,0.5,1.,1.5,2.] {for v in [0.,0.5,1.,1.5,2.] {let p=s.evaluate(u,v).unwrap().point;assert!((p.iter().map(|x|x*x).sum::<f64>()-4.).abs()<1e-13);}}
    let mut scaled=s.clone();for row in &mut scaled.weights {for w in row {*w*=8.;}}
    assert!(inspect_sphere(&scaled,[0.;3],2.,1_000_000,4096).unwrap().regular_implicit_support);
}
#[test]
fn last_span_mutations_and_a_short_shared_ledger_cannot_inherit_first_span_credit() {
    let s=sphere(1.);let full=inspect_sphere(&s,[0.;3],1.,1_000_000,4096).unwrap();assert!(full.regular_implicit_support);
    assert!(inspect_sphere(&s,[0.;3],1.,full.exact_work,full.cells).unwrap().regular_implicit_support);
    for (work,cells) in [(0,4096),(full.exact_work-1,4096),(1_000_000,full.cells-1)] {
        let r=inspect_sphere(&s,[0.;3],1.,work,cells).unwrap();assert!(!r.regular_implicit_support);assert!(r.exact_work<=work&&r.cells<=cells);
    }
    let mut changed=s.clone();changed.control_points[4][4][0]=changed.control_points[4][4][0].next_up();assert!(!inspect_sphere(&changed,[0.;3],1.,1_000_000,4096).unwrap().exact_membership);
    let mut changed=s.clone();changed.weights[4][4]=changed.weights[4][4].next_up();assert!(!inspect_sphere(&changed,[0.;3],1.,1_000_000,4096).unwrap().exact_membership);
}
#[test]
fn nonunit_knot_domains_and_transposed_tensor_are_not_normalized_into_new_sources() {
    let s=graph(vec![-2.,-2.,-2.,-1.,2.,2.,2.],vec![0.5,0.5,0.5,1.,2.,2.,2.]);assert!(inspect(&s,paraboloid(),1_000_000,4096).unwrap().regular_implicit_support);
    let mut t=s.clone();t.knots_u=s.knots_v.clone();t.knots_v=s.knots_u.clone();t.control_points=(0..4).map(|j|(0..4).map(|i|s.control_points[i][j].clone()).collect()).collect();assert!(inspect(&t,paraboloid(),1_000_000,4096).unwrap().regular_implicit_support);
}
#[test]
fn original_decimal_radius_products_survive_multispan_extraction() {
    let s=sphere(0.1);assert!(inspect_sphere(&s,[0.;3],0.1,1_000_000,4096).unwrap().regular_implicit_support);
    let q=[[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.],[0.,0.,0.,-(0.1_f64*0.1)]];
    assert!(!inspect(&s,q,1_000_000,4096).unwrap().exact_membership);
}
#[test]
fn periodic_and_unclamped_storage_still_refuse_support() {
    let periodic=Surface{degree_u:2,degree_v:1,knots_u:vec![-2.,-1.,0.,1.,2.,3.,4.,5.],knots_v:vec![0.,0.,1.,1.],control_points:vec![vec![vec![0.;3];2];5],weights:vec![vec![1.;2];5],periodic_u:true,periodic_v:false};
    periodic.validate().unwrap();
    assert!(!inspect(&periodic,paraboloid(),1_000_000,4096).unwrap().exact_membership);
    let mut s=graph(vec![0.,0.,0.,0.25,1.,1.,1.],vec![0.,0.,0.,0.75,1.,1.,1.]);
    s.periodic_u=false;s.knots_u=vec![-2.,-1.,0.,0.25,1.,2.,3.];
    let r=inspect(&s,paraboloid(),1_000_000,4096).unwrap();assert!(!r.exact_membership);assert_eq!(r.reason,Some("implicit-clamped-nonperiodic-required"));
}

#[test]
fn bilinear_multispan_saddle_retains_the_original_cross_term() {
    let k=[0.,0.,0.25,1.,1.];let l=[0.,0.,0.75,1.,1.];
    let s=Surface{degree_u:1,degree_v:1,knots_u:k.to_vec(),knots_v:l.to_vec(),control_points:(0..3).map(|i|(0..3).map(|j|vec![k[i+1],l[j+1],k[i+1]*l[j+1]]).collect()).collect(),weights:vec![vec![1.;3];3],periodic_u:false,periodic_v:false};
    let q=[[0.,0.5,0.,0.],[0.5,0.,0.,0.],[0.,0.,0.,-0.5],[0.,0.,-0.5,0.]];
    assert!(inspect(&s,q,1_000_000,4096).unwrap().regular_implicit_support);
}
#[test]
fn different_degrees_and_nonuniform_other_axis_weights_share_one_tensor_gauge() {
    let x=[1.,1.,0.,-1.,-1.];let y=[0.,1.,1.,1.,0.];let w=[1.,1.,2.,2.,4.];let v=[1.,4.,2.];
    let s=Surface{degree_u:2,degree_v:1,knots_u:vec![0.,0.,0.,1.,1.,2.,2.,2.],knots_v:vec![0.,0.,0.25,1.,1.],control_points:(0..5).map(|i|(0..3).map(|j|vec![x[i],y[i],j as f64]).collect()).collect(),weights:(0..5).map(|i|(0..3).map(|j|w[i]*v[j]).collect()).collect(),periodic_u:false,periodic_v:false};
    let q=[[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,0.,0.],[0.,0.,0.,-1.]];
    assert!(inspect(&s,q,1_000_000,4096).unwrap().regular_implicit_support);
    let t=Surface{degree_u:1,degree_v:2,knots_u:s.knots_v.clone(),knots_v:s.knots_u.clone(),control_points:(0..3).map(|j|(0..5).map(|i|s.control_points[i][j].clone()).collect()).collect(),weights:(0..3).map(|j|(0..5).map(|i|s.weights[i][j]).collect()).collect(),periodic_u:false,periodic_v:false};
    assert!(inspect(&t,q,1_000_000,4096).unwrap().regular_implicit_support);
}
#[test]
fn existing_degree_eight_ceiling_owns_both_original_spans() {
    // Independently authored Bernstein controls of x=56*u, z=3136*u².
    // The degree-eight elevation denominators cancel against integer scales.
    let u=(0usize..=8).map(|i|(7*i,56*i*(i.saturating_sub(1))))
        .chain((1..=8).map(|i|(56+7*i,3136+784*i+56*i*(i-1)))).collect::<Vec<(usize,usize)>>();
    let s=Surface{degree_u:8,degree_v:1,knots_u:[vec![0.;9],vec![1.;8],vec![2.;9]].concat(),knots_v:vec![-1.,-1.,3.,3.],control_points:u.iter().map(|&(x,z)|vec![vec![x as f64,-1.,z as f64],vec![x as f64,3.,z as f64]]).collect(),weights:vec![vec![1.;2];17],periodic_u:false,periodic_v:false};
    let q=[[1.,0.,0.,0.],[0.,0.,0.,0.],[0.,0.,0.,-0.5],[0.,0.,-0.5,0.]];
    let r=inspect(&s,q,1_000_000,4096).unwrap();assert!(r.regular_implicit_support,"{r:?}");
    for u in [0.,0.25,1.,1.75,2.] {let p=s.evaluate(u,0.).unwrap().point;assert!((p[0]-56.*u).abs()<1e-11);assert!((p[2]-3136.*u*u).abs()<1e-9);}
}
