//! Exact represented strip jets in a common along-seam basis plus independent regularity.
use super::{Boundary, regularity};
use crate::{Result, check, surface::Surface};
use cad_predicates::{
    AuthoredScalar, BezierIdentity, Limits, PredicateContext, SourceArena, ToleranceContext,
    rational_bezier_strip_jet_identity,
};

#[derive(Clone, Debug)]
pub struct ExactStripJetReport {
    pub certified: bool,
    pub exact_identity: bool,
    pub regularity_certified: bool,
    pub work: u64,
    pub reason: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nonuniform_along_bezier_chain_requires_geometric_not_equal_speed_jets() {
        let surface = |y:f64| Surface {
            degree_u:2,degree_v:1,
            knots_u:vec![0.,0.,0.,0.25,0.25,0.5,0.5,1.,1.,1.],
            knots_v:vec![0.,0.,1.,1.],
            control_points:[0.,1.5,3.,6.5,10.,15.5,21.].iter().map(|&x|
                vec![vec![x,y,0.],vec![x,y+1.,0.]]).collect(),
            weights:vec![vec![1.;2];7],periodic_u:false,periodic_v:false,
        };
        let a=surface(0.);let b=surface(1.);
        let proof=inspect_surface_projective_strip_jets(&a,&b,"vMax","vMin",2,1.,1000000).unwrap();
        assert!(proof.certified,"{proof:?}");
        assert!(!inspect_surface_projective_strip_jets(&a,&b,"vMax","vMin",2,1.,proof.work-1).unwrap().certified);
        let mut kink_a=a.clone();let mut kink_b=b.clone();
        for s in [&mut kink_a,&mut kink_b] {for p in &mut s.control_points[3] {p[2]=0.125;}}
        assert!(!inspect_surface_projective_strip_jets(&kink_a,&kink_b,"vMax","vMin",1,1.,1000000).unwrap().certified);
    }
    fn quarter(second: bool) -> Surface {
        let points = if second { [[0.,1.],[-1.,1.],[-1.,0.]] }
            else { [[1.,0.],[1.,1.],[0.,1.]] };
        Surface { degree_u:1, degree_v:2, knots_u:vec![0.,0.,1.,1.],
            knots_v:vec![0.,0.,0.,1.,1.,1.],
            control_points:(0..2).map(|z|points.iter().map(|p|vec![p[0],p[1],z as f64]).collect()).collect(),
            weights:vec![vec![1.,std::f64::consts::FRAC_1_SQRT_2,1.];2],
            periodic_u:false,periodic_v:false }
    }
    #[test]
    fn projective_rational_quarters_certify_geometry_and_reject_curvature_change() {
        let a=quarter(false);let b=quarter(true);
        let audit=|b:&Surface,order,work|inspect_surface_projective_strip_jets(&a,b,"vMax","vMin",order,1.,work).unwrap();
        assert!(!inspect_surface_exact_strip_jets(&a,&b,"vMax","vMin",2,1.,1000000).unwrap().exact_identity);
        let positive=audit(&b,2,1000000);
        assert!(positive.certified,"{positive:?}");
        assert!(!audit(&b,2,positive.work-1).certified);
        assert!(!audit(&b,2,0).certified);
        let mut scaled=b.clone();for row in &mut scaled.weights {for w in row {*w*=2.;}}
        let scaled_report=audit(&scaled,2,1000000); assert!(scaled_report.certified,"{scaled_report:?}");
        let mut changed=b.clone();for row in &mut changed.control_points {row[2][1]=0.125;}
        assert!(audit(&changed,1,1000000).certified);
        assert!(!audit(&changed,2,1000000).exact_identity);
        let mut singular=b.clone();singular.control_points[1]=singular.control_points[0].clone();
        assert!(!audit(&singular,2,1000000).certified);
    }
    #[test]
    fn linear_multispan_requires_exact_internal_surface_jet_agreement() {
        let multi=|second|{
            let mut s=quarter(second);
            let row=s.control_points[0].clone();
            s.control_points=[0.,0.5,1.].iter().map(|&z|row.iter().map(|p|vec![p[0]+z,p[1],z]).collect()).collect();
            s.weights=vec![s.weights[0].clone();3];s.knots_u=vec![0.,0.,0.5,1.,1.];s
        };
        let a=multi(false);let b=multi(true);
        let audit=|a:&Surface,b:&Surface,work|inspect_surface_projective_strip_jets(a,b,"vMax","vMin",2,1.,work).unwrap();
        let positive=audit(&a,&b,1000000);
        assert!(positive.certified,"{positive:?}");
        assert!(!audit(&a,&b,positive.work-1).certified);
        let mut changed_a=a.clone();let mut changed_b=b.clone();
        for s in [&mut changed_a,&mut changed_b] {for p in &mut s.control_points[1] {p[2]=0.625;}}
        let changed=audit(&changed_a,&changed_b,1000000);
        assert!(!changed.certified);
        assert_eq!(changed.reason,"linear-along-jet-smoothness-unproved");
        for s in [&mut changed_a,&mut changed_b] {for p in &mut s.control_points[1] {p[2]=f64::from_bits(0.5f64.to_bits()+1);}}
        assert!(!audit(&changed_a,&changed_b,1000000).certified);
        let mut flat_a=a.clone();let mut flat_b=b.clone();
        for s in [&mut flat_a,&mut flat_b] {s.control_points=vec![s.control_points[0].clone();3];}
        let singular=audit(&flat_a,&flat_b,1000000);
        assert!(singular.exact_identity && !singular.regularity_certified && !singular.certified);
        let mut knots=a.clone();knots.knots_u[2]=0.25;
        let mut other=b.clone();other.knots_u[2]=0.25;
        // Knot spacing changes parameter speed, not this straight geometry.
        assert!(audit(&knots,&other,1000000).certified);
    }
    #[test]
    fn represented_full_multiplicity_chain_preserves_exact_binary_speeds() {
        let seam=f64::from_bits(28f64.to_bits()+2);
        let left_mid=f64::from_bits(27.5f64.to_bits()+1);
        let right_mid=f64::from_bits(28.5f64.to_bits()+1);
        let surface=|y:f64|Surface{degree_u:2,degree_v:1,
            knots_u:vec![0.,0.,0.,0.5,0.5,1.,1.,1.],knots_v:vec![0.,0.,1.,1.],
            control_points:[27.,left_mid,seam,right_mid,29.].iter().map(|&x|
                vec![vec![x,y,0.],vec![x,y+1.,0.]]).collect(),
            weights:vec![vec![1.;2];5],periodic_u:false,periodic_v:false};
        let a=surface(0.);let b=surface(1.);
        let proof=inspect_surface_projective_strip_jets(&a,&b,"vMax","vMin",2,1.,1000000).unwrap();
        assert!(proof.certified && proof.exact_identity && proof.regularity_certified,"{proof:?}");
        assert!(!inspect_surface_projective_strip_jets(&a,&b,"vMax","vMin",2,1.,proof.work-1).unwrap().certified);
        assert!(!inspect_surface_projective_strip_jets(&a,&b,"vMax","vMin",2,1.,0).unwrap().certified);
        let saved=a.control_points.clone();
        let mut kink_a=a.clone();let mut kink_b=b.clone();
        for s in [&mut kink_a,&mut kink_b] {for p in &mut s.control_points[3] {p[2]=f64::EPSILON;}}
        assert!(!inspect_surface_projective_strip_jets(&kink_a,&kink_b,"vMax","vMin",2,1.,1000000).unwrap().certified);
        assert_eq!(a.control_points,saved);
    }
    fn plane(y: f64) -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..2)
                .map(|x| vec![vec![x as f64, y, 0.], vec![x as f64, y + 1., 0.]])
                .collect(),
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    #[cfg(feature="transport")]
    fn transport_preserves_exact_identity_and_regularity(){
        let request=value_codec::json!({"op":"surface_exact_strip_jets_audit",
            "reference":plane(0.),"edited":plane(1.),"referenceBoundary":"vMax",
            "editedBoundary":"vMin","order":2,"normalScale":1.,"maxWork":1000000});
        let report=crate::transport::dispatch(request).unwrap();
        assert_eq!(report["certified"],true);
        assert_eq!(report["exactIdentity"],true);
        assert_eq!(report["regularityCertified"],true);
        let projective=crate::transport::dispatch(value_codec::json!({"op":"surface_projective_strip_jets_audit",
            "reference":quarter(false),"edited":quarter(true),"referenceBoundary":"vMax",
            "editedBoundary":"vMin","order":2,"normalScale":1.,"maxWork":1000000})).unwrap();
        assert_eq!(projective["certified"],true);
        assert_eq!(projective["certifiedOrder"],2);
        assert_eq!(projective["method"],"constant-projective-strip-jets");
    }
    #[test]
    fn common_multispan_basis_preserves_exact_jets_and_checks_every_span() {
        let multi = |y: f64| Surface {
            degree_u: 3, degree_v: 1,
            knots_u: vec![0.,0.,0.,0.,0.5,1.,1.,1.,1.], knots_v: vec![0.,0.,1.,1.],
            control_points: [0.,0.125,0.5,0.875,1.].iter().map(|&x|
                vec![vec![x,y,0.],vec![x,y+1.,0.]]).collect(),
            weights: vec![vec![1.,1.],vec![2.,2.],vec![3.,3.],vec![2.,2.],vec![1.,1.]],
            periodic_u:false, periodic_v:false,
        };
        let a=multi(0.); let b=multi(1.);
        let audit=|a:&Surface,b:&Surface,work|inspect_surface_exact_strip_jets(a,b,"vMax","vMin",2,1.,work).unwrap();
        let positive=audit(&a,&b,1000000);
        assert!(positive.certified && positive.exact_identity && positive.regularity_certified);
        assert_eq!(positive.reason,"exact-regular-bspline-strip-jets");
        assert!(!audit(&a,&b,positive.work-1).certified);
        let mut unclamped_a=a.clone(); let mut unclamped_b=b.clone();
        for surface in [&mut unclamped_a,&mut unclamped_b] { surface.knots_u=vec![-1.,-0.5,-0.25,0.,0.5,1.,1.25,1.5,2.]; }
        assert_eq!(audit(&unclamped_a,&unclamped_b,1000000).work,0);
        let mut changed=b.clone(); changed.knots_u[4]=0.25;
        assert!(!audit(&a,&changed,1000000).certified);
        let mut changed=b.clone(); changed.control_points[4][1][1]=f64::from_bits(2f64.to_bits()+1);
        assert!(!audit(&a,&changed,1000000).exact_identity);
        let mut singular_a=a.clone(); let mut singular_b=b.clone();
        for surface in [&mut singular_a,&mut singular_b] {
            for row in &mut surface.control_points { for point in row { point[0]=0.; } }
        }
        let singular=audit(&singular_a,&singular_b,1000000);
        assert!(singular.exact_identity && !singular.regularity_certified && !singular.certified);
    }
    #[test]
    fn original_surface_jets_require_exact_basis_and_regularity() {
        let a = plane(0.);
        let mut b = plane(1.);
        let r = inspect_surface_exact_strip_jets(&a, &b, "vMax", "vMin", 2, 1., 1000000).unwrap();
        assert!(r.certified && r.exact_identity && r.regularity_certified);
        assert!(r.work > 0 && r.work <= 1000000);
        assert!(
            !inspect_surface_exact_strip_jets(&a, &b, "vMax", "vMin", 2, 1., 0)
                .unwrap()
                .certified
        );
        b.control_points[0][1][1] = f64::from_bits(2f64.to_bits() + 1);
        assert!(
            !inspect_surface_exact_strip_jets(&a, &b, "vMax", "vMin", 2, 1., 1000000)
                .unwrap()
                .exact_identity
        );
        let mut a = plane(0.);
        let mut b = plane(1.);
        a.control_points[1] = a.control_points[0].clone();
        b.control_points[1] = b.control_points[0].clone();
        let r = inspect_surface_exact_strip_jets(&a, &b, "vMax", "vMin", 2, 1., 1000000).unwrap();
        assert!(r.exact_identity && !r.certified && !r.regularity_certified);
    }
}
/// Parameter scales concern normalized cross parameters. Different identifies
/// failure of this sufficient homogeneous condition, not general G1/G2 failure.
pub fn inspect_surface_exact_strip_jets(
    a: &Surface,
    b: &Surface,
    ab: &str,
    bb: &str,
    order: usize,
    scale: f64,
    max_work: u64,
) -> Result<ExactStripJetReport> {
    inspect_strip_jets(a,b,ab,bb,order,scale,max_work,false)
}
/// Sufficient exact geometric G1/G2 with constant transverse reparameterization.
/// Unlike homogeneous C1/C2, this admits projective weight jet differences.
pub fn inspect_surface_projective_strip_jets(a:&Surface,b:&Surface,ab:&str,bb:&str,order:usize,scale:f64,max_work:u64)->Result<ExactStripJetReport>{
    inspect_strip_jets(a,b,ab,bb,order,scale,max_work,true)
}
// Full-multiplicity interior knots already encode literal Bezier pieces.
// Copying their poles/weights introduces no rounded extraction or refit.
fn along_piece(s:&Surface,b:Boundary,span:usize)->Surface {
    let (p,_,k,_)=b.along(s);let start=span-p;
    let mut out=s.clone();let knots=[vec![k[span];p+1],vec![k[span+1];p+1]].concat();
    if b.cross_u {
        out.knots_v=knots;out.periodic_v=false;
        out.control_points=s.control_points.iter().map(|row|row[start..=span].to_vec()).collect();
        out.weights=s.weights.iter().map(|row|row[start..=span].to_vec()).collect();
    }else{
        out.knots_u=knots;out.periodic_u=false;
        out.control_points=s.control_points[start..=span].to_vec();out.weights=s.weights[start..=span].to_vec();
    }
    out
}
fn certify_along_chain(s:&Surface,b:Boundary,order:usize,max_work:u64)->Result<(bool,u64)> {
    let (p,n,k,_)=b.along(s);let spans=(p..n).filter(|&i|k[i]<k[i+1]).collect::<Vec<_>>();
    let (end,start)=if b.cross_u {("vMax","vMin")}else{("uMax","uMin")};
    let pieces=spans.iter().map(|&i|along_piece(s,b,i)).collect::<Vec<_>>();
    let mut work=0;
    for pair in pieces.windows(2){
        let scale=super::station_scale::propose_boundary_normal_scale(&pair[0],&pair[1],end,start)?;
        let proof=inspect_proposed_station_projective_strip_jets(&pair[0],&pair[1],end,start,order,scale,max_work-work)?;
        work+=proof.work;if !proof.certified{return Ok((false,work));}
    }
    // Explicit represented end coincidence additionally requires closure jets.
    let closed=(0..b.cross(s).1).all(|layer|{let x=b.index(s,0,layer);let y=b.index(s,n-1,layer);s.control_points[x.0][x.1]==s.control_points[y.0][y.1]});
    if closed&&pieces.len()>1 {
        let last=pieces.last().unwrap();
        let scale=super::station_scale::propose_boundary_normal_scale(last,&pieces[0],end,start)?;
        let proof=inspect_proposed_station_projective_strip_jets(last,&pieces[0],end,start,order,scale,max_work-work)?;
        work+=proof.work;if !proof.certified{return Ok((false,work));}
    }
    Ok((true,work))
}
pub(crate) fn inspect_proposed_station_projective_strip_jets(a:&Surface,b:&Surface,ab:&str,bb:&str,order:usize,scale:f64,max_work:u64)->Result<ExactStripJetReport>{
    let proposal=super::station_scale::propose_station_scale_scalar(scale);
    let mut proof=inspect_station_projective_strip_jets(a,b,ab,bb,order,scale,proposal,max_work)?;
    if !proof.certified && !proof.exact_identity && proof.work<max_work {
        if let Some(scalar)=super::station_scale::propose_boundary_binary_ratio_scalar(a,b,ab,bb)? {
            let prior=proof.work;
            proof=inspect_station_projective_strip_jets(a,b,ab,bb,order,scale,scalar,max_work-prior)?;
            proof.work+=prior;
        }
    }
    check(proof.work<=max_work,"Invalid native along-chain work accounting")?;
    Ok(proof)
}
fn inspect_strip_jets(a:&Surface,b:&Surface,ab:&str,bb:&str,order:usize,scale:f64,max_work:u64,projective:bool)->Result<ExactStripJetReport>{
    inspect_strip_jets_scalar(a,b,ab,bb,order,scale,max_work,projective,None)
}
/// An exact rational proposal still requires independent whole-strip identity
/// and regularity. The approximate value is used only for option validation.
pub(crate) fn inspect_station_projective_strip_jets(a:&Surface,b:&Surface,ab:&str,bb:&str,order:usize,scale:f64,scalar:AuthoredScalar,max_work:u64)->Result<ExactStripJetReport>{
    inspect_strip_jets_scalar(a,b,ab,bb,order,scale,max_work,true,Some(scalar))
}
fn inspect_strip_jets_scalar(a:&Surface,b:&Surface,ab:&str,bb:&str,order:usize,scale:f64,max_work:u64,projective:bool,scalar:Option<AuthoredScalar>)->Result<ExactStripJetReport>{
    a.validate()?;
    b.validate()?;
    check(
        [a, b]
            .iter()
            .all(|s| s.control_points.iter().flatten().all(|p| p.len() == 3)),
        "Exact strip jets require XYZ surfaces",
    )?;
    check(
        matches!(order, 1 | 2) && scale.is_finite() && scale > 0. && max_work <= 1000000,
        "Invalid exact strip jet options",
    )?;
    let r = Boundary::parse(ab)?;
    let e = Boundary::parse(bb)?;
    let mut out = ExactStripJetReport {
        certified: false,
        exact_identity: false,
        regularity_certified: false,
        work: 0,
        reason: "unsupported-bezier-strip-basis",
    };
    let bezier_direction = |(p, n, k, periodic): (usize, usize, &[f64], bool)| {
                !periodic
                    && n == p + 1
                    && p <= 32
                    && k.len() == 2 * (p + 1)
                    && k[..p + 1].iter().all(|v| *v == k[p])
                    && k[p + 1..].iter().all(|v| *v == k[n])
                    && k[p] < k[n]
    };
    let aa = r.along(a);
    let ba = e.along(b);
    let common_multispan = !aa.3 && !ba.3 && aa.0 <= 32 && (2..=33).contains(&aa.1)
        && aa.0 == ba.0 && aa.1 == ba.1 && aa.2 == ba.2
        && aa.0 >= order
        && aa.2[..=aa.0].iter().all(|&k| k == aa.2[aa.0])
        && aa.2[aa.1..].iter().all(|&k| k == aa.2[aa.1])
        && aa.2[aa.0..=aa.1].iter().filter(|&&k| k > aa.2[aa.0] && k < aa.2[aa.1])
            .all(|&k| aa.2.iter().filter(|&&v| v == k).count() <= aa.0 - order);
    let common_linear = projective && !aa.3 && !ba.3 && aa.0==1 && ba.0==1 && (3..=33).contains(&aa.1) && aa.1==ba.1 && aa.2==ba.2 && aa.2.len()==aa.1+2 && aa.2[0]==aa.2[1] && aa.2[aa.1]==aa.2[aa.1+1];
    let common_chain=projective&&!aa.3&&!ba.3&&(2..=32).contains(&aa.0)&&(aa.0+2..=33).contains(&aa.1)
        &&aa.0==ba.0&&aa.1==ba.1&&aa.2==ba.2
        &&aa.2[..=aa.0].iter().all(|&k|k==aa.2[aa.0])&&aa.2[aa.1..].iter().all(|&k|k==aa.2[aa.1])
        &&aa.2[aa.0..=aa.1].iter().filter(|&&k|aa.2[aa.0]<k&&k<aa.2[aa.1]).all(|k|aa.2.iter().filter(|x|*x==k).count()==aa.0);
    let common_bezier = bezier_direction(aa) && bezier_direction(ba) && aa.0 == ba.0;
    // The predicate compares every homogeneous cross jet coefficient. The
    // along-seam basis need not be Bernstein, provided it is exactly shared.
    // Cross directions remain Bezier so their normalized jet factors stay exact.
    if !bezier_direction(r.cross(a)) || !bezier_direction(e.cross(b))
        || !(common_bezier || common_multispan || common_linear || common_chain) {
        return Ok(out);
    }
    if common_chain {
        for (surface,boundary) in [(a,r),(b,e)] {
            let (certified,work)=certify_along_chain(surface,boundary,order,max_work-out.work)?;
            out.work+=work;
            if !certified{out.reason="projective-along-chain-jets-unproved";return Ok(out);}
        }
    }
    let mut values = Vec::new();
    let mut strips = Vec::new();
    for (s, edge) in [(a, r), (b, e)] {
        let mut strip = Vec::new();
        for i in 0..edge.along(s).1 {
            let mut row = Vec::new();
            for layer in 0..edge.cross(s).1 {
                let (u, v) = edge.index(s, i, layer);
                let start = values.len();
                values.extend(s.control_points[u][v].iter().copied());
                values.push(s.weights[u][v]);
                row.push(start);
            }
            strip.push(row);
        }
        strips.push(strip);
    }
    let scale_index = values.len();
    values.push(scale);
    let knots_start=values.len();
    if common_linear {values.extend_from_slice(aa.2);}
    let mut authored = values.into_iter().map(|v| AuthoredScalar::Binary64Bits(v.to_bits())).collect::<Vec<_>>();
    if let Some(scalar)=scalar {authored[scale_index]=scalar;}
    let source = SourceArena::authored(
        if projective {"surface-projective-strip-jets"}else{"surface-exact-strip-jets"},
        1,
        authored,
    )
    .map_err(|e| crate::input(e.to_string()))?;
    let refs = |strip: &Vec<Vec<usize>>| {
        strip
            .iter()
            .map(|row| {
                row.iter()
                    .map(|start| std::array::from_fn(|k| source.leaf(start + k).unwrap()))
                    .collect()
            })
            .collect::<Vec<_>>()
    };
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &source,
        &tolerance,
        Limits {
            max_work:max_work-out.work,
            ..Limits::default()
        },
        None,
    );
    if common_linear {
        let knots=(0..aa.2.len()).map(|i|source.leaf(knots_start+i).unwrap()).collect::<Vec<_>>();
        let mut needs_geometric_chain=false;
        for strip in &strips {
            let decision=cad_predicates::rational_linear_strip_along_identity(&mut ctx,&refs(strip),&knots)
                .map_err(|e|crate::input(e.to_string()))?;
            if decision.outcome!=BezierIdentity::Equal {
                out.work+=decision.work_used;
                needs_geometric_chain=true;
                break;
            }
        }
        if needs_geometric_chain {
            // Preserve the fast exact affine test. Unequal represented knot or
            // station speeds may still have geometric G1/G2, proved independently
            // on every original piece and closure under the remaining owner.
            for (surface,boundary) in [(a,r),(b,e)] {
                let (certified,work)=certify_along_chain(surface,boundary,order,max_work-out.work)?;
                out.work+=work;
                if !certified{out.reason="linear-along-jet-smoothness-unproved";return Ok(out);}
            }
            // The prior context's cumulative work is already charged above.
            // A fresh context cannot reuse its original, larger remaining limit.
            ctx=PredicateContext::new(&source,&tolerance,
                Limits {max_work:max_work-out.work,..Limits::default()},None);
        }
    }
    let predicate=if projective {cad_predicates::rational_projective_strip_jet_identity}else{rational_bezier_strip_jet_identity};
    let decision = predicate(
        &mut ctx,
        &refs(&strips[0]),
        &refs(&strips[1]),
        source.leaf(scale_index).unwrap(),
        order,
    )
    .map_err(|e| crate::input(e.to_string()))?;
    out.work += decision.work_used;
    out.exact_identity = decision.outcome == BezierIdentity::Equal;
    out.reason = match decision.outcome {
        BezierIdentity::Equal => "seam-regularity-unproved",
        BezierIdentity::Different => if projective {"constant-projective-jet-relation-different"}else{"homogeneous-jets-different"},
        BezierIdentity::Indeterminate(_) => "exact-jet-work-unresolved",
    };
    if out.exact_identity {
        out.regularity_certified =
            if common_linear||common_chain {regularity::certify_after_exact_along_jets(a,r)?.certified() && regularity::certify_after_exact_along_jets(b,e)?.certified()}
            else {regularity::certify(a, r)?.certified() && regularity::certify(b, e)?.certified()};
        out.certified = out.regularity_certified;
        if out.certified {
            out.reason = if projective {"exact-regular-projective-strip-jets"}
                else if common_bezier { "exact-regular-bezier-strip-jets" }
                else { "exact-regular-bspline-strip-jets" };
        }
    }
    Ok(out)
}
