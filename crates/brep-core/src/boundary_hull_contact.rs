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
// Bounded floating-point proposals for a strict supporting plane. The caller
// verifies the resulting binary64 anchors exactly against every original pole.
fn propose_vertex_plane(point:[f64;3],poles:&[Vec<&Vec<f64>>;2])->Option<[Vec<f64>;2]> {
    let mut directions=Vec::new();
    for (face,ps) in poles.iter().enumerate() {for p in ps {
        if p.as_slice()==point {continue;}
        let mut d:[f64;3]=std::array::from_fn(|k|p[k]-point[k]);
        let length=d.iter().map(|x|x*x).sum::<f64>().sqrt();
        if !length.is_finite() || length<=0. {return None;}
        for x in &mut d {*x/=length;if face==1 {*x = -*x;}}
        directions.push(d);
    }}
    let mut normal=[0.;3];
    for _ in 0..256 {
        let mut settled=true;
        for d in &directions {
            let dot=(0..3).map(|k|normal[k]*d[k]).sum::<f64>();
            if dot<1. {settled=false;for k in 0..3 {normal[k]+=(1.-dot)*d[k];}}
        }
        if settled {break;}
    }
    let axis=(0..3).max_by(|&a,&b|normal[a].abs().total_cmp(&normal[b].abs()))?;
    if !normal[axis].is_finite() || normal[axis]==0. {return None;}
    let free=(0..3).filter(|&k|k!=axis).collect::<Vec<_>>();
    let anchors=std::array::from_fn(|i| {
        let mut p=point.to_vec();p[free[i]]+=1.;
        p[axis]-=normal[free[i]]/normal[axis];p
    });
    anchors.iter().flatten().all(|x|x.is_finite()).then_some(anchors)
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
    let mut intersection=std::array::from_fn::<_,3,_>(|k|[hulls[0][k][0].max(hulls[1][k][0]),hulls[0][k][1].min(hulls[1][k][1])]);
    if intersection.iter().any(|r|r[0]>r[1]){return None;}
    // At an exact supporting plane, a positive rational convex combination
    // can use only poles on that plane. Intersecting several such planes
    // restricts support simultaneously, even where AABBs overlap along a line.
    // No tolerance or arithmetic displacement is used to select poles.
    for _ in 0..3 {
        let restricted=std::array::from_fn::<_,2,_>(|i| {
            let f=faces[i];
            let support=(0..3).filter(|&k|intersection[k][0]==intersection[k][1]
                && (hulls[i][k][0]==intersection[k][0]
                    || hulls[i][k][1]==intersection[k][0])).collect::<Vec<_>>();
            let mut h=[[f64::INFINITY,f64::NEG_INFINITY];3];
            for p in model.faces[f].surface.control_points.iter().flatten()
                .filter(|p|support.iter().all(|&k|p[k]==intersection[k][0])) {
                for k in 0..3 { h[k][0]=h[k][0].min(p[k]); h[k][1]=h[k][1].max(p[k]); }
            }
            h
        });
        let next=std::array::from_fn(|k|[
            intersection[k][0].max(restricted[0][k][0]).max(restricted[1][k][0]),
            intersection[k][1].min(restricted[0][k][1]).min(restricted[1][k][1]),
        ]);
        if next.iter().any(|r|r[0]>r[1]) { return None; }
        if next==intersection { break; }
        intersection=next;
    }
    let free=(0..3).filter(|&k|intersection[k][0]<intersection[k][1]).collect::<Vec<_>>();
    if free.len()==3 {
        // A 3D supporting plane may expose a single common topological vertex
        // even when all coordinate hull intervals overlap with positive width.
        let vertices=edges.clone().map(|set|set.into_iter().flat_map(|e|
            model.edges[e].vertices).collect::<BTreeSet<_>>());
        let poles=faces.map(|f|model.faces[f].surface.control_points.iter().flatten().collect::<Vec<_>>());
        if poles.iter().all(|ps|ps.len()<=16) {
            for &vertex in vertices[0].intersection(&vertices[1]) {
                let point=model.vertices[vertex].point;
                if !edges.iter().all(|set|set.iter().any(|&e|edge_ends(model,e)
                    .is_some_and(|ends|(0..2).any(|i|model.edges[e].vertices[i]==vertex&&ends[i]==point)))) { continue; }
                let mut candidates=vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![0.,1.,0.],vec![0.,0.,1.]];
                if let Some(anchors)=propose_vertex_plane(point,&poles) {candidates.extend(anchors);}
                candidates.extend(poles.iter().flat_map(|ps|ps.iter().map(|p|(*p).clone())));
                for a in 0..candidates.len() { for b in a+1..candidates.len() {
                    let sides=poles.each_ref().map(|ps| {
                        let mut side=None;
                        for p in ps {
                            let sign=crate::shared_boundary::orient(&[&point,&candidates[a],&candidates[b],p],None)?;
                            if sign==cad_predicates::Sign::Zero {
                                if p.as_slice()!=point {return None;}
                            } else {
                                if side.is_some_and(|s|s!=sign) {return None;}
                                side=Some(sign);
                            }
                        }
                        side
                    });
                    if matches!(sides,[Some(cad_predicates::Sign::Positive),Some(cad_predicates::Sign::Negative)]
                        |[Some(cad_predicates::Sign::Negative),Some(cad_predicates::Sign::Positive)]) {
                        return Some(Certificate{faces,hull_intersection:point.map(|x|[x,x]),edges:Vec::new(),vertex:Some(vertex)});
                    }
                }}
            }
        }
    }
    if !free.is_empty() && free.len()<=2 {
        let axes=[free[0],if free.len()==2 {free[1]}else{(0..3).find(|&k|k!=free[0]).unwrap()}];
        // Shared station support can leave two transverse coordinates free.
        // An exact projected separator may restrict that contact to one
        // authored vertex even when its coordinate AABBs overlap in area.
        let vertices=edges.clone().map(|set|set.into_iter().flat_map(|e|
            model.edges[e].vertices).collect::<BTreeSet<_>>());
        for &vertex in vertices[0].intersection(&vertices[1]) {
            let point=model.vertices[vertex].point;
            if (0..3).any(|k|point[k]<intersection[k][0]||point[k]>intersection[k][1]) {continue;}
            let poles=faces.map(|f|model.faces[f].surface.control_points.iter().flatten()
                .filter(|p|(0..3).all(|k|intersection[k][0]!=intersection[k][1]
                    || (hulls[usize::from(f==faces[1])][k][0]!=intersection[k][0]
                        && hulls[usize::from(f==faces[1])][k][1]!=intersection[k][0])
                    || p[k]==intersection[k][0])).collect::<Vec<_>>());
            for candidate in [[0.,0.,0.],[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]] {
                let sides=poles.each_ref().map(|ps| {
                    let mut side=None;
                    for p in ps {
                        let sign=crate::shared_boundary::orient(&[&point,&candidate,p],Some(axes))?;
                        if sign==cad_predicates::Sign::Zero {
                            if p.as_slice()!=point {return None;}
                        } else {
                            if side.is_some_and(|s|s!=sign) {return None;}
                            side=Some(sign);
                        }
                    }
                    side
                });
                if matches!(sides,[Some(cad_predicates::Sign::Positive),Some(cad_predicates::Sign::Negative)]
                    |[Some(cad_predicates::Sign::Negative),Some(cad_predicates::Sign::Positive)]) {
                    // Each face must actually own an exact endpoint here.
                    if edges.iter().all(|set|set.iter().any(|&e|edge_ends(model,e)
                        .is_some_and(|ends|(0..2).any(|i|model.edges[e].vertices[i]==vertex&&ends[i]==point)))) {
                        return Some(Certificate{faces,hull_intersection:point.map(|x|[x,x]),edges:Vec::new(),vertex:Some(vertex)});
                    }
                }
            }
        }
    }
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
    fn skewed_station_hulls_meet_only_at_owned_vertex() {
        let section=|z:f64,t:f64|vec![[
            [[0.5,0.],[0.5,0.5],[0.,0.5]],[[0.,0.5],[-0.5,0.5],[-0.5,0.]],
            [[-0.5,0.],[-0.5,-0.5],[0.,-0.5]],[[0.,-0.5],[0.5,-0.5],[0.5,0.]],
        ].into_iter().map(|p|nurbs_core::curve::Curve{degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:p.into_iter().map(|[x,y]|vec![x-t*y,y+t*x,z]).collect(),
            weights:vec![1.,std::f64::consts::FRAC_1_SQRT_2,1.],periodic:false}).collect()];
        let model=crate::rational_section_loft(&[section(0.,0.),section(5.,0.25),section(10.,0.5)]).unwrap();
        let c=certify(&model,[0,7]).expect("skewed common station vertex");
        assert!(c.vertex.is_some());assert!(c.edges.is_empty());
        let mut changed=model.clone();
        changed.vertices[c.vertex.unwrap()].point[0]=changed.vertices[c.vertex.unwrap()].point[0].next_up();
        assert!(certify(&changed,[0,7]).is_none());
    }
    #[test]
    fn retained_hollow_sweep_vertex_contacts_require_authored_vertex_ownership() {
        let section = |z| {
            // Author exact quadrant endpoints rather than rounded trig values.
            let ring = |r:f64| [
                [[r,0.],[r,r],[0.,r]], [[0.,r],[-r,r],[-r,0.]],
                [[-r,0.],[-r,-r],[0.,-r]], [[0.,-r],[r,-r],[r,0.]],
            ].into_iter().map(|p|nurbs_core::curve::Curve {
                degree:2, knots:vec![0.,0.,0.,1.,1.,1.],
                control_points:p.into_iter().map(|xy|vec![xy[0],xy[1],z]).collect(),
                weights:vec![1.,std::f64::consts::FRAC_1_SQRT_2,1.], periodic:false,
            }).collect::<Vec<_>>();
            let outer=ring(0.5);
            let inner=ring(0.2).into_iter().rev().map(|c|c.reverse().unwrap()).collect();
            vec![outer,inner]
        };
        let model=crate::rational_section_loft(&[section(0.),section(5.),section(10.)]).unwrap();
        let before=model.clone();
        let walls=model.faces.len()-2;
        let mut vertices=Vec::new();
        for a in 0..walls { for b in a+1..walls {
            if let Some(c)=certify(&model,[a,b]) {
                if c.vertex.is_some() { vertices.push(c); }
            }
        }}
        assert!(!vertices.is_empty(), "actual diagonal wall panels must exercise vertex-only contact");
        for c in vertices {
            let vertex=c.vertex.unwrap();
            let mut broken=model.clone();
            // Keep all geometry identical but give one face independent edges
            // and vertices. Coordinate coincidence cannot authorize contact.
            let f=&model.faces[c.faces[1]];
            for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
                for i in 0..broken.loops[wire].coedges.len() {
                    let mut edge=model.edges[broken.loops[wire].coedges[i].edge].clone();
                    for v in &mut edge.vertices {
                        let point=model.vertices[*v].clone();
                        *v=broken.vertices.len(); broken.vertices.push(point);
                    }
                    broken.loops[wire].coedges[i].edge=broken.edges.len(); broken.edges.push(edge);
                }
            }
            assert!(certify(&broken,c.faces).is_none(), "{c:?}");
            let mut displaced=model.clone();
            displaced.vertices[vertex].point[0]=displaced.vertices[vertex].point[0].next_up();
            assert!(certify(&displaced,c.faces).is_none(), "one ULP invalidates exact ownership");
        }
        assert_eq!(model,before);
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
