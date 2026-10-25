use super::*;

/// A straight shared edge still admits an authored coordinate plane. Strict
/// control-net sidedness excludes every off-boundary point from that plane.
/// Zero intermediate Bezier rows are allowed with a strict opposite row.
pub(super) fn opposite_axis_sides(a:&Surface,b:&Surface,pa:&Curve,pb:&Curve,edge:&Curve)->bool{
    let (Some(ba),Some(bb))=(boundary(a,pa,edge),boundary(b,pb,edge)) else{return false};
    if [a,b].iter().zip([ba,bb]).any(|(s,bound)|[s.degree_u,s.degree_v][bound.0]==0){return false;}
    for axis in 0..3{
        let value=edge.control_points[0][axis];
        if !edge.control_points.iter().all(|p|p[axis]==value){continue;}
        let side=|s:&Surface,bound:(usize,usize)|{
            let mut positive=None;
            for (u,row) in s.control_points.iter().enumerate(){for (v,p) in row.iter().enumerate(){
                if [u,v][bound.0]==bound.1{if p[axis]!=value{return None;}}
                else{if p[axis]==value{continue;}let next=p[axis]>value;if positive.is_some_and(|v|v!=next){return None;}positive=Some(next);}
            }}
            let zero_off=s.control_points.iter().enumerate().any(|(u,row)|row.iter().enumerate().any(|(v,p)|[u,v][bound.0]!=bound.1&&p[axis]==value));
            if zero_off {
                let size=[s.control_points.len(),s.control_points[0].len()][bound.0];
                let degree=[s.degree_u,s.degree_v][bound.0];
                if size!=degree+1{return None;}
                // Positive Bernstein bases expose only complete zero edges or
                // zero corners. Permit a collapsed pole only when it is an
                // endpoint of the exact shared edge, never an extra contact.
                let mut plane=[vec![0.;3],vec![0.;3],vec![0.;3]];
                for point in &mut plane {point[axis]=value;}
                plane[1][(axis+1)%3]=1.; plane[2][(axis+2)%3]=1.;
                if !bezier_plane_edge_only(s,[&plane[0],&plane[1],&plane[2]],bound){return None;}
            }
            positive
        };
        if let (Some(sa),Some(sb))=(side(a,ba),side(b,bb)){if sa!=sb{return true;}}
    }
    false
}
// Exact quarter-disk trim: its interior has 1-u²-v²>0 and its circular
// boundary has equality. This is a UV-domain certificate, independent of
// recognition of any particular analytic solid.
fn quarter_disk_chart(model:&Model,face:&crate::Face)->Option<Surface> {
    if !face.holes.is_empty(){return None;}
    let uses=&model.loops[face.outer].coedges;if uses.len()!=3 || uses.iter().any(|c|c.pcurve.control_points.iter().any(|p|p.len()!=2)){return None;}
    let s=&face.surface;
    if s.degree_u!=2 || s.degree_v!=2 || s.control_points.len()!=3 || s.control_points[0].len()!=3
        || s.knots_u!=[0.,0.,0.,1.,1.,1.] || s.knots_v!=[0.,0.,0.,1.,1.,1.] {return None;}
    let line=|p:&Curve,a:[f64;2],b:[f64;2]|p.degree==1&&!p.periodic&&p.knots==[0.,0.,1.,1.] && p.weights.len()==2 && p.weights.iter().all(|w|w.is_finite()&&*w>0.)
        && (p.control_points==[a.to_vec(),b.to_vec()] || p.control_points==[b.to_vec(),a.to_vec()]);
    let arc=|p:&Curve|p.degree==2&&!p.periodic&&p.knots==[0.,0.,0.,1.,1.,1.] && p.weights.len()==3 && p.weights.iter().all(|w|w.is_finite()&&*w>0.)
        && ((p.control_points==[vec![1.,0.],vec![1.,1.],vec![0.,1.]] && p.weights[0]==p.weights[1] && p.weights[2]==2.*p.weights[0])
            ||(p.control_points==[vec![0.,1.],vec![1.,1.],vec![1.,0.]] && p.weights[2]==p.weights[1] && p.weights[0]==2.*p.weights[2]));
    for gauge in 0_u8..8 {
        let mut curves=uses.iter().map(|c|c.pcurve.clone()).collect::<Vec<_>>();let mut exact=true;
        for c in &mut curves {for p in &mut c.control_points {
            if gauge&4!=0 {p.swap(0,1);}
            for k in 0..2 {if gauge&(1<<k)!=0 {
                // Error-free 1-x, never a snapped UV coefficient.
                let x=p[k];let sum=1.-x;let bv=sum-1.;let av=sum-bv;
                if !sum.is_finite() || (1.-av)+(-x-bv)!=0. {exact=false;break;}
                p[k]=sum;
            }}
        }}
        if !exact || !curves.iter().any(|p|line(p,[0.,0.],[1.,0.])) || !curves.iter().any(|p|line(p,[0.,1.],[0.,0.])) || !curves.iter().any(arc) {continue;}
        // A signed parameter-square bijection, using ONLY original coefficients.
        // No fitting or world-geometry change transfers the canonical disk proof.
        let mut canonical=crate::face_injectivity::canonical_pole_surface(s,usize::from(gauge&4!=0));
        if gauge&1!=0 {canonical.control_points.reverse();canonical.weights.reverse();}
        if gauge&2!=0 {for r in &mut canonical.control_points {r.reverse();}for r in &mut canonical.weights {r.reverse();}}
        return Some(canonical);
    }
    None
}
fn disk_sided(s:&Surface,axis:usize,plane:f64)->Option<bool>{
    if s.degree_u!=2||s.degree_v!=2||s.periodic_u||s.periodic_v
        ||s.knots_u!=[0.,0.,0.,1.,1.,1.]||s.knots_v!=[0.,0.,0.,1.,1.,1.]
        ||s.control_points.len()!=3||s.control_points[0].len()!=3{return None;}
    let pole=s.control_points[0][0][axis];if pole==plane{return None;}
    for i in 0..3{for j in 0..3{
        let factor=(1.-f64::from(i==2)-f64::from(j==2))*s.weights[0][0];
        // This determinant is (z-plane)*weight-(pole-plane)*factor.
        // Subtractions remain inside the exact predicate, not rounded inputs.
        let values=[plane,0.,0.,s.control_points[i][j][axis],factor,0.,pole,s.weights[i][j],1.,plane,0.,1.];
        let source=SourceArena::authored("trimmed-disk-plane-numerator",1,values.into_iter().map(|x|AuthoredScalar::Binary64Bits(x.to_bits())).collect()).ok()?;
        let tolerance=ToleranceContext::default_valid();let mut ctx=PredicateContext::new(&source,&tolerance,Limits::default(),None);
        let point=|n|std::array::from_fn(|k|source.leaf(n+k).unwrap());
        if cad_predicates::orient3d(&mut ctx,point(0),point(3),point(6),point(9)).ok()?.outcome!=Outcome::Sign(Sign::Zero){return None;}
    }}
    Some(pole>plane)
}
fn opposite_disk_sides(model:&Model,faces:[usize;2],ca:&crate::Coedge,cb:&crate::Coedge)->crate::Result<bool>{
    let a=&model.faces[faces[0]];let b=&model.faces[faces[1]];
    let (Some(qa),Some(qb))=(quarter_disk_chart(model,a),quarter_disk_chart(model,b)) else{return Ok(false);};
    let edge=&model.edges[ca.edge].curve;
    for axis in 0..3{
        let plane=edge.control_points[0][axis];
        if !edge.control_points.iter().all(|p|p[axis]==plane){continue;}
        if let (Some(sa),Some(sb))=(disk_sided(&qa,axis,plane),disk_sided(&qb,axis,plane)){
            if sa==sb{continue;}
            let equal=|face:&crate::Face,c:&crate::Coedge|nurbs_core::curve_surface_agreement::verify_exact(edge,&c.pcurve,&face.surface,c.reversed,32768)
                .map(|d|d.is_some_and(|d|d.outcome==cad_predicates::BezierIdentity::Equal));
            if equal(a,ca)?&&equal(b,cb)?{return Ok(true);}
        }
    }
    Ok(false)
}
pub(crate) fn certify_opposite(
    model: &Model,
    faces: [usize; 2],
) -> crate::Result<Option<OppositeSidesCertificate>> {
    let a = &model.faces[faces[0]];
    let b = &model.faces[faces[1]];
    let uses = |f: &crate::Face| {
        std::iter::once(f.outer)
            .chain(f.holes.iter().copied())
            .flat_map(|l| model.loops[l].coedges.iter())
            .collect::<Vec<_>>()
    };
    let mut by_edge = std::collections::BTreeMap::<usize, Vec<_>>::new();
    for c in uses(b) {
        by_edge.entry(c.edge).or_default().push(c);
    }
    for ca in uses(a) {
        if let Some(others) = by_edge.get(&ca.edge) {
            for cb in others {
                if opposite_axis_sides(&a.surface,&b.surface,&ca.pcurve,&cb.pcurve,&model.edges[ca.edge].curve) || opposite_disk_sides(model,faces,ca,cb)? || monotone_coordinate::separates(
                    &a.surface, &b.surface, &ca.pcurve, &cb.pcurve,
                    &model.edges[ca.edge].curve,
                ) || separates_surfaces(
                    &a.surface,
                    &b.surface,
                    &ca.pcurve,
                    &cb.pcurve,
                    &model.edges[ca.edge].curve,
                )? {
                    return Ok(Some(OppositeSidesCertificate {
                        edge: ca.edge,
                        faces,
                    }));
                }
            }
        }
    }
    Ok(None)
}

