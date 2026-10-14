//! Exact stored-coefficient premise for ruled retained walls.
//! Domain coverage, body ownership and geometric embedding remain independent.
use crate::{curve::Curve, surface::Surface};

/// Exact original-to-Bezier coefficient identity, including knot insertion.
/// Rounded homogeneous arithmetic or Euclidean division withholds the result.
pub fn exact_bezier_controls(c: &Curve, max_control_rows: usize) -> Option<Vec<Curve>> {
    if max_control_rows == 0 || max_control_rows > 4096 || c.validate().is_err() {
        return None;
    }
    let rows = crate::certificates::audit::curve_span_domains(c).len()
        .checked_mul(c.degree.checked_add(1)?)?;
    if rows > max_control_rows { return None; }
    crate::exact_curve_segments::inspect(c)
}

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

/// Slice already segmented clamped B-spline controls without knot insertion or
/// changing authored controls/weights. Internal multiplicity must be >= degree.
/// The total copied control rows are charged before producing an atomic result.
pub fn segmented_bezier_controls(c:&Curve,max_control_rows:usize)->Option<Vec<Curve>> {
    let p=c.degree;let n=c.control_points.len();let width=p.checked_add(1)?;
    if max_control_rows==0 || max_control_rows>1000000 || p==0 || n<width || n>4096
        || c.periodic || c.weights.len()!=n || c.knots.len()!=n.checked_add(width)?
        || c.control_points.iter().any(|v|v.len()!=3||v.iter().any(|x|!x.is_finite()))
        || c.weights.iter().any(|w|!w.is_finite()||*w<=0.)
        || c.knots.iter().any(|x|!x.is_finite())
        || c.knots.windows(2).any(|v|v[0]>v[1]) {return None;}
    let a=c.knots[p];let b=c.knots[n];
    if a>=b || c.knots[..width].iter().any(|&k|k!=a)
        || c.knots[n..].iter().any(|&k|k!=b) {return None;}
    let mut i=width;
    while i<n {
        let k=c.knots[i];let mut end=i+1;
        while end<c.knots.len()&&c.knots[end]==k {end+=1;}
        if k>a && k<b && end-i<p {return None;} i=end;
    }
    let count=(p..n).filter(|&i|c.knots[i]<c.knots[i+1]).count();
    if count==0 || count.checked_mul(width)?>max_control_rows {return None;}
    Some((p..n).filter(|&i|c.knots[i]<c.knots[i+1]).map(|i|Curve {
        degree:p,knots:std::iter::repeat_n(0.,width).chain(std::iter::repeat_n(1.,width)).collect(),
        control_points:c.control_points[i-p..=i].to_vec(),weights:c.weights[i-p..=i].to_vec(),periodic:false,
    }).collect())
}

/// Complete coefficient-family premise, including endpoint/ring partition and
/// all retained wall faces. This does not replace UV or shell ownership audits.
pub fn family_matches(surfaces:&[Surface],sections:&[Vec<Vec<Curve>>],closed:bool,max_faces:usize)->bool {
    if max_faces==0 || max_faces>1024 || !(2..=1025).contains(&sections.len())
        || surfaces.len()>max_faces+if closed {0}else{2} {return false;}
    let mut prepared=Vec::new();let mut rows_left=1000000usize;
    for station in sections {
        if station.is_empty()||station.len()>1024 {return false;}
        let mut rings=Vec::new();
        for ring in station {
            if ring.is_empty()||ring.len()>1024 {return false;}
            let mut curves=Vec::new();
            for curve in ring {
                let Some(pieces)=segmented_bezier_controls(curve,rows_left) else {return false;};
                let rows:usize=pieces.iter().map(|p|p.control_points.len()).sum();rows_left-=rows;
                curves.push(pieces);
            }
            rings.push(curves);
        }
        prepared.push(rings);
    }
    let mut face=0;
    for pair in prepared.windows(2) {
        if pair[0].len()!=pair[1].len() {return false;}
        for (a,b) in pair[0].iter().zip(&pair[1]) {
            if a.len()!=b.len() {return false;}
            for (a,b) in a.iter().zip(b) {
                if a.len()!=b.len() {return false;}
                for (a,b) in a.iter().zip(b) {
                    if face>=max_faces {return false;}
                    let Some(surface)=surfaces.get(face) else {return false;};
                    if !matches(surface,a,b,4096) {return false;}face+=1;
                }
            }
        }
    }
    face>0 && face==surfaces.len().saturating_sub(if closed {0}else{2})
}

