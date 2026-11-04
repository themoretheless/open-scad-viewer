//! Candidate finite-face subdivision retaining original world and UV geometry.
use super::*;
#[derive(Clone,Debug)]
pub struct RootFaceChordSplit {
    source:Arc<RootBrep>,output:Arc<RootBrep>,face:usize,chord:ss::OriginalSsRootEdge,
    support:usize,hole_sides:Vec<bool>,new_face:usize,new_edge:usize,
}
impl RootFaceChordSplit {
    /// Hole sides are explicit candidate assignments (false: original face,
    /// true: appended face). This operation proves incidence, not material or
    /// nonintersection of the chord with other boundary geometry.
    pub fn split(source:Arc<RootBrep>,face:usize,chord:ss::OriginalSsRootEdge,
        support:usize,hole_sides:Vec<bool>)->Result<Self> {
        let t=source.topology();let f=t.faces.get(face).ok_or_else(||unproved("Missing chord-split face"))?;
        if support>=2 || hole_sides.len()!=f.holes.len() {return Err(unproved("Invalid chord support or hole assignments"));}
        let s=chord.curve().source();let selected=if support==0 {s.first()} else {s.second()};
        if selected!=&f.surface {return Err(unproved("Chord belongs to another original surface"));}
        if t.faces.iter().enumerate().any(|(i,x)| i!=face && (x.outer==f.outer || x.holes.contains(&f.outer))) {
            return Err(unproved("Chord split needs a uniquely owned outer loop"));
        }
        let uses=&t.loops[f.outer].coedges;
        let mut positions=[Vec::new(),Vec::new()];
        for (i,c) in uses.iter().enumerate() {
            let edge=&t.edges[c.edge].curve;
            let start=if c.reversed {edge.end()} else {edge.start()};
            for (k,p) in [chord.start(),chord.end()].into_iter().enumerate() {
                if start.point().same_selector(p.point()) {positions[k].push(i);}
            }
        }
        if positions.iter().any(|v|v.len()!=1) || positions[0][0]==positions[1][0] {
            return Err(unproved("Chord endpoints must be distinct unique outer-loop vertices"));
        }
        let a=positions[0][0];let b=positions[1][0];
        // Check both incident boundary ports at each endpoint, preserving UV
        // branches even on surfaces that are not world-injective.
        for (index,port) in [(a,chord.start()),(b,chord.end())] {
            let prev=&uses[(index+uses.len()-1)%uses.len()];let next=&uses[index];
            let ep=&t.edges[prev.edge].curve;let en=&t.edges[next.edge].curve;
            qualify_root_uv_joint(if prev.reversed {ep.start()} else {ep.end()},prev.pcurve.support,port,support)?;
            qualify_root_uv_joint(port,support,if next.reversed {en.end()} else {en.start()},next.pcurve.support)?;
        }
        let mut output=t.clone();let new_edge=output.edges.len();let new_face=output.faces.len();
        let vertices=[a,b].map(|i| {let c=&uses[i];t.edges[c.edge].vertices[usize::from(c.reversed)]});
        output.edges.push(brep_topology::Edge {curve:chord.clone(),vertices,degenerate:false});
        let path=|start:usize,end:usize| {
            let mut out=Vec::new();let mut i=start;
            while i!=end {out.push(uses[i].clone());i=(i+1)%uses.len();}out
        };
        let mut first=path(a,b);let mut second=path(b,a);
        first.push(brep_topology::Coedge::new(new_edge,true,RootPcurve {world_edge:new_edge,support,reversed:true}));
        second.push(brep_topology::Coedge::new(new_edge,false,RootPcurve {world_edge:new_edge,support,reversed:false}));
        output.loops[f.outer]=brep_topology::Loop {coedges:first};
        let new_loop=output.loops.len();output.loops.push(brep_topology::Loop {coedges:second});
        let mut first_holes=Vec::new();let mut second_holes=Vec::new();
        for (&id,&side) in f.holes.iter().zip(&hole_sides) {if side {second_holes.push(id)} else {first_holes.push(id)}}
        output.faces[face].holes=first_holes;
        output.faces.push(brep_topology::Face {surface:f.surface.clone(),outer:new_loop,holes:second_holes});
        for shell in &mut output.shells {
            if let Some(u)=shell.faces.iter().find(|u|u.face==face).cloned() {
                shell.faces.push(brep_topology::FaceUse {face:new_face,reversed:u.reversed});
            }
        }
        let output=Arc::new(RootBrep::from_topology(output)?);
        Ok(Self {source,output,face,chord,support,hole_sides,new_face,new_edge})
    }
    pub fn source(&self)->&Arc<RootBrep> {&self.source}
    pub fn output(&self)->&Arc<RootBrep> {&self.output}
    pub fn face_children(&self)->[usize;2] {[self.face,self.new_face]}
    pub fn inserted_edge(&self)->usize {self.new_edge}
    pub fn reverify(&self)->Result<()> {
        self.source.reverify()?;self.chord.reverify()?;self.output.reverify()?;
        let fresh=Self::split(self.source.clone(),self.face,self.chord.clone(),self.support,self.hole_sides.clone())?;
        if fresh.new_face!=self.new_face || fresh.new_edge!=self.new_edge {return Err(unproved("Chord subdivision lineage changed"));}Ok(())
    }
}
