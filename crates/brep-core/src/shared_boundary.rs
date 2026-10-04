//! Sufficient shared-boundary certificate using original control points.
//! One surface lies in a plane; the other has exactly one boundary control row
//! in that plane and remaining control points on the same side. When zeros
//! occur off that boundary, a complete opposite Bezier row must be strict.
//! Positive rational basis weights then forbid any off-boundary contact.
use crate::Model;
use cad_predicates::{
    AuthoredScalar, Limits, Outcome, PredicateContext, Sign, SourceArena, ToleranceContext,
};
use nurbs_core::{curve::Curve, surface::Surface};
mod monotone_coordinate;
#[derive(Clone, Debug)]
pub struct Certificate {
    pub edge: usize,
    pub planar_face: usize,
    pub sided_face: usize,
}
pub(crate) fn orient(points: &[&[f64]], projection: Option<[usize; 2]>) -> Option<Sign> {
    let values = points
        .iter()
        .flat_map(|p| p.iter().map(|x| AuthoredScalar::Binary64Bits(x.to_bits())))
        .collect();
    let source = SourceArena::authored("shared-boundary-control-net", 1, values).ok()?;
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
    let outcome = if let Some(axes) = projection {
        let p = |i: usize| axes.map(|k| source.leaf(3 * i + k).unwrap());
        cad_predicates::orient2d(&mut ctx, p(0), p(1), p(2))
            .ok()?
            .outcome
    } else {
        let p = |i: usize| [0, 1, 2].map(|k| source.leaf(3 * i + k).unwrap());
        cad_predicates::orient3d(&mut ctx, p(0), p(1), p(2), p(3))
            .ok()?
            .outcome
    };
    match outcome {
        Outcome::Sign(s) => Some(s),
        _ => None,
    }
}
fn clamped(k: &[f64], p: usize, n: usize) -> bool {
    k.len() == n + p + 1
        && k[..=p].iter().all(|x| *x == k[p])
        && k[n..].iter().all(|x| *x == k[n])
        && k[p] < k[n]
}
fn bezier(c: &Curve) -> bool {
    c.control_points.len() == c.degree + 1 && clamped(&c.knots, c.degree, c.control_points.len())
}
fn same_boundary(s: &Surface, axis: usize, row: usize, edge: &Curve, reverse: bool) -> bool {
    let count = if axis == 0 {
        s.control_points[0].len()
    } else {
        s.control_points.len()
    };
    let control = |i: usize| {
        let (u, v) = if axis == 0 { (row, i) } else { (i, row) };
        [
            s.control_points[u][v][0],
            s.control_points[u][v][1],
            s.control_points[u][v][2],
            s.weights[u][v],
        ]
    };
    let other = |i: usize| {
        let k = if reverse {
            edge.control_points.len() - 1 - i
        } else {
            i
        };
        [
            edge.control_points[k][0],
            edge.control_points[k][1],
            edge.control_points[k][2],
            edge.weights[k],
        ]
    };
    if count == edge.control_points.len() && (0..count).all(|i| control(i) == other(i)) {
        return true;
    }
    if count > 33 || edge.control_points.len() > 33 {
        return false;
    }
    let values = (0..count)
        .map(control)
        .chain((0..edge.control_points.len()).map(other))
        .flatten()
        .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
        .collect();
    let Ok(source) = SourceArena::authored("shared-boundary-bezier-identity", 1, values) else {
        return false;
    };
    let refs = |start: usize, n: usize| {
        (start..start + n)
            .map(|i| std::array::from_fn(|k| source.leaf(4 * i + k).unwrap()))
            .collect::<Vec<_>>()
    };
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
    cad_predicates::rational_bezier_identity(
        &mut ctx,
        &refs(0, count),
        &refs(count, edge.control_points.len()),
    )
    .is_ok_and(|r| r.outcome == cad_predicates::BezierIdentity::Equal)
}

