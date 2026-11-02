use super::*;
use poly::{add, mul, scale, Poly};
#[derive(Clone)]
pub(super) struct ArcData {
    pub curve: crate::Curve,
    pub breaks: Vec<f64>,
    pub uv: Vec<[Poly; 3]>,
    pub lifted: [Vec<[Poly; 4]>; 2],
}
fn pieces(c: &crate::Curve) -> Result<(Vec<f64>, Vec<Vec<sb::H>>)> {
    c.validate()?;
    if c.periodic || c.control_points[0].len() != 2 || c.degree > 8 || c.control_points.len() > 1024
    {
        return Err(fail(
            "ordinary Model arc proposal shape/degree budget exceeded",
        ));
    }
    let mut breaks = vec![];
    let mut pieces = vec![];
    for k in c.degree..c.control_points.len() {
        if c.knots[k] < c.knots[k + 1] {
            if breaks.is_empty() {
                breaks.push(c.knots[k]);
            }
            breaks.push(c.knots[k + 1]);
            let h = sb::curve(
                c,
                &Q::from_float(c.knots[k]).unwrap(),
                &Q::from_float(c.knots[k + 1]).unwrap(),
            )?;
            if h.iter().any(|p| p[3] <= Q::zero()) {
                return Err(fail(
                    "ordinary Model arc requires positive rational weights",
                ));
            }
            pieces.push(h);
        }
    }
    Ok((breaks, pieces))
}
fn uv_endpoint(h: &sb::H) -> [Q; 2] {
    [&h[0] / &h[3], &h[1] / &h[3]]
}
fn monotone(uv: &[Poly; 3], axis: usize, work: &mut Work) -> Result<bool> {
    let n = add(
        &mul(&poly::derivative(&uv[axis]), &uv[2], work)?,
        &scale(
            &mul(&uv[axis], &poly::derivative(&uv[2]), work)?,
            &-Q::one(),
        ),
    )?;
    let b = poly::bernstein(&n, n.len().saturating_sub(1))?;
    Ok(b.iter().all(|q| *q >= Q::zero()) && b.iter().any(Q::is_positive))
}
pub(super) fn validate(
    pocket: &AlgebraicModelPocket,
    w: &BoundaryArcWitness,
    vertices: &mut BTreeMap<u32, [Q; 2]>,
    work: &mut Work,
) -> Result<ArcData> {
    let p = &pocket.certificate.partition;
    let e = p
        .edges
        .get(w.domain_edge as usize)
        .ok_or_else(|| fail("ordinary Model arc refers to an absent CAD edge"))?;
    let (breaks, nets) = pieces(&w.curve)?;
    if nets.is_empty() {
        return Err(fail("ordinary Model arc has no active span"));
    }
    let start = uv_endpoint(&nets[0][0]);
    let end = uv_endpoint(nets.last().unwrap().last().unwrap());
    for (id, uv) in [(e.endpoints[0], start), (e.endpoints[1], end)] {
        if domains::locate(p, &uv[0], &uv[1])? != DomainLocation::Vertex(id) {
            return Err(fail(
                "rational arc endpoint is not the exact selected algebraic vertex",
            ));
        }
        if vertices.get(&id).is_some_and(|old| old != &uv) {
            return Err(fail(
                "ordinary Model arc endpoints disagree at a shared vertex",
            ));
        }
        vertices.insert(id, uv);
    }
    let relation = p
        .arrangement
        .fiber_relation
        .iter()
        .map(|r| r.iter().map(roots::decode).collect::<Result<Vec<_>>>())
        .collect::<Result<Bi>>()?;
    let mut uv_pieces = vec![];
    let mut last = None;
    for h in &nets {
        let a = uv_endpoint(&h[0]);
        if last.as_ref().is_some_and(|p| p != &a) {
            return Err(fail(
                "ordinary Model arc is discontinuous at an authored knot",
            ));
        }
        last = Some(uv_endpoint(h.last().unwrap()));
        let uv = [poly::power(h, 0), poly::power(h, 1), poly::power(h, 3)];
        let axis = match e.kind {
            domains::DomainEdgeKind::Section { .. } => 0,
            domains::DomainEdgeKind::Fiber { .. } => 1,
        };
        if !monotone(&uv, axis, work)? {
            return Err(fail(
                "ordinary Model rational arc has no proved monotone parameter coverage",
            ));
        }
        match e.kind {
            domains::DomainEdgeKind::Section { .. } => {
                if !poly::substitute(&relation, &uv, poly::total_degree(&relation), work)?
                    .is_empty()
                {
                    return Err(fail(
                        "rational curve is not identically on the original algebraic branch",
                    ));
                }
            }
            domains::DomainEdgeKind::Fiber { .. } => {
                let x = &vertices[&e.endpoints[0]][0];
                if !add(&uv[0], &scale(&uv[2], &-x))?.is_empty() {
                    return Err(fail(
                        "rational fiber arc leaves its exact critical parameter",
                    ));
                }
            }
        }
        let t = Q::new(1.into(), 2.into());
        let den = poly::at(&uv[2], &t);
        let u = poly::at(&uv[0], &t) / &den;
        let v = poly::at(&uv[1], &t) / &den;
        if domains::locate(p, &u, &v)? != DomainLocation::Edge(w.domain_edge) {
            return Err(fail(
                "rational arc lies on a different ordered root section",
            ));
        }
        uv_pieces.push(uv);
    }
    let mut lifted: [Vec<[Poly; 4]>; 2] = std::array::from_fn(|_| vec![]);
    for carrier in 0..2 {
        let degree = pocket.powers[carrier]
            .iter()
            .map(poly::total_degree)
            .max()
            .unwrap();
        for uv in &uv_pieces {
            let mut coordinates = std::array::from_fn(|_| vec![]);
            for axis in 0..4 {
                coordinates[axis] =
                    poly::substitute(&pocket.powers[carrier][axis], uv, degree, work)?;
            }
            lifted[carrier].push(coordinates);
        }
    }
    Ok(ArcData {
        curve: w.curve.clone(),
        breaks,
        uv: uv_pieces,
        lifted,
    })
}
fn vertex(p: &domains::DomainPartition, id: u32) -> Result<[Q; 2]> {
    let v = &p.vertices[id as usize];
    let cut = &p.arrangement.cuts[v.cut];
    if cut.parameter[0] != cut.parameter[1] || cut.fibers[v.rank][0] != cut.fibers[v.rank][1] {
        return Err(fail("automatic rational arc construction needs exact rational critical vertices; supply an exact arc witness"));
    }
    Ok([
        roots::decode(&cut.parameter[0])?,
        roots::decode(&cut.fibers[v.rank][0])?,
    ])
}
fn line(a: &[Q; 2], b: &[Q; 2]) -> Result<crate::Curve> {
    Ok(crate::Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![
            a.iter().map(poly::exact).collect::<Result<_>>()?,
            b.iter().map(poly::exact).collect::<Result<_>>()?,
        ],
        weights: vec![1., 1.],
        periodic: false,
    })
}
fn circle(p: &DomainPolynomial) -> Result<Option<([Q; 2], Q)>> {
    let p = decode(p)?;
    let coeff = |i: usize, j: usize| p.get(j).and_then(|r| r.get(i)).cloned().unwrap_or_default();
    let a = coeff(2, 0);
    if a.is_zero()
        || coeff(0, 2) != a
        || p.iter().enumerate().any(|(j, r)| {
            r.iter().enumerate().any(|(i, q)| {
                !q.is_zero() && ![(0, 0), (1, 0), (0, 1), (2, 0), (0, 2)].contains(&(i, j))
            })
        })
    {
        return Ok(None);
    }
    let c = [
        -coeff(1, 0) / (&a * integer(2)),
        -coeff(0, 1) / (&a * integer(2)),
    ];
    let r2 = &c[0] * &c[0] + &c[1] * &c[1] - coeff(0, 0) / a;
    if r2 <= Q::zero() {
        return Ok(None);
    }
    let n = r2.numer().nth_root(2);
    let d = r2.denom().nth_root(2);
    if &n * &n != *r2.numer() || &d * &d != *r2.denom() {
        return Ok(None);
    }
    Ok(Some((c, Q::new(n, d))))
}
fn half_circle(c: &[Q; 2], r: &Q, sign: i64) -> Result<crate::Curve> {
    let s = integer(sign);
    let points = [
        [&c[0] - r, c[1].clone()],
        [&c[0] - r, &c[1] + &s * r],
        [c[0].clone(), &c[1] + &s * r],
        [&c[0] + r, &c[1] + &s * r],
        [&c[0] + r, c[1].clone()],
    ];
    Ok(crate::Curve {
        degree: 2,
        knots: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
        control_points: points
            .iter()
            .map(|p| p.iter().map(poly::exact).collect())
            .collect::<Result<_>>()?,
        weights: vec![1., 1., 2., 2., 4.],
        periodic: false,
    })
}
pub(super) fn automatic(pocket: &AlgebraicModelPocket) -> Result<Vec<BoundaryArcWitness>> {
    let ids = pocket
        .certificate
        .faces
        .iter()
        .flat_map(|f| &f.loops)
        .flatten()
        .filter_map(|u| {
            if let PocketEdge::Top(id) = u.edge {
                Some(id)
            } else {
                None
            }
        })
        .collect::<BTreeSet<_>>();
    let p = &pocket.certificate.partition;
    let mut result = vec![];
    let candidates = std::iter::once(&pocket.certificate.domain)
        .chain(&pocket.certificate.factors)
        .map(circle)
        .collect::<Result<Vec<_>>>()?;
    for id in ids {
        let e = &p.edges[id as usize];
        let a = vertex(p, e.endpoints[0])?;
        let b = vertex(p, e.endpoints[1])?;
        let straight = BoundaryArcWitness {
            domain_edge: id,
            curve: line(&a, &b)?,
        };
        if validate(
            pocket,
            &straight,
            &mut BTreeMap::new(),
            &mut Work::default(),
        )
        .is_ok()
        {
            result.push(straight);
            continue;
        }
        let mut found = None;
        for (c, r) in candidates.iter().flatten() {
            if a == [&c[0] - r, c[1].clone()] && b == [&c[0] + r, c[1].clone()] {
                for sign in [-1, 1] {
                    let w = BoundaryArcWitness {
                        domain_edge: id,
                        curve: half_circle(c, r, sign)?,
                    };
                    if validate(pocket, &w, &mut BTreeMap::new(), &mut Work::default()).is_ok() {
                        found = Some(w);
                        break;
                    }
                }
            }
            if found.is_some() {
                break;
            }
        }
        result.push(found.ok_or_else(||fail("algebraic edge has no automatic exact rational NURBS witness; approximation still needs a certified topology/error proof"))?);
    }
    Ok(result)
}
