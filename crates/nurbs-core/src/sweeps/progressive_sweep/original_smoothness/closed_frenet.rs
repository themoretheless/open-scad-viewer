//! Periodic Ck of the actual original Frenet frame, k=1,2.
//! T=P'/|P'|, B=(P'×P'')/|P'×P''|, N=B×T. Exact periodic
//! Cartesian C(k+2) path jets and Ck twist jets give periodic Ck frame jets
//! after a complete nonzero speed/curvature cover. Arc mode also accepts
//! exact C(k+1) geometric unit-tangent jets. This proves no embedding.
use super::*;
pub(in crate::sweeps::progressive_sweep) fn certify(s:&Sweep<'_>,order:usize,max_cells:usize,max_work:u64)->Result<ClosedPathFrameSmoothnessReport> {
    let mut out=ClosedPathFrameSmoothnessReport{order,closed_source_frame_smoothness_certified:false,
        cells:0,exact_work:0,reason:Some("closed-frenet-path-jets-unproved")};
    if s.options.orientation == Orientation::CorrectedFrenet {
        out.reason = Some("closed-corrected-regular-branch-unproved");
        if s.contact_point.is_some() {
            return Ok(out);
        }
        let (regular, work) = authored_error::certify_corrected_regular_branch(
            s.path, max_cells.min(max_work as usize),
        )?;
        out.cells = work;
        out.exact_work = work as u64;
        if !regular {
            return Ok(out);
        }
    }
    out.reason = Some("closed-frenet-path-jets-unproved");
    let mut path_ok=knot_continuity(s.path,order+2,&mut out.exact_work,max_work)?;
    if path_ok {
        let p=super::super::endpoint_jets::certify(s.path,order+2,max_work-out.exact_work)?;
        out.exact_work+=p.exact_work;path_ok=p.certified;
    }
    if !path_ok {
        if s.options.spacing==Spacing::Parameter {return Ok(out);}
        // In arc coordinates P_s=T and N=T_s/|T_s|. Exact C(k+1)
        // unit-tangent joins therefore give a Ck Frenet field, provided the
        // full speed/curvature cover below also succeeds. Raw speed jumps
        // do not supply, or invalidate, this geometric identity.
        for endpoints in [false,true] {
            out.reason=Some("closed-frenet-geometric-position-unproved");
            let p=super::super::endpoint_jets::certify_position(s.path,endpoints,max_work-out.exact_work)?;
            out.exact_work+=p.exact_work;if !p.certified {return Ok(out);}
            out.reason=Some("closed-frenet-geometric-direction-jets-unproved");
            let p=super::super::endpoint_jets::certify_direction(s.path,order+1,endpoints,true,max_work-out.exact_work)?;
            out.exact_work+=p.exact_work;if !p.certified {return Ok(out);}
        }
    }
    out.reason=Some("closed-frenet-twist-jets-unproved");
    if !knot_continuity(s.twist,order,&mut out.exact_work,max_work)? {return Ok(out);}
    let p=super::super::endpoint_jets::certify(s.twist,order,max_work-out.exact_work)?;
    out.exact_work+=p.exact_work;if !p.certified {return Ok(out);}
    // For normalized arc composition, this same cover also owns the positive
    // source speed required by the smooth inverse clock. No sampled normal,
    // planar sign rule, or corrected-Frenet transport supplies these premises.
    let cover=frame::certify_frenet_cover(s.path,s.twist,max_cells-out.cells)?;
    out.cells+=cover.cells;
    out.closed_source_frame_smoothness_certified=cover.status==Status::Certified;
    out.reason=if out.closed_source_frame_smoothness_certified {None}else{cover.reason};
    Ok(out)
}
#[cfg(test)] mod tests;