/// Identify a complete natural boundary traversed by a straight pcurve and
/// verify that its rational Bezier image equals the shared authored edge.
pub(crate) fn boundary(s: &Surface, p: &Curve, edge: &Curve) -> Option<(usize, usize)> {
    if p.degree != 1 || !bezier(p) || !bezier(edge) || p.control_points.iter().any(|p| p.len() != 2)
    {
        return None;
    }
    let sizes = [s.control_points.len(), s.control_points[0].len()];
    let degrees = [s.degree_u, s.degree_v];
    let knots = [&s.knots_u, &s.knots_v];
    for axis in 0..2 {
        for end in 0..2 {
            if !clamped(knots[axis], degrees[axis], sizes[axis]) {
                continue;
            }
            let fixed = knots[axis][if end == 0 { degrees[axis] } else { sizes[axis] }];
            let free = 1 - axis;
            let range = [knots[free][degrees[free]], knots[free][sizes[free]]];
            let points = &p.control_points;
            if points[0][axis] != fixed
                || points[1][axis] != fixed
                || !((points[0][free] == range[0] && points[1][free] == range[1])
                    || (points[0][free] == range[1] && points[1][free] == range[0]))
            {
                continue;
            }
            if sizes[free] != degrees[free] + 1 || !clamped(knots[free], degrees[free], sizes[free])
            {
                continue;
            }
            let row = if end == 0 { 0 } else { sizes[axis] - 1 };
            for reverse in [false, true] {
                let matches = same_boundary(s, axis, row, edge, reverse);
                if matches {
                    return Some((axis, row));
                }
            }
        }
    }
    None
}
// Zero controls off the owned boundary need not be reachable surface points.
// Positive single-Bezier bases restrict support-plane contact to complete
// zero natural edges and zero corners. Extra collapsed edges are allowed only
// when all their controls equal an endpoint of the owned boundary curve.
fn bezier_plane_edge_only(s:&Surface,plane:[&[f64];3],boundary:(usize,usize))->bool {
    let sizes=[s.control_points.len(),s.control_points[0].len()];
    let degrees=[s.degree_u,s.degree_v];let knots=[&s.knots_u,&s.knots_v];
    if (0..2).any(|k|degrees[k]==0||sizes[k]!=degrees[k]+1||!clamped(knots[k],degrees[k],sizes[k])){return false;}
    let at=|fixed:usize,free:usize|if boundary.0==0{&s.control_points[fixed][free]}else{&s.control_points[free][fixed]};
    let endpoints=[at(boundary.1,0),at(boundary.1,sizes[1-boundary.0]-1)];
    let zero=|p:&[f64]|orient(&[plane[0],plane[1],plane[2],p],None)==Some(Sign::Zero);
    for u in [0,sizes[0]-1]{for v in [0,sizes[1]-1]{
        if [u,v][boundary.0]!=boundary.1&&zero(&s.control_points[u][v])
            && !endpoints.contains(&&s.control_points[u][v]){return false;}
    }}
    for axis in 0..2 {for fixed in [0,sizes[axis]-1] {
        if axis==boundary.0&&fixed==boundary.1{continue;}
        let controls=(0..sizes[1-axis]).map(|free|if axis==0{&s.control_points[fixed][free]}else{&s.control_points[free][fixed]}).collect::<Vec<_>>();
        if controls.iter().all(|p|zero(p)) && !endpoints.iter().any(|endpoint|controls.iter().all(|p|*p==*endpoint)){return false;}
    }}
    true
}

