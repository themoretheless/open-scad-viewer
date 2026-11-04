//! Whole-boundary hole routing after an original-root chord split.
use super::*;
#[derive(Clone,Copy,Debug)]
pub struct RootHoleAssignmentLimits {
    pub max_cells:usize,pub max_queries:usize,pub max_depth:usize,
    pub winding:ss::OriginalSsWindingLimits,
}
impl Default for RootHoleAssignmentLimits {
    fn default()->Self {Self {max_cells:4096,max_queries:8192,max_depth:20,winding:Default::default()}}
}
#[derive(Clone,Debug)]
pub struct QualifiedRootHoleAssignment {
    candidate:RootFaceChordSplit,output:RootFaceChordSplit,limits:RootHoleAssignmentLimits,
    sides:Vec<bool>,cells:usize,queries:usize,
}
impl QualifiedRootHoleAssignment {
    /// Every complete hole boundary must lie in exactly one child outer winding
    /// region and outside the other. No failed query is treated as exclusion.
    pub fn qualify(candidate:RootFaceChordSplit,limits:RootHoleAssignmentLimits)->Result<Self> {
        if limits.max_cells==0 || limits.max_cells>65536 || limits.max_queries==0
            || limits.max_queries>131072 || limits.max_depth>24 {
            return Err(unproved("Invalid whole-hole assignment budget"));
        }
        let source=candidate.source();let face=candidate.face_children()[0];
        let t=source.topology();let mut sides=Vec::new();let mut cells=0;let mut queries=0;
        for &hole in &t.faces[face].holes {
            let mut side=None;
            for c in &t.loops[hole].coedges {
                let edge=&t.edges[c.edge].curve;
                let whole=edge.enclose(limits.winding.geometry)?;
                // Use the complete graph clock cover; endpoint uncertainty is
                // included, not discarded by an interior midpoint sample.
                let p=edge.parameter_bounds();
                use num_traits::ToPrimitive;
                let mut d=[p[0].to_f64().ok_or_else(||unproved("Hole clock overflow"))?,p[1].to_f64().ok_or_else(||unproved("Hole clock overflow"))?];
                let q=|x|num_rational::BigRational::from_float(x).unwrap();
                if q(d[0])>p[0] {d[0]=d[0].next_down();}if q(d[1])<p[1] {d[1]=d[1].next_up();}
                let natural=edge.curve().parameter_domain();d[0]=d[0].max(natural[0]);d[1]=d[1].min(natural[1]);
                let mut pending=vec![(d,0,Some(whole))];
                while let Some((d,depth,cached))=pending.pop() {
                    if cells==limits.max_cells {return Err(unproved("Whole-hole cell budget exhausted"));}cells+=1;
                    let bounds=match cached {Some(b)=>b,None=>edge.curve().enclose(d,limits.winding.geometry)?};
                    let uv=[bounds.uv[2*c.pcurve.support],bounds.uv[2*c.pcurve.support+1]];
                    let mut values=Vec::new();
                    for child in candidate.face_children() {
                        if queries==limits.max_queries {return Err(unproved("Whole-hole winding query budget exhausted"));}queries+=1;
                        values.push(QualifiedRootLoopWinding::qualify(candidate.output().clone(),child,0,uv,limits.winding).map(|p|p.winding()));
                    }
                    let classified=match (&values[0],&values[1]) {
                        (Ok(a),Ok(b)) if *a!=0 && *b==0=>Some(false),
                        (Ok(a),Ok(b)) if *a==0 && *b!=0=>Some(true),
                        _=>None,
                    };
                    if let Some(s)=classified {
                        if side.is_some_and(|old|old!=s) {return Err(unproved("Hole boundary crosses child winding regions"));}
                        side=Some(s);continue;
                    }
                    let mid=d[0]*0.5+d[1]*0.5;
                    if depth==limits.max_depth || !(d[0]<mid && mid<d[1]) {return Err(unproved("Whole-hole child membership remains unresolved"));}
                    pending.push(([mid,d[1]],depth+1,None));pending.push(([d[0],mid],depth+1,None));
                }
            }
            sides.push(side.ok_or_else(||unproved("Hole boundary is empty"))?);
        }
        let output=RootFaceChordSplit::split(source.clone(),face,candidate.chord().clone(),candidate.support(),sides.clone())?;
        Ok(Self {candidate,output,limits,sides,cells,queries})
    }
    pub fn output(&self)->&RootFaceChordSplit {&self.output}
    pub fn sides(&self)->&[bool] {&self.sides}
    pub fn cells(&self)->usize {self.cells}
    pub fn queries(&self)->usize {self.queries}
    pub fn reverify(&self)->Result<()> {
        self.candidate.reverify()?;let fresh=Self::qualify(self.candidate.clone(),self.limits)?;
        if self.sides!=fresh.sides || self.cells!=fresh.cells || self.queries!=fresh.queries {return Err(unproved("Whole-hole assignment proof changed"));}
        self.output.reverify()
    }
}
