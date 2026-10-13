use super::*;

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
pub(super) fn clamped(k: &[f64], p: usize, n: usize) -> bool {
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
pub(super) fn bezier_plane_edge_only(s:&Surface,plane:[&[f64];3],boundary:(usize,usize))->bool {
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

pub(super) fn sided(first: &Surface, second: &Surface, boundary: (usize, usize)) -> bool {
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
