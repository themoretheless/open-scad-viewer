//! Whole-box membership in an explicitly authored outer-minus-holes UV domain.
use super::*;
#[derive(Clone,Debug,PartialEq,Eq)]
pub enum RootFaceRegion {
    OutsideOuter,
    ExcludedByHoles(Vec<usize>),
    InsideSupport,
}
#[derive(Clone,Debug)]
pub struct QualifiedRootFaceRegion {
    source:Arc<RootBrep>,face:usize,query:[[f64;2];2],limits:ss::OriginalSsWindingLimits,
    loops:Vec<QualifiedRootLoopWinding>,region:RootFaceRegion,segments:usize,
}
impl QualifiedRootFaceRegion {
    /// Classify the complete query box using nonzero outer winding minus the
    /// union of all nonzero hole windings. This defines a UV support; it does
    /// not assert Jordan embedding or physical solid material orientation.
    /// Segment allowance is shared across loops; geometry limits are per loop.
    pub fn qualify(source:Arc<RootBrep>,face:usize,query:[[f64;2];2],limits:ss::OriginalSsWindingLimits)->Result<Self> {
        let f=source.topology().faces.get(face).ok_or_else(||unproved("Missing root-region face"))?;
        if f.holes.len()>64 {return Err(unproved("Root-region hole budget exhausted"));}
        let mut loops=Vec::new();let mut segments=0;
        for i in 0..=f.holes.len() {
            let remaining=limits.max_segments.checked_sub(segments).ok_or_else(||unproved("Root-region segment budget exhausted"))?;
            let proof=QualifiedRootLoopWinding::qualify(source.clone(),face,i,query,ss::OriginalSsWindingLimits {max_segments:remaining,..limits})?;
            segments+=proof.segments();loops.push(proof);
        }
        let holes=loops.iter().enumerate().skip(1).filter_map(|(i,p)|(p.winding()!=0).then_some(i-1)).collect::<Vec<_>>();
        let region=if loops[0].winding()==0 {RootFaceRegion::OutsideOuter}
            else if holes.is_empty() {RootFaceRegion::InsideSupport} else {RootFaceRegion::ExcludedByHoles(holes)};
        Ok(Self {source,face,query,limits,loops,region,segments})
    }
    pub fn region(&self)->&RootFaceRegion {&self.region}
    pub fn loop_windings(&self)->Vec<i32> {self.loops.iter().map(|p|p.winding()).collect()}
    pub fn segments(&self)->usize {self.segments}
    pub fn source(&self)->&Arc<RootBrep> {&self.source}
    pub fn reverify(&self)->Result<()> {
        self.source.reverify()?;
        let fresh=Self::qualify(self.source.clone(),self.face,self.query,self.limits)?;
        if self.region!=fresh.region || self.segments!=fresh.segments || self.loop_windings()!=fresh.loop_windings() {
            return Err(unproved("Root face UV support proof changed"));
        }Ok(())
    }
}
