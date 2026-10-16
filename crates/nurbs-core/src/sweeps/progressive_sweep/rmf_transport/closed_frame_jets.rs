//! Derivatives of the owned ORIGINAL arc-holonomy-corrected Bishop solution.
//! A raw open normal and a corrected normal solve DIFFERENT angular ODEs.
use super::*;
use crate::sweeps::progressive_miter::authored_frame_certificate as frame;

pub(in crate::sweeps::progressive_sweep) fn closed_frame_jets(
    transport: &OriginalRmfTransportReport,
    path: &Curve,
    twist: &Curve,
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<frame::Report> {
    closed_jets(
        transport,
        path,
        twist,
        traversal,
        frame::PathPhase::identity(traversal),
        max_cells,
        false,
    )
}
/// `phase` MUST be the SAME original path's normalized arc inverse owner.
pub(in crate::sweeps::progressive_sweep) fn closed_phase_jets(
    transport: &OriginalRmfTransportReport,
    path: &Curve,
    twist: &Curve,
    station: [f64; 2],
    phase: frame::PathPhase,
    max_cells: usize,
) -> Result<frame::Report> {
    closed_jets(transport, path, twist, station, phase, max_cells, true)
}
fn closure_anchor(
    transport:&OriginalRmfTransportReport,path:&Curve,traversal:[f64;2],
    cells:&mut usize,max:usize,
)->Result<Option<Vec3>>{
    let Some(cover)=transport.cover.as_ref() else{return Ok(None);};
    let (Some(first),Some(last),Some(angle),Some(length))=(cover.first(),cover.last(),transport.closing_angle,transport.original_arc_length) else{return Ok(None);};
    if first.traversal[0]!=0. || last.traversal[1]!=1. || *cells==max {return Ok(None);}
    let left=traversal[1];let right=1.-traversal[0];
    let (strip,distance)=if left<=right {([0.,traversal[1]],I::point(traversal[1]))}
        else {([traversal[0],1.],I::point(1.).sub(I::point(traversal[0]))?)};
    if distance.hi>0.125 {return Ok(None);}
    *cells+=1; // bounded source endpoint/ODE anchor proposal, including failure
    let radius=if distance.hi==0. {0.} else {
        let jets=vector_certificate::certify_traversal(path,strip,max-*cells,false)?;
        *cells+=jets.cells;
        if jets.status!=Status::Certified {return Ok(None);}
        let [a,b]=path.domain();let width=I::point(b).sub(I::point(a))?;
        let v=scale(decode(jets.first.unwrap())?,width)?;
        let acc=scale(decode(jets.second.unwrap())?,width.mul(width)?)?;
        let speed=norm(v)?;if speed.lo<=0. {return Ok(None);}
        let omega=div(cross(v,acc)?,speed.mul(speed)?)?;
        let correction=I::point(angle[0].abs().max(angle[1].abs()))
            .mul(speed)?.div(I::new(length[0],length[1])?)?;
        // The corrected skew ODE preserves |N|=1. Its entire strip angular
        // speed bounds |N(t)-N(endpoint)| by the integral of that speed.
        distance.mul(norm(omega)?.add(correction)?)?.hi
    };
    let initial=decode(first.normal)?;
    Ok(Some(add(initial,[I::new(-radius,radius)?;3])?))
}
fn closed_jets(
    transport: &OriginalRmfTransportReport,
    path: &Curve,
    twist: &Curve,
    station: [f64; 2],
    phase: frame::PathPhase,
    max_cells: usize,
    original_arc: bool,
) -> Result<frame::Report> {
    let traversal = phase.traversal;
    check(
        max_cells <= 100000
            && traversal.iter().all(|x| x.is_finite())
            && 0. <= traversal[0]
            && traversal[0] <= traversal[1]
            && traversal[1] <= 1.,
        "Invalid original closed Bishop jet restriction",
    )?;
    let unresolved = |cells, reason| frame::Report {
        status: Status::Unresolved,
        cells,
        longitudinal: None,
        transverse: None,
        binormal: None,
        single_span: false,
        reason: Some(reason),
    };
    let (Some(cover), Some(normals), Some(binormals), Some(angle), Some(length)) = (
        &transport.cover,
        &transport.arc_corrected_normals,
        &transport.arc_corrected_binormals,
        transport.closing_angle,
        transport.original_arc_length,
    ) else {
        return Ok(unresolved(0, "bishop-complete-corrected-cover-required"));
    };
    if transport.status != Status::Certified
        || cover.is_empty()
        || cover.len() != normals.len()
        || cover.len() != binormals.len()
        || length[0] <= 0.
        || length[0] > length[1]
        || length.iter().any(|x| !x.is_finite())
    {
        return Ok(unresolved(0, "bishop-complete-corrected-cover-required"));
    }
    let mut cells = 0;
    let mut covered = traversal[0];
    let mut normal: Option<Vec3> = None;
    for (cell, n) in cover.iter().zip(normals) {
        if cell.traversal[1] < traversal[0] || cell.traversal[0] > traversal[1] {
            continue;
        }
        if cells == max_cells {
            return Ok(unresolved(cells, "bishop-corrected-cover-query-work-limit"));
        }
        cells += 1;
        if cell.traversal[0] > covered || cell.traversal[0] >= cell.traversal[1] {
            return Ok(unresolved(cells, "bishop-corrected-cover-query-gap"));
        }
        covered = covered.max(cell.traversal[1]);
        let n = decode(*n)?;
        if let Some(acc) = &mut normal {
            for k in 0..3 {
                acc[k] = I::new(acc[k].lo.min(n[k].lo), acc[k].hi.max(n[k].hi))?;
            }
        } else {
            normal = Some(n);
        }
    }
    let Some(mut normal) = normal.filter(|_| covered >= traversal[1]) else {
        return Ok(unresolved(cells, "bishop-corrected-cover-query-missing"));
    };
    // The actual source closing angle maps its terminal normal to its initial
    // normal exactly. Re-anchor a short endpoint strip using the corrected
    // source ODE, instead of retaining unrelated accumulated N/angle boxes.
    if let Some(anchor)=closure_anchor(transport,path,traversal,&mut cells,max_cells)? {
        for k in 0..3 {normal[k]=normal[k].intersect(anchor[k].lo,anchor[k].hi)?;}
    }
    // Original length and holonomy were charged ONCE when this transport was
    // built. Only selected cover cells and fresh path/twist restrictions spend
    // query work here. No sampled-chord length or fitted correction is used.
    let mut result = if original_arc {
        frame::certify_bishop_phase_jets(
            path,
            encode(normal),
            twist,
            station,
            phase,
            max_cells - cells,
            Some(frame::BishopCorrection::OriginalArc {
                angle: I::new(angle[0], angle[1])?,
            }),
        )?
    } else {
        frame::certify_arc_corrected_bishop_jets(
            path,
            encode(normal),
            twist,
            traversal,
            max_cells - cells,
            angle,
            length,
        )?
    };
    result.cells += cells;
    Ok(result)
}
#[cfg(test)]
mod tests;
