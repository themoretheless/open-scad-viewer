//! Exact hull contact restricted to common authored topology.
//! Axis bounds and exact oblique supporting planes provide sufficient proofs.
//! Only used after exact edge/lift agreement, simple trims and chart injectivity.
use crate::Model;
use std::collections::BTreeSet;
#[derive(Clone, Debug)]
pub struct Certificate {
    pub faces: [usize;2],
    /// Encloses possible surface contact after all proof restrictions.
    /// Bezier support restrictions can be tighter than control-hull overlap.
    pub contact_enclosure: [[f64;2];3],
    pub edges: Vec<usize>,
    pub vertex: Option<usize>,
    pub joined_proof:Option<JoinedProof>,
}
#[derive(Clone,Debug)]
pub struct JoinedProof {
    pub blend_face:usize,
    pub wall_face:usize,
    pub collapsed_end:usize,
    pub projection:[[f64;3];2],
    pub report:nurbs_core::surface_quotient_injectivity::JoinedReport,
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
// With exact boundary agreement, a simple positively wound planar outer
// loop bounds the trimmed face inside the convex hull of its boundary curves.
// Positive rational curve weights enclose each curve by its original controls.
// This avoids using the much larger untrimmed plane chart as a face enclosure.
fn contact_controls(model:&Model,face:usize)->Vec<&[f64]>{
    let f=&model.faces[face];
    let surface=f.surface.control_points.iter().flatten().map(|p|p.as_slice()).collect::<Vec<_>>();
    for axis in 0..3 {
        let value=surface[0][axis];
        if !surface.iter().all(|p|p[axis]==value){continue;}
        let outer=model.loops[f.outer].coedges.iter().flat_map(|c|model.edges[c.edge].curve.control_points.iter())
            .map(|p|p.as_slice()).collect::<Vec<_>>();
        if !outer.is_empty()&&outer.iter().all(|p|p[axis]==value){return outer;}
    }
    surface
}
/// Input must be structurally valid. Does not establish its prerequisites.
pub(crate) fn certify(model:&Model, faces:[usize;2])->Option<Certificate>{
    certify_axis(model,faces).or_else(||certify_edge_plane(model,faces)).or_else(||certify_vertex_plane(model,faces)).or_else(||certify_joined_charts(model,faces))
}
fn certify_axis(model:&Model, faces:[usize;2])->Option<Certificate>{
    let edges=faces.map(|f|std::iter::once(model.faces[f].outer).chain(model.faces[f].holes.iter().copied())
        .flat_map(|l|model.loops[l].coedges.iter().map(|c|c.edge)).collect::<BTreeSet<_>>());
    let mut hulls=faces.map(|f|{
        let mut h=[[f64::INFINITY,f64::NEG_INFINITY];3];
        for p in contact_controls(model,f){for k in 0..3{h[k][0]=h[k][0].min(p[k]);h[k][1]=h[k][1].max(p[k]);}}
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
            for p in contact_controls(model,f){
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
                        return Some(Certificate{faces,contact_enclosure:point.map(|x|[x,x]),edges:Vec::new(),vertex:Some(vertex),joined_proof:None});
                    }
                }}
            }
        }
    }
    if !free.is_empty() && free.len()<=2 {
        let projections=if free.len()==2 {vec![[free[0],free[1]]]} else {
            (0..3).filter(|&k|k!=free[0]).map(|k|[free[0],k]).collect()
        };
        for axes in projections {
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
            let mut candidates=vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![0.,1.,0.],vec![0.,0.,1.]];
            // Coordinate support already restricts the contact to this plane.
            // A rotated corner need not be separated by a world-axis proposal;
            // use bounded candidate generation, then verify all retained
            // source poles with the exact projected orientation predicate.
            if poles.iter().all(|ps|ps.len()<=16) {
                let mut projected=[0.;2];
                let mut directions=Vec::new();
                for (face,ps) in poles.iter().enumerate() {for p in ps {
                    let mut d=[p[axes[0]]-point[axes[0]],p[axes[1]]-point[axes[1]]];
                    let length=d[0].hypot(d[1]);
                    if length>0. && length.is_finite() {
                        for x in &mut d {*x/=length;if face==1 {*x = -*x;}}
                        directions.push(d);
                    }
                }}
                for _ in 0..256 {for d in &directions {
                    let dot=projected[0]*d[0]+projected[1]*d[1];
                    if dot<1. {for k in 0..2 {projected[k]+=(1.-dot)*d[k];}}
                }}
                let length=projected[0].hypot(projected[1]);
                if length>0. && length.is_finite() {
                    let mut anchor=point.to_vec();
                    anchor[axes[0]]-=projected[1]/length;
                    anchor[axes[1]]+=projected[0]/length;
                    candidates.push(anchor);
                }
            }
            for candidate in candidates {
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
                        return Some(Certificate{faces,contact_enclosure:point.map(|x|[x,x]),edges:Vec::new(),vertex:Some(vertex),joined_proof:None});
                    }
                }
            }
        }
        }
    }
    if free.len()>1{return None;}
    let mut out=Certificate{faces,contact_enclosure:intersection,edges:Vec::new(),vertex:None,joined_proof:None};
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

