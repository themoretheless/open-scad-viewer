//! Source-bound NURBS face homotopy and filled Minkowski inclusion certificates.
//! Full rectangular trimmed charts, positive rational weights, multiple knot
//! spans and exact oriented C0 chart seams are checked independently.
use super::surface_bounds as sb;
use super::*;
use crate::Model;
use value_codec::{Deserialize, Serialize, Value, error, json};
mod prismatic;
mod shear;
mod topology;
mod trimmed;
pub use prismatic::SumReport as SumTopologyReport;
pub use topology::Report as SourceTopologyReport;
#[derive(Clone, Debug, PartialEq)]
pub enum SourceAtlasTopologyReport {
    Prismatic(prismatic::Report),
    Polynomial(shear::Report),
}
impl Serialize for SourceAtlasTopologyReport {
    fn to_value(&self) -> Value {
        match self {
            Self::Prismatic(r) => r.to_value(),
            Self::Polynomial(r) => r.to_value(),
        }
    }
}
impl<'de> Deserialize<'de> for SourceAtlasTopologyReport {
    fn from_value(v: Value) -> value_codec::Result<Self> {
        match v["kind"].as_str() {
            Some("signed-prismatic-ambient-isotopy") => {
                Ok(Self::Prismatic(value_codec::from_value(v)?))
            }
            Some("polynomial-shear-ambient-isotopy") => {
                Ok(Self::Polynomial(value_codec::from_value(v)?))
            }
            _ => Err(error("Unknown source ambient topology proof")),
        }
    }
}
pub const SCOPE: &str = "represented-nurbs-filled-minkowski-enclosures";
#[derive(Clone, Debug)]
pub enum Source {
    Mesh(SourceMesh),
    Brep(Model),
}
#[derive(Clone, Debug, PartialEq)]
pub struct EnclosureCertificate {
    pub version: u32,
    pub sources: [String; 2],
    pub depths: [u8; 2],
    pub maximum_source_deviation_mm: f64,
    pub source_homotopy_bounds: [Ratio; 2],
    pub filled_hausdorff_bound: Option<Ratio>,
    pub nominal: Certificate,
    pub inner: Certificate,
    pub outer: Certificate,
    pub source_topology: [Option<SourceTopologyReport>; 2],
    pub source_atlas_topology: [Option<SourceAtlasTopologyReport>; 2],
    pub sum_topology: Option<SumTopologyReport>,
}
#[derive(Clone, Debug)]
pub struct VerifiedEnclosure {
    pub certificate: EnclosureCertificate,
    pub nominal: Verified,
}
fn up(q: &Q) -> f64 {
    let x = q.to_f64().unwrap();
    if Q::from_float(x).unwrap() < *q {
        f64::from_bits(x.to_bits() + 1)
    } else {
        x
    }
}
impl VerifiedEnclosure {
    /// A full-sum claim needs its own checked recipe. Two source ambient
    /// certificates alone do not prove topology of their Minkowski sum.
    pub fn approximation_topology(&self) -> Option<Value> {
        if self.certificate.version >= 3 {
            if self
                .certificate
                .source_homotopy_bounds
                .iter()
                .all(|r| r[0] == "0")
            {
                return Some(
                    json!({"status":"proved","kind":"exact-source-minkowski-identity","includesMeshRounding":true}),
                );
            }
            if let Some(s) = &self.certificate.sum_topology {
                return Some(
                    json!({"status":"proved","kind":"prismatic-minkowski-homeomorphism","axis":s.axis,"survivingHoles":s.surviving_holes,"includesMeshRounding":true}),
                );
            }
        }
        let [Some(a), Some(b)] = &self.certificate.source_topology else {
            return None;
        };
        Some(
            json!({"status":"proved","kind":"radial-minkowski-homeomorphism","sourceCenters":[a.center.clone(),b.center.clone()],"filledSet":"closed-three-ball","boundary":"two-sphere","includesMeshRounding":true}),
        )
    }
    pub fn source_approximation_topology(&self) -> [bool; 2] {
        std::array::from_fn(|i| {
            self.certificate.source_homotopy_bounds[i][0] == "0"
                || self.certificate.source_topology[i].is_some()
                || self.certificate.source_atlas_topology[i].is_some()
        })
    }
    pub fn source_surface_deviations_mm(&self) -> [f64; 2] {
        std::array::from_fn(|i| {
            let r = &self.certificate.source_homotopy_bounds[i];
            up(&(Q::new(r[0].parse().unwrap(), r[1].parse().unwrap()) * integer(2)))
        })
    }
    /// Euclidean distance <= 2*L-infinity distance. Include the separately proved
    /// embedding/rounding error of the actual returned (f64 or f32) mesh.
    pub fn maximum_set_deviation(&self) -> Option<(Ratio, f64)> {
        self.certificate.filled_hausdorff_bound.as_ref().map(|r| {
            let q = (Q::new(r[0].parse().unwrap(), r[1].parse().unwrap())
                + Q::from_float(self.nominal.maximum_coordinate_rounding_mm).unwrap())
                * integer(2);
            (ratio(&q), up(&q))
        })
    }
}
struct Region {
    nominal: Vec<Poly>,
    inner: Vec<Poly>,
    outer: Vec<Poly>,
    error: Q,
    hausdorff: Option<Q>,
    topology: Option<SourceTopologyReport>,
    atlas: Option<SourceAtlasTopologyReport>,
    prism: Option<prismatic::Data>,
}
fn ratio(q: &Q) -> Ratio {
    [q.numer().to_string(), q.denom().to_string()]
}
fn source_id(s: &Source) -> Result<String> {
    let bytes = value_codec::encode_binary(&s.to_value()).map_err(|e| fail(&e.to_string()))?;
    let mut h = Sha256::new();
    h.update(b"nurbs-minkowski-source-v1");
    h.update(bytes);
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}
fn rectangle(m: &Model, face: usize) -> Result<()> {
    let f = &m.faces[face];
    let s = &f.surface;
    let lo = [s.knots_u[s.degree_u], s.knots_v[s.degree_v]];
    let hi = [
        s.knots_u[s.control_points.len()],
        s.knots_v[s.control_points[0].len()],
    ];
    let wire = &m.loops[f.outer];
    if !f.holes.is_empty() || wire.coedges.len() != 4 {
        return Err(fail(
            "curved source requires complete rectangular UV faces; curved or holed trim charts are unproved",
        ));
    }
    let corners = [
        [lo[0], lo[1]],
        [hi[0], lo[1]],
        [hi[0], hi[1]],
        [lo[0], hi[1]],
    ];
    let mut starts = Vec::new();
    for c in &wire.coedges {
        let p = &c.pcurve;
        let d = p.domain();
        if p.control_points.len() != p.degree + 1
            || p.knots[..=p.degree].iter().any(|x| *x != d[0])
            || p.knots[p.degree + 1..].iter().any(|x| *x != d[1])
        {
            return Err(fail(
                "rectangular trim needs a clamped rational boundary segment",
            ));
        }
        let a = &p.control_points[0];
        let b = p.control_points.last().unwrap();
        let side = (0..4)
            .find(|i| a[..] == corners[*i] && b[..] == corners[(*i + 1) % 4])
            .ok_or_else(|| fail("trim does not exactly traverse a parameter rectangle side"))?;
        for q in &p.control_points {
            for k in 0..2 {
                if corners[side][k] == corners[(side + 1) % 4][k] {
                    if q[k] != corners[side][k] {
                        return Err(fail("trim leaves its exact rectangle side"));
                    }
                } else if q[k] < lo[k] || q[k] > hi[k] {
                    return Err(fail("trim exits its full rectangle interval"));
                }
            }
        }
        starts.push(side);
    }
    if (0..4).any(|i| starts[(i + 1) % 4] != (starts[i] + 1) % 4) {
        return Err(fail("rectangular trim side census is incomplete"));
    }
    Ok(())
}
fn geometry(
    m: &Model,
    depth: u8,
    limit: &Q,
    work: &mut Work,
    version: u32,
) -> Result<(Input, Vec<Poly>, Q)> {
    m.validate()?;
    if m.bodies.is_empty() || m.faces.len() > 64 || depth > 3 {
        return Err(fail(
            "NURBS source needs bounded solid bodies and depth <=3",
        ));
    }
    if m.faces
        .iter()
        .map(|f| f.surface.control_points.iter().map(Vec::len).sum::<usize>())
        .sum::<usize>()
        > 8192
    {
        return Err(fail("NURBS source exceeds 8192 surface controls"));
    }
    let mut uses = Vec::new();
    for body in &m.bodies {
        for shell in std::iter::once(body.outer_shell).chain(body.inner_shells.iter().copied()) {
            if !m.shells[shell].closed {
                return Err(fail("NURBS source shell is open"));
            }
            uses.extend(m.shells[shell].faces.iter().cloned())
        }
    }
    let mut stations = BTreeSet::from([Q::zero(), Q::one()]);
    for u in &uses {
        if version == 1 {
            rectangle(m, u.face)?;
        }
        let s = &m.faces[u.face].surface;
        if !(1..=8).contains(&s.degree_u)
            || !(1..=8).contains(&s.degree_v)
            || s.control_points
                .iter()
                .flatten()
                .any(|p| p.len() != 3 || p.iter().any(|x| !x.is_finite() || x.abs() > 1e6))
        {
            return Err(fail("NURBS source degree or coordinate budget exceeded"));
        }
        if version >= 2 {
            sb::continuous(s, work)?;
        }
        for (knots, degree, count) in [
            (&s.knots_u, s.degree_u, s.control_points.len()),
            (&s.knots_v, s.degree_v, s.control_points[0].len()),
        ] {
            let a = Q::from_float(knots[degree]).unwrap();
            let b = Q::from_float(knots[count]).unwrap();
            for k in &knots[degree..=count] {
                let x = (Q::from_float(*k).unwrap() - &a) / (&b - &a);
                stations.insert(x.clone());
                stations.insert(Q::one() - x);
            }
        }
    }
    if version >= 2 {
        trimmed::stations(m, &uses, &mut stations)?;
    }
    for i in 0..=(1usize << depth) {
        stations.insert(Q::new(i.into(), (1usize << depth).into()));
    }
    let stations = stations.into_iter().collect::<Vec<_>>();
    if stations.len() > 33
        || uses.iter().filter(|u| rectangle(m, u.face).is_ok()).count()
            * (stations.len() - 1).pow(2)
            > 512
    {
        return Err(fail("NURBS exact chart cell budget exceeded"));
    }
    let mut vertices = BTreeMap::<P, u32>::new();
    let mut triangles = Vec::new();
    let mut seams = trimmed::Seams::new();
    let mut tubes = Vec::new();
    let mut maximum = Q::zero();
    let mut has_trimmed = false;
    for use_ in uses {
        if rectangle(m, use_.face).is_err() {
            has_trimmed = true;
            trimmed::append(
                m,
                &use_,
                &stations,
                depth,
                &mut vertices,
                &mut triangles,
                &mut seams,
                &mut tubes,
                &mut maximum,
                limit,
                work,
            )?;
            continue;
        }
        let s = &m.faces[use_.face].surface;
        let da = [
            Q::from_float(s.knots_u[s.degree_u]).unwrap(),
            Q::from_float(s.knots_u[s.control_points.len()]).unwrap(),
        ];
        let db = [
            Q::from_float(s.knots_v[s.degree_v]).unwrap(),
            Q::from_float(s.knots_v[s.control_points[0].len()]).unwrap(),
        ];
        for u in stations.windows(2) {
            for v in stations.windows(2) {
                work.charge(1)?;
                let a = |t: &Q| &da[0] + t * (&da[1] - &da[0]);
                let b = |t: &Q| &db[0] + t * (&db[1] - &db[0]);
                let h = sb::cell(s, [a(&u[0]), a(&u[1])], [b(&v[0]), b(&v[1])])?;
                let c = sb::corners(&h);
                let err = sb::error(&h);
                maximum = maximum.max(err.iter().cloned().max().unwrap());
                if &maximum * integer(2) > *limit {
                    return Err(fail(
                        "continuous NURBS source deviation exceeds the requested bound",
                    ));
                }
                for edge in sb::edges(&h) {
                    trimmed::insert_seam(sb::Trace::Bezier(edge), use_.reversed, &mut seams, work)?;
                }
                let ids = c
                    .iter()
                    .map(|p| {
                        let next = vertices.len() as u32;
                        *vertices.entry(p.clone()).or_insert(next)
                    })
                    .collect::<Vec<_>>();
                let diagonals = if dot(&sub(&c[0], &c[2]), &sub(&c[0], &c[2]))
                    >= dot(&sub(&c[1], &c[3]), &sub(&c[1], &c[3]))
                {
                    [[0, 1, 2], [0, 2, 3]]
                } else {
                    [[0, 1, 3], [1, 2, 3]]
                };
                for mut t in diagonals {
                    if zero(&cross(&sub(&c[t[1]], &c[t[0]]), &sub(&c[t[2]], &c[t[0]]))) {
                        continue;
                    }
                    if use_.reversed {
                        t.swap(1, 2)
                    }
                    triangles.extend(t.map(|i| ids[i]));
                }
                if !sb::planar(&h) {
                    let points = h.iter().flatten().map(sb::point).collect::<Vec<_>>();
                    let lo: P = std::array::from_fn(|i| {
                        points
                            .iter()
                            .map(|p| p[i].clone())
                            .min()
                            .unwrap()
                            .max(c.iter().map(|p| p[i].clone()).min().unwrap() - &err[i])
                    });
                    let hi: P = std::array::from_fn(|i| {
                        points
                            .iter()
                            .map(|p| p[i].clone())
                            .max()
                            .unwrap()
                            .min(c.iter().map(|p| p[i].clone()).max().unwrap() + &err[i])
                    });
                    if (0..3).all(|i| lo[i] < hi[i]) {
                        let bounds = bounding_box(&[lo, hi])?;
                        if version == 1 {
                            tubes.push(bounds)
                        } else {
                            let mut tube = Some(hull(points, work)?);
                            for f in &bounds.faces {
                                if let Some(t) = tube {
                                    tube = t.clip(&plane(f), work)?
                                }
                            }
                            if let Some(t) = tube {
                                tubes.push(t)
                            }
                        }
                    }
                }
            }
        }
    }
    trimmed::verify_seams(seams, work)?;
    let mut points = vec![[Q::zero(), Q::zero(), Q::zero()]; vertices.len()];
    for (p, i) in vertices {
        points[i as usize] = p
    }
    Ok((
        input_chain(&points, &triangles, !has_trimmed)?,
        tubes,
        maximum,
    ))
}
fn enclose(nominal: &[Poly], tubes: &[Poly], work: &mut Work) -> Result<(Vec<Poly>, Vec<Poly>)> {
    if tubes.is_empty() {
        return Ok((nominal.to_vec(), nominal.to_vec()));
    }
    let all = nominal
        .iter()
        .chain(tubes)
        .flat_map(Poly::vertices)
        .collect::<Vec<_>>();
    let planes = nominal
        .iter()
        .chain(tubes)
        .flat_map(|c| c.faces.iter().map(|f| normalized(&plane(f), false)))
        .collect::<BTreeSet<_>>();
    if planes.len() > 192 {
        return Err(fail("NURBS enclosure exceeds 192 planes"));
    }
    let mut inner = Vec::new();
    let mut outer = Vec::new();
    for c in partition(
        vec![bounding_box(&all)?],
        &planes.into_iter().collect::<Vec<_>>(),
        work,
    )? {
        let p = centroid(&c.vertices());
        work.charge(nominal.iter().chain(tubes).map(|c| c.faces.len()).sum())?;
        let inside = nominal.iter().any(|c| c.contains(&p));
        let tube = tubes.iter().any(|c| c.contains(&p));
        if inside && !tube {
            inner.push(c.clone())
        }
        if inside || tube {
            outer.push(c)
        }
    }
    Ok((inner, outer))
}
/// Exact convex subtraction for the inner set; a checked convex cover is a
/// conservative outer set. The outer may bridge concavities: no finite filled
/// distance is claimed unless the separate coverage proof succeeds.
fn enclose_chain(
    nominal: &[Poly],
    tubes: &[Poly],
    work: &mut Work,
) -> Result<(Vec<Poly>, Vec<Poly>)> {
    if tubes.is_empty() {
        return Ok((nominal.to_vec(), nominal.to_vec()));
    }
    let mut inner = nominal.to_vec();
    for tube in tubes {
        let planes = tube
            .faces
            .iter()
            .map(|f| normalized(&plane(f), true))
            .collect::<BTreeSet<_>>();
        let mut next = Vec::new();
        for c in inner {
            let vertices = c.vertices();
            work.charge(planes.len() * vertices.len())?;
            if planes
                .iter()
                .any(|h| vertices.iter().all(|p| eval(h, p) >= Q::zero()))
            {
                next.push(c);
                continue;
            }
            let mut inside = Some(c);
            for h in &planes {
                let Some(c) = inside.take() else { break };
                if let Some(out) = c.clip(&neg(h), work)? {
                    next.push(out)
                }
                inside = c.clip(h, work)?;
            }
        }
        if next.len() > 2048 {
            return Err(fail("trimmed-source inner cell budget exceeded"));
        }
        inner = next;
    }
    let points = nominal
        .iter()
        .chain(tubes)
        .flat_map(Poly::vertices)
        .collect::<BTreeSet<_>>();
    let outer = hull(points.into_iter().collect(), work)?;
    Ok((inner, vec![outer]))
}
fn volume(c: &Poly) -> Q {
    c.faces
        .iter()
        .map(|f| {
            (1..f.len() - 1)
                .map(|i| dot(&f[0], &cross(&f[i], &f[i + 1])) / integer(6))
                .sum::<Q>()
        })
        .sum()
}
/// Arrangement cells have disjoint interiors. Their union is contained in its
/// convex hull; equality of exact volumes proves equality of the closed sets.
/// This removes artificial partitions before pair sums, without a tolerance.
fn convex_union(cells: Vec<Poly>) -> Vec<Poly> {
    if cells.len() < 2 {
        return cells;
    }
    let points = cells
        .iter()
        .flat_map(Poly::vertices)
        .collect::<BTreeSet<_>>();
    if points.len() > 512 {
        return cells;
    }
    let total = cells.iter().map(volume).sum::<Q>();
    if let Ok(h) = hull(points.into_iter().collect(), &mut Work::default()) {
        if volume(&h) == total {
            return vec![h];
        }
    }
    cells
}
fn metric_bound(
    nominal: &[Poly],
    inner: &[Poly],
    outer: &[Poly],
    error: &Q,
    version: u32,
) -> Option<Q> {
    if error.is_zero() {
        return Some(Q::zero());
    }
    if inner.is_empty() || nominal.is_empty() {
        return None;
    }
    // One exact anchor in the nominal material and one in its proved inner
    // enclosure give a coarse, finite two-sided Hausdorff witness. No costly
    // convex inflation is needed for a highly partitioned nonconvex source.
    let anchor = || {
        let n = centroid(&nominal[0].vertices());
        let i = centroid(&inner[0].vertices());
        let a = outer
            .iter()
            .flat_map(Poly::vertices)
            .flat_map(|p| (0..3).map(|k| (&p[k] - &n[k]).abs()).collect::<Vec<_>>())
            .max()
            .unwrap_or_else(Q::zero);
        let b = nominal
            .iter()
            .flat_map(Poly::vertices)
            .flat_map(|p| (0..3).map(|k| (&p[k] - &i[k]).abs()).collect::<Vec<_>>())
            .max()
            .unwrap_or_else(Q::zero);
        a.max(b)
    };
    if inner.len() > 32 || nominal.len() > 32 || outer.len() > 128 {
        return (version >= 2).then(anchor);
    }
    if version >= 2 && (inner.len() > 4 || nominal.len() > 4) {
        return Some(anchor());
    }
    let attempt = || -> Result<Option<Q>> {
        let mut work = Work::default();
        for multiplier in [1, 2, 4, 8] {
            let delta = error * integer(multiplier);
            let inflate = |cells: &[Poly], work: &mut Work| -> Result<Vec<Poly>> {
                let corners = (0..8)
                    .map(|b| {
                        std::array::from_fn(|i| {
                            if b & (1 << i) == 0 {
                                -&delta
                            } else {
                                delta.clone()
                            }
                        })
                    })
                    .collect::<Vec<P>>();
                cells
                    .iter()
                    .map(|c| {
                        hull(
                            c.vertices()
                                .iter()
                                .flat_map(|p| corners.iter().map(move |q| add(p, q)))
                                .collect(),
                            work,
                        )
                    })
                    .collect()
            };
            let n = inflate(nominal, &mut work)?;
            let i = inflate(inner, &mut work)?;
            let covered = |a: &[Poly], b: &[Poly]| {
                a.iter().all(|c| {
                    let v = c.vertices();
                    b.iter().any(|d| v.iter().all(|p| d.contains(p)))
                })
            };
            if covered(outer, &n) && covered(nominal, &i) {
                return Ok(Some(delta));
            }
        }
        Ok(None)
    };
    attempt()
        .ok()
        .flatten()
        .or_else(|| (version >= 2).then(anchor))
}
fn region(s: &Source, depth: u8, limit: &Q, work: &mut Work, version: u32) -> Result<Region> {
    match s {
        Source::Mesh(m) => {
            if depth != 0 {
                return Err(fail("a mesh source has no NURBS subdivision depth"));
            }
            let nominal = decompose(&input(m)?, work)?;
            let topology = if version >= 2 {
                topology::prove(s, &input(m)?, &nominal, &[], &mut Work::default())
            } else {
                None
            };
            Ok(Region {
                inner: nominal.clone(),
                outer: nominal.clone(),
                nominal,
                error: Q::zero(),
                hausdorff: Some(Q::zero()),
                topology,
                atlas: None,
                prism: None,
            })
        }
        Source::Brep(m) => {
            if version >= 2 {
                trimmed::preflight(m, &mut Work::default())?;
            }
            let (input, tubes, error) = geometry(m, depth, limit, work, version)?;
            let nominal = decompose(&input, work)?;
            let topology = if version >= 2 {
                topology::prove(s, &input, &nominal, &tubes, &mut Work::default())
            } else {
                None
            };
            let (atlas, prism) = if version >= 3 {
                match prismatic::inspect(s, depth, &nominal, &mut Work::default()) {
                    Some((r, d)) => (Some(SourceAtlasTopologyReport::Prismatic(r)), d),
                    None => (
                        shear::prove(s, &nominal, &mut Work::default())
                            .map(SourceAtlasTopologyReport::Polynomial),
                        None,
                    ),
                }
            } else {
                (None, None)
            };
            let (inner, outer) = if version == 1 {
                enclose(&nominal, &tubes, work)?
            } else {
                enclose_chain(&nominal, &tubes, work)?
            };
            let inner = convex_union(inner);
            let outer = convex_union(outer);
            let hausdorff = if tubes.is_empty() {
                Some(Q::zero())
            } else {
                metric_bound(&nominal, &inner, &outer, &error, version)
            };
            Ok(Region {
                nominal,
                inner,
                outer,
                error,
                hausdorff,
                topology,
                atlas,
                prism,
            })
        }
    }
}
fn derive(
    a: &Source,
    b: &Source,
    depths: [u8; 2],
    maximum: f64,
    version: u32,
) -> Result<EnclosureCertificate> {
    if !maximum.is_finite() || !(1e-9..=1e4).contains(&maximum) {
        return Err(fail("invalid continuous source deviation bound"));
    }
    let limit = Q::from_float(maximum).unwrap();
    let mut work = Work::default();
    let a0 = region(a, depths[0], &limit, &mut work, version)?;
    let b0 = region(b, depths[1], &limit, &mut work, version)?;
    let sources = [source_id(a)?, source_id(b)?];
    let mut make = |x: &[Poly], y: &[Poly]| -> Result<Certificate> {
        let m = sum_material(x, y, &mut work)?;
        if version == 1 {
            return certificate_for(
                &m,
                sources.clone(),
                [x.len(), y.len()],
                x.len() * y.len(),
                &mut work,
            );
        }
        let (points, triangles) = boundary_dyadic(&m, &mut work)?;
        Ok(Certificate {
            version: 1,
            sources: sources.clone(),
            vertices: points
                .iter()
                .map(|p| std::array::from_fn(|k| ratio(&p[k])))
                .collect(),
            triangles,
            source_cells: [x.len(), y.len()],
            sum_cells: x.len() * y.len(),
        })
    };
    let nominal = make(&a0.nominal, &b0.nominal)?;
    let inner = make(&a0.inner, &b0.inner)?;
    let outer = make(&a0.outer, &b0.outer)?;
    let sum_topology = if version >= 3 {
        prismatic::sum(
            a,
            b,
            &a0.nominal,
            &b0.nominal,
            a0.prism.as_ref(),
            b0.prism.as_ref(),
            &nominal,
        )
    } else {
        None
    };
    Ok(EnclosureCertificate {
        version,
        sources,
        depths,
        maximum_source_deviation_mm: maximum,
        source_homotopy_bounds: [ratio(&a0.error), ratio(&b0.error)],
        filled_hausdorff_bound: a0.hausdorff.zip(b0.hausdorff).map(|(a, b)| ratio(&(a + b))),
        nominal,
        inner,
        outer,
        source_topology: [a0.topology, b0.topology],
        source_atlas_topology: [a0.atlas, b0.atlas],
        sum_topology,
    })
}
pub fn certify(a: &Source, b: &Source, maximum: f64, f32_mesh: bool) -> Result<VerifiedEnclosure> {
    if !maximum.is_finite() || !(1e-9..=1e4).contains(&maximum) {
        return Err(fail("invalid continuous source deviation bound"));
    }
    let find = |s: &Source| -> Result<u8> {
        if matches!(s, Source::Mesh(_)) {
            return Ok(0);
        }
        let Source::Brep(m) = s else { unreachable!() };
        trimmed::preflight(m, &mut Work::default())?;
        let mut last = fail("NURBS source subdivision unproved");
        for d in 0..=3 {
            match geometry(
                m,
                d,
                &Q::from_float(maximum).unwrap(),
                &mut Work::default(),
                2,
            ) {
                Ok(_) => return Ok(d),
                Err(e) => last = e,
            }
        }
        Err(last)
    };
    let c = derive(a, b, [find(a)?, find(b)?], maximum, 3)?;
    let nominal = display_precision(&c.nominal, if f32_mesh { "f32" } else { "f64" })?;
    Ok(VerifiedEnclosure {
        certificate: c,
        nominal,
    })
}
pub fn verify(
    a: &Source,
    b: &Source,
    c: &EnclosureCertificate,
    f32_mesh: bool,
) -> Result<VerifiedEnclosure> {
    if ![1, 2, 3].contains(&c.version)
        || c.sources != [source_id(a)?, source_id(b)?]
        || derive(a, b, c.depths, c.maximum_source_deviation_mm, c.version)? != *c
    {
        return Err(fail(
            "NURBS source, continuous bounds or filled inclusion certificate is false",
        ));
    }
    Ok(VerifiedEnclosure {
        nominal: display_precision(&c.nominal, if f32_mesh { "f32" } else { "f64" })?,
        certificate: c.clone(),
    })
}

