//! Original-source quadric membership is distinct from sampled primitive fitting.
use nurbs_core::{analysis::implicit_quadric::{inspect,inspect_sphere},surface::{Surface,Axis}};
fn patch(p:usize,q:usize,points:Vec<Vec<Vec<f64>>>,weights:Vec<Vec<f64>>)->Surface {
    Surface{degree_u:p,degree_v:q,knots_u:[vec![2.;p+1],vec![7.;p+1]].concat(),knots_v:[vec![-3.;q+1],vec![5.;q+1]].concat(),control_points:points,weights,periodic_u:false,periodic_v:false}
}
fn graph(parabolic:bool)->Surface {
    if parabolic {
        patch(2,2,(0..3).map(|i|(0..3).map(|j|vec![i as f64/2.,j as f64/2.,(i==2) as u8 as f64+(j==2) as u8 as f64]).collect()).collect(),vec![vec![1.;3];3])
    }else{
        patch(1,1,(0..2).map(|i|(0..2).map(|j|vec![i as f64,j as f64,(i*j) as f64]).collect()).collect(),vec![vec![1.;2];2])
    }
}
fn sphere(axes:[f64;3])->Surface {
    let c=[1.,1.,0.];let z=[0.,1.,1.];let w=[1.,1.,2.];let xy=[[1.,0.],[1.,1.],[0.,1.]];
    patch(2,2,(0..3).map(|i|(0..3).map(|j|vec![axes[0]*c[i]*xy[j][0],axes[1]*c[i]*xy[j][1],axes[2]*z[i]]).collect()).collect(),(0..3).map(|i|(0..3).map(|j|w[i]*w[j]).collect()).collect())
}
fn cylinder(cone:bool)->Surface {
    let xy=[[1.,0.],[1.,1.],[0.,1.]];let w=[1.,1.,2.];
    patch(2,1,(0..3).map(|i|(0..2).map(|j|{let r=if cone {2.*j as f64}else{2.};vec![r*xy[i][0],r*xy[i][1],2.*j as f64]}).collect()).collect(),(0..3).map(|i|vec![w[i];2]).collect())
}
fn diagonal(q:[f64;4])->[[f64;4];4] {std::array::from_fn(|i|std::array::from_fn(|j|if i==j {q[i]}else{0.}))}
#[test]
fn graph_quadrics_have_exact_membership_and_continuous_regular_support() {
    for parabolic in [false,true] {
        let s=graph(parabolic);let saved=s.clone();let mut q=[[0.;4];4];q[2][3]=-0.5;q[3][2]=-0.5;
        if parabolic {q[0][0]=1.;q[1][1]=1.;}else{q[0][1]=0.5;q[1][0]=0.5;}
        let r=inspect(&s,q,1_000_000,4096).unwrap();
        assert!(r.exact_membership&&r.gradient_nonzero&&r.regular_implicit_support,"{r:?}");assert!(r.gradient_lower.unwrap()>0.9);assert_eq!(s,saved);
        for u in [2.,3.,5.,7.] {for v in [-3.,0.,5.] {
            let p=s.evaluate(u,v).unwrap().point;let residual=if parabolic {p[0]*p[0]+p[1]*p[1]-p[2]}else{p[0]*p[1]-p[2]};assert!(residual.abs()<1e-14);
        }}
    }
}
#[test]
fn rational_cylinder_and_ellipsoid_support_are_proved_in_original_weight_gauge() {
    for (s,q) in [(cylinder(false),diagonal([1.,1.,0.,-4.])),(sphere([2.,1.,0.5]),diagonal([0.25,1.,4.,-1.]))] {
        let r=inspect(&s,q,1_000_000,4096).unwrap();assert!(r.regular_implicit_support,"{r:?}");
        let mut scaled=s.clone();for row in &mut scaled.weights {for w in row {*w*=4.;}}
        let r=inspect(&scaled,q,1_000_000,4096).unwrap();assert!(r.regular_implicit_support,"{r:?}");
    }
}
#[test]
fn literal_radius_product_is_retained_instead_of_rounded_square() {
    let s=sphere([0.1;3]);let r=inspect_sphere(&s,[0.;3],0.1,1_000_000,4096).unwrap();assert!(r.regular_implicit_support,"{r:?}");
    // 0.1*0.1 is a different nominal binary64 constant from exact authored r².
    let q=diagonal([1.,1.,1.,-(0.1_f64*0.1)]);assert!(!inspect(&s,q,1_000_000,4096).unwrap().exact_membership);
}
#[test]
fn each_shared_ledger_boundary_and_one_ulp_source_change_are_rechecked() {
    let s=cylinder(false);let q=diagonal([1.,1.,0.,-4.]);let full=inspect(&s,q,1_000_000,4096).unwrap();assert!(full.regular_implicit_support);
    assert!(inspect(&s,q,full.exact_work,full.cells).unwrap().regular_implicit_support);
    let capped=inspect(&s,q,16_000_000,4096).unwrap();
    assert!(capped.regular_implicit_support&&capped.exact_work<=1_000_000);
    for (work,cells) in [(full.exact_work-1,4096),(1_000_000,full.cells-1),(0,0)] {
        let r=inspect(&s,q,work,cells).unwrap();assert!(!r.regular_implicit_support);assert!(r.exact_work<=work&&r.cells<=cells);
    }
    let mut moved=s.clone();moved.control_points[1][1][0]=moved.control_points[1][1][0].next_up();assert!(!inspect(&moved,q,1_000_000,4096).unwrap().exact_membership);
    let mut weighted=s.clone();weighted.weights[1][1]=weighted.weights[1][1].next_up();assert!(!inspect(&weighted,q,1_000_000,4096).unwrap().exact_membership);
    let mut changed=q;changed[3][3]=changed[3][3].next_up();assert!(!inspect(&s,changed,1_000_000,4096).unwrap().exact_membership);
}
#[test]
fn singular_quadric_and_collapsed_parameter_patch_have_distinct_authority() {
    let s=cylinder(true);let r=inspect(&s,diagonal([1.,1.,-1.,0.]),1_000_000,32).unwrap();
    assert!(r.exact_membership);assert!(!r.gradient_nonzero&&!r.regular_implicit_support);assert!(r.cells<=32);
    let collapsed=patch(1,1,vec![vec![vec![0.;3];2];2],vec![vec![1.;2];2]);
    let mut plane=[[0.;4];4];plane[2][3]=0.5;plane[3][2]=0.5;
    let r=inspect(&collapsed,plane,1_000_000,16).unwrap();assert!(r.regular_implicit_support);
    // This reports ambient plane support even for a constant parameter map;
    // it supplies no rank, coverage, injectivity, pole quotient or B-rep proof.
    assert_eq!(collapsed.evaluate(4.,1.).unwrap().point,[0.;3]);
    let r=inspect(&collapsed,[[0.;4];4],1_000_000,16).unwrap();assert!(r.exact_membership&&!r.regular_implicit_support);
}
#[test]
fn multispan_and_invalid_equations_never_inherit_single_patch_credit() {
    let s=graph(true);let mut q=diagonal([1.,1.,0.,0.]);q[2][3]=-0.5;q[3][2]=-0.5;
    let multispan=nurbs_core::editing::surface_edit::insert(&s,Axis::U,4.,1).unwrap();let r=inspect(&multispan,q,1_000_000,4096).unwrap();assert!(!r.exact_membership&&!r.regular_implicit_support);assert_eq!(r.reason,Some("implicit-single-bezier-required"));
    let mut asymmetric=q;asymmetric[0][1]=1.;assert!(inspect(&s,asymmetric,1_000_000,4096).is_err());
    let mut invalid=q;invalid[0][0]=f64::NAN;assert!(inspect(&s,invalid,1_000_000,4096).is_err());
    assert!(inspect(&s,q,16_000_001,4096).is_err());assert!(inspect(&s,q,1_000_000,100_001).is_err());
    for radius in [0.,-1.,f64::INFINITY,f64::NAN] {assert!(inspect_sphere(&s,[0.;3],radius,1_000_000,4096).is_err());}
}
