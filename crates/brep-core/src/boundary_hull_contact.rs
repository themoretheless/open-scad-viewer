//! Exact axis-aligned hull contact restricted to common authored topology.
//! Only used after exact edge/lift agreement, simple trims and chart injectivity.
use crate::Model;
use std::collections::BTreeSet;
#[derive(Clone, Debug)]
pub struct Certificate {
    pub faces: [usize;2],
    pub hull_intersection: [[f64;2];3],
    pub edges: Vec<usize>,
    pub vertex: Option<usize>,
}
fn edge_ends(model: &Model, edge: usize) -> Option<[[f64;3];2]> {
    let c=&model.edges[edge].curve;
    let n=c.control_points.len();
    if c.periodic || c.control_points.iter().any(|p|p.len()!=3) || c.knots[..=c.degree].iter().any(|k|*k!=c.knots[c.degree])
        || c.knots[n..].iter().any(|k|*k!=c.knots[n]) {return None;}
    Some([std::array::from_fn(|k|c.control_points[0][k]),std::array::from_fn(|k|c.control_points[n-1][k])])
}
/// Input must be structurally valid. Does not establish its prerequisites.
pub(crate) fn certify(model:&Model, faces:[usize;2])->Option<Certificate>{
    let edges=faces.map(|f|std::iter::once(model.faces[f].outer).chain(model.faces[f].holes.iter().copied())
        .flat_map(|l|model.loops[l].coedges.iter().map(|c|c.edge)).collect::<BTreeSet<_>>());
    let hulls=faces.map(|f|{
        let mut h=[[f64::INFINITY,f64::NEG_INFINITY];3];
        for p in model.faces[f].surface.control_points.iter().flatten(){for k in 0..3{h[k][0]=h[k][0].min(p[k]);h[k][1]=h[k][1].max(p[k]);}}
        h
    });
    // Extrema are authored binary64 values: comparisons introduce no rounding.
    let intersection=std::array::from_fn::<_,3,_>(|k|[hulls[0][k][0].max(hulls[1][k][0]),hulls[0][k][1].min(hulls[1][k][1])]);
    if intersection.iter().any(|r|r[0]>r[1]){return None;}
    let free=(0..3).filter(|&k|intersection[k][0]<intersection[k][1]).collect::<Vec<_>>();
    if free.len()>1{return None;}
    let mut out=Certificate{faces,hull_intersection:intersection,edges:Vec::new(),vertex:None};
    if free.is_empty(){
        let point=intersection.map(|r|r[0]);
        // Require the same topological vertex and an exact curve endpoint on
        // both faces. Merely nearby vertex coordinates do not authorize contact.
        let vertices=edges.map(|set|set.into_iter().flat_map(|e|{
            let ends=edge_ends(model,e);
            (0..2).filter_map(move |i|ends.filter(|p|p[i]==point).map(|_|model.edges[e].vertices[i]))
        }).collect::<BTreeSet<_>>());
        out.vertex=vertices[0].intersection(&vertices[1]).copied().find(|&v|model.vertices[v].point==point);
        return out.vertex.map(|_|out);
    }
    let axis=free[0];
    let mut segments=Vec::new();
    for &e in edges[0].intersection(&edges[1]){
        let c=&model.edges[e].curve;
        let Some(ends)=edge_ends(model,e) else{continue};
        if c.control_points.iter().any(|p|(0..3).any(|k|k!=axis&&p[k]!=intersection[k][0])){continue;}
        // Continuity covers the whole interval between endpoints. Exact lift
        // agreement places that interval on both authored face boundaries.
        segments.push((ends[0][axis].min(ends[1][axis]),ends[0][axis].max(ends[1][axis]),e));
    }
    segments.sort_by(|a,b|a.0.total_cmp(&b.0));
    let mut covered=intersection[axis][0];
    for (lo,hi,e) in segments{
        if lo>covered{break;}
        if hi>covered{covered=hi;out.edges.push(e);}
        if covered>=intersection[axis][1]{return Some(out);}
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coincident_geometry_without_shared_topology_is_not_authorized() {
        let m=crate::operations::boolean(&crate::cuboid([0.;3],[10.;3]).unwrap(),
            &crate::cuboid([2.;3],[8.;3]).unwrap(),"difference").unwrap();
        let mut examples=[None,None];
        for a in 0..m.faces.len(){for b in a+1..m.faces.len(){
            if let Some(c)=certify(&m,[a,b]){let k=usize::from(c.vertex.is_some());if examples[k].is_none(){examples[k]=Some(c);}}
        }}
        for c in examples.into_iter().map(Option::unwrap){
            let mut broken=m.clone();
            let face=&m.faces[c.faces[1]];
            for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()){
                for i in 0..broken.loops[wire].coedges.len(){
                    let old=broken.loops[wire].coedges[i].edge;
                    let mut edge=broken.edges[old].clone();
                    for v in &mut edge.vertices{let vertex=broken.vertices[*v].clone();*v=broken.vertices.len();broken.vertices.push(vertex);}
                    broken.loops[wire].coedges[i].edge=broken.edges.len();broken.edges.push(edge);
                }
            }
            assert!(certify(&broken,c.faces).is_none());
        }
    }
    #[test]
    fn overlapping_face_interiors_and_microscopic_gaps_are_not_shared_boundaries(){
        let mut m=crate::cuboid([0.;3],[1.;3]).unwrap();
        assert!(certify(&m,[0,0]).is_none());
        let c=(0..6).flat_map(|a|(a+1..6).map(move|b|[a,b])).find_map(|p|certify(&m,p)).unwrap();
        let axis=(0..3).find(|&k|c.hull_intersection[k][0]<c.hull_intersection[k][1]).unwrap();
        for e in c.edges{for p in &mut m.edges[e].curve.control_points{p[(axis+1)%3]+=1e-12;}}
        assert!(certify(&m,c.faces).is_none());
    }
}
