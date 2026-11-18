//! Exact support-plane contact confined to two opposite authored boundaries.
//! The caller MUST independently own exact lifts, valid material trims and
//! whole-chart injectivity. This is not a solid or smoothness certificate.
use super::*;
use std::collections::BTreeSet;
pub(crate) struct Attempt {
    pub certificate:Option<crate::boundary_hull_contact::Certificate>,
    pub cells:usize,
}
struct Work {spent:usize,max:usize}
impl Work {
    fn charge(&mut self)->Option<()> {
        if self.spent==self.max {None}else{self.spent+=1;Some(())}
    }
    fn side(&mut self,plane:[&[f64];3],p:&[f64])->Option<Sign> {
        self.charge()?;orient(&[plane[0],plane[1],plane[2],p],None)
    }
}
fn prove(model:&Model,faces:[usize;2],planar:usize,w:&mut Work)
    ->Option<crate::boundary_hull_contact::Certificate> {
    let a=&model.faces[faces[1-planar]];
    let b=&model.faces[faces[planar]];
    let s=&a.surface;
    let net=strip::signed_net(s,&b.surface,w)?;
    let n=net.n;
    for axis in 0..2 {
        if n[axis]<3 || !net.covered(axis,w)? {continue;}
        let mut other=BTreeSet::new();
        for wire in std::iter::once(b.outer).chain(b.holes.iter().copied()) {
            for c in &model.loops[wire].coedges {w.charge()?;other.insert(c.edge);}
        }
        let mut edges=Vec::new();
        for row in [0,n[axis]-1] {
            edges.push(strip::boundary_edge(model,a,s,&other,(axis,row),w)??);
        }
        if edges[0]==edges[1] {return None;}
        let enclosure=strip::edge_enclosure(model,&edges,w)?;
        return Some(crate::boundary_hull_contact::Certificate {faces,contact_enclosure:enclosure,
            edges,vertex:None,joined_proof:None,polar_join:None,source_join:None});
    }
    None
}
pub(crate) fn certify(model:&Model,faces:[usize;2],max:usize)->Attempt {
    let mut w=Work{spent:0,max};
    let certificate=prove(model,faces,0,&mut w).or_else(||prove(model,faces,1,&mut w));
    Attempt{certificate,cells:w.spent}
}

mod restricted;
mod strip;
/// Additional source-owned hull restriction; original two-edge proofs retain
/// their old entry point, ordering and work reports.
pub(crate) fn certify_restricted(model:&Model,faces:[usize;2],max:usize)->Attempt {
    let vertices=faces.map(|i| {
        let f=&model.faces[i];std::iter::once(f.outer).chain(f.holes.iter().copied())
            .flat_map(|l|model.loops[l].coedges.iter()).filter(|c|c.trim().is_none())
            .flat_map(|c|model.edges[c.edge].vertices).collect::<BTreeSet<_>>()
    });
    if vertices[0].intersection(&vertices[1]).next().is_none() {return Attempt{certificate:None,cells:0};}
    let mut w=Work{spent:0,max};
    let certificate=restricted::prove(model,faces,0,&mut w).or_else(||restricted::prove(model,faces,1,&mut w));
    Attempt{certificate,cells:w.spent}
}
