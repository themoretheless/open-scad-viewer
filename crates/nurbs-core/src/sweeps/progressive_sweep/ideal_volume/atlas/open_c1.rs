//! A full C1 rectangular source owner, before any P-matrix admission.
use super::*;
pub(super) fn certify(context:&cell::Context<'_, '_>,projection:[[f64;3];3],
    out:&mut IdealVolumeReport,max:usize,max_work:u64)->Result<bool> {
    // An exact normalized-map identity identifies C1 as well as first jets.
    // Keep its representative and pay the separate identity proof in the
    // parent ledger, rather than making theorem cost depend on its encoding.
    let caps=if context.same_cap() {[context.caps[0];2]}else{context.caps};
    let cost=1+caps.iter().map(|s|s.knots_u.len()+s.knots_v.len()).sum::<usize>() as u64;
    if cost>max_work-out.exact_work {return Ok(false);}
    out.exact_work+=cost;
    for (i,s) in caps.into_iter().enumerate() {
        if i==1 && s==caps[0] {continue;}
        if !charts::surface_c1(s) && !cap_identity::certify_continuity(s,1,out,max,max_work) {return Ok(false);}
    }
    // Source poses include actual path translation, scale, affine laws and
    // optional contact fit. Frame smoothness alone is NOT motion C1.
    if super::super::super::original_smooth_volume::motion::certify_open(
        context.s,1,&mut out.cells,&mut out.exact_work,max,max_work)?.is_some() {return Ok(false);}
    if !p_matrix::certify(|b,out,max|context.enclose(b,out,max),projection,out,max)? {return Ok(false);}
    out.embedding_certified=true;out.regularity_certified=true;
    out.parameter_topology_preserved=true;out.reason=None;Ok(true)
}
#[cfg(test)] mod tests;
