//! Exact original endpoint identity: sufficient homogeneous jets, then general Cartesian jets.
//! Common degree/domain factors cancel; no rounded derivative is a premise.
use super::*;
#[derive(Debug)]
pub(super) struct Report {
    pub certified: bool,
    pub exact_work: u64,
}
pub(super) fn certify(curve:&Curve,order:usize,max_work:u64)->Result<Report>{
    check((1..=4).contains(&order)&&max_work<=1000000,"Invalid endpoint jet proof limits")?;
    curve.validate()?;
    let periodic = certify_periodic_basis(curve, order, max_work)?;
    if periodic.certified { return Ok(periodic); }
    // Charge any unsuccessful sufficient proposal before the general proof.
    let remaining = max_work - periodic.exact_work;
    // The homogeneous shortcut below establishes only orders1/2. Higher
    // orders use the complete Cartesian recurrence on original coefficients.
    let first=if order<=2 {certify_sufficient(curve,order,remaining)?}
        else {Report {certified:false,exact_work:0}};
    let first=Report{certified:first.certified,exact_work:first.exact_work+periodic.exact_work};
    // Periodic storage evaluates the same original active B-spline basis.
    // Validate actual one-sided Cartesian jets: repeated poles and approximate
    // exterior-knot periodicity alone are never a closure premise.
    if first.certified||curve.degree==0||curve.degree>32{return Ok(first);}
    check(curve.control_points.iter().all(|p|p.len()==3),"Endpoint jets require XYZ laws")?;
    use cad_predicates::{AuthoredScalar,BezierIdentity,Limits,PredicateContext,SourceArena,ToleranceContext,
        rational_bspline_cartesian_endpoint_jet_identity};
    let mut values=curve.control_points.iter().zip(&curve.weights)
        .flat_map(|(p,w)|[p[0],p[1],p[2],*w]).collect::<Vec<_>>();
    let start=values.len();values.extend_from_slice(&curve.knots);
    let source=SourceArena::authored("original-cartesian-endpoint-jets",1,
        values.into_iter().map(|v|AuthoredScalar::Binary64Bits(v.to_bits())).collect())
        .map_err(|e|crate::input(e.to_string()))?;
    let controls=(0..curve.control_points.len()).map(|i|std::array::from_fn(|j|source.leaf(i*4+j).unwrap())).collect::<Vec<_>>();
    let knots=(0..curve.knots.len()).map(|i|source.leaf(start+i).unwrap()).collect::<Vec<_>>();
    let tolerance=ToleranceContext::default_valid();
    let mut ctx=PredicateContext::new(&source,&tolerance,Limits{max_work:max_work-first.exact_work,..Limits::default()},None);
    let proof=rational_bspline_cartesian_endpoint_jet_identity(&mut ctx,&controls,&knots,curve.degree,order)
        .map_err(|e|crate::input(e.to_string()))?;
    Ok(Report{certified:proof.outcome==BezierIdentity::Equal,exact_work:first.exact_work+proof.work_used})
}
fn certify_sufficient(curve: &Curve, order: usize, max_work: u64) -> Result<Report> {
    check(
        matches!(order, 1 | 2) && max_work <= 1000000,
        "Invalid endpoint jet proof limits",
    )?;
    curve.validate()?;
    let mut out = Report {
        certified: false,
        exact_work: 0,
    };
    let p = curve.degree;
    let [a, b] = curve.domain();
    if curve.periodic
        || p == 0
        || p > 32
        || curve.knots[..p + 1].iter().any(|&u| u != a)
        || curve.knots[curve.control_points.len()..]
            .iter()
            .any(|&u| u != b)
    {
        return Ok(out);
    }
    check(
        curve.control_points.iter().all(|v| v.len() == 3),
        "Endpoint jets require XYZ laws",
    )?;
    if curve.control_points.len() != p + 1 || curve.weights[0] != *curve.weights.last().unwrap() {
        return certify_clamped_expansion(curve, order, max_work);
    }
    let differences = [
        vec![(p, 1.), (0, -1.)],
        vec![(p, 1.), (p - 1, -1.), (1, -1.), (0, 1.)],
        if p >= 2 {
            vec![
                (p, 1.),
                (p - 1, -1.),
                (p - 1, -1.),
                (p - 2, 1.),
                (2, -1.),
                (1, 1.),
                (1, 1.),
                (0, -1.),
            ]
        } else {
            vec![]
        },
    ];
    for jet in differences.iter().take(order + 1) {
        if jet.is_empty() {
            continue;
        }
        for coordinate in 0..4 {
            if max_work - out.exact_work < jet.len() as u64 {
                return Ok(out);
            }
            let terms: Vec<_> = jet
                .iter()
                .map(|&(i, sign)| {
                    (
                        curve.weights[i],
                        sign * if coordinate == 3 {
                            1.
                        } else {
                            curve.control_points[i][coordinate]
                        },
                    )
                })
                .collect();
            out.exact_work += terms.len() as u64;
            if crate::exact_products::sum_sign(&terms) != std::cmp::Ordering::Equal {
                return Ok(out);
            }
        }
    }
    out.certified = true;
    Ok(out)
}
// Exact expansion work units belong to cad-predicates; the Bezier fast path
// retains its original product-term accounting.
fn certify_clamped_expansion(curve: &Curve, order: usize, max_work: u64) -> Result<Report> {
    use cad_predicates::{
        AuthoredScalar, BezierIdentity, Limits, PredicateContext, SourceArena, ToleranceContext,
        rational_bspline_endpoint_jet_identity,
    };
    let mut values = curve
        .control_points
        .iter()
        .zip(&curve.weights)
        .flat_map(|(p, w)| [p[0], p[1], p[2], *w])
        .collect::<Vec<_>>();
    let knots_start = values.len();
    values.extend_from_slice(&curve.knots);
    let source = SourceArena::authored(
        "original-bspline-endpoint-jets",
        1,
        values
            .into_iter()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .map_err(|e| crate::input(e.to_string()))?;
    let controls = (0..curve.control_points.len())
        .map(|i| std::array::from_fn(|j| source.leaf(i * 4 + j).unwrap()))
        .collect::<Vec<_>>();
    let knots = (0..curve.knots.len())
        .map(|i| source.leaf(knots_start + i).unwrap())
        .collect::<Vec<_>>();
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &source,
        &tolerance,
        Limits {
            max_work,
            ..Limits::default()
        },
        None,
    );
    let decision =
        rational_bspline_endpoint_jet_identity(&mut ctx, &controls, &knots, curve.degree, order)
            .map_err(|e| crate::input(e.to_string()))?;
    Ok(Report {
        certified: decision.outcome == BezierIdentity::Equal,
        exact_work: decision.work_used,
    })
}
pub(super) fn certify_knots(curve:&Curve,order:usize,max_work:u64)->Result<Report>{
    curve.validate()?;
    if !(1..=4).contains(&order)||curve.control_points.iter().any(|p|p.len()!=3) {
        return Ok(Report{certified:false,exact_work:0});
    }
    use cad_predicates::{AuthoredScalar,BezierIdentity,Limits,PredicateContext,SourceArena,ToleranceContext,
        rational_bspline_internal_jet_identity,rational_piecewise_bezier_knot_jet_identity};
    let mut values=curve.control_points.iter().zip(&curve.weights)
        .flat_map(|(p,w)|[p[0],p[1],p[2],*w]).collect::<Vec<_>>();
    let knots_start=values.len(); values.extend_from_slice(&curve.knots);
    let source=SourceArena::authored("original-piecewise-bezier-knot-jets",1,
        values.into_iter().map(|v|AuthoredScalar::Binary64Bits(v.to_bits())).collect())
        .map_err(|e|crate::input(e.to_string()))?;
    let controls=(0..curve.control_points.len()).map(|i|std::array::from_fn(|j|source.leaf(i*4+j).unwrap())).collect::<Vec<_>>();
    let knots=(0..curve.knots.len()).map(|i|source.leaf(knots_start+i).unwrap()).collect::<Vec<_>>();
    let tolerance=ToleranceContext::default_valid();
    let mut ctx=PredicateContext::new(&source,&tolerance,Limits{max_work,..Limits::default()},None);
    if order<=2 {
        let proposal=rational_piecewise_bezier_knot_jet_identity(&mut ctx,&controls,&knots,curve.degree,order)
            .map_err(|e|crate::input(e.to_string()))?;
        if proposal.outcome==BezierIdentity::Equal{return Ok(Report{certified:true,exact_work:proposal.work_used});}
    }
    let proof=rational_bspline_internal_jet_identity(&mut ctx,&controls,&knots,curve.degree,order)
        .map_err(|e|crate::input(e.to_string()))?;
    Ok(Report{certified:proof.outcome==BezierIdentity::Equal,exact_work:proof.work_used})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn periodic_original_jets_require_exact_active_basis_closure() {
        let ring = [[1.,0.,0.],[0.5,1.,0.5],[-0.5,1.,1.],[-1.,0.,0.],[-0.5,-1.,0.25],[0.5,-1.,-0.5]];
        let mut points = ring.iter().map(|p|p.to_vec()).collect::<Vec<_>>();
        points.extend(ring[..3].iter().map(|p|p.to_vec()));
        let curve = Curve {degree:3,knots:(0..13).map(|k|k as f64).collect(),
            control_points:points,weights:vec![1.;9],periodic:true};
        curve.validate().unwrap();
        let before = curve.clone();
        let seam = certify(&curve,2,1000000).unwrap();
        assert!(seam.certified,"{seam:?}");
        assert!(seam.exact_work>0);
        assert!(!certify(&curve,2,seam.exact_work-1).unwrap().certified);
        assert!(certify_knots(&curve,2,1000000).unwrap().certified);
        let mut damaged=curve.clone();damaged.knots[1]=1_f64.next_up();
        // Numerical periodic-layout validation deliberately tolerates this.
        // The original exact coefficient proof must still reject its seam.
        damaged.validate().unwrap();
        assert!(!certify(&damaged,1,1000000).unwrap().certified);
        assert_eq!(curve,before);
    }

    #[test]
    fn closed_endpoint_c3_c4_use_complete_original_cartesian_jets(){
        let curve=Curve {degree:7,knots:[vec![2.;8],vec![5.;8]].concat(),
            control_points:vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![2.,1.,0.],vec![3.,3.,0.],
                vec![-3.,3.,0.],vec![-2.,1.,0.],vec![-1.,0.,0.],vec![0.,0.,0.]],weights:vec![1.;8],periodic:false};
        let before=curve.clone();let proof=certify(&curve,3,1000000).unwrap();
        assert!(proof.certified,"{proof:?}");assert!(proof.exact_work>0);
        assert!(!certify(&curve,3,0).unwrap().certified);
        assert!(!certify(&curve,3,proof.exact_work-1).unwrap().certified);
        let mut damaged=curve.clone();damaged.control_points[3][1]=3_f64.next_up();
        assert!(certify(&damaged,2,1000000).unwrap().certified);
        assert!(!certify(&damaged,3,1000000).unwrap().certified);
        let constant=Curve {control_points:vec![vec![1.,2.,3.];8],weights:vec![1.,2.,1.,3.,4.,1.,2.,1.],..curve.clone()};
        let fourth=certify(&constant,4,1000000).unwrap();assert!(fourth.certified,"{fourth:?}");
        assert!(!certify(&constant,4,fourth.exact_work-1).unwrap().certified);
        assert_eq!(curve.control_points,before.control_points);assert_eq!(curve.weights,before.weights);assert_eq!(curve.knots,before.knots);
    }
    #[test]
    fn exact_original_endpoint_jets_detect_single_bit_and_charge_whole_proof() {
        let curve = Curve {
            degree: 5,
            knots: vec![0., 0., 0., 0., 0., 0., 1., 1., 1., 1., 1., 1.],
            control_points: vec![
                vec![0., 0., 1.],
                vec![0.125, 0., 1.],
                vec![0.25, 0.125, 1.],
                vec![-0.25, 0.125, 1.],
                vec![-0.125, 0., 1.],
                vec![0., 0., 1.],
            ],
            weights: vec![1.; 6],
            periodic: false,
        };
        let proof = certify(&curve, 2, 1000).unwrap();
        assert!(proof.certified);
        assert_eq!(proof.exact_work, 56);
        assert!(!certify(&curve, 2, proof.exact_work - 1).unwrap().certified);
        let mut wrong = curve.clone();
        wrong.control_points[2][0] = 0.25_f64.next_up();
        assert!(!certify(&wrong, 2, 1000).unwrap().certified);
        assert!(certify(&wrong, 1, 1000).unwrap().certified);
    }
    #[test]
    fn original_bezier_projective_endpoint_jets_keep_cartesian_c2() {
        let curve = Curve {
            degree: 5,
            knots: vec![0., 0., 0., 0., 0., 0., 1., 1., 1., 1., 1., 1.],
            control_points: vec![
                vec![0., 0., 1.],
                vec![0.125, 0., 1.],
                vec![0.25, 0.125, 1.],
                vec![-0.25, 0.125, 1.],
                vec![-0.125, 0., 1.],
                vec![0., 0., 1.],
            ],
            weights: vec![1., 1., 1., 2., 2., 2.],
            periodic: false,
        };
        let proof = certify(&curve, 2, 1000000).unwrap();
        assert!(proof.certified);
        assert!(!certify(&curve, 2, proof.exact_work - 1).unwrap().certified);
        let mut wrong = curve.clone();
        wrong.control_points[2][0] = 0.25_f64.next_up();
        assert!(!certify(&wrong, 2, 1000000).unwrap().certified);
        assert!(certify(&wrong, 1, 1000000).unwrap().certified);
    }
    #[test]
    fn original_products_keep_large_cancellation_and_subnormal_residual() {
        let mut curve =
            crate::sweeps::progressive_sweep::constant_vector_law([1e9, 0., 1.]).unwrap();
        curve.weights = vec![1e12; 2];
        assert!(certify(&curve, 2, 1000).unwrap().certified);
        let tiny = f64::from_bits(1);
        curve.control_points = vec![vec![0.; 3], vec![tiny, 0., 0.]];
        curve.weights = vec![1e-12; 2];
        assert!(!certify(&curve, 1, 1000).unwrap().certified);
    }
}

