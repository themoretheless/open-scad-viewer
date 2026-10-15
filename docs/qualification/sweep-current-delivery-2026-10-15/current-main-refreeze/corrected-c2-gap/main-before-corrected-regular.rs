//! Eligibility of a continuous nonzero principal branch, not retained error.
//! Actual corrected sign choices and stored section displacement remain separate.
use super::*;
use crate::numerics::interval_vec3::{cross,dot_tight};
#[derive(Debug)]
pub(super) struct Report {
    pub certified: bool,
    pub cells: usize,
    pub reason: Option<&'static str>,
}
pub(super) fn certify(path:&Curve,max_cells:usize)->Result<Report>{
    check(max_cells<=100000,"Corrected regular work exceeds100000 cells")?;
    let mut out=Report{certified:false,cells:0,reason:Some("corrected-regular-source-jets-unproved")};
    // A closed principal field needs exact original position, velocity and
    // acceleration identity. Numerical constructor endpoint agreement alone
    // is insufficient. This proves C0 frame closure, not retained G1/G2.
    if path_is_closed(path)? {
        let seam=super::super::endpoint_jets::certify(path,2,max_cells as u64)?;
        out.cells=seam.exact_work as usize;
        if !seam.certified {
            out.reason=Some("corrected-regular-endpoint-jets-unproved");
            return Ok(out);
        }
    }
    // C4 is sufficient for continuous Frenet second jets through original knots.
    // Exact Cartesian jets, rather than knot multiplicity or sampled normals.
    let basis=super::super::endpoint_jets::certify_basis_knots(path,4,(max_cells-out.cells) as u64)?;
    out.cells+=basis.exact_work as usize;
    let knots=if basis.certified {super::super::endpoint_jets::Report{certified:true,exact_work:0}}
        else {super::super::endpoint_jets::certify_knots(path,4,(max_cells-out.cells) as u64)?};
    out.cells+=knots.exact_work as usize;
    if !knots.certified{return Ok(out);}
    let start=vector_certificate::certify_traversal(path,[0.,0.],max_cells-out.cells,false)?;
    out.cells+=start.cells;
    out.reason=Some("corrected-regular-initial-curvature-unproved");
    if start.status!=Status::Certified{return Ok(out);}
    let decode=|v:[[f64;2];3]|std::array::from_fn(|k|I{lo:v[k][0],hi:v[k][1]});
    let velocity=decode(start.first.unwrap());let acceleration=decode(start.second.unwrap());
    let curvature=cross(velocity,acceleration)?;
    let curvature_norm=dot_tight(curvature,curvature)?;
    let denominator=dot_tight(velocity,velocity)?.mul(dot_tight(acceleration,acceleration)?)?;
    // The constructor tests the length of the transverse unit acceleration.
    // Its regular initial branch must be proved before using its principal seed.
    if curvature_norm.lo<=0. || denominator.hi<=0. || curvature_norm.lo<=denominator.mul(I::point(1e-12).mul(I::point(1e-12))?)?.hi{return Ok(out);}
    let zero=constant_vector_law([0.;3])?;
    let cover=authored_frame_certificate::certify_frenet_cover(path,&zero,max_cells-out.cells)?;
    out.cells+=cover.cells;
    out.reason=cover.reason.or(Some("corrected-regular-original-frame-unproved"));
    if cover.status!=Status::Certified{return Ok(out);}
    // Every original interval must remain above the constructor's transverse
    // threshold. Nonzero curvature alone would not exclude transported-normal
    // fallback in an almost tangential acceleration interval.
    let mut pending=cover.intervals.as_ref().unwrap().iter().rev().map(|i|(i.traversal,0usize)).collect::<Vec<_>>();
    while let Some((interval,depth))=pending.pop() {
        out.reason=Some("corrected-regular-transverse-threshold-unproved");
        if out.cells==max_cells{return Ok(out);}
        let raw=vector_certificate::certify_traversal(path,interval,max_cells-out.cells,false)?;
        out.cells+=raw.cells;
        let proved=if raw.status==Status::Certified {
            let v=decode(raw.first.unwrap());let a=decode(raw.second.unwrap());
            let c=cross(v,a)?;let numerator=dot_tight(c,c)?;
            let denominator=dot_tight(v,v)?.mul(dot_tight(a,a)?)?;
            numerator.lo>0. && denominator.hi>0. && numerator.lo>denominator.mul(I::point(1e-12).mul(I::point(1e-12))?)?.hi
        }else{false};
        if !proved {
            let mid=interval[0]+0.5*(interval[1]-interval[0]);
            if depth==32||mid<=interval[0]||mid>=interval[1]{return Ok(out);}
            pending.push(([mid,interval[1]],depth+1));
            pending.push(([interval[0],mid],depth+1));
        }
    }
    out.certified=true;out.reason=None;Ok(out)
}
#[cfg(test)]
mod tests {
 use super::*;
 #[test]
 fn nonplanar_original_regular_branch_owns_whole_cover_and_shared_refusal(){
  let path=crate::paths::bezier(vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![2.,1.,0.],vec![3.,1.,1.]],None).unwrap();
  let saved=path.clone();let proof=certify(&path,10000).unwrap();
  assert!(proof.certified,"{proof:?}");assert!(proof.cells>0);assert!(proof.cells<=10000);
  assert!(!certify(&path,0).unwrap().certified);
  let short=certify(&path,proof.cells-1).unwrap();assert!(!short.certified);assert!(short.cells<proof.cells);
  assert_eq!(path,saved);
  let mut rational=path.clone();rational.weights=vec![1.,2.,2.,1.];
  rational.knots=rational.knots.iter().map(|u|17.+12.*u).collect();
  let rational_proof=certify(&rational,10000).unwrap();
  assert!(rational_proof.certified,"{rational_proof:?}");
  let straight=crate::primitives::line([0.;3],[3.,0.,0.]).unwrap();
  assert!(!certify(&straight,10000).unwrap().certified);
  let inflection=crate::paths::bezier(vec![vec![0.,0.,0.],vec![1.,1.,0.],vec![2.,-1.,0.],vec![3.,0.,0.]],None).unwrap();
  assert!(!certify(&inflection,10000).unwrap().certified);
  let near_tangential=crate::paths::bezier(vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![3.,1e-14,0.],vec![6.,1e-14,1e-14]],None).unwrap();
  assert!(!certify(&near_tangential,10000).unwrap().certified);
  let endpoint_fallback=crate::paths::bezier(vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![3.,1.,0.],vec![6.,2.5,1e-14]],None).unwrap();
  assert!(!certify(&endpoint_fallback,10000).unwrap().certified);
 }
 #[test]
 fn periodic_spatial_regular_branch_proves_original_active_basis_seam(){
  let mut points=vec![vec![4.,0.,0.],vec![3.,3.,1.],vec![0.,4.,0.],vec![-3.,3.,-1.],vec![-4.,0.,0.],vec![-3.,-3.,1.],vec![0.,-4.,0.],vec![3.,-3.,-1.]];
  points.extend_from_within(..6);
  let path=Curve{degree:6,knots:(0..=20).map(|i|(i as f64-6.)/8.).collect(),control_points:points,weights:vec![1.;14],periodic:true};
  let saved=path.clone();let proof=certify(&path,100000).unwrap();
  assert!(proof.certified,"{proof:?}");assert!(proof.cells>0&&proof.cells<=100000);
  assert!(!certify(&path,0).unwrap().certified);
  assert!(!certify(&path,proof.cells-1).unwrap().certified);
  assert_eq!(path,saved);
  let mut changed=path.clone();changed.knots[1]=changed.knots[1].next_up();
  changed.validate().unwrap();
  assert!(!certify(&changed,100000).unwrap().certified);
 }

}
