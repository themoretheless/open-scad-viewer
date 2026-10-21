//! Same-operation reuse must preserve every interval bit, including eviction.
use super::*;
fn original() -> Surface {
    Surface {
        degree_u: 2, degree_v: 13,
        knots_u: vec![0.,0.,0.,1.,1.,1.],
        knots_v: [vec![0.;14],vec![1.;14]].concat(),
        control_points: (0..3).map(|u| (0..14).map(|v| {
            let x = u as f64 / 2.; let y = v as f64 / 13.;
            vec![x + 0.7*y, y - 0.25*x, x*y - y*y*0.1]
        }).collect()).collect(),
        weights: (0..3).map(|u| (0..14).map(|v|
            if u==1 {0.7071067811865476} else {1. + v as f64/32.}).collect()).collect(),
        periodic_u: false, periodic_v: false,
    }
}
fn bits(v: Vec<Interval>) -> Vec<u64> {
    v.into_iter().flat_map(|x|[x.lo.to_bits(),x.hi.to_bits()]).collect()
}
#[test]
fn degree_thirteen_original_rectangles_share_exact_bits_and_bounded_controls() {
    let s = original(); s.validate().unwrap(); let before=s.clone();
    let mut c = PreparedControlSurface::new(&s);
    for i in (0..80).chain((0..80).rev()) {
        let x = i as f64 / 160.;
        for domain in [[[x,x+0.5],[0.,1.]],[[0.,1.],[x,x+0.5]],
            [[x,x],[0.,1.]],[[0.,1.],[x,x]],[[0.,0.],[1.,1.]],[[0.,1.],[0.,1.]]] {
            let expected=rectangle_bounds(&s,domain).unwrap().into_iter()
                .flat_map(|x|x.map(f64::to_bits)).collect::<Vec<_>>();
            for _ in 0..2 { assert_eq!(bits(c.single_span_bounds(domain).unwrap().unwrap()),expected); }
            assert!(c.entries.len()<=32 && c.controls<=2048);
            // The following oblique query must reuse the identical patch, not
            // consume a second entry for a different proof operation.
            let count=c.entries.len(); c.patch([2,13],domain).unwrap();
            assert_eq!(c.entries.len(),count);
        }
    }
    assert_eq!(s.control_points,before.control_points);
    assert_eq!(s.weights,before.weights);
}
#[test]
fn invalid_domains_and_other_span_stencils_retain_original_fallback() {
    let mut s=original();
    let mut c=PreparedControlSurface::new(&s);
    for d in [[[1.,0.],[0.,1.]],[[0.,1.],[-0.1,0.5]],[[f64::NAN,1.],[0.,1.]],[[0.,1.],[0.,f64::INFINITY]]] {
        assert!(c.single_span_bounds(d).unwrap().is_none());
        assert!(rectangle_bounds(&s,d).is_err()); assert!(c.entries.is_empty());
    }
    s.control_points.insert(1,s.control_points[1].clone());
    s.weights.insert(1,s.weights[1].clone()); s.knots_u.insert(3,0.5);
    s.validate().unwrap();
    let mut c=PreparedControlSurface::new(&s);
    assert!(c.single_span_bounds([[0.,1.];2]).unwrap().is_none());
    assert!(c.entries.is_empty());
    assert!(rectangle_bounds(&s,[[0.,1.];2]).is_ok());
}
