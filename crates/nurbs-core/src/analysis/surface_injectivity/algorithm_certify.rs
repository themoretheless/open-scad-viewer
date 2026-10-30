//! surface injectivity algorithm: certify.
use super::*;
/// Compatibility entry point: every contraction and refinement cell consumes
/// the original shared span budget. Explicit sweep stages use contraction only.
pub fn certify(s:&Surface,max_spans:usize)->Result<Report> {
    let mut report=certify_contraction(s,max_spans)?;
    if report.proven || report.reason=="collapsed-boundary-requires-quotient-proof" || report.reason=="periodic-domain" || report.reason=="work-limit" {return Ok(report);}
    if s.control_points.iter().flatten().all(|p|p==&s.control_points[0][0]) {return Ok(report);}
    // A reflected/rotated parameter chart may move the useful projective
    // origin away from control[0][0]. Corner data only selects ONE proposal;
    // the complete original interval-Jacobian proof still supplies admission.
    let hint=affine_corner_projective_hint(s);
    if let Some(projection)=hint {
        if report.spans==max_spans {report.reason="work-limit";return Ok(report);}
        let(q,cells)=projective_projection_work(s,[projection[0],projection[1]],projection[2],max_spans-report.spans)?;
        report.spans+=cells;
        if let Some(q)=q {report.proven=true;report.projective_projection=Some(projection);report.contraction_upper=Some(q);report.reason="global-projective-projection-contraction";return Ok(report);}
    }
    for basis in LINEAR_PROJECTIONS {
        if report.spans==max_spans { report.reason="work-limit"; return Ok(report); }
        let (q,cells)=linear_projection_work(s,basis,max_spans-report.spans)?;
        report.spans+=cells;
        if let Some(q)=q {report.proven=true;report.linear_projection=Some(basis);report.contraction_upper=Some(q);report.reason="global-linear-projection-contraction";return Ok(report);}
    }
    for projection in projective_projections(s){
        if hint==Some(projection) {continue;}
        if report.spans==max_spans{report.reason="work-limit";return Ok(report);}
        let (q,cells)=projective_projection_work(s,[projection[0],projection[1]],projection[2],max_spans-report.spans)?;
        report.spans+=cells;
        if let Some(q)=q{
            report.proven=true;report.projective_projection=Some(projection);report.contraction_upper=Some(q);
            report.reason="global-projective-projection-contraction";return Ok(report);
        }
    }
    let cells = polar_cells(s,16);
    if s.degree_u <= 8 && s.degree_v <= 8 && cells <= max_spans-report.spans {
        for coordinates in polar_projection_candidates(s)? {
            if cells > max_spans-report.spans { break; }
            let proof = certify_polar_projection(s,coordinates,16,cells)?;
            // Reserve the complete coverage cost even if a candidate is
            // rejected early. The shared budget never understates work.
            report.spans += cells;
            if let Some(q) = proof {
                report.proven=true;report.polar_projection=Some(coordinates);report.contraction_upper=Some(q);
                report.reason="global-polar-projection-contraction";return Ok(report);
            }
        }
    }
    // Axis frames merely propose one continuous polar branch; every original
    // cell must pass the global P-matrix proof before injectivity is admitted.
    if s.degree_u <= 8 && s.degree_v <= 8 && cells <= max_spans-report.spans {
        'frames: for axis in 0..3 { for sign in [1.,-1.] { for zsign in [1.,-1.] {
            if cells > max_spans-report.spans { break 'frames; }
            let mut coordinates=[[0.;4];3];
            coordinates[0][axis]=sign;
            coordinates[1][(axis+1)%3]=sign;
            coordinates[2][(axis+2)%3]=zsign;
            let proof=certify_polar_p_matrix(s,coordinates,16,cells)?;
            report.spans+=cells;
            if let Some(proof)=proof {
                report.proven=true;report.polar_projection=Some(coordinates);
                report.polar_p_matrix=Some(proof);
                report.reason="global-polar-p-matrix";return Ok(report);
            }
        }}}
    }
    if report.spans==max_spans {report.reason="work-limit";}
    Ok(report)
}