// The normalized chart union is globally injective under the projected
// weighted bounds. Exact shared natural-boundary lifts bind that union to
// the authored edge, including its owned collapsed endpoint.
fn certify_joined_charts(model:&Model,faces:[usize;2])->Option<Certificate>{
    let uses=faces.map(|f|std::iter::once(model.faces[f].outer).chain(model.faces[f].holes.iter().copied())
        .flat_map(|l|model.loops[l].coedges.iter()).collect::<Vec<_>>());
    for (blend_face,wall_face,blend_uses,wall_uses) in [
        (faces[0],faces[1],&uses[0],&uses[1]),(faces[1],faces[0],&uses[1],&uses[0])] {
        let blend=&model.faces[blend_face].surface;let wall=&model.faces[wall_face].surface;
        if wall.degree_v!=1||wall.degree_u!=blend.degree_u{continue;}
        for end in 0..2 {
            let row=if end==0{0}else{blend.control_points.len()-1};
            if !blend.control_points[row].iter().all(|p|p==&blend.control_points[row][0]){continue;}
            for ca in blend_uses.iter(){for cb in wall_uses.iter(){
                if ca.edge!=cb.edge{continue;}
                let edge=&model.edges[ca.edge].curve;
                if crate::shared_boundary::boundary(blend,&ca.pcurve,edge)!=Some((1,blend.control_points[0].len()-1))
                    ||crate::shared_boundary::boundary(wall,&cb.pcurve,edge)!=Some((1,1)){continue;}
                // Try a bounded family of coordinate projections. Each admission still
                // requires the full original-chart ruled-join certificate below.
                let mut projections=vec![[[0.,1.,0.],[1.,0.,-1.]],[[1.,0.,0.],[0.,1.,-1.]]];
                for axis in 0..3 {for first_sign in [-1.,1.] {for a in [-1.,1.] {for b in [-1.,1.] {
                    let mut first=[0.;3];first[axis]=first_sign;
                    let other=(0..3).filter(|k|*k!=axis).collect::<Vec<_>>();
                    let mut second=[0.;3];second[other[0]]=a;second[other[1]]=b;
                    let projection=[first,second];
                    if !projections.contains(&projection){projections.push(projection);}
                }}}}
                for projection in projections {
                    // Filter exact structural requirements before expensive bounds.
                    if (0..3).any(|k| projection[0][k]!=0. && wall.control_points.iter().any(|r|r[0][k]!=r[1][k])) {continue;}
                    let pole=if end==0 {0} else {wall.control_points.len()-1};
                    let neighbor=if end==0 {1} else {pole-1};
                    if (0..2).any(|j|(0..3).any(|k|projection[1][k]!=0. && wall.control_points[pole][j][k]!=wall.control_points[neighbor][j][k])) {continue;}

                    let report=nurbs_core::surface_quotient_injectivity::certify_ruled_join(blend,wall,end,projection,16,512).ok()?;
                    if !report.proven{continue;}
                    let mut enclosure=[[f64::INFINITY,f64::NEG_INFINITY];3];
                    for p in &edge.control_points {for k in 0..3{enclosure[k][0]=enclosure[k][0].min(p[k]);enclosure[k][1]=enclosure[k][1].max(p[k]);}}
                    return Some(Certificate{faces,contact_enclosure:enclosure,edges:vec![ca.edge],vertex:None,
                        joined_proof:Some(JoinedProof{blend_face,wall_face,collapsed_end:end,projection,report})});
                }
            }}
        }
    }
    None
}

