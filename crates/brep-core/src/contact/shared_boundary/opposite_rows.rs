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
    let s=&a.surface;let p=&b.surface;
    let n=[s.control_points.len(),s.control_points[0].len()];
    let degree=[s.degree_u,s.degree_v];let knots=[&s.knots_u,&s.knots_v];
    if s.periodic_u || s.periodic_v || n!=[degree[0]+1,degree[1]+1]
        || n.iter().any(|&n|n<2 || n>33) {return None;}
    for axis in 0..2 {for (i,&k) in knots[axis].iter().enumerate() {
        w.charge()?;
        if k!=knots[axis][if i<=degree[axis] {degree[axis]}else{n[axis]}] {return None;}
    }}
    let plane=[p.control_points[0][0].as_slice(),p.control_points[p.control_points.len()-1][0].as_slice(),
        p.control_points[0][p.control_points[0].len()-1].as_slice()];
    let mut noncollinear=false;
    for axes in [[0,1],[0,2],[1,2]] {
        w.charge()?;
        match orient(&plane,Some(axes))? {Sign::Zero=>{},_=>{noncollinear=true;break;}}
    }
    if !noncollinear {return None;}
    for (point,&weight) in p.control_points.iter().flatten().zip(p.weights.iter().flatten()) {
        w.charge()?;if !weight.is_finite() || weight<=0. || w.side(plane,point)?!=Sign::Zero {return None;}
    }
    let mut side=None;let mut signs=vec![vec![Sign::Zero;n[1]];n[0]];
    for u in 0..n[0] {for v in 0..n[1] {
        w.charge()?;if !s.weights[u][v].is_finite() || s.weights[u][v]<=0. {return None;}
        let sign=w.side(plane,&s.control_points[u][v])?;signs[u][v]=sign;
        if sign!=Sign::Zero {
            if side.is_some_and(|old|old!=sign) {return None;}
            side=Some(sign);
        }
    }}
    side?; // Coplanar surfaces require independent region/overlap proofs.
    for axis in 0..2 {
        if n[axis]<3 {continue;}
        let at=|i:usize,j:usize|if axis==0 {signs[i][j]}else{signs[j][i]};
        // Both end rows vanish. Every free-coordinate control column has a
        // strict interior coefficient. Positive rational Bernstein bases make
        // the ENTIRE open strip strictly sided, even at its other boundaries.
        let mut covered=true;
        for j in 0..n[1-axis] {
            w.charge()?;
            covered &= at(0,j)==Sign::Zero && at(n[axis]-1,j)==Sign::Zero
                && (1..n[axis]-1).any(|i|at(i,j)!=Sign::Zero);
        }
        if !covered {continue;}
        let mut other=BTreeSet::new();
        for wire in std::iter::once(b.outer).chain(b.holes.iter().copied()) {
            for c in &model.loops[wire].coedges {w.charge()?;other.insert(c.edge);}
        }
        let mut edges=Vec::new();
        for row in [0,n[axis]-1] {
            let mut found=None;
            'wires:for wire in std::iter::once(a.outer).chain(a.holes.iter().copied()) {
                for c in &model.loops[wire].coedges {
                    w.charge()?;
                    if !other.contains(&c.edge) || c.trim().is_some() {continue;}
                    let e=&model.edges[c.edge];let curve=&e.curve;
                    if curve.control_points.first()?.as_slice()!=model.vertices[e.vertices[0]].point
                        || curve.control_points.last()?.as_slice()!=model.vertices[e.vertices[1]].point {continue;}
                    // Complete rational boundary identity, never endpoint-only
                    // recognition. Failed exact predicate work is charged too.
                    w.charge()?;
                    if boundary(s,&c.pcurve,curve)==Some((axis,row)) {found=Some(c.edge);break 'wires;}
                }
            }
            edges.push(found?);
        }
        if edges[0]==edges[1] {return None;}
        let mut enclosure=[[f64::INFINITY,f64::NEG_INFINITY];3];
        for &e in &edges {for pole in &model.edges[e].curve.control_points {
            w.charge()?;
            for k in 0..3 {enclosure[k][0]=enclosure[k][0].min(pole[k]);enclosure[k][1]=enclosure[k][1].max(pole[k]);}
        }}
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
