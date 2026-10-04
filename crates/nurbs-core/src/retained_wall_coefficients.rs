//! Exact stored-coefficient premise for ruled retained walls.
//! Domain coverage, body ownership and geometric embedding remain independent.
use crate::{curve::Curve, surface::Surface};

pub fn matches(surface: &Surface, start: &Curve, end: &Curve, max_controls: usize) -> bool {
    let p=start.degree;
    let Some(n)=p.checked_add(1) else {return false;};
    if max_controls==0 || max_controls>4096 || n>max_controls || p==0
        || surface.validate().is_err() || start.validate().is_err() || end.validate().is_err()
        || start.periodic || end.periodic || surface.periodic_u || surface.periodic_v
        || end.degree!=p || surface.degree_u!=p || surface.degree_v!=1
        || start.control_points.len()!=n || end.control_points.len()!=n
        || surface.control_points.len()!=n || surface.weights.len()!=n {return false;}
    let bezier=|c:&Curve| c.knots.len()==2*n
        && c.knots[..n].iter().all(|&k|k==c.knots[p])
        && c.knots[n..].iter().all(|&k|k==c.knots[n]);
    if !bezier(start) || !bezier(end) || surface.knots_u.len()!=2*n
        || surface.knots_u.iter().enumerate().any(|(i,&k)|k!=if i<n {0.}else{1.})
        || surface.knots_v!=[0.,0.,1.,1.] {return false;}
    (0..n).all(|i| {
        let row=&surface.control_points[i];let weights=&surface.weights[i];
        row.len()==2 && weights.len()==2 && start.control_points[i].len()==3
            && end.control_points[i].len()==3
            && row[0]==start.control_points[i] && row[1]==end.control_points[i]
            && weights[0]==start.weights[i] && weights[1]==end.weights[i]
            && start.weights[i]==end.weights[i]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture()->(Surface,Curve,Curve) {
        let a=Curve{degree:2,knots:vec![7.,7.,7.,19.,19.,19.],
            control_points:vec![vec![0.,0.,0.],vec![1.,1.,0.],vec![2.,0.,0.]],
            weights:vec![1.,0.5,1.],periodic:false};
        let mut b=a.clone(); b.knots=vec![2.,2.,2.,5.,5.,5.];
        for point in &mut b.control_points {point[2]=4.;}
        let s=Surface{degree_u:2,degree_v:1,knots_u:vec![0.,0.,0.,1.,1.,1.],
            knots_v:vec![0.,0.,1.,1.],control_points:(0..3).map(|i|vec![a.control_points[i].clone(),b.control_points[i].clone()]).collect(),
            weights:(0..3).map(|i|vec![a.weights[i],b.weights[i]]).collect(),periodic_u:false,periodic_v:false};
        (s,a,b)
    }
    #[test]
    fn normalized_ruled_wall_keeps_authored_rational_controls() {
        let (s,a,b)=fixture(); assert!(matches(&s,&a,&b,3));
        assert!(!matches(&s,&a,&b,2));assert!(!matches(&s,&a,&b,0));
        let mut bad=s.clone();bad.control_points[1][0][0]=f64::from_bits(1f64.to_bits()+1);
        assert!(!matches(&bad,&a,&b,3));
        let mut bad=s.clone();bad.weights[1][1]=0.5000000000000001;
        assert!(!matches(&bad,&a,&b,3));
        let mut bad=s;bad.knots_u=vec![7.,7.,7.,19.,19.,19.];
        assert!(!matches(&bad,&a,&b,3));
    }
    #[test]
    fn malformed_or_different_endpoint_weights_never_certify() {
        let (s,a,mut b)=fixture();b.weights[1]=0.25;assert!(!matches(&s,&a,&b,3));
        let (mut s,a,b)=fixture();s.control_points[0][0].pop();assert!(!matches(&s,&a,&b,3));
        let (s,mut a,b)=fixture();a.weights[0]=f64::NAN;assert!(!matches(&s,&a,&b,3));
    }
    #[cfg(feature="transport")]
    #[test]
    fn json_boundary_reports_only_coefficient_identity() {
        let(s,a,b)=fixture();let r=crate::transport::dispatch(value_codec::json!({
            "op":"sweep_retained_wall_coefficients_audit","surface":s,"start":a,"end":b,"maxControls":3})).unwrap();
        assert_eq!(r["coefficientIdentity"].as_bool(),Some(true));
        assert_eq!(r["globalEmbeddingCertified"].as_bool(),Some(false));
    }
}
