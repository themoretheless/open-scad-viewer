//! Ck of the ORIGINAL contact fit, after source frame and base motion Ck.
//! fit=|G-P|/(scale*axis_x*anchor+center_x). Source-frame nondegeneracy proves
//! |G-P|>0; guide/path Ck and a complete positive denominator cover make this
//! quotient Ck. This does NOT certify rail hits, retained joins, caps or trims.
use super::*;
use crate::distance_bounds::Interval as I;
use crate::sweeps::progressive_miter::{scalar_certificate,vector_certificate};
fn guide_position(s:&Sweep<'_>,guide:&Curve,order:usize,closed:bool,
    cells:&mut usize,work:&mut u64,max_cells:usize,max_work:u64)->Result<bool> {
    let raw=super::super::endpoint_jets::certify_knots(guide,order,max_work-*work)?;
    *work+=raw.exact_work;
    let mut smooth=raw.certified;
    if smooth && closed {
        let seam=super::super::endpoint_jets::certify(guide,order,max_work-*work)?;
        *work+=seam.exact_work;smooth=seam.certified;
    }
    if smooth {return Ok(true);}
    if s.options.spacing==Spacing::Parameter {return Ok(false);}
    // A regular normalized arc position obeys X_s=L*T. Whole original
    // position and forward unit-tangent jets through k-1 give Ck across all
    // source knots and, when closed, the exact quotient seam.
    for endpoints in [false].into_iter().chain(closed.then_some(true)) {
        let p=super::super::endpoint_jets::certify_position(guide,endpoints,max_work-*work)?;
        *work+=p.exact_work;if !p.certified {return Ok(false);}
        let t=super::super::endpoint_jets::certify_direction(guide,order-1,endpoints,true,max_work-*work)?;
        *work+=t.exact_work;if !t.certified {return Ok(false);}
    }
    if *cells==max_cells {return Ok(false);}
    let r=crate::curve_regularity::inspect(guide,(max_cells-*cells).min(100000))?;
    *cells+=r.cells;Ok(r.spanwise_regular)
}
fn denominator(s:&Sweep<'_>,anchor:I,t:[f64;2],cells:&mut usize,max:usize)->Result<Option<I>> {
    let r=scalar_certificate::certify_traversal(s.scale,t,(max-*cells).min(100000))?;
    *cells+=r.cells;let Some(v)=r.value.filter(|_|r.status==Status::Certified) else {return Ok(None);};
    let mut d=I::new(v[0],v[1])?.mul(anchor)?;
    if let Some((axes,center))=s.affine_laws {
        let r=vector_certificate::certify_values_traversal(axes,t,(max-*cells).min(100000),true)?;
        *cells+=r.cells;let Some(v)=r.value else{return Ok(None);};
        d=d.mul(I::new(v[0][0],v[0][1])?)?;
        let r=vector_certificate::certify_values_traversal(center,t,(max-*cells).min(100000),false)?;
        *cells+=r.cells;let Some(v)=r.value else{return Ok(None);};
        d=d.add(I::new(v[0][0],v[0][1])?)?;
    }
    Ok(Some(d))
}
/// Private composition gate: the caller has freshly proved path translation,
/// scale/affine laws and original guided frame Ck on this same selected clock.
/// The frame premise is not accepted from any caller-supplied success flag.
pub(crate) fn certify(s:&Sweep<'_>,order:usize,closed:bool,cells:&mut usize,work:&mut u64,
    max_cells:usize,max_work:u64)->Result<Option<&'static str>> {
    if s.contact_point.is_none() {return Ok(None);}
    let Some(guide)=s.orientation_guide else{return Ok(Some("original-contact-guide-missing"));};
    if !guide_position(s,guide,order,closed,cells,work,max_cells,max_work)? {
        return Ok(Some("original-contact-guide-position-jets-unproved"));
    }
    let r=super::super::contact_anchor::certify(s,(max_cells-*cells).min(100000))?;
    *cells+=r.cells;
    let Some(q)=r.coordinates.filter(|_|r.status==Status::Certified) else {
        return Ok(Some("original-contact-anchor-unproved"));
    };
    let anchor=I::new(q[0][0],q[0][1])?;
    let mut pending=vec![[0.,1.]];
    while let Some(t)=pending.pop() {
        if *cells==max_cells {return Ok(Some("original-contact-fit-work-limit"));}
        *cells+=1;
        let Some(d)=denominator(s,anchor,t,cells,max_cells)? else {
            return Ok(Some("original-contact-fit-enclosure-unproved"));
        };
        if d.lo>0. {continue;}
        if d.hi<=0. {return Ok(Some("original-contact-fit-positivity-unproved"));}
        let mid=crate::distance_bounds::midpoint(t[0],t[1]);
        if !(t[0]<mid && mid<t[1]) {return Ok(Some("original-contact-fit-precision-limit"));}
        pending.extend([[mid,t[1]],[t[0],mid]]);
    }
    Ok(None)
}
