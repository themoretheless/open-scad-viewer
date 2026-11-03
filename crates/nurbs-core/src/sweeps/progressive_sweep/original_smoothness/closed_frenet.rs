//! Periodic Ck of the actual original Frenet frame, k=1,2.
//! T=P'/|P'|, B=(P'×P'')/|P'×P''|, N=B×T. Exact periodic
//! Cartesian C(k+2) path jets and Ck twist jets give periodic Ck frame jets
//! after a complete nonzero speed/curvature cover. This proves no embedding.
use super::*;
pub(super) fn certify(s:&Sweep<'_>,order:usize,max_cells:usize,max_work:u64)->Result<ClosedPathFrameSmoothnessReport> {
    let mut out=ClosedPathFrameSmoothnessReport{order,closed_source_frame_smoothness_certified:false,
        cells:0,exact_work:0,reason:Some("closed-frenet-path-jets-unproved")};
    if !knot_continuity(s.path,order+2,&mut out.exact_work,max_work)? {return Ok(out);}
    let p=super::super::endpoint_jets::certify(s.path,order+2,max_work-out.exact_work)?;
    out.exact_work+=p.exact_work;if !p.certified {return Ok(out);}
    out.reason=Some("closed-frenet-twist-jets-unproved");
    if !knot_continuity(s.twist,order,&mut out.exact_work,max_work)? {return Ok(out);}
    let p=super::super::endpoint_jets::certify(s.twist,order,max_work-out.exact_work)?;
    out.exact_work+=p.exact_work;if !p.certified {return Ok(out);}
    // For normalized arc composition, this same cover also owns the positive
    // source speed required by the smooth inverse clock. No sampled normal,
    // planar sign rule, or corrected-Frenet transport supplies these premises.
    let cover=frame::certify_frenet_cover(s.path,s.twist,max_cells)?;
    out.cells=cover.cells;
    out.closed_source_frame_smoothness_certified=cover.status==Status::Certified;
    out.reason=if out.closed_source_frame_smoothness_certified {None}else{cover.reason};
    Ok(out)
}
#[cfg(test)] mod tests;
