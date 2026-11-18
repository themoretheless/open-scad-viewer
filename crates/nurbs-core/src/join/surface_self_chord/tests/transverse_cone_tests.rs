use super::*;
#[test]
fn same_sign_legs_cover_short_chords_without_accepting_cancellation() {
    let columns=[[[1.,1.],[0.,0.],[0.,0.]],[[0.,0.],[1.,1.],[0.,0.]],[[2.,2.],[0.,0.],[0.,0.]]];
    let normal=[[0.,0.],[0.,0.],[1.,1.]];
    for tiny in [0.,1e-200,1.] {
        for sign in [1.,-1.] {
            let d=if sign>0. {[0.,tiny]}else{[-tiny,0.]};
            assert!(inspect_same_sign_transverse_chords(columns,[d,d],1.,normal,1e-4).unwrap());
        }
    }
    assert!(!inspect_same_sign_transverse_chords(columns,[[0.,1.];2],1.,[[1.,1.],[0.,0.],[0.,0.]],1e-4).unwrap());
    // Opposite coefficients cancel x and leave a chord parallel to z.
    let cancellation=[[[1.,1.],[0.,0.],[0.001,0.001]],columns[1],[[1.,1.],[0.,0.],[-0.001,-0.001]]];
    assert!(!inspect_same_sign_transverse_chords(cancellation,[[1.,1.],[-1.,-1.]],1.,normal,1e-4).unwrap());
    let wrongly_merged=[[[0.,0.],[1.,1.]],[[1.,1.],[0.,0.]],[[0.,0.],[-0.001,0.001]]];
    assert!(inspect_jacobian_endpoint_chords(wrongly_merged,normal,1e-4).unwrap().0,
        "The sign guard must prevent a falsely admitted normal chord");
}
#[test]
fn independent_complete_path_hulls_can_resolve_an_inconclusive_cone() {
    let normal=[[0.,0.],[0.,0.],[1.,1.]];
    let narrow=[[[1.,1.],[0.,0.],[0.,0.]],
        [[0.,0.],[1.,1.],[0.,0.]],[[1.,1.],[0.,0.],[0.,0.]]];
    let mut broad=narrow;
    broad[0][0]=[-1.,1.];
    for transverse in [[[0.,1.];2],[[-1.,0.];2]] {
        assert!(!inspect_same_sign_transverse_chords(broad,transverse,1.,normal,1e-4).unwrap());
        assert!(inspect_same_sign_transverse_chords(narrow,transverse,1.,normal,1e-4).unwrap());
    }
    // Narrow columns cannot repair missing signed ownership.
    assert!(!inspect_same_sign_transverse_chords(narrow,[[0.,1.],[-1.,0.]],1.,normal,1e-4).unwrap());
    // These are independent algebraic enclosures, not a surface fixture
    // or proof that either enclosure belongs to a specific CAD path.
}
#[test]
fn scale_rank_normal_and_invalid_inputs_remain_conservative() {
    let normal=[[0.,0.],[0.,0.],[1.,1.]];
    for scale in [1e-90,1.,1e90] {
        let columns=[[[1./scale,1./scale],[0.,0.],[0.,0.]],[[0.,0.],[1.,1.],[0.,0.]],[[1.,1.],[0.,0.],[0.,0.]]];
        assert!(inspect_same_sign_transverse_chords(columns,[[0.,1.];2],scale,normal,1e-4).unwrap());
        assert!(!inspect_same_sign_transverse_chords(columns,[[0.,1.];2],scale,[[-1.,1.];3],1e-4).unwrap());
        for bad in [0.,-1.,f64::INFINITY,f64::NAN] {
            assert!(inspect_same_sign_transverse_chords(columns,[[0.,1.];2],bad,normal,1e-4).is_err());
        }
        assert!(inspect_same_sign_transverse_chords(columns,[[1.,0.];2],scale,normal,1e-4).is_err());
        assert!(inspect_same_sign_transverse_chords(columns,[[0.,1.];2],scale,normal,1.).is_err());
    }
    assert!(!inspect_same_sign_transverse_chords([[[0.,0.];3];3],[[0.,1.];2],1.,normal,1e-4).unwrap());
}