// A straight owned edge covers its endpoint segment. A separating plane
// can restrict the intersection of the enclosing hulls to this segment even
// when its world-axis bounding box has two or three nonzero dimensions.
fn certify_edge_plane(model:&Model,faces:[usize;2])->Option<Certificate>{
    use cad_predicates::Sign;
    let nets=faces.map(|f|contact_controls(model,f));
    if nets.iter().any(|n|n.len()>64){return None;}
    let edges=faces.map(|f|std::iter::once(model.faces[f].outer).chain(model.faces[f].holes.iter().copied())
        .flat_map(|l|model.loops[l].coedges.iter().map(|c|c.edge)).collect::<BTreeSet<_>>());
    for &edge in edges[0].intersection(&edges[1]) {
        let curve=&model.edges[edge].curve;
        if curve.degree!=1||curve.control_points.len()!=2{continue;}
        let Some([a,b])=edge_ends(model,edge) else{continue};
        if a==b||[a,b].iter().enumerate().any(|(i,p)|*p!=model.vertices[model.edges[edge].vertices[i]].point){continue;}
        let on_segment=|p:&[f64]|(0..3).all(|k|p[k]>=a[k].min(b[k])&&p[k]<=a[k].max(b[k]))
            && [[0,1],[0,2],[1,2]].into_iter().all(|axes|
                crate::shared_boundary::orient(&[&a,&b,p],Some(axes))==Some(Sign::Zero));
        // Coplanar trim hulls need a transverse candidate, since their own
        // controls only generate their common support plane. Arbitrary finite
        // candidate points are safe: exact signs on every original enclosure
        // control, rather than candidate generation, establish separation.
        let mut candidates=nets.iter().flatten().take(128).map(|p|[p[0],p[1],p[2]]).collect::<Vec<_>>();
        for axis in 0..3 {let mut p=a;p[axis]+=1.;if p.iter().all(|v|v.is_finite())&&p!=a{candidates.push(p);}}
        for candidate in &candidates {
            let mut signs=[None,None];let mut segment_only=[true,true];let mut valid=true;
            for side in 0..2 {for &p in &nets[side] {
                let Some(sign)=crate::shared_boundary::orient(&[&a,&b,candidate,p],None) else{valid=false;break};
                if sign==Sign::Zero {segment_only[side]&=on_segment(p);}
                else if signs[side].is_some_and(|previous|previous!=sign){valid=false;break;}
                else{signs[side]=Some(sign);}
            } if !valid{break;} }
            if valid&&(segment_only[0]||segment_only[1])&&signs.iter().any(Option::is_some)
                && !(signs[0].is_some()&&signs[0]==signs[1]) {
                return Some(Certificate{faces,contact_enclosure:std::array::from_fn(|k|[a[k].min(b[k]),a[k].max(b[k])]),
                    edges:vec![edge],vertex:None,joined_proof:None});
            }
        }
    }
    None
}

// In a single clamped tensor Bezier chart every interior basis value is
// positive. A nonplanar one-sided net can reach its support plane only on
// complete zero-sign natural edges or zero-sign corners, not at an isolated
// intermediate control. Enumerate those strata before allowing a vertex.
fn bezier_plane_vertex_only(surface:&nurbs_core::surface::Surface,plane:[&[f64];3],point:&[f64])->bool {
    use cad_predicates::Sign;
    let sizes=[surface.control_points.len(),surface.control_points[0].len()];
    let degrees=[surface.degree_u,surface.degree_v];let knots=[&surface.knots_u,&surface.knots_v];
    if (0..2).any(|k|degrees[k]==0||sizes[k]!=degrees[k]+1
        ||knots[k][..=degrees[k]].iter().any(|v|*v!=knots[k][degrees[k]])
        ||knots[k][sizes[k]..].iter().any(|v|*v!=knots[k][sizes[k]])){return false;}
    let mut sign=None;let mut zeros=vec![vec![false;sizes[1]];sizes[0]];
    for (u,row) in surface.control_points.iter().enumerate(){for (v,p) in row.iter().enumerate(){
        let Some(s)=crate::shared_boundary::orient(&[plane[0],plane[1],plane[2],p],None) else{return false};
        if s==Sign::Zero{zeros[u][v]=true;}else{
            if sign.is_some_and(|previous|previous!=s){return false;}sign=Some(s);
        }
    }}
    if sign.is_none(){return false;}
    for u in [0,sizes[0]-1]{for v in [0,sizes[1]-1]{
        if zeros[u][v]&&surface.control_points[u][v]!=point{return false;}
    }}
    for u in [0,sizes[0]-1]{if zeros[u].iter().all(|z|*z)
        && surface.control_points[u].iter().any(|p|p!=point){return false;}}
    for v in [0,sizes[1]-1]{if (0..sizes[0]).all(|u|zeros[u][v])
        && (0..sizes[0]).any(|u|surface.control_points[u][v]!=point){return false;}}
    true
}

