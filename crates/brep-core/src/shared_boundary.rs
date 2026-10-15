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
pub mod bilinear_graph;
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
mod pole_support;
use pole_support::bezier_plane_edge_only;

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
mod tests;
