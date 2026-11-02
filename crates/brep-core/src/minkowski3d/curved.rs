//! Source-bound NURBS face homotopy and filled Minkowski inclusion certificates.
//! Full rectangular trimmed charts, positive rational weights, multiple knot
//! spans and exact oriented C0 chart seams are checked independently.
use super::surface_bounds as sb;
use super::*;
use crate::Model;
use value_codec::{Deserialize, Serialize, Value, error, json};
mod atlas;
pub use atlas::algebraic::{
    AtlasAlgebraicParameters, AtlasAlgebraicRoot, certify_algebraic_knot_parameters,
    certify_algebraic_seam_parameters, verify_algebraic_knot_parameters,
    verify_algebraic_seam_parameters,
};
pub use atlas::difference::{
    CertifiedEnclosedDifference, EnclosedDifferenceCertificate, certify_enclosed_difference,
    verify_enclosed_difference,
};
pub use atlas::domains::{
    AlgebraicDomainAtlas, DomainEdge, DomainEdgeKind, DomainEvaluation, DomainFace, DomainLocation,
    DomainPartition, DomainPolynomial, DomainSeam, DomainVertex, VerifiedDomainAtlas,
    certify_algebraic_domain_atlas, check_algebraic_domain_atlas, verify_algebraic_domain_atlas,
};
pub use atlas::pullback::{
    AtlasAlgebraicPullback, AtlasExactTriangle, certify_algebraic_surface_pullback,
    certify_algebraic_triangle_pullback, verify_algebraic_surface_pullback,
    verify_algebraic_triangle_pullback,
};
mod atlas_legacy;
mod prismatic;
pub use atlas::Certificate as SourceAtlasCertificate;
/// Independent source atlas certificate, without requiring a Minkowski sum.
pub fn certify_source_atlas(m: &Model, depth: u8) -> Result<SourceAtlasCertificate> {
    atlas::derive(m, depth)
}
pub fn verify_source_atlas(m: &Model, c: &SourceAtlasCertificate) -> Result<()> {
    if c.version != 1
        || c.depth > 3
        || c.positions.len() > 1024
        || c.triangles.len() > 1024
        || c.report.chart_triangles != c.triangles.len()
        || c.report.vertex_stars != c.positions.len()
        || c.triangles.iter().any(|t| {
            t.iter().any(|i| *i as usize >= c.positions.len())
                || t[0] == t[1]
                || t[1] == t[2]
                || t[0] == t[2]
        })
        || c.source != source_id(&Source::Brep(m.clone()))?
        || atlas::derive(m, c.depth)? != *c
    {
        return Err(fail("source atlas certificate is false"));
    }
    Ok(())
}
mod difference;
mod shear;
pub use difference::{
    CertifiedQuadraticDifference, QuadraticDifferenceCertificate, certify_quadratic_difference,
    verify_quadratic_difference,
};
mod topology;
mod trimmed;
pub use prismatic::SumReport as SumTopologyReport;
pub use topology::Report as SourceTopologyReport;
#[derive(Clone, Debug, PartialEq)]
pub enum SourceAtlasTopologyReport {
    Prismatic(prismatic::Report),
    Polynomial(shear::Report),
    Rational(atlas::Report),
}
impl Serialize for SourceAtlasTopologyReport {
    fn to_value(&self) -> Value {
        match self {
            Self::Prismatic(r) => r.to_value(),
            Self::Polynomial(r) => r.to_value(),
            Self::Rational(r) => r.to_value(),
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
            Some("rational-atlas-ambient-isotopy") => {
                Ok(Self::Rational(value_codec::from_value(v)?))
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
            let (mut atlas, prism) = if version >= 3 {
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
            if version >= 4 && atlas.is_none() {
                atlas = atlas::nominal_report(m, depth, &input)
                    .ok()
                    .map(SourceAtlasTopologyReport::Rational);
            }
            let (inner, outer) = if version == 1 {
                enclose(&nominal, &tubes, work)?
            } else if atlas.is_some() && topology.is_none() && !tubes.is_empty() {
                // The ambient proof is independent of a tight metric cover.
                // Positive rational tensor surfaces lie in the convex hull of
                // their original Cartesian controls. Their entire boundary,
                // and hence the bounded positive-winding material, lies in
                // this exact box. Empty inner material is always a valid
                // inclusion; no filled Hausdorff bound follows from it.
                let points = m
                    .faces
                    .iter()
                    .flat_map(|f| f.surface.control_points.iter().flatten())
                    .map(|p| std::array::from_fn(|i| Q::from_float(p[i]).unwrap()))
                    .collect::<Vec<P>>();
                (vec![], vec![bounding_box(&points)?])
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
    let mut c = derive(a, b, [find(a)?, find(b)?], maximum, 4)?;
    // Preserve the existing v3 wire format when its old proof recipes suffice.
    // Only the general rational atlas requires the new reconstruction recipe.
    if c.source_atlas_topology
        .iter()
        .all(|r| !matches!(r, Some(SourceAtlasTopologyReport::Rational(_))))
    {
        c.version = 3;
    }
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
    if ![1, 2, 3, 4].contains(&c.version)
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
        if matches!(v["version"].as_u64(), Some(2 | 3 | 4)) {
            keys.push("sourceTopology")
        }
        if matches!(v["version"].as_u64(), Some(3 | 4)) {
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
            source_topology: if matches!(v["version"].as_u64(), Some(2 | 3 | 4)) {
                value_codec::from_value(v["sourceTopology"].clone())?
            } else {
                [None, None]
            },
            source_atlas_topology: if matches!(v["version"].as_u64(), Some(3 | 4)) {
                value_codec::from_value(v["sourceAtlasTopology"].clone())?
            } else {
                [None, None]
            },
            sum_topology: if matches!(v["version"].as_u64(), Some(3 | 4)) {
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
#[path = "curved/tests/tests_integration.rs"]
mod tests;

#[path = "curved/source_geometry.rs"]
mod source_geometry;
use source_geometry::geometry;

pub use atlas::region_brep::{
    AlgebraicRegionBrep, AlgebraicRegionCertificate, AlgebraicRegionFace, AlgebraicRegionSeam,
    certify_graph_region_brep, certify_graph_region_brep_factored, verify_graph_region_brep,
};

pub use atlas::region_brep::{
    AlgebraicModelPocket, ModelPocketCertificate, PocketEdge, PocketEdgeUse, PocketFace,
    PocketFaceKind, PocketVertex, certify_model_pocket, verify_model_pocket,
};

pub use atlas::region_brep::{
    BoundaryArcWitness, CertifiedPocketModel, PocketModelCertificate, certify_pocket_boolean_model,
    certify_pocket_boolean_model_with_arcs, verify_pocket_boolean_model,
};

pub use atlas::region_brep::{
    CertifiedFramedPocketModel, CertifiedPocketModelBatch, FramedPocketModelCertificate,
    ModelPocketRequest, PocketCoordinateFrame, PocketModelBatchCertificate,
    certify_framed_pocket_boolean_model, certify_model_pocket_batch,
    certify_oriented_pocket_boolean_model, verify_framed_pocket_boolean_model,
    verify_model_pocket_batch,
};

#[path = "curved/source_charts.rs"]
mod source_charts;
use source_charts::rectangle;

pub use difference::{
    CertifiedQuadraticBoolean, QuadraticBooleanCertificate, certify_quadratic_boolean,
    verify_quadratic_boolean,
};

pub(crate) use difference::try_common_quadratic_boolean;

pub(crate) use atlas::boolean::try_nonintersecting_nurbs_boolean;
pub use atlas::boolean::{
    CertifiedNonintersectingNurbsBoolean, NonintersectingNurbsBooleanCertificate,
    NurbsMaterialRelation, certify_nonintersecting_nurbs_boolean,
    verify_nonintersecting_nurbs_boolean,
};

pub use atlas::region_brep::{CertifiedGraphBoxBoolean, GraphBoxBooleanCertificate, certify_graph_box_boolean, verify_graph_box_boolean};
pub(crate) use atlas::region_brep::try_graph_box_boolean;

mod same_carrier;
pub use same_carrier::{
    CertifiedSameCarrierMerge, SameCarrierMergeCertificate,
    certify_same_carrier_merge, verify_same_carrier_merge,
};
pub(crate) use same_carrier::same_carrier_component;

pub(crate) use same_carrier::seam_model_id;

mod intersection_graph;
pub use intersection_graph::{
    GraphEndpoint, NurbsGraphLimits, NurbsGraphWork, NurbsIntersectionGraphCertificate,
    OriginalFacePairGraph, OriginalGraphEvent, OriginalGraphPiece, OriginalIntersectionArc,
    OriginalIntersectionCycle, OriginalRootKey, QualifiedNurbsIntersectionGraph,
    certify_nurbs_intersection_graph, verify_nurbs_intersection_graph,
};

mod retained_boolean;
pub use retained_boolean::{
    CertifiedRetainedNurbsBoolean, NurbsFacePartition, RegionEdgeOrigin,
    RetainedNurbsBooleanCertificate, RetainedRegionMaterial,
    certify_retained_nurbs_boolean, verify_retained_nurbs_boolean, author_retained_nurbs_boolean,
};
pub(crate) use retained_boolean::try_retained_nurbs_boolean;