impl Serialize for Source {
    fn to_value(&self) -> Value {
        match self {
            Self::Mesh(m) => json!({"kind":"mesh","mesh":m}),
            Self::Brep(m) => json!({"kind":"brep","model":m}),
        }
    }
}
impl<'de> Deserialize<'de> for Source {
    fn from_value(v: Value) -> value_codec::Result<Self> {
        match v["kind"].as_str() {
            Some("mesh") => Ok(Self::Mesh(value_codec::from_value(v["mesh"].clone())?)),
            Some("brep") => Ok(Self::Brep(value_codec::from_value(v["model"].clone())?)),
            _ => Err(error("Unknown Minkowski source kind")),
        }
    }
}
impl Serialize for EnclosureCertificate {
    fn to_value(&self) -> Value {
        let mut v = json!({"version":self.version,"scope":SCOPE,"sources":self.sources,"depths":self.depths,"maximumSourceDeviationMm":self.maximum_source_deviation_mm,"sourceHomotopyBounds":self.source_homotopy_bounds,"filledHausdorffBound":self.filled_hausdorff_bound,"nominal":self.nominal,"inner":self.inner,"outer":self.outer});
        if self.version >= 2 {
            v["sourceTopology"] = self.source_topology.to_value()
        }
        if self.version >= 3 {
            v["sourceAtlasTopology"] = self.source_atlas_topology.to_value();
            v["sumTopology"] = self.sum_topology.to_value();
        }
        v
    }
}
impl<'de> Deserialize<'de> for EnclosureCertificate {
    fn from_value(v: Value) -> value_codec::Result<Self> {
        let mut keys = vec![
            "version",
            "scope",
            "sources",
            "depths",
            "maximumSourceDeviationMm",
            "sourceHomotopyBounds",
            "filledHausdorffBound",
            "nominal",
            "inner",
            "outer",
        ];
        if matches!(v["version"].as_u64(), Some(2 | 3)) {
            keys.push("sourceTopology")
        }
        if v["version"].as_u64() == Some(3) {
            keys.extend(["sourceAtlasTopology", "sumTopology"]);
        }
        if v.as_object()
            .is_none_or(|o| o.len() != keys.len() || keys.iter().any(|k| !o.contains_key(*k)))
            || v["scope"].as_str() != Some(SCOPE)
        {
            return Err(error("NURBS Minkowski enclosure schema mismatch"));
        }
        Ok(Self {
            version: value_codec::from_value(v["version"].clone())?,
            sources: value_codec::from_value(v["sources"].clone())?,
            depths: value_codec::from_value(v["depths"].clone())?,
            maximum_source_deviation_mm: value_codec::from_value(
                v["maximumSourceDeviationMm"].clone(),
            )?,
            source_homotopy_bounds: value_codec::from_value(v["sourceHomotopyBounds"].clone())?,
            filled_hausdorff_bound: value_codec::from_value(v["filledHausdorffBound"].clone())?,
            nominal: value_codec::from_value(v["nominal"].clone())?,
            inner: value_codec::from_value(v["inner"].clone())?,
            outer: value_codec::from_value(v["outer"].clone())?,
            source_topology: if matches!(v["version"].as_u64(), Some(2 | 3)) {
                value_codec::from_value(v["sourceTopology"].clone())?
            } else {
                [None, None]
            },
            source_atlas_topology: if v["version"].as_u64() == Some(3) {
                value_codec::from_value(v["sourceAtlasTopology"].clone())?
            } else {
                [None, None]
            },
            sum_topology: if v["version"].as_u64() == Some(3) {
                value_codec::from_value(v["sumTopology"].clone())?
            } else {
                None
            },
        })
    }
}