fn sided(first: &Surface, second: &Surface, boundary: (usize, usize)) -> bool {
    let points: Vec<_> = second
        .control_points
        .iter()
        .flatten()
        .map(|p| p.as_slice())
        .collect();
    let p0 = points[0];
    let Some(p1) = points.iter().copied().find(|p| *p != p0) else {
        return false;
    };
    let Some(p2) = points.iter().copied().find(|p| {
        [[0, 1], [0, 2], [1, 2]].iter().any(|&axes| {
            matches!(
                orient(&[p0, p1, p], Some(axes)),
                Some(Sign::Positive | Sign::Negative)
            )
        })
    }) else {
        return false;
    };
    if !points
        .iter()
        .all(|p| orient(&[p0, p1, p2, p], None) == Some(Sign::Zero))
    {
        return false;
    }
    let mut side = None;
    for (u, row) in first.control_points.iter().enumerate() {
        for (v, p) in row.iter().enumerate() {
            let s = orient(&[p0, p1, p2, p], None);
            if [u, v][boundary.0] == boundary.1 {
                if s != Some(Sign::Zero) {
                    return false;
                }
            } else {
                if s==Some(Sign::Zero){continue;}
                if !matches!(s, Some(Sign::Positive | Sign::Negative)) {return false;}
                if side.is_some() && side != s {
                    return false;
                }
                side = s;
            }
        }
    }
    let size=[first.control_points.len(),first.control_points[0].len()][boundary.0];
    let opposite=if boundary.1==0{size-1}else{0};
    let mut zeros=false;
    for (u,row) in first.control_points.iter().enumerate(){for (v,p) in row.iter().enumerate(){
        if [u,v][boundary.0]==boundary.1{continue;}
        if orient(&[p0,p1,p2,p],None)==Some(Sign::Zero){zeros=true;if [u,v][boundary.0]==opposite{return side.is_some()&&bezier_plane_edge_only(first,[p0,p1,p2],boundary);}}
    }}
    if zeros&&size!=[first.degree_u,first.degree_v][boundary.0]+1{return false;}
    side.is_some()
}
/// Validate the diagnostic input and try the sufficient shared-edge criterion.
/// None means unproven, never an intersection or validity conclusion.
pub fn inspect_pair(model: &Model, faces: [usize; 2]) -> crate::Result<Option<Certificate>> {
    model.validate_boundary_diagnostic_inputs()?;
    if faces[0] == faces[1] || faces.iter().any(|&f| f >= model.faces.len()) {
        return Err(crate::Error::new(
            "BREP_BOUNDARY_INPUT",
            "Choose two distinct existing faces",
        ));
    }
    Ok(certify(model, faces))
}