/// Original direction chart jets; speed/nonparallel premises are separate.
pub(super) fn certify_direction(curve:&Curve,order:usize,endpoints:bool,arc_length:bool,max_work:u64)->Result<Report>{
 check(matches!(order,1|2)&&max_work<=1000000,"Invalid direction jet limits")?;
 curve.validate()?;
 if curve.periodic{return Ok(Report{certified:false,exact_work:0});}
 check(curve.control_points.iter().all(|p|p.len()==3),"Direction jets require XYZ source")?;
 use cad_predicates::{AuthoredScalar,BezierIdentity,Limits,PredicateContext,SourceArena,ToleranceContext,rational_bspline_direction_jet_identity};
 let mut values=curve.control_points.iter().zip(&curve.weights).flat_map(|(p,w)|[p[0],p[1],p[2],*w]).collect::<Vec<_>>();
 let start=values.len();values.extend_from_slice(&curve.knots);
 let source=SourceArena::authored("original-direction-frame-jets",1,values.into_iter().map(|v|AuthoredScalar::Binary64Bits(v.to_bits())).collect()).map_err(|e|crate::input(e.to_string()))?;
 let controls=(0..curve.control_points.len()).map(|i|std::array::from_fn(|j|source.leaf(i*4+j).unwrap())).collect::<Vec<_>>();
 let knots=(0..curve.knots.len()).map(|i|source.leaf(start+i).unwrap()).collect::<Vec<_>>();
 let tol=ToleranceContext::default_valid();let mut ctx=PredicateContext::new(&source,&tol,Limits{max_work,..Limits::default()},None);
 let proof=rational_bspline_direction_jet_identity(&mut ctx,&controls,&knots,curve.degree,order,endpoints,arc_length).map_err(|e|crate::input(e.to_string()))?;
 Ok(Report{certified:proof.outcome==BezierIdentity::Equal,exact_work:proof.work_used})
}

