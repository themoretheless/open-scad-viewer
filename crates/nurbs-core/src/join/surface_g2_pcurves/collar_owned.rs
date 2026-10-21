//! Fresh composed-collar geometry and private original-position ownership.
use super::*;
pub struct OwnedCollarInspection {
    pub owned: OwnedInspection,
    pub coefficient_work: usize,
}
/// Compare actual stored controls to the ORIGINAL support composition. The
/// rounded construction proposal and previously issued reports grant no proof.
pub fn inspect_owned_collar(
    curve:&Curve, pcurve:&Curve, source:&Surface, direction:[f64;2], candidate:&Surface,
    tolerances:[f64;3],max_cells:usize,max_exact:u64,max_coefficients:usize,
)->Result<OwnedCollarInspection>{
    check(tolerances[1].is_finite()&&(0. ..=2.).contains(&tolerances[1])
        &&tolerances[2].is_finite()&&tolerances[2]>=0.
        &&(1..=2000000).contains(&max_coefficients),"Owned collar geometry limits are invalid")?;
    let edge=Curve::from_polyline(vec![vec![0.,0.],vec![1.,0.]])?;
    let positions=verify_positions(curve,[pcurve,&edge],[source,candidate],[false;2],
        tolerances[0],max_cells,max_exact)?;
    let owner=positions.proven.then(||OwnedPositions{
        curve:curve.clone(),pcurves:[pcurve.clone(),edge],surfaces:[source.clone(),candidate.clone()],
        reversed:[false;2],tolerance:tolerances[0],
    });
    let mut geometric=surface_g2::Report{
        position_bounds:[0.,positions.upper.unwrap_or(f64::INFINITY)],
        branches:std::array::from_fn(|_|BranchBounds{normal_lower:0.,tensor_lower:0.,normal_upper:None,tensor_upper:None}),
        orientation:None,regularity_proven:false,decision:Decision::Unresolved,cells:positions.cells,
    };
    let mut coefficient_work=0;
    if positions.proven && positions.cells<max_cells {
        let proof=crate::surface_collar::inspect_g2(source,pcurve,direction,candidate,tolerances,
            max_cells-positions.cells,max_coefficients)?;
        coefficient_work=proof.coefficient_work;
        geometric.cells+=proof.cells;
        if let (Decision::WithinTolerance,Some(upper),Some(orientation))=(proof.decision,proof.upper,proof.orientation){
            let at=usize::from(orientation==Orientation::Opposite);
            geometric.position_bounds[1]=geometric.position_bounds[1].min(upper[0]);
            geometric.branches[at].normal_upper=Some(upper[1]);
            geometric.branches[at].tensor_upper=Some(upper[2]);
            geometric.orientation=Some(orientation);geometric.regularity_proven=true;
            geometric.decision=Decision::WithinTolerance;
        }else{geometric.decision=proof.decision;}
    }
    Ok(OwnedCollarInspection{owned:OwnedInspection{report:Report{positions,geometric},positions:owner},coefficient_work})
}
