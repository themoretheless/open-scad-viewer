//! Exact hull contact restricted to common authored topology.
//! Axis bounds and exact oblique supporting planes provide sufficient proofs.
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
    certify_axis(model,faces).or_else(||certify_vertex_plane(model,faces))
}
fn certify_axis(model:&Model, faces:[usize;2])->Option<Certificate>{
    let edges=faces.map(|f|std::iter::once(model.faces[f].outer).chain(model.faces[f].holes.iter().copied())
        .flat_map(|l|model.loops[l].coedges.iter().map(|c|c.edge)).collect::<BTreeSet<_>>());
    let mut hulls=faces.map(|f|{
        let mut h=[[f64::INFINITY,f64::NEG_INFINITY];3];
        for p in model.faces[f].surface.control_points.iter().flatten(){for k in 0..3{h[k][0]=h[k][0].min(p[k]);h[k][1]=h[k][1].max(p[k]);}}
        h
    });
    // Extrema are authored binary64 values: comparisons introduce no rounding.
    let mut intersection=std::array::from_fn::<_,3,_>(|k|[hulls[0][k][0].max(hulls[1][k][0]),hulls[0][k][1].min(hulls[1][k][1])]);
    if intersection.iter().any(|r|r[0]>r[1]){return None;}
    // A positive rational convex combination can reach a supporting plane
    // only through controls lying on that plane. Simultaneous support-plane
    // constraints can isolate a corner even when the initial hulls overlap
    // along a larger interval. Never apply this to a plane cutting a hull.
    let mut previous=0;
    for _ in 0..3{
        let supports=(0..3).filter(|&k|intersection[k][0]==intersection[k][1]
            && hulls.iter().all(|h|intersection[k][0]==h[k][0]||intersection[k][0]==h[k][1])).collect::<Vec<_>>();
        if supports.len()<=previous{break;}previous=supports.len();
        let restricted=faces.map(|f|{
            let mut h=[[f64::INFINITY,f64::NEG_INFINITY];3];
            for p in model.faces[f].surface.control_points.iter().flatten(){
                if supports.iter().all(|&k|p[k]==intersection[k][0]){
                    for k in 0..3{h[k][0]=h[k][0].min(p[k]);h[k][1]=h[k][1].max(p[k]);}
                }
            }
            h
        });
        intersection=std::array::from_fn(|k|[restricted[0][k][0].max(restricted[1][k][0]),restricted[0][k][1].min(restricted[1][k][1])]);
        if intersection.iter().any(|r|r[0]>r[1]){return None;}
        hulls=restricted;
    }
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

// A supporting plane need not align with world axes. Original binary64
// controls and an exact orientation predicate establish its two half spaces.
// If one net reaches the plane only at the shared vertex, every possible
// surface contact is that vertex. No tolerance or sampled normal is used.
fn certify_vertex_plane(model:&Model, faces:[usize;2])->Option<Certificate>{
    use cad_predicates::Sign;
    let nets=faces.map(|f|model.faces[f].surface.control_points.iter().flatten()
        .map(|p|p.as_slice()).collect::<Vec<_>>());
    if nets.iter().any(|n|n.len()>64){return None;}
    let vertices=faces.map(|f|std::iter::once(model.faces[f].outer).chain(model.faces[f].holes.iter().copied())
        .flat_map(|l|model.loops[l].coedges.iter()).flat_map(|c| {
            let ends=edge_ends(model,c.edge);
            (0..2).filter_map(move |i|ends.filter(|p|p[i]==model.vertices[model.edges[c.edge].vertices[i]].point)
                .map(|_|model.edges[c.edge].vertices[i]))
        }).collect::<BTreeSet<_>>());
    for &vertex in vertices[0].intersection(&vertices[1]) {
        let point=model.vertices[vertex].point;
        let mut candidates=Vec::<&[f64]>::new();
        for p in nets.iter().flatten().copied(){
            if p!=point && !candidates.contains(&p){candidates.push(p);}
            if candidates.len()==16{break;}
        }
        for a in 0..candidates.len(){for b in a+1..candidates.len(){
            let mut signs=[None,None];let mut vertex_only=[true,true];let mut valid=true;
            for side in 0..2 {for &p in &nets[side] {
                let Some(sign)=crate::shared_boundary::orient(&[&point,candidates[a],candidates[b],p],None) else {valid=false;break};
                if sign==Sign::Zero {vertex_only[side]&=p==point;}
                else if signs[side].is_some_and(|previous|previous!=sign){valid=false;break;}
                else {signs[side]=Some(sign);}
            } if !valid{break;} }
            if valid && (vertex_only[0]||vertex_only[1]) && signs.iter().any(Option::is_some)
                && !(signs[0].is_some()&&signs[0]==signs[1]) {
                return Some(Certificate{faces,hull_intersection:point.map(|v|[v,v]),edges:Vec::new(),vertex:Some(vertex)});
            }
        }}
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oblique_support_is_exact_and_requires_the_owned_vertex() {
        let mut model=crate::analytic::sphere(3.).unwrap();
        let map=|p:&mut [f64]| {let [x,y,z]=[p[0],p[1],p[2]];p[0]=x-y;p[1]=-x+2.*y;p[2]=x-2.*y+z;};
        for v in &mut model.vertices{map(&mut v.point);}
        for e in &mut model.edges{for p in &mut e.curve.control_points{map(p);}}
        for f in &mut model.faces{for p in f.surface.control_points.iter_mut().flatten(){map(p);}}
        model.validate().unwrap();

        assert!(certify_axis(&model,[0,2]).is_none());
        let c=certify_vertex_plane(&model,[0,2]).unwrap();
        assert_eq!(c.vertex,Some(4));assert!(c.edges.is_empty());
        assert_eq!(c.hull_intersection,model.vertices[4].point.map(|v|[v,v]));
        let mut overlap=model.clone();overlap.faces[2].surface=overlap.faces[0].surface.clone();
        assert!(certify_vertex_plane(&overlap,[0,2]).is_none());
        let mut moved=model.clone();moved.vertices[4].point[0]+=1e-12;
        assert!(certify_vertex_plane(&moved,[0,2]).is_none());
    }
    #[test]
    fn opposite_sphere_quadrants_meet_only_at_the_authored_pole(){
        let model=crate::analytic::sphere(3.).unwrap();let before=format!("{model:?}");
        for (faces,pole) in [([0,2],4),([1,3],4),([4,6],5),([5,7],5)]{
            let c=certify(&model,faces).unwrap();assert_eq!(c.vertex,Some(pole));assert!(c.edges.is_empty());
            assert_eq!(c.hull_intersection,model.vertices[pole].point.map(|x|[x,x]));
            let mut separate=model.clone();
            for wire in std::iter::once(separate.faces[faces[1]].outer).chain(separate.faces[faces[1]].holes.clone()){
                for i in 0..separate.loops[wire].coedges.len(){
                    let old=separate.loops[wire].coedges[i].edge;let mut edge=separate.edges[old].clone();
                    for v in &mut edge.vertices{let vertex=separate.vertices[*v].clone();*v=separate.vertices.len();separate.vertices.push(vertex);}
                    separate.loops[wire].coedges[i].edge=separate.edges.len();separate.edges.push(edge);
                }
            }
            assert!(certify(&separate,faces).is_none());
        }
        assert_eq!(format!("{model:?}"),before);
    }
    #[test]
    fn adjacent_hemisphere_quadrants_meet_only_at_a_shared_equator_vertex(){
        let model=crate::analytic::sphere(3.).unwrap();
        for upper in 0..4{for quarter in [(upper+1)%4,(upper+3)%4]{
            let c=certify(&model,[upper,quarter+4]).unwrap();
            assert_eq!(c.vertex,Some(if quarter==(upper+1)%4{quarter}else{upper}));
            assert!(c.edges.is_empty());
        }}
    }
    #[test]
    fn a_plane_cutting_a_control_hull_cannot_discard_its_interior_controls(){
        let mut model=crate::analytic::sphere(3.).unwrap();
        for row in &mut model.faces[2].surface.control_points{for p in row{p[0]=0.;}}
        // Face 0 straddles X=0; equality there is not a supporting-plane
        // constraint. Filtering only its X=0 controls would hide intersections.
        model.faces[0].surface.control_points[1][0][0]=-3.;
        assert!(certify(&model,[0,2]).is_none());
    }
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