/// Returns no certificate on insufficient precision, periodic charts, oversized
/// nets, unsupported boundary representations or additional plane contacts.
/// Callers must structurally validate the model first.
pub(crate) fn certify(model: &Model, faces: [usize; 2]) -> Option<Certificate> {
    let a = model.faces.get(faces[0])?;
    let b = model.faces.get(faces[1])?;
    for s in [&a.surface, &b.surface] {
        if s.periodic_u
            || s.periodic_v
            || s.control_points
                .len()
                .checked_mul(s.control_points[0].len())?
                > 4096
        {
            return None;
        }
    }
    let uses = |f: &crate::Face| {
        std::iter::once(f.outer)
            .chain(f.holes.iter().copied())
            .flat_map(|l| model.loops[l].coedges.iter())
            .collect::<Vec<_>>()
    };
    for ca in uses(a) {
        for cb in uses(b) {
            if ca.edge != cb.edge {
                continue;
            }
            let edge = &model.edges[ca.edge].curve;
            for (first,second,first_use,second_use,fi,si) in [
                (&a.surface,&b.surface,ca,cb,faces[0],faces[1]),
                (&b.surface,&a.surface,cb,ca,faces[1],faces[0]),
            ] {
                let Some(bound)=boundary(first,&first_use.pcurve,edge) else{continue};
                if !sided(first,second,bound){continue;}
                // The planar face may have a curved trim rather than a natural
                // rectangle side. Prove its complete lift exactly; proximity
                // or a shared topological index cannot establish this boundary.
                let exact=boundary(second,&second_use.pcurve,edge).is_some()
                    || nurbs_core::curve_surface_agreement::verify_exact(edge,&second_use.pcurve,second,second_use.reversed,32768)
                        .ok().flatten().is_some_and(|d|d.outcome==cad_predicates::BezierIdentity::Equal);
                if exact{return Some(Certificate{edge:ca.edge,sided_face:fi,planar_face:si});}
            }
        }
    }
    None
}
#[derive(Clone, Debug)]
pub struct OppositeSidesCertificate {
    pub edge: usize,
    pub faces: [usize; 2],
}
/// A positive result proves that the two authored face images meet only along
/// the identified shared edge. It does not assert either face is injective.
pub fn inspect_opposite_pair(
    model: &Model,
    faces: [usize; 2],
) -> crate::Result<Option<OppositeSidesCertificate>> {
    model.validate_boundary_diagnostic_inputs()?;
    if faces[0] == faces[1] || faces.iter().any(|&f| f >= model.faces.len()) {
        return Err(crate::Error::new(
            "BREP_BOUNDARY_INPUT",
            "Choose two distinct existing faces",
        ));
    }
    certify_opposite(model, faces)
}
/// A straight shared edge still admits an authored coordinate plane. Strict
/// control-net sidedness excludes every off-boundary point from that plane.
/// Zero intermediate Bezier rows are allowed with a strict opposite row.
fn opposite_axis_sides(a:&Surface,b:&Surface,pa:&Curve,pb:&Curve,edge:&Curve)->bool{
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
                // The opposite endpoint Bernstein basis is strictly positive
                // everywhere away from the declared boundary. Its complete
                // row must lie strictly on the common side, including corners.
                let opposite=if bound.1==0{size-1}else{0};
                if s.control_points.iter().enumerate().any(|(u,row)|row.iter().enumerate().any(|(v,p)|[u,v][bound.0]==opposite&&p[axis]==value)){return None;}
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
fn quarter_disk_trim(model:&Model,face:&crate::Face)->bool{
    if !face.holes.is_empty(){return false;}
    let uses=&model.loops[face.outer].coedges;if uses.len()!=3{return false;}
    let line=|p:&Curve,a:[f64;2],b:[f64;2]|p.degree==1&&!p.periodic&&p.knots==[0.,0.,1.,1.]&&p.weights==[1.,1.]&&p.control_points==[a.to_vec(),b.to_vec()];
    uses.iter().any(|c|line(&c.pcurve,[0.,0.],[1.,0.]))
        &&uses.iter().any(|c|line(&c.pcurve,[0.,1.],[0.,0.]))
        &&uses.iter().any(|c|{let p=&c.pcurve;p.degree==2&&!p.periodic&&p.knots==[0.,0.,0.,1.,1.,1.]&&p.weights==[1.,1.,2.]&&p.control_points==[vec![1.,0.],vec![1.,1.],vec![0.,1.]]})
}
fn disk_sided(s:&Surface,axis:usize,plane:f64)->Option<bool>{
    if s.degree_u!=2||s.degree_v!=2||s.periodic_u||s.periodic_v
        ||s.knots_u!=[0.,0.,0.,1.,1.,1.]||s.knots_v!=[0.,0.,0.,1.,1.,1.]
        ||s.control_points.len()!=3||s.control_points[0].len()!=3||s.weights[0][0]!=1.{return None;}
    let pole=s.control_points[0][0][axis];if pole==plane{return None;}
    for i in 0..3{for j in 0..3{
        let factor=1.-f64::from(i==2)-f64::from(j==2);
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
    if !quarter_disk_trim(model,a)||!quarter_disk_trim(model,b){return Ok(false);}
    let edge=&model.edges[ca.edge].curve;
    for axis in 0..3{
        let plane=edge.control_points[0][axis];
        if !edge.control_points.iter().all(|p|p[axis]==plane){continue;}
        if let (Some(sa),Some(sb))=(disk_sided(&a.surface,axis,plane),disk_sided(&b.surface,axis,plane)){
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sphere_equator_is_the_only_contact_of_opposite_exact_disk_sides(){
        let model=crate::analytic::sphere(3.).unwrap();let before=format!("{model:?}");
        for faces in [[0,4],[1,5],[2,6],[3,7]]{
            assert!(certify_opposite(&model,faces).unwrap().is_some(),"{faces:?}");
        }
        let mut changed=model.clone();changed.faces[4].surface.control_points[1][1][2]+=1e-12;
        assert!(certify_opposite(&changed,[0,4]).unwrap().is_none());
        let mut same_side=model.clone();for row in &mut same_side.faces[4].surface.control_points{for p in row{p[2]=-p[2];}}
        assert!(certify_opposite(&same_side,[0,4]).unwrap().is_none());
        let rounded=crate::analytic::sphere(2.).unwrap();assert!(certify_opposite(&rounded,[0,4]).unwrap().is_none());
        assert_eq!(format!("{model:?}"),before);
    }
    #[test]
    fn adjacent_cylinder_sides_use_exact_authored_axis_planes(){
        let model=crate::analytic::cylinder(2.,4.).unwrap();
        for faces in [[0,1],[1,2],[2,3],[0,3]]{
            assert!(inspect_opposite_pair(&model,faces).unwrap().is_some(),"{faces:?}");
        }
        let a=&model.faces[0].surface;let b=&model.faces[1].surface;
        let edge=model.loops[model.faces[0].outer].coedges.iter().find(|ca|model.loops[model.faces[1].outer].coedges.iter().any(|cb|cb.edge==ca.edge)).unwrap();
        let other=model.loops[model.faces[1].outer].coedges.iter().find(|cb|cb.edge==edge.edge).unwrap();
        let mut same_side=b.clone();for row in &mut same_side.control_points{for p in row{p[0]=p[0].abs();}}
        assert!(!opposite_axis_sides(a,&same_side,&edge.pcurve,&other.pcurve,&model.edges[edge.edge].curve));
    }
    #[test]
    fn collapsed_secondary_boundary_must_be_an_owned_curve_endpoint() {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        for pair in [[0,1],[10,11]] {
            let c=certify(&model,pair).unwrap();assert_eq!(c.sided_face,pair[0]);
            let mut extra=model.clone();
            // Moving an opposite zero corner within the support plane creates
            // an extra reachable contact rather than an allowed collapsed end.
            let controls=&mut extra.faces[pair[0]].surface.control_points;
            let pole=[0,controls.len()-1].into_iter().find(|&i|controls[i].iter().all(|p|p==&controls[i][0])).unwrap();
            controls[pole][2][0]+=1e-12;
            assert!(!bezier_plane_edge_only(&extra.faces[pair[0]].surface,[&[0.,0.,6.],&[1.,0.,6.],&[0.,1.,6.]],(1,0)),"direct helper pair={pair:?}");
            assert!(certify(&extra,pair).is_none(),"pair={pair:?} certificate={:?}",certify(&extra,pair));
        }
    }
    #[test]
    fn tangent_boundary_control_rows_remain_strictly_separated() {
        let model=crate::analytic::cylinder(2.,4.).unwrap();
        let edge=model.loops[model.faces[0].outer].coedges.iter().find(|ca|model.loops[model.faces[1].outer].coedges.iter().any(|cb|cb.edge==ca.edge)).unwrap();
        let other=model.loops[model.faces[1].outer].coedges.iter().find(|cb|cb.edge==edge.edge).unwrap();
        let mut a=model.faces[0].surface.clone();let b=&model.faces[1].surface;
        let (fixed,row)=boundary(&a,&edge.pcurve,&model.edges[edge.edge].curve).unwrap();
        let axis=(0..3).find(|&k|model.edges[edge.edge].curve.control_points.iter().all(|p|p[k]==model.edges[edge.edge].curve.control_points[0][k])).unwrap();
        let plane=model.edges[edge.edge].curve.control_points[0][axis];
        for (u,r) in a.control_points.iter_mut().enumerate(){for (v,p) in r.iter_mut().enumerate(){if [u,v][fixed]==1{p[axis]=plane;}}}
        assert!(opposite_axis_sides(&a,b,&edge.pcurve,&other.pcurve,&model.edges[edge.edge].curve));
        let opposite=if row==0{2}else{0};
        if fixed==0{a.control_points[opposite][0][axis]=plane;}else{a.control_points[0][opposite][axis]=plane;}
        assert!(!opposite_axis_sides(&a,b,&edge.pcurve,&other.pcurve,&model.edges[edge.edge].curve));
    }
    #[test]
    fn cylinder_caps_admit_curved_exact_trims_but_not_nearby_lifts(){
        let model=crate::analytic::cylinder(2.,4.).unwrap();
        for side in 0..4{for cap in 4..6{
            let c=inspect_pair(&model,[side,cap]).unwrap().expect("exact curved cap trim");
            assert_eq!(c.sided_face,side);assert_eq!(c.planar_face,cap);
            let mut broken=model.clone();
            let wire=broken.faces[cap].outer;
            let use_=broken.loops[wire].coedges.iter_mut().find(|u|u.edge==c.edge).unwrap();
            use_.pcurve.control_points[1][0]+=1e-12;
            assert!(certify(&broken,[side,cap]).is_none());
        }}
    }
    #[test]
    fn cube_only_certifies_adjacent_pairs() {
        let m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let mut certified = 0;
        for a in 0..6 {
            for b in a + 1..6 {
                if let Some(c) = certify(&m, [a, b]) {
                    certified += 1;
                    assert!(c.edge < m.edges.len());
                    assert_ne!(c.planar_face, c.sided_face);
                }
            }
        }
        assert_eq!(certified, 12);
    }
    #[test]
    fn topology_alone_cannot_hide_a_fold_or_shifted_boundary() {
        let m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let pair = (0..6)
            .flat_map(|a| (a + 1..6).map(move |b| [a, b]))
            .find(|&p| certify(&m, p).is_some())
            .unwrap();
        let mut bad = m.clone();
        for row in &mut bad.faces[pair[0]].surface.control_points {
            for p in row {
                p[0] += 0.01;
                p[1] += 0.01;
                p[2] += 0.01;
            }
        }
        assert!(certify(&bad, pair).is_none());
    }
    #[test]
    fn curved_sided_patch_passes_but_fold_and_extra_plane_controls_do_not() {
        let plane = nurbs_core::surface::Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let mut curved = plane.clone();
        curved.degree_u = 2;
        curved.knots_u = vec![0., 0., 0., 1., 1., 1.];
        curved.weights = vec![vec![1.; 2], vec![1.25; 2], vec![0.75; 2]];
        curved.control_points = (0..3)
            .map(|i| {
                (0..2)
                    .map(|j| vec![i as f64 * 0.5, j as f64, [0., 0.25, 1.][i]])
                    .collect()
            })
            .collect();
        curved.validate().unwrap();
        assert!(sided(&curved, &plane, (0, 0)));
        curved.control_points[1][0][2] = -0.25;
        assert!(!sided(&curved, &plane, (0, 0)));
        curved.control_points[1][0][2] = 0.;
        assert!(sided(&curved, &plane, (0, 0)));
        curved.control_points[2][0][2] = 0.;
        assert!(!sided(&curved, &plane, (0, 0)));
    }
    #[test]
    fn sheared_translated_cube_retains_twelve_shared_boundaries() {
        let mut m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let transform = |p: &mut [f64]| {
            let q = [p[0], p[1], p[2]];
            p[0] = 16. + q[0] + 0.25 * q[1];
            p[1] = -8. + q[1] + 0.5 * q[2];
            p[2] = 4. + q[2] + 0.125 * q[0];
        };
        for v in &mut m.vertices {
            transform(&mut v.point);
        }
        for e in &mut m.edges {
            for p in &mut e.curve.control_points {
                transform(p);
            }
        }
        for f in &mut m.faces {
            for row in &mut f.surface.control_points {
                for p in row {
                    transform(p);
                }
            }
        }
        m.validate().unwrap();
        let mut count = 0;
        for a in 0..6 {
            for b in a + 1..6 {
                count += usize::from(inspect_pair(&m, [a, b]).unwrap().is_some());
            }
        }
        assert_eq!(count, 12);
        assert!(inspect_pair(&m, [0, 0]).is_err());
        assert!(inspect_pair(&m, [0, 6]).is_err());
    }
    #[test]
    fn straight_shared_edge_requires_exact_opposite_strict_sides() {
        let edge=Curve::from_polyline(vec![vec![1.,0.,0.],vec![1.,0.,2.]]).unwrap();
        let uv=Curve::from_polyline(vec![vec![0.,0.],vec![0.,1.]]).unwrap();
        let make=|side:f64|Surface{degree_u:1,degree_v:1,
            knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
            control_points:vec![edge.control_points.clone(),vec![vec![2.,side,0.],vec![2.,side,2.]]],
            weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false};
        let a=make(1.);let b=make(-1.);
        assert!(separates_surfaces(&a,&b,&uv,&uv,&edge).unwrap());
        assert!(!separates_surfaces(&a,&a,&uv,&uv,&edge).unwrap());
        let mut changed=b.clone();changed.control_points[1][1][1]=0.;
        assert!(separates_surfaces(&a,&changed,&uv,&uv,&edge).unwrap());
        // An off-boundary pole on the shared line defeats every strict plane.
        let mut bad=b.clone();bad.control_points[1][1]=edge.control_points[1].clone();
        assert!(!separates_surfaces(&a,&bad,&uv,&uv,&edge).unwrap());
        let mut shifted=b;shifted.control_points[0][0][1]=f64::EPSILON;
        assert!(!separates_surfaces(&a,&shifted,&uv,&uv,&edge).unwrap());
    }
    #[test]
    fn curved_faces_with_rational_curved_shared_edge_are_separated() {
        let edge = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![0.5, 1., 0.], vec![1., 0., 0.]],
            weights: vec![1., 0.75, 1.],
            periodic: false,
        };
        let pcurve = Curve::from_polyline(vec![vec![0., 0.], vec![0., 1.]]).unwrap();
        let make = |side: f64| Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: edge.knots.clone(),
            control_points: vec![
                edge.control_points.clone(),
                vec![
                    vec![0., 0.25, side],
                    vec![0.5, 1.5, side * 0.25],
                    vec![1., 0.25, side * 2.],
                ],
            ],
            weights: vec![edge.weights.clone(), vec![0.75, 1.25, 1.]],
            periodic_u: false,
            periodic_v: false,
        };
        let a = make(1.);
        let b = make(-1.);
        assert!(separates_surfaces(&a, &b, &pcurve, &pcurve, &edge).unwrap());
        assert!(separates_surfaces(&b, &a, &pcurve, &pcurve, &edge).unwrap());
        assert!(!separates_surfaces(&a, &a, &pcurve, &pcurve, &edge).unwrap());
        let mut fold = b.clone();
        fold.control_points[1][1][2] = 0.5;
        assert!(!separates_surfaces(&a, &fold, &pcurve, &pcurve, &edge).unwrap());
        fold.control_points[1][1][2] = 0.;
        assert!(!separates_surfaces(&a, &fold, &pcurve, &pcurve, &edge).unwrap());
        let mut shifted = b.clone();
        shifted.weights[0][1] = 0.5;
        assert!(!separates_surfaces(&a, &shifted, &pcurve, &pcurve, &edge).unwrap());
        let tangent = |mut s:Surface| {
            s.degree_u=2;s.knots_u=vec![0.,0.,0.,1.,1.,1.];
            s.control_points.insert(1,s.control_points[0].clone());
            s.weights.insert(1,s.weights[0].clone());s
        };
        let mut ta=tangent(a.clone());let mut tb=tangent(b.clone());let mut oblique=edge.clone();
        let place=|p:&mut Vec<f64>|{p[2]+=p[0]+2.*p[1]+8.;p[0]+=16.;p[1]-=4.;};
        for s in [&mut ta,&mut tb]{for row in &mut s.control_points{for p in row{place(p);}}}
        for p in &mut oblique.control_points{place(p);}
        assert!(separates_surfaces(&ta,&tb,&pcurve,&pcurve,&oblique).unwrap());
        let mut extra=tb.clone();extra.control_points[2][0]=extra.control_points[0][0].clone();
        assert!(!separates_surfaces(&ta,&extra,&pcurve,&pcurve,&oblique).unwrap());
        let reversed = Curve::from_polyline(vec![vec![0., 1.], vec![0., 0.]]).unwrap();
        assert!(separates_surfaces(&a, &b, &reversed, &pcurve, &edge).unwrap());
    }
    #[test]
    fn brep_pair_identifies_curved_edge_without_claiming_face_validity() {
        let mut m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let faces = [0, 2];
        let old = certify(&m, faces).unwrap();
        let edge = m.edges[old.edge].curve.clone();
        let mut bowed = edge.clone();
        bowed.degree = 2;
        bowed.knots = vec![0., 0., 0., 1., 1., 1.];
        bowed.weights = vec![1.; 3];
        let mut mid: Vec<_> = edge.control_points[0]
            .iter()
            .zip(&edge.control_points[1])
            .map(|(a, b)| (a + b) * 0.5)
            .collect();
        // Bow along the sum of inward face directions. The bisector plane
        // contains the resulting curved edge and separates both face interiors.
        let mut prepared = Vec::new();
        for face in faces {
            let f = &m.faces[face];
            let c = m.loops[f.outer]
                .coedges
                .iter()
                .find(|c| c.edge == old.edge)
                .unwrap();
            let (axis, row) = boundary(&f.surface, &c.pcurve, &edge).unwrap();
            let at = |r: usize| {
                if axis == 0 {
                    f.surface.control_points[r][0].clone()
                } else {
                    f.surface.control_points[0][r].clone()
                }
            };
            let p = at(row);
            let q = at(1 - row);
            for k in 0..3 {
                mid[k] += (q[k] - p[k]) * 0.125;
            }
            prepared.push((face, axis, row));
        }
        bowed.control_points = vec![
            edge.control_points[0].clone(),
            mid,
            edge.control_points[1].clone(),
        ];
        for (face, axis, row) in prepared {
            let s = &mut m.faces[face].surface;
            let original = s.control_points.clone();
            let start = if axis == 0 {
                &original[row][0]
            } else {
                &original[0][row]
            };
            let reverse = *start != edge.control_points[0];
            let sizes = if axis == 0 { [2, 3] } else { [3, 2] };
            s.control_points = (0..sizes[0])
                .map(|u| {
                    (0..sizes[1])
                        .map(|v| {
                            let (r, t) = if axis == 0 { (u, v) } else { (v, u) };
                            if r == row {
                                return bowed.control_points[if reverse { 2 - t } else { t }]
                                    .clone();
                            }
                            let (p, q) = if axis == 0 {
                                (&original[r][0], &original[r][1])
                            } else {
                                (&original[0][r], &original[1][r])
                            };
                            (0..3)
                                .map(|k| p[k] + (q[k] - p[k]) * t as f64 * 0.5)
                                .collect()
                        })
                        .collect()
                })
                .collect();
            s.weights = vec![vec![1.; sizes[1]]; sizes[0]];
            if axis == 0 {
                s.degree_v = 2;
                s.knots_v = bowed.knots.clone();
            } else {
                s.degree_u = 2;
                s.knots_u = bowed.knots.clone();
            }
        }
        m.edges[old.edge].curve = bowed;
        let certificate = inspect_opposite_pair(&m, faces).unwrap().unwrap();
        assert_eq!(certificate.edge, old.edge);
        assert_eq!(certificate.faces, faces);
        let report = crate::face_contacts::inspect(
            &m,
            1e-8,
            crate::face_contacts::Limits {
                pairs: 100,
                cells: 10000,
                domain_cells: 100000,
                cells_per_pair: 16,
                domain_cells_per_pair: 1000,
            },
        )
        .unwrap();
        let pair = report.pairs.iter().find(|p| p.faces == faces).unwrap();
        assert_eq!(pair.reason, "shared-boundary");
        assert!(matches!(
            pair.boundary,
            Some(crate::face_contacts::SharedBoundary::OppositeSides(_))
        ));
        assert!(pair.result.is_none());
        assert!(report.all_pairs_classified);
        assert_eq!(
            report
                .pairs
                .iter()
                .filter(|p| p.reason == "shared-boundary")
                .count(),
            12
        );

        assert!(inspect_pair(&m, faces).unwrap().is_none());
        assert!(inspect_opposite_pair(&m, [0, 0]).is_err());
    }
}