// Translating the complete original knot basis and homogeneous controls by
// one period identifies the two adjacent original spans. A knot of multiplicity
// m in degree p has C^(p-m) basis jets; positive rational weights preserve them.
fn certify_periodic_basis(curve:&Curve,order:usize,max_work:u64)->Result<Report>{
 let mut out=Report{certified:false,exact_work:0};
 let p=curve.degree;let n=curve.control_points.len();
 if !curve.periodic||p==0||p>32||n<=p||curve.control_points.iter().any(|v|v.len()!=3){return Ok(out);}
 let period=n-p;let [a,b]=curve.domain();
 let mut multiplicity=0usize;
 for &k in &curve.knots {
  if out.exact_work==max_work{return Ok(out);}out.exact_work+=1;
  if k==a{multiplicity+=1;}
 }
 if multiplicity>p||order>p-multiplicity{return Ok(out);}
 for i in 0..p {
  for axis in 0..4 {
   if out.exact_work==max_work{return Ok(out);}out.exact_work+=1;
   let value=|j:usize|if axis==3{curve.weights[j]}else{curve.control_points[j][axis]};
   if value(i)!=value(i+period){return Ok(out);}
  }
 }
 for i in 0..curve.knots.len()-period {
  if max_work-out.exact_work<4{return Ok(out);}out.exact_work+=4;
  if crate::exact_products::sum_sign(&[(1.,curve.knots[i+period]),(-1.,curve.knots[i]),(-1.,b),(1.,a)])!=std::cmp::Ordering::Equal{return Ok(out);}
 }
 out.certified=true;Ok(out)
}