/// Exact cyclic contour identity, allowing opposite traversal. Every attempted
/// control-row comparison consumes shared work, including failed candidates.
/// This premise alone does not certify the filled cap region.
pub fn contour_matches(expected:&[Curve],actual:&[Curve],reversed:&[bool],max_work:usize)->(bool,usize) {
    if max_work==0 || max_work>1000000 || expected.is_empty() || expected.len()>1024
        || actual.len()!=expected.len() || reversed.len()!=actual.len() {return (false,0);}
    let valid=|c:&Curve| {
        let n=c.control_points.len();
        n>=2 && n<=4096 && c.degree.checked_add(1)==Some(n) && !c.periodic
            && c.weights.len()==n && c.knots.len()==2*n
            && c.knots.iter().enumerate().all(|(i,&k)|k==if i<n {0.}else{1.})
            && c.weights.iter().all(|w|w.is_finite()&&*w>0.)
            && c.control_points.iter().all(|p|p.len()==3&&p.iter().all(|x|x.is_finite()))
    };
    if !expected.iter().chain(actual).all(valid) {return (false,0);}
    let mut work=0;
    for offset in 0..expected.len() {
        for reverse in [false,true] {
            let mut matched=true;
            for (i,c) in actual.iter().enumerate() {
                let index=if reverse {(offset+expected.len()-i)%expected.len()}else{(offset+i)%expected.len()};
                let part=&expected[index];
                if c.degree!=part.degree {matched=false;break;}
                for row in 0..c.control_points.len() {
                    if work==max_work {return (false,work);}
                    work+=1;
                    let j=if reversed[i]!=reverse {part.control_points.len()-1-row}else{row};
                    if c.weights[row]!=part.weights[j] || c.control_points[row]!=part.control_points[j] {
                        matched=false;break;
                    }
                }
                if !matched {break;}
            }
            if matched {return (true,work);}
        }
    }
    (false,work)
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
    fn cyclic_contours_preserve_weights_orientation_and_atomic_budget() {
        let (_,mut a,mut b)=fixture();
        a.knots=vec![0.,0.,0.,1.,1.,1.];b.knots=a.knots.clone();
        let expected=vec![a.clone(),b.clone()];
        let actual=vec![b.clone(),a.clone()];
        let (matched,work)=contour_matches(&expected,&actual,&[false,false],100);
        assert!(matched);assert!(work>6);
        assert_eq!(contour_matches(&expected,&actual,&[false,false],work),(true,work));
        assert!(!contour_matches(&expected,&actual,&[false,false],work-1).0);
        let mut backwards=actual.clone();
        for c in &mut backwards {c.control_points.reverse();c.weights.reverse();}
        assert!(contour_matches(&expected,&backwards,&[true,true],100).0);
        assert!(!contour_matches(&expected,&backwards,&[false,true],100).0);
        let mut damaged=actual;damaged[0].weights[1]=f64::from_bits(0.5f64.to_bits()+1);
        assert!(!contour_matches(&expected,&damaged,&[false,false],100).0);
        assert!(!contour_matches(&expected,&damaged,&[false],100).0);
        damaged[0].control_points[0].clear();
        assert_eq!(contour_matches(&expected,&damaged,&[false,false],100),(false,0));
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
    #[test]
    fn segmented_controls_preserve_authored_rows_and_refuse_partial_budget() {
        let c=Curve{degree:2,knots:vec![7.,7.,7.,13.,13.,19.,19.,19.],
            control_points:(0..5).map(|i|vec![i as f64,(i%2)as f64,0.]).collect(),
            weights:vec![1.,0.5,1.,2.,1.],periodic:false};
        let before=c.clone();let parts=segmented_bezier_controls(&c,6).unwrap();
        assert_eq!(parts.len(),2);assert_eq!(parts[0].control_points,c.control_points[..3]);
        assert_eq!(parts[1].weights,c.weights[2..]);assert_eq!(c,before);
        assert!(segmented_bezier_controls(&c,5).is_none());
        let mut unsegmented=c.clone();unsegmented.knots.remove(4);
        unsegmented.control_points.pop();unsegmented.weights.pop();
        assert!(segmented_bezier_controls(&unsegmented,6).is_none());
        let mut periodic=c;periodic.periodic=true;assert!(segmented_bezier_controls(&periodic,6).is_none());
    }
    #[test]
    fn complete_family_checks_face_count_partition_and_source_damage() {
        let(s,a,b)=fixture();let sections=vec![vec![vec![a.clone()]],vec![vec![b.clone()]]];
        let surfaces=vec![s.clone(),s.clone(),s.clone()];
        assert!(family_matches(&surfaces,&sections,false,1));
        assert!(!family_matches(&surfaces[..2],&sections,false,1));
        assert!(!family_matches(&surfaces,&sections,false,0));
        let mut damaged=sections.clone();damaged[1][0][0].control_points[1][0]+=0.125;
        assert!(!family_matches(&surfaces,&damaged,false,1));
        let mut partition=sections;partition[1].push(vec![b]);
        assert!(!family_matches(&surfaces,&partition,false,1));
    }
    #[test]
    fn three_edge_contour_rejects_permutation_and_accepts_opposite_traversal() {
        let (_,mut a,mut b)=fixture();a.knots=vec![0.,0.,0.,1.,1.,1.];b.knots=a.knots.clone();
        let mut c=a.clone();for p in &mut c.control_points {p[0]+=8.;}
        let expected=vec![a.clone(),b.clone(),c.clone()];
        assert!(!contour_matches(&expected,&[a.clone(),c.clone(),b.clone()],&[false;3],100).0);
        let mut reverse=vec![c,b,a];
        for curve in &mut reverse {curve.control_points.reverse();curve.weights.reverse();}
        assert!(contour_matches(&expected,&reverse,&[false;3],100).0);
        for curve in &mut reverse {curve.control_points.reverse();curve.weights.reverse();}
        assert!(contour_matches(&expected,&reverse,&[true;3],100).0);
    }

    #[cfg(feature="transport")]
    #[test]
    fn contour_json_boundary_does_not_claim_a_filled_region() {
        let (_,mut a,_)=fixture();a.knots=vec![0.,0.,0.,1.,1.,1.];
        let r=crate::transport::dispatch(value_codec::json!({
            "op":"sweep_retained_cap_contour_audit","expected":[a.clone()],
            "actual":[a],"reversed":[false],"maxWork":3})).unwrap();
        assert_eq!(r["contourIdentity"].as_bool(),Some(true));
        assert_eq!(r["work"].as_u64(),Some(3));
        assert_eq!(r["filledRegionCertified"].as_bool(),Some(false));
    }

    #[cfg(feature="transport")]
    #[test]
    fn family_and_segmentation_json_boundaries_preserve_proof_scope() {
        let(s,a,b)=fixture();let r=crate::transport::dispatch(value_codec::json!({
            "op":"curve_segmented_bezier_controls","curve":a.clone(),"maxControlRows":3})).unwrap();
        let parts:value_codec::Value=r;assert_eq!(parts.as_array().unwrap().len(),1);
        let r=crate::transport::dispatch(value_codec::json!({"op":"sweep_retained_wall_family_audit",
            "surfaces":[s.clone(),s.clone(),s],"sections":[[[a]],[[b]]],"closed":false,"maxFaces":1})).unwrap();
        assert_eq!(r["coefficientFamilyIdentity"].as_bool(),Some(true));
        assert_eq!(r["globalEmbeddingCertified"].as_bool(),Some(false));
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
