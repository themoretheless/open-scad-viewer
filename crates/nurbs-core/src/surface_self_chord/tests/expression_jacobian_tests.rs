use super::*;
#[test]
fn fixed_parameter_shear_strengthens_the_complete_interval_bound() {
    let j = [[[1.,1.],[19.62,20.38]],[[0.,0.],[1.,1.]],[[0.,0.],[0.,0.]]];
    let mut interval = [[I::point(0.);2];3];
    for k in 0..3 {for axis in 0..2 {
        interval[k][axis]=I::new(j[k][axis][0],j[k][axis][1]).unwrap();
    }}
    let mut diagonal = interval;
    for row in &mut diagonal {row[1]=row[1].div(I::point(20.38)).unwrap();}
    assert!(cosine_upper(interval).unwrap()>=1.);
    assert!(cosine_upper(diagonal).unwrap()>=1.,"Diagonal-only bound must remain unresolved on this fixture");
    assert!(inspect_jacobian_self_chords(j,1e-4).unwrap().0);
    for scale in [1e-90,1.,1e90] {
        let scaled=j.map(|row|row.map(|d|d.map(|x|x*scale)));
        assert!(inspect_jacobian_self_chords(scaled,1e-4).unwrap().0);
    }
    // An enclosed singular matrix still forbids intrinsic exclusion.
    let singular=[[[1.,1.],[19.62,20.38]],[[0.,0.],[-1.,1.]],[[0.,0.],[0.,0.]]];
    assert!(!inspect_jacobian_self_chords(singular,1e-4).unwrap().0);
}
#[test]
fn independent_interval_widths_use_operator_bound_without_hiding_singular_charts() {
    // Complete independent derivative intervals. The old Frobenius
    // numerator is .72, exceeding the squared minimum singular value .49.
    // The width-Gram row bound is .36 and proves the same full enclosure.
    let j=[[[0.7,1.3],[0.,0.]],[[0.,0.],[0.7,1.3]],[[0.,0.],[0.,0.]]];
    assert!(0.72/0.49>1.);
    for scale in [1e-90,1.,1e90] {
        let scaled=j.map(|row|row.map(|d|d.map(|x|x*scale)));
        let (excluded,upper)=inspect_jacobian_self_chords(scaled,1e-4).unwrap();
        assert!(excluded && upper<0.74);
    }
    let singular=[[[ -0.3,0.3],[0.,0.]],[[0.,0.],[0.7,1.3]],[[0.,0.],[0.,0.]]];
    assert!(!inspect_jacobian_self_chords(singular,1e-4).unwrap().0);
    // Correlated width columns do not gain the independent-column factor.
    let uncertain=[[[0.7,1.3],[0.7,1.3]],[[0.,0.],[-0.3,0.3]],[[0.,0.],[0.,0.]]];
    assert!(!inspect_jacobian_self_chords(uncertain,1e-4).unwrap().0);
}
#[test]
fn nonlinear_planar_parameterization_qualifies_its_complete_original_chart() {
    // x(u)=.7u+.3u^2, y(v)=.7v+.3v^2. Both original derivatives
    // vary independently over [.7,1.3], while every chord lies in XY
    // and every regular normal is parallel to Z.
    let coordinates=[0.,0.35,1.];
    let surface=Surface {degree_u:2,degree_v:2,
        knots_u:vec![0.,0.,0.,1.,1.,1.],knots_v:vec![0.,0.,0.,1.,1.,1.],
        control_points:coordinates.iter().map(|x|coordinates.iter().map(|y|vec![*x,*y,0.]).collect()).collect(),
        weights:vec![vec![1.;3];3],periodic_u:false,periodic_v:false};
    let report=qualify_rectangle(&surface,[[0.,1.];2],1e-4,100).unwrap();
    assert!(report.complete && report.certificate.is_some());
    assert!(report.cosine_squared_upper.unwrap()<1.-1e-4);
    // Original-surface qualification requires a positive span limit;
    // unlike the diagnostic profile API, zero is an input refusal.
    assert!(qualify_rectangle(&surface,[[0.,1.];2],1e-4,0).is_err());
}
#[test]
fn enclosed_plane_excludes_normal_chords_and_rejects_invalid_inputs() {
    let j = [[[1., 1.], [0., 0.]], [[0., 0.], [1., 1.]], [[0., 0.], [0., 0.]]];
    assert!(inspect_jacobian_self_chords(j, 1e-4).unwrap().0);
    let mut invalid = j;
    invalid[0][0] = [2., 1.];
    assert!(inspect_jacobian_self_chords(invalid, 1e-4).is_err());
    invalid[0][0] = [f64::NAN, 1.];
    assert!(inspect_jacobian_self_chords(invalid, 1e-4).is_err());
    assert!(inspect_jacobian_self_chords(j, 1.).is_err());
    assert!(!inspect_jacobian_self_chords([[[0., 0.]; 2]; 3], 1e-4).unwrap().0);
}