// A supporting plane need not align with world axes. Original binary64
// controls and an exact orientation predicate establish its two half spaces.
// If one net reaches the plane only at the shared vertex, every possible
// surface contact is that vertex. No tolerance or sampled normal is used.
fn certify_vertex_plane(model:&Model, faces:[usize;2])->Option<Certificate>{
    use cad_predicates::Sign;
    let nets=faces.map(|f|contact_controls(model,f));
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
            if valid {for side in 0..2 {
                if !vertex_only[side]{vertex_only[side]=bezier_plane_vertex_only(
                    &model.faces[faces[side]].surface,[&point,candidates[a],candidates[b]],&point);}
            }}
            if valid && (vertex_only[0]||vertex_only[1]) && signs.iter().any(Option::is_some)
                && !(signs[0].is_some()&&signs[0]==signs[1]) {
                return Some(Certificate{faces,contact_enclosure:point.map(|v|[v,v]),edges:Vec::new(),vertex:Some(vertex),joined_proof:None});
            }
        }}
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
        let axis=(0..3).find(|&k|c.contact_enclosure[k][0]<c.contact_enclosure[k][1]).unwrap();
        for e in c.edges{for p in &mut m.edges[e].curve.control_points{p[(axis+1)%3]+=1e-12;}}
        assert!(certify(&m,c.faces).is_none());
    }

    #[test]
    fn joined_projection_requires_source_seam_budget_order_and_dominance() {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        for (a,b,end,projection) in [(0,2,0,[[0.,1.,0.],[1.,0.,-1.]]),(10,12,1,[[1.,0.,0.],[0.,1.,-1.]])] {
            let blend=&model.faces[a].surface;let wall=&model.faces[b].surface;
            let run=|wall:&nurbs_core::surface::Surface,budget|nurbs_core::surface_quotient_injectivity::certify_ruled_join(blend,wall,end,projection,16,budget).unwrap();
            let r=run(wall,512);assert!(r.proven);assert_eq!(r.cells,512);assert!(r.dominance_margin_lower.unwrap()>0.);
            assert!(!run(wall,511).proven);
            let mut weak=projection;weak[1][if a==0{0}else{1}]=0.25;
            assert!(nurbs_core::surface_quotient_injectivity::certify(blend,end,weak,16,256).unwrap().proven);
            assert!(!nurbs_core::surface_quotient_injectivity::certify_ruled_join(blend,wall,end,weak,16,512).unwrap().proven);
            let mut moved=wall.clone();moved.control_points[0][1][0]+=1e-12;
            assert!(!run(&moved,512).proven);
            let mut weight=wall.clone();weight.weights[0][0]*=2.;
            assert!(!run(&weight,512).proven);
            let mut order=wall.clone();let i=if end==0{1}else{order.control_points.len()-2};
            order.control_points[i][0][2]+=1e-12;
            assert!(!run(&order,512).proven);
            let mut reversed=wall.clone();for row in &mut reversed.control_points{row[0][2]=12.;}
            assert!(!run(&reversed,512).proven);
            let c=certify_joined_charts(&model,[a,b]).unwrap();
            assert!(c.joined_proof.as_ref().unwrap().report.proven);
            assert!(c.edges.contains(&if a==0{2}else{27}));
        }
    }
    #[test]
    fn joined_projection_is_covariant_for_coordinate_rotation_and_translation() {
        let source=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let model=crate::transform::affine(&source,[[0.,-1.,0.,123.],[0.,0.,-1.,-45.],[1.,0.,0.,67.],[0.,0.,0.,1.]]).unwrap();
        for faces in [[0,2],[10,12]] {
            let certificate=certify_joined_charts(&model,faces).unwrap();
            let proof=certificate.joined_proof.unwrap();
            assert!(proof.report.proven && proof.report.cells==512);
            let mut broken=model.clone();
            broken.faces[proof.wall_face].surface.control_points[0][1][0]+=1e-12;
            assert!(certify_joined_charts(&broken,faces).is_none());
        }
    }
    #[test]
    fn strict_bezier_basis_excludes_intermediate_support_controls_from_vertex_contact() {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let plane:[&[f64];3]=[&[0.,0.,6.],&[1.,0.,6.],&[0.,1.,6.]];
        let point=&[20.,0.,6.];let surface=&model.faces[2].surface;
        assert_ne!(surface.control_points[1][1],point);
        assert_eq!(surface.control_points[1][1][2],6.);
        assert!(bezier_plane_vertex_only(surface,plane,point));
        let mut extra=surface.clone();let n=extra.control_points.len();extra.control_points[n-1][1][2]=6.;
        assert!(!bezier_plane_vertex_only(&extra,plane,point));
        assert!(certify_vertex_plane(&model,[1,2]).is_some());
    }
    #[test]
    fn oblique_straight_edge_support_requires_exact_segment_ownership() {
        let mut model=crate::cuboid([0.;3],[2.;3]).unwrap();
        let map=|p:&mut [f64]|{let [x,y,z]=[p[0],p[1],p[2]];p[0]=x+y;p[1]=y+z;p[2]=z+x;};
        for v in &mut model.vertices{map(&mut v.point);}
        for e in &mut model.edges{for p in &mut e.curve.control_points{map(p);}}
        for f in &mut model.faces{for p in f.surface.control_points.iter_mut().flatten(){map(p);}}
        model.validate().unwrap();
        let certificate=(0..6).flat_map(|a|(a+1..6).map(move|b|[a,b]))
            .filter(|p|certify_axis(&model,*p).is_none()).find_map(|p|certify_edge_plane(&model,p)).unwrap();
        assert_eq!(certificate.edges.len(),1);assert!(certificate.vertex.is_none());
        let mut moved=model.clone();let edge=certificate.edges[0];
        moved.edges[edge].curve.control_points[0][0]+=1e-12;
        assert!(certify_edge_plane(&moved,certificate.faces).is_none());
        let mut overlap=model.clone();overlap.faces[certificate.faces[1]].surface=overlap.faces[certificate.faces[0]].surface.clone();
        assert!(certify_edge_plane(&overlap,certificate.faces).is_none());
    }
    #[test]
    fn oblique_support_is_exact_and_requires_the_owned_vertex() {
        let mut model=crate::analytic::sphere(3.).unwrap();
        let map=|p:&mut [f64]| {let [x,y,z]=[p[0],p[1],p[2]];p[0]=x-y;p[1]=-x+2.*y;p[2]=x-2.*y+z;};
        for v in &mut model.vertices{map(&mut v.point);}
        for e in &mut model.edges{for p in &mut e.curve.control_points{map(p);}}
        for f in &mut model.faces{for p in f.surface.control_points.iter_mut().flatten(){map(p);}}
        model.validate().unwrap();

        // The supporting-plane refinement can now certify this fixture as well.
        if let Some(axis) = certify_axis(&model,[0,2]) {
            assert_eq!(axis.vertex,Some(4));
            assert_eq!(axis.contact_enclosure,model.vertices[4].point.map(|v|[v,v]));
        }
        let c=certify_vertex_plane(&model,[0,2]).unwrap();
        assert_eq!(c.vertex,Some(4));assert!(c.edges.is_empty());
        assert_eq!(c.contact_enclosure,model.vertices[4].point.map(|v|[v,v]));
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
            assert_eq!(c.contact_enclosure,model.vertices[pole].point.map(|x|[x,x]));
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
}