// Standard basis continuity suffices when every original interior knot has
// multiplicity <= degree-order. No sampled derivative or coefficient snapping.
pub(super) fn certify_basis_knots(curve:&Curve,order:usize,max_work:u64)->Result<Report>{
 curve.validate()?;let mut out=Report{certified:false,exact_work:0};
 if !(1..=4).contains(&order)||curve.degree<order{return Ok(out);}
 let [a,b]=curve.domain();let mut previous=None;let mut multiplicity=0usize;
 for &k in &curve.knots {
  if out.exact_work==max_work{return Ok(out);}out.exact_work+=1;
  if !(a<k&&k<b){continue;}
  if previous==Some(k){multiplicity+=1;}else{previous=Some(k);multiplicity=1;}
  if multiplicity>curve.degree-order{return Ok(out);}
 }
 out.certified=true;Ok(out)
}

#[cfg(test)]
mod periodic_basis_tests {
 use super::*;
 #[test]
 fn original_periodic_rational_basis_preserves_work_and_weight_refusals(){
  let mut points=vec![vec![1.,0.,0.],vec![0.5,1.,0.5],vec![-0.5,1.,1.],vec![-1.,0.,0.],vec![-0.5,-1.,0.25],vec![0.5,-1.,-0.5]];
  points.extend_from_within(..3);
  let mut weights=vec![1.,2.,0.75,1.5,0.5,1.25];weights.extend_from_within(..3);
  let path=Curve{degree:3,knots:(0..13).map(|i|i as f64).collect(),control_points:points,weights,periodic:true};
  let saved=path.clone();
  let proof=certify(&path,2,100000).unwrap();assert!(proof.certified);assert!(proof.exact_work>0);
  assert!(!certify(&path,2,0).unwrap().certified);
  assert!(!certify(&path,2,proof.exact_work-1).unwrap().certified);
  let mut damaged=path.clone();damaged.weights[8]=damaged.weights[8].next_up();
  match certify(&damaged,2,100000) {Ok(report)=>assert!(!report.certified),Err(_)=>{}}
  assert_eq!(path,saved);
 }
}
