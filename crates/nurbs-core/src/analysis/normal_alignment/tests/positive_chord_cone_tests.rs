use super::positive_chord_cone_excluded as excluded;
#[test]
fn covers_positive_combinations_and_refuses_cancellation_or_alignment() {
    let n=[[0.,0.],[0.,0.],[1.,1.]];
    let g=[[[1.,2.],[-2.,2.],[-0.01,0.01]],[[2.,3.],[-1.,1.],[-0.02,0.02]]];
    assert!(excluded(&g,n,[1.,0.,0.],1e-6).unwrap());
    assert!(!excluded(&g,n,[-1.,0.,0.],1e-6).unwrap());
    assert!(!excluded(&[[[0.,0.],[0.,0.],[1.,2.]]],n,[0.,0.,1.],1e-6).unwrap());
    assert!(!excluded(&[[[-1.,1.],[0.,0.],[0.,0.]]],n,[1.,0.,0.],1e-6).unwrap());
    assert!(!excluded(&g,[[0.,0.];3],[1.,0.,0.],1e-6).unwrap());
}
#[test]
fn shared_tangent_is_free_but_transverse_cancellation_refuses() {
    let point=|p:[f64;3]|p.map(|x|[x,x]);
    let tangent=point([1.,0.,0.]);let up=point([0.,1.,0.]);let down=point([0.,-1.,0.]);let n=point([0.,0.,1.]);
    assert!(super::shared_chord_cone_excluded(tangent,[up,up],n,1e-6).unwrap());
    assert!(!super::shared_chord_cone_excluded(tangent,[up,down],n,1e-6).unwrap());
    assert!(!super::shared_chord_cone_excluded(tangent,[up,up],up,1e-6).unwrap());
    assert!(!super::shared_chord_cone_excluded(tangent,[up,up],tangent,1e-6).unwrap());
    assert!(!super::shared_chord_cone_excluded([[0.,0.];3],[up,up],n,1e-6).unwrap());
    assert!(super::shared_chord_cone_excluded(tangent,[up,up],[[f64::NAN,1.],[0.,0.],[1.,1.]],1e-6).is_err());
}
#[test]
fn shared_cone_bounds_include_nonorthogonal_and_uncertain_paths() {
    let c=[[1.,1.],[-0.01,0.01],[0.,0.]];
    let a=[[0.,0.],[1.,1.],[-0.001,0.001]];let n=[[0.,0.],[0.,0.],[1.,1.]];
    assert!(super::shared_chord_cone_excluded(c,[a,a],n,1e-6).unwrap());
    let wide=[[0.,0.],[-1.,1.],[0.,0.]];
    assert!(!super::shared_chord_cone_excluded(c,[wide,a],n,1e-6).unwrap());
    assert!(!super::shared_chord_cone_excluded(c,[a,a],[[0.,0.];3],1e-6).unwrap());
}
#[test]
fn proposed_bisector_covers_narrow_cone_missed_by_integer_directions() {
    let g=[[[1.,1.],[0.,0.],[0.,0.]],[[-0.995,-0.995],[0.1,0.1],[0.,0.]]];
    let normal=[[0.,0.],[0.,0.],[1.,1.]];
    for x in -1..=1 {for y in -1..=1 {for z in -1..=1 {
        assert!(!excluded(&g,normal,[x as f64,y as f64,z as f64],1e-6).unwrap());
    }}}
    let proposal=super::chord_cone_projection_proposal(&g).unwrap();
    assert!(excluded(&g,normal,proposal,1e-6).unwrap());
    let mut uncertain=g;uncertain[0][0]=[-1.,1.];
    assert!(!excluded(&uncertain,normal,proposal,1e-6).unwrap());
    assert!(super::chord_cone_projection_proposal(&[[[0.,0.];3]]).is_none());
    assert!(super::chord_cone_projection_proposal(&[[[f64::NAN,1.],[0.,0.],[0.,0.]]]).is_none());
}
#[test]
fn validates_all_generators_before_any_early_refusal() {
    let g=[[[0.,0.];3],[[f64::NAN,1.],[0.,0.],[0.,0.]]];
    assert!(excluded(&g,[[0.,0.],[0.,0.],[1.,1.]],[1.,0.,0.],1e-6).is_err());
    assert!(excluded(&[],[[0.,0.];3],[1.,0.,0.],1e-6).is_err());
}
