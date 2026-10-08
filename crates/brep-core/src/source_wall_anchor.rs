//! Reusable, budgeted original anchor paths for one unchanged Body face pair.
//! Product direction proofs never establish complete pair or wall coverage.
use crate::source_volume::Body;
use nurbs_core::{Error,Result,surface_self_chord as chord,rational_pole_differential as polar};

pub struct PreparedPair<'a> {
    body:&'a Body,faces:[usize;2],
    anchors:[chord::SurfaceAnchorCertificate<'a>;2],
    differential:[Option<polar::Certificate<'a>>;2],
}
pub struct Preparation<'a> {pub prepared:Option<PreparedPair<'a>>,pub exact_work:u64}
pub struct ProductCertificate<'a> {
    body:&'a Body,faces:[usize;2],rectangles:[[[f64;2];2];2],max_sine_squared:f64,
}
impl<'a> ProductCertificate<'a> {
    pub fn body(&self)->&'a Body{self.body}
    pub fn faces(&self)->[usize;2]{self.faces}
    pub fn rectangles(&self)->[[[f64;2];2];2]{self.rectangles}
    pub fn max_sine_squared(&self)->f64{self.max_sine_squared}
}
/// UV and world inputs are proposals; exact composition on both original
/// surfaces must prove their common identity before preparing any paths.
pub fn prepare<'a>(body:&'a Body,faces:[usize;2],uv:[[f64;2];2],world:[f64;3],max_work:u64)
    ->Result<Preparation<'a>> {
    let regions=body.geometry().shell().regions().ok_or_else(||Error::new("BREP_SOURCE_WALL_ANCHOR","Require original retained regions"))?;
    if faces[0]==faces[1]||faces.iter().any(|&f|f>=regions.len())||max_work>100000000 {
        return Err(Error::new("BREP_SOURCE_WALL_ANCHOR","Choose distinct owned faces and bounded exact work"));
    }
    let sources=faces.map(|f|regions[f].loops()[0][0].surface());
    let mut used=0;let mut anchors=Vec::new();
    for side in 0..2 {
        let (anchor,work)=chord::qualify_surface_anchor(sources[side],uv[side],world,max_work-used)?;
        used+=work;
        let Some(anchor)=anchor else{return Ok(Preparation{prepared:None,exact_work:used});};
        anchors.push(anchor);
    }
    let mut differential=[None,None];
    for side in 0..2 {
        if used==max_work {break;}
        let s=sources[side];let low=s.knots_u[s.degree_u];let high=s.knots_u[s.control_points.len()];
        let end=if uv[side][0]==low {false}else if uv[side][0]==high {true}else{continue;};
        match polar::prepare(s,end,max_work-used) {
            Ok(r)=>{used+=r.work;differential[side]=r.certificate;}
            Err(e) if e.code=="NURBS_INVALID_INPUT"=>{}
            Err(e) if e.code=="NURBS_NUMERIC_ERROR"=>{used=max_work;break;}
            Err(e)=>return Err(e),
        }
    }
    let anchors=anchors.try_into().ok().expect("two original anchors");
    Ok(Preparation{prepared:Some(PreparedPair{body,faces,anchors,differential}),exact_work:used})
}
impl<'a> PreparedPair<'a> {
    /// All reusable facts remain attached to this immutable original Body.
    /// Query work includes path restrictions, endpoint enclosures and normals.
    pub fn qualify_product(&self,rectangles:[[[f64;2];2];2],projection:[f64;3],sine:f64,max_spans:usize)
        ->Result<(Option<ProductCertificate<'a>>,usize)> {
        let (proof,used)=chord::qualify_polar_anchored_chords(self.anchors.each_ref(),rectangles,
            self.differential.each_ref().map(Option::as_ref),[None,None],projection,sine,max_spans)?;
        Ok((proof.map(|_|ProductCertificate{body:self.body,faces:self.faces,rectangles,max_sine_squared:sine}),used))
    }
}
