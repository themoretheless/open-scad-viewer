//! Explicit candidate face retention with stable topology remapping.
use super::*;
#[derive(Clone,Debug)]
pub struct RootFaceRetention {
    source:Arc<RootBrep>,output:Arc<RootBrep>,selected:Vec<usize>,
    vertices:Vec<Option<usize>>,edges:Vec<Option<usize>>,loops:Vec<Option<usize>>,
    faces:Vec<Option<usize>>,shells:Vec<Option<usize>>,bodies:Vec<Option<usize>>,
}
fn mapping(used:&[bool])->Vec<Option<usize>> {
    let mut next=0;used.iter().map(|&yes|if yes {let i=next;next+=1;Some(i)} else {None}).collect()
}
fn keep<T:Clone>(items:&[T],map:&[Option<usize>])->Vec<T> {
    items.iter().zip(map).filter_map(|(x,id)|id.map(|_|x.clone())).collect()
}
impl RootFaceRetention {
    /// Retain explicit candidate faces. This does not infer which face is
    /// physical material from samples or promote an open shell into a body.
    pub fn retain(source:Arc<RootBrep>,selected:Vec<usize>)->Result<Self> {
        let t=source.topology();let mut fs=vec![false;t.faces.len()];
        for &id in &selected {if id>=fs.len() || fs[id] {return Err(unproved("Invalid or duplicate retained face"));}fs[id]=true;}
        let mut ls=vec![false;t.loops.len()];let mut es=vec![false;t.edges.len()];let mut vs=vec![false;t.vertices.len()];
        for (_,f) in t.faces.iter().enumerate().filter(|(i,_)|fs[*i]) {
            for id in std::iter::once(f.outer).chain(f.holes.iter().copied()) {ls[id]=true;}
        }
        for (_,l) in t.loops.iter().enumerate().filter(|(i,_)|ls[*i]) {for c in &l.coedges {es[c.edge]=true;}}
        for (_,e) in t.edges.iter().enumerate().filter(|(i,_)|es[*i]) {for &v in &e.vertices {vs[v]=true;}}
        let vertices=mapping(&vs);let edges=mapping(&es);let loops=mapping(&ls);let faces=mapping(&fs);
        let shell_counts:Vec<_>=t.shells.iter().map(|s|s.faces.iter().filter(|u|fs[u.face]).count()).collect();
        let shells=mapping(&shell_counts.iter().map(|n|*n>0).collect::<Vec<_>>());
        let complete=|i:usize|shells[i].is_some() && shell_counts[i]==t.shells[i].faces.len();
        let bodies=mapping(&t.bodies.iter().map(|b|complete(b.outer_shell) && b.inner_shells.iter().all(|&s|complete(s))).collect::<Vec<_>>());
        let mut output=RootTopology {vertices:keep(&t.vertices,&vertices),edges:keep(&t.edges,&edges),
            loops:keep(&t.loops,&loops),faces:keep(&t.faces,&faces),shells:keep(&t.shells,&shells),
            bodies:keep(&t.bodies,&bodies),tolerance_mm:t.tolerance_mm};
        for e in &mut output.edges {e.vertices=e.vertices.map(|i|vertices[i].unwrap());}
        for l in &mut output.loops {for c in &mut l.coedges {c.edge=edges[c.edge].unwrap();c.pcurve.world_edge=c.edge;}}
        for f in &mut output.faces {f.outer=loops[f.outer].unwrap();f.holes=f.holes.iter().map(|&i|loops[i].unwrap()).collect();}
        for (old,new) in shells.iter().enumerate().filter_map(|(old,new)|new.map(|new|(old,new))) {
            let s=&mut output.shells[new];s.faces=s.faces.iter().filter_map(|u|faces[u.face].map(|face|brep_topology::FaceUse {face,reversed:u.reversed})).collect();
            s.closed=s.closed && complete(old);
        }
        for b in &mut output.bodies {b.outer_shell=shells[b.outer_shell].unwrap();b.inner_shells=b.inner_shells.iter().map(|&i|shells[i].unwrap()).collect();}
        let output=Arc::new(RootBrep::from_topology(output)?);
        Ok(Self {source,output,selected,vertices,edges,loops,faces,shells,bodies})
    }
    pub fn output(&self)->&Arc<RootBrep> {&self.output}
    pub fn vertex_map(&self)->&[Option<usize>] {&self.vertices}
    pub fn edge_map(&self)->&[Option<usize>] {&self.edges}
    pub fn loop_map(&self)->&[Option<usize>] {&self.loops}
    pub fn face_map(&self)->&[Option<usize>] {&self.faces}
    pub fn shell_map(&self)->&[Option<usize>] {&self.shells}
    pub fn body_map(&self)->&[Option<usize>] {&self.bodies}
    pub fn reverify(&self)->Result<()> {
        self.source.reverify()?;self.output.reverify()?;
        let fresh=Self::retain(self.source.clone(),self.selected.clone())?;
        if self.vertices!=fresh.vertices || self.edges!=fresh.edges || self.loops!=fresh.loops
            || self.faces!=fresh.faces || self.shells!=fresh.shells || self.bodies!=fresh.bodies {
            return Err(unproved("Root face-retention lineage changed"));
        }Ok(())
    }
}
