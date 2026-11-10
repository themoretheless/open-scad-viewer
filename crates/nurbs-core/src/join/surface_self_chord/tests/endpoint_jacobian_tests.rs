use super::*;
#[test]
fn rank_precondition_handles_independent_scales_and_uncertain_rank() {
    for scales in [[1e-90,1e90],[1e90,1e-90],[1.,1.]] {
        let j=[[[scales[0],scales[0]],[0.,0.]],
            [[0.,0.],[scales[1],scales[1]]],[[0.,0.],[0.,0.]]];
        assert!(inspect_jacobian_rank_bound(j).unwrap());
    }
    // Both columns can coincide in this enclosure, even though some
    // individual matrices inside it have full rank.
    let uncertain=[[[1.,1.],[1.,1.]],[[0.,0.],[-1.,1.]],[[0.,0.],[0.,0.]]];
    assert!(!inspect_jacobian_rank_bound(uncertain).unwrap());
    assert!(!inspect_jacobian_endpoint_chords(uncertain,[[0.,0.],[0.,0.],[1.,1.]],1e-4).unwrap().0);
    let mut invalid=uncertain;
    invalid[1][1]=[1.,-1.];
    assert!(inspect_jacobian_rank_bound(invalid).is_err());
    invalid[1][1]=[0.,f64::INFINITY];
    assert!(inspect_jacobian_rank_bound(invalid).is_err());
}
#[test]
fn direct_chord_excludes_without_uniform_matrix_rank() {
    let j=[[[1.,1.],[1.,1.]],[[0.,0.],[-1.,1.]],[[0.,0.],[0.,0.]]];
    assert!(!inspect_jacobian_rank_bound(j).unwrap());
    let pair=[[[0.,0.1],[0.,0.1]],[[0.9,1.],[0.,0.1]]];
    let normal=[[0.,0.],[0.,0.],[1.,1.]];
    for domains in [pair,[pair[1],pair[0]]] {
        assert!(inspect_jacobian_endpoint_pair_chords(j,domains,normal,1e-4).unwrap());
        assert!(!inspect_jacobian_endpoint_pair_chords(j,domains,
            [[1.,1.],[0.,0.],[0.,0.]],1e-4).unwrap(),
            "a rank-deficient enclosure must preserve an aligned chord candidate");
    }
    assert!(!inspect_jacobian_endpoint_chords(j,normal,1e-4).unwrap().0);
    assert!(!inspect_jacobian_endpoint_pair_chords(j,[pair[0];2],normal,1e-4).unwrap());
    assert!(!inspect_jacobian_endpoint_pair_chords(j,pair,[[0.,0.];3],1e-4).unwrap());
}
#[test]
fn endpoint_pair_chord_enclosure_preserves_aligned_and_zero_candidates() {
    let j=[[[1.,1.],[0.,0.]],[[0.,0.],[1.,1.]],[[0.,0.],[0.,0.]]];
    let pair=[[[0.,0.1],[0.,0.1]],[[0.9,1.],[0.,0.1]]];
    assert!(inspect_jacobian_endpoint_pair_chords(j,pair,[[0.,0.],[0.,0.],[1.,1.]],1e-4).unwrap());
    assert!(!inspect_jacobian_endpoint_pair_chords(j,pair,[[1.,1.],[0.,0.],[0.,0.]],1e-4).unwrap());
    assert!(!inspect_jacobian_endpoint_pair_chords(j,[pair[0];2],[[0.,0.],[0.,0.],[1.,1.]],1e-4).unwrap());
    assert!(inspect_jacobian_endpoint_pair_chords(j,pair,[[f64::NAN;2];3],1e-4).is_err());
    for scale in [1e-200,1.,1e200] {
        let matrix=j.map(|row|row.map(|d|d.map(|x|x*scale)));
        let domains=pair.map(|uv|uv.map(|d|d.map(|x|x*scale)));
        for domains in [domains,[domains[1],domains[0]]] {
            assert!(inspect_jacobian_endpoint_pair_chords(matrix,domains,
                [[0.,0.],[0.,0.],[scale,scale]],1e-4).unwrap());
            assert!(!inspect_jacobian_endpoint_pair_chords(matrix,domains,
                [[scale,scale],[0.,0.],[0.,0.]],1e-4).unwrap());
        }
    }
}
#[test]
fn endpoint_mean_path_refuses_tangent_and_singular_candidates() {
    let j=[[[1.,1.],[0.,0.]],[[0.,0.],[1.,1.]],[[0.,0.],[0.,0.]]];
    assert!(inspect_jacobian_rank_bound(j).unwrap());
    assert!(!inspect_jacobian_rank_bound([[[0.,0.];2];3]).unwrap());
    assert!(inspect_jacobian_endpoint_chords(j,[[0.,0.],[0.,0.],[1.,1.]],1e-4).unwrap().0);
    assert!(!inspect_jacobian_endpoint_chords(j,[[1.,1.],[0.,0.],[0.,0.]],1e-4).unwrap().0);
    assert!(!inspect_jacobian_endpoint_chords([[[0.,0.];2];3],[[0.,0.],[0.,0.],[1.,1.]],1e-4).unwrap().0);
    assert!(!inspect_jacobian_endpoint_chords(j,[[-1.,1.];3],1e-4).unwrap().0);
    assert!(inspect_jacobian_endpoint_chords(j,[[f64::NAN;2];3],1e-4).is_err());
    for scale in [1e-90,1.,1e90] {
        let scaled=j.map(|row|row.map(|d|d.map(|x|x*scale)));
        assert!(inspect_jacobian_endpoint_chords(scaled,[[0.,0.],[0.,0.],[scale,scale]],1e-4).unwrap().0);
    }
}