#[cfg(test)]
fn save_fixture(name: &str, m: &Model) {
    if let Ok(dir) = std::env::var("OSV_NONSTAR_FIXTURE_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            std::path::Path::new(&dir).join(format!("{name}.json")),
            value_codec::to_string(m).unwrap(),
        )
        .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_multispan_restriction_and_unproved_seams_refuse() {
        let mut m = crate::cuboid([0., 0., 0.], [2., 2., 2.]).unwrap();
        m.faces[1].surface = nurbs_core::editing::surface_edit::insert(
            &m.faces[1].surface,
            nurbs_core::surface::Axis::U,
            0.5,
            1,
        )
        .unwrap();
        m.rebuild_topology_ids();
        let (_, tubes, e) = geometry(&m, 0, &integer(1), &mut Work::default(), 2).unwrap();
        assert!(tubes.is_empty());
        assert!(e.is_zero());
        m.faces[1].surface.control_points[1][0][0] += 1e-9;
        assert!(
            geometry(&m, 0, &integer(1), &mut Work::default(), 2)
                .err()
                .unwrap()
                .message
                .contains("seams")
        );
    }
    #[test]
    fn curved_trim_is_source_bound_and_invalid_trims_do_not_fall_back() {
        let model = crate::cylinder(1., 2.).unwrap();
        let source = Source::Brep(model.clone());
        let b = Source::Mesh(super::super::tests::cube([0, 0, 0], [1, 1, 1]));
        let r = certify(&source, &b, 2., true).unwrap();
        assert!(r.nominal.volume_mm3 > 0.);
        verify(&source, &b, &r.certificate, true).unwrap();
        assert!(r.certificate.source_topology.iter().all(Option::is_some));
        assert!(r.approximation_topology().is_some());
        let mut invalid = model;
        let outer = invalid.faces[4].outer;
        invalid.loops[outer].coedges[0].pcurve.control_points[1][0] = 2.;
        assert!(certify(&Source::Brep(invalid), &b, 2., false).is_err());
    }
    #[test]
    fn holed_and_nonconvex_trimmed_winding_is_not_convexified() {
        let tube = Source::Brep(crate::tube(2., 1., 2.).unwrap());
        let b = Source::Mesh(super::super::tests::cube([0, 0, 0], [1, 1, 1]));
        let mut small = super::super::tests::cube([0, 0, 0], [1, 1, 1]);
        for x in &mut small.positions {
            *x *= 0.25
        }
        let small = Source::Mesh(small);
        let r = certify(&tube, &small, 4., false).unwrap();
        assert!(r.nominal.volume_mm3 > 0.);
        assert!(r.certificate.source_topology[0].is_none());
        assert_eq!(
            r.approximation_topology().unwrap()["survivingHoles"].as_u64(),
            Some(1)
        );
        verify(&tube, &small, &r.certificate, false).unwrap();
        let display = input(&SourceMesh {
            positions: r.nominal.positions.clone(),
            indices: r.nominal.indices.clone(),
        })
        .unwrap();
        assert_eq!(
            winding(
                &[
                    Q::new(1.into(), 8.into()),
                    Q::new(1.into(), 8.into()),
                    integer(1)
                ],
                &display.faces,
                &mut Work::default()
            )
            .unwrap(),
            0
        );
        let l = crate::extrude_polygon(
            &[[0., 0.], [4., 0.], [4., 1.], [1., 1.], [1., 4.], [0., 4.]],
            0.,
            2.,
        )
        .unwrap();
        let r = certify(&Source::Brep(l), &b, 10., false).unwrap();
        assert_eq!(r.nominal.volume_mm3, 48.);
        assert_eq!(r.maximum_set_deviation().unwrap().1, 0.);
    }
    #[test]
    fn topology_receipt_is_checked_and_legacy_enclosures_still_verify() {
        let a = Source::Brep(crate::cuboid([0., 0., 0.], [2., 2., 2.]).unwrap());
        let b = a.clone();
        let r = certify(&a, &b, 0.1, false).unwrap();
        let mut bad = r.certificate.clone();
        bad.source_topology[0].as_mut().unwrap().center[0] = ["999".into(), "1".into()];
        assert!(verify(&a, &b, &bad, false).is_err());
        let mut bad = r.certificate.clone();
        let SourceAtlasTopologyReport::Prismatic(atlas) =
            bad.source_atlas_topology[0].as_mut().unwrap()
        else {
            panic!("expected atlas")
        };
        atlas.shells[0].coefficients[0] = -1;
        assert!(verify(&a, &b, &bad, false).is_err());
        let legacy_v2 = derive(&a, &b, [0, 0], 0.1, 2).unwrap();
        verify(&a, &b, &legacy_v2, false).unwrap();
        let legacy = derive(&a, &b, [0, 0], 0.1, 1).unwrap();
        assert!(legacy.source_topology.iter().all(Option::is_none));
        verify(&a, &b, &legacy, false).unwrap();
    }
    #[test]
    fn nonexact_spherical_equator_refuses_before_approximation_search() {
        let sphere = crate::sphere(1.).unwrap();
        let error = trimmed::preflight(&sphere, &mut Work::default()).unwrap_err();
        assert_eq!(error.code, trimmed::SOURCE_SEAM_CODE);
        let a = Source::Brep(sphere);
        let b = Source::Mesh(super::super::tests::cube([0, 0, 0], [1, 1, 1]));
        assert_eq!(
            certify(&a, &b, 1., true).unwrap_err().code,
            trimmed::SOURCE_SEAM_CODE
        );
    }
    #[test]
    fn curved_triangular_trim_uses_the_original_tensor_composition() {
        let mut m = crate::cuboid([0., 0., 0.], [2., 2., 2.]).unwrap();
        let s = &mut m.faces[1].surface;
        s.degree_u = 2;
        s.degree_v = 2;
        s.knots_u = vec![0., 0., 0., 1., 1., 1.];
        s.knots_v = s.knots_u.clone();
        s.control_points = (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| vec![i as f64, j as f64, if i == 1 && j == 1 { 3. } else { 2. }])
                    .collect()
            })
            .collect();
        s.weights = vec![vec![1.; 3]; 3];
        let outer = m.faces[1].outer;
        let old = m.loops[outer].coedges.clone();
        let start = m.edges[old[0].edge].vertices[usize::from(old[0].reversed)];
        let end = m.edges[old[1].edge].vertices[usize::from(!old[1].reversed)];
        let edge = m.edges.len();
        m.edges.push(crate::Edge {
            degenerate: false,
            vertices: [start, end],
            curve: crate::Curve {
                degree: 4,
                knots: vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.],
                weights: vec![1.; 5],
                control_points: vec![
                    vec![0., 0., 2.],
                    vec![0.5, 0.5, 2.],
                    vec![1., 1., 8. / 3.],
                    vec![1.5, 1.5, 2.],
                    vec![2., 2., 2.],
                ],
                periodic: false,
            },
        });
        let pc = crate::Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
        m.loops[outer].coedges = vec![
            old[0].clone(),
            old[1].clone(),
            crate::Coedge {
                edge,
                reversed: true,
                pcurve: pc.reverse().unwrap(),
            },
        ];
        let second = m.loops.len();
        m.loops.push(crate::Loop {
            coedges: vec![
                crate::Coedge {
                    edge,
                    reversed: false,
                    pcurve: pc,
                },
                old[2].clone(),
                old[3].clone(),
            ],
        });
        let face = m.faces.len();
        let surface = m.faces[1].surface.clone();
        m.faces.push(crate::Face {
            surface,
            outer: second,
            holes: vec![],
        });
        m.shells[0].faces.push(crate::FaceUse {
            face,
            reversed: false,
        });
        m.rebuild_topology_ids();
        let a = Source::Brep(m);
        let b = Source::Mesh(super::super::tests::cube([-1, -1, -1], [1, 1, 1]));
        let r = certify(&a, &b, 1., true).unwrap();
        assert!(r.nominal.volume_mm3 >= 64.);
        assert!(r.approximation_topology().is_some());
        verify(&a, &b, &r.certificate, true).unwrap();
    }
    #[test]
    fn exact_original_planar_brep_sum() {
        let a = Source::Brep(crate::cuboid([0., 0., 0.], [2., 3., 4.]).unwrap());
        let b = Source::Brep(crate::cuboid([-1., -1., -1.], [1., 1., 1.]).unwrap());
        let r = certify(&a, &b, 0.1, false).unwrap();
        assert!(r.approximation_topology().is_some());
        assert_eq!(r.nominal.volume_mm3, 120.);
        assert_eq!(
            r.certificate.filled_hausdorff_bound,
            Some(["0".into(), "1".into()])
        );
        verify(&a, &b, &r.certificate, false).unwrap();
        let mut bad = r.certificate;
        bad.source_homotopy_bounds[0] = ["1".into(), "1".into()];
        assert!(verify(&a, &b, &bad, false).is_err())
    }
    #[test]
    fn rational_surface_homotopy_encloses_the_original_body() {
        let mut model = crate::cuboid([0., 0., 0.], [2., 2., 2.]).unwrap();
        let s = &mut model.faces[1].surface;
        s.degree_u = 2;
        s.degree_v = 2;
        s.knots_u = vec![0., 0., 0., 1., 1., 1.];
        s.knots_v = s.knots_u.clone();
        s.control_points = (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| vec![i as f64, j as f64, if i == 1 && j == 1 { 3. } else { 2. }])
                    .collect()
            })
            .collect();
        s.weights = vec![vec![1.; 3]; 3];
        s.weights[1][1] = 2.;
        model.rebuild_topology_ids();
        let a = Source::Brep(model);
        let b = Source::Mesh(super::super::tests::cube([-1, -1, -1], [1, 1, 1]));
        let r = certify(&a, &b, 1., false).unwrap();
        assert!(r.approximation_topology().is_some());
        assert_ne!(r.certificate.source_homotopy_bounds[0], ["0", "1"]);
        assert!(r.nominal.volume_mm3 >= 64.);
        assert!(r.maximum_set_deviation().is_some());
        assert!(r.source_surface_deviations_mm()[0] <= 1.);
        verify(&a, &b, &r.certificate, false).unwrap();
        let rounded = certify(&a, &b, 1., true).unwrap();
        assert_eq!(rounded.nominal.rounding_topology.precision, "f32");
        verify(&a, &b, &rounded.certificate, true).unwrap();
        let mut bad = r.certificate;
        bad.outer.vertices[0][0][0] = "17".into();
        assert!(verify(&a, &b, &bad, false).is_err())
    }
}