/// Sufficient certificate for two curved surfaces on opposite sides of the
/// plane of a shared noncollinear Bezier edge. All inputs are original source
/// representations; no rounded normal or fitted plane enters the proof.
pub fn separates_surfaces(
    a: &Surface,
    b: &Surface,
    pcurve_a: &Curve,
    pcurve_b: &Curve,
    edge: &Curve,
) -> crate::Result<bool> {
    a.validate()?;
    b.validate()?;
    pcurve_a.validate()?;
    pcurve_b.validate()?;
    edge.validate()?;
    for s in [a, b] {
        if s.periodic_u
            || s.periodic_v
            || s.control_points
                .len()
                .saturating_mul(s.control_points[0].len())
                > 4096
        {
            return Ok(false);
        }
    }
    let (Some(ba), Some(bb)) = (boundary(a, pcurve_a, edge), boundary(b, pcurve_b, edge)) else {
        return Ok(false);
    };
    let points: Vec<_> = edge.control_points.iter().map(|p| p.as_slice()).collect();
    let p0 = points[0];
    let Some(p1) = points.iter().copied().find(|p| *p != p0) else {
        return Ok(false);
    };
    // Candidate planes for a straight edge still require exact edge incidence
    // and opposite strict signs for every off-boundary control point below.
    let mut candidates=vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![0.,1.,0.],vec![0.,0.,1.]];
    let controls=[a,b].map(|s|s.control_points.iter().flatten().collect::<Vec<_>>());
    if controls.iter().all(|ps|ps.len()<=16) {
        for x in &controls[0] { for y in &controls[1] {
            let p=(0..3).map(|k|0.5*x[k]+0.5*y[k]).collect::<Vec<_>>();
            if p.iter().all(|x|x.is_finite()) {candidates.push(p);}
        }}
    }
    // Rounded midpoints propose planes only; every incidence and strict side
    // below is decided exactly on original retained coefficients.
    for p2 in points.iter().copied().chain(candidates.iter().map(|p|p.as_slice())) {
        if ![[0,1],[0,2],[1,2]].iter().any(|&axes|matches!(
            orient(&[p0,p1,p2],Some(axes)),Some(Sign::Positive|Sign::Negative))) {continue;}
    if !points
        .iter()
        .all(|p| orient(&[p0, p1, p2, p], None) == Some(Sign::Zero))
    {
        continue;
    }
    let strict_side = |surface: &Surface, bound: (usize, usize)| -> Option<Sign> {
        let mut side = None;
        for (u, row) in surface.control_points.iter().enumerate() {
            for (v, p) in row.iter().enumerate() {
                let sign = orient(&[p0, p1, p2, p], None)?;
                if [u, v][bound.0] == bound.1 {
                    if sign != Sign::Zero {
                        return None;
                    }
                } else {
                    if sign == Sign::Zero {continue;}
                    if side.is_some_and(|s| s != sign) {return None;}
                    side = Some(sign);
                }
            }
        }
        let size=[surface.control_points.len(),surface.control_points[0].len()][bound.0];
        let opposite=if bound.1==0{size-1}else{0};
        let mut zeros=false;
        for (u,row) in surface.control_points.iter().enumerate(){for (v,p) in row.iter().enumerate(){
            if [u,v][bound.0]==bound.1{continue;}
            if orient(&[p0,p1,p2,p],None)?==Sign::Zero{zeros=true;if [u,v][bound.0]==opposite{return None;}}
        }}
        if zeros&&size!=[surface.degree_u,surface.degree_v][bound.0]+1{return None;}
        // With positive weights, the opposite endpoint Bernstein row has
        // positive basis everywhere off this boundary, including free ends.
        // Zero intermediate rows cannot add a second contact with the plane.
        side
    };
    if matches!(
        (strict_side(a, ba), strict_side(b, bb)),
        (Some(Sign::Positive), Some(Sign::Negative)) | (Some(Sign::Negative), Some(Sign::Positive))
    ) { return Ok(true); }
    }
    Ok(false)
}
