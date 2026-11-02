//! Algebraic trimmed B-reps between independent rational graph carriers.
//! Native algebraic edge selectors retain irrational branches; they are not
//! fitted back into the legacy Model's rational Curve representation.
use super::*;
use crate::{
    profile_certificate::{atlas_roots as roots, ExactRatio},
    Surface,
};
use domains::{DomainLocation, DomainPolynomial};
mod polynomial;
mod topology;
use polynomial::{at, decode, derivative, encode, guard_poly, product, scale, sum, trim, Bi};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlgebraicRegionFace {
    pub carrier: u8,
    /// Outward face orientation relative to the original carrier's UV normal.
    pub reversed: bool,
    pub regions: Vec<u32>,
    /// Outward shell traversal in the common domain (already oriented), not
    /// legacy Model CCW loops. Map through normalized_parameter_maps to UV.
    /// Signed original CAD edge selectors, not approximated control polygons.
    pub boundary_cycles: Vec<Vec<i32>>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlgebraicRegionSeam {
    pub domain_edge: u32,
    pub faces: [u32; 2],
    pub oriented_uses: [i32; 2],
}
#[derive(Clone, Debug, PartialEq)]
pub struct AlgebraicRegionCertificate {
    pub version: u32,
    pub sources: [String; 2],
    /// From the common triangle to each original knot cell's normalized UV.
    pub normalized_parameter_maps: [[[ExactRatio; 3]; 2]; 2],
    pub regularity_multiplier: DomainPolynomial,
    pub intersection_factors: Vec<DomainPolynomial>,
    pub height_equation: DomainPolynomial,
    pub partition: domains::DomainPartition,
    pub selected: Vec<u32>,
    pub components: Vec<Vec<u32>>,
    pub faces: Vec<AlgebraicRegionFace>,
    pub seams: Vec<AlgebraicRegionSeam>,
}
#[derive(Clone, Debug)]
pub struct AlgebraicRegionBrep {
    carriers: [Surface; 2],
    powers: [[Bi; 4]; 2],
    certificate: AlgebraicRegionCertificate,
}
impl AlgebraicRegionBrep {
    pub fn carriers(&self) -> &[Surface; 2] {
        &self.carriers
    }
    pub fn certificate(&self) -> &AlgebraicRegionCertificate {
        &self.certificate
    }
    /// Edit the original NURBS carriers then recertify; the checked handle
    /// never silently keeps a certificate after geometry mutation.
    pub fn into_carriers(self) -> [Surface; 2] {
        self.carriers
    }
    /// Query normalized source coordinates without rounding algebraic edges.
    pub fn evaluate_rational(
        &self,
        face: u32,
        u: &ExactRatio,
        v: &ExactRatio,
    ) -> Result<[ExactRatio; 3]> {
        let f = self
            .certificate
            .faces
            .get(face as usize)
            .ok_or_else(|| fail("algebraic B-rep face is absent"))?;
        let u = roots::decode(u)?;
        let v = roots::decode(v)?;
        if u < Q::zero()
            || v < Q::zero()
            || &u + &v > Q::one()
            || u.numer().bits() > 512
            || u.denom().bits() > 512
            || v.numer().bits() > 512
            || v.denom().bits() > 512
        {
            return Err(fail("algebraic B-rep query outside triangle or bit budget"));
        }
        let p = &self.certificate.partition;
        let location = domains::locate(p, &u, &v)?;
        let admitted = match location {
            DomainLocation::Face(id) => f.regions.contains(&id),
            DomainLocation::Edge(id) => f.regions.iter().any(|r| {
                p.faces[*r as usize]
                    .boundary
                    .iter()
                    .any(|e| e.unsigned_abs() - 1 == id)
            }),
            DomainLocation::Vertex(id) => f.regions.iter().any(|r| {
                p.faces[*r as usize].boundary.iter().any(|e| {
                    p.edges[e.unsigned_abs() as usize - 1]
                        .endpoints
                        .contains(&id)
                })
            }),
        };
        if !admitted {
            return Err(fail("query is outside selected algebraic face"));
        }
        let h = &self.powers[f.carrier as usize];
        let w = at(&h[3], &u, &v);
        if w <= Q::zero() {
            return Err(fail("algebraic B-rep query denominator is not positive"));
        }
        let point = std::array::from_fn::<_, 3, _>(|axis| at(&h[axis], &u, &v) / &w);
        guard(&point)?;
        Ok(point.each_ref().map(roots::ratio))
    }
}
fn source(s: &Surface) -> Result<String> {
    let bytes = super::super::identity::surface(s)?;
    let mut h = Sha256::new();
    h.update(b"independent-graph-algebraic-brep-v1");
    h.update(bytes);
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}
fn input(s: &Surface, work: &mut Work) -> Result<([Bi; 4], [[Q; 3]; 2])> {
    if !(1..=8).contains(&s.degree_u)
        || !(1..=8).contains(&s.degree_v)
        || s.control_points.iter().map(Vec::len).sum::<usize>() > 1024
    {
        return Err(fail(
            "algebraic graph source degree/control budget exceeded",
        ));
    }
    s.validate()?;
    if s.periodic_u || s.periodic_v {
        return Err(fail(
            "algebraic graph requires an original nonperiodic knot cell",
        ));
    }
    let range = |knots: &[f64], degree: usize, count: usize| {
        [
            Q::from_float(knots[degree]).unwrap(),
            Q::from_float(knots[count]).unwrap(),
        ]
    };
    let net = sb::cell(
        s,
        range(&s.knots_u, s.degree_u, s.control_points.len()),
        range(&s.knots_v, s.degree_v, s.control_points[0].len()),
    )?;
    if net.iter().flatten().any(|h| h[3] <= Q::zero()) {
        return Err(fail(
            "algebraic graph denominator controls must be positive",
        ));
    }
    let mut powers: [Bi; 4] = std::array::from_fn(|_| vec![]);
    for axis in 0..4 {
        powers[axis] = trim(pullback::relation(&net, axis, &Q::zero(), work)?);
        guard_poly(&powers[axis])?;
    }
    let w = &powers[3];
    let mut xy: [[Q; 3]; 2] = std::array::from_fn(|_| std::array::from_fn(|_| Q::zero()));
    for axis in 0..2 {
        let origin = at(&powers[axis], &Q::zero(), &Q::zero()) / at(w, &Q::zero(), &Q::zero());
        let u = at(&powers[axis], &Q::one(), &Q::zero()) / at(w, &Q::one(), &Q::zero()) - &origin;
        let v = at(&powers[axis], &Q::zero(), &Q::one()) / at(w, &Q::zero(), &Q::one()) - &origin;
        let affine = trim(vec![vec![origin.clone(), u.clone()], vec![v.clone()]]);
        if product(&affine, w, work)? != powers[axis] {
            return Err(fail(
                "independent carrier is not an exact affine-projection rational graph",
            ));
        }
        xy[axis] = [origin, u, v];
    }
    if &xy[0][1] * &xy[1][2] - &xy[0][2] * &xy[1][1] == Q::zero() {
        return Err(fail("independent graph projection is singular"));
    }
    Ok((powers, xy))
}
/// Construct closed cap pairs for chosen positive components of the exact
/// height difference. All chart-boundary signs must be negative. On F=0 a
/// supplied multiplier M must prove |grad F|²+M*F>0 by exact Bernstein bounds.
/// These are independent original NURBS carriers, not a common ambient shear.
pub fn certify_graph_region_brep(
    upper: &Surface,
    lower: &Surface,
    multiplier: &DomainPolynomial,
    selection: Option<&[u32]>,
) -> Result<AlgebraicRegionBrep> {
    construct(upper, lower, multiplier, &[], selection)
}
/// Optional exact factorization avoids monolithic resultant expression growth;
/// its complete product must equal the original height equation coefficientwise.
pub fn certify_graph_region_brep_factored(
    upper: &Surface,
    lower: &Surface,
    multiplier: &DomainPolynomial,
    intersection_factors: &[DomainPolynomial],
    selection: Option<&[u32]>,
) -> Result<AlgebraicRegionBrep> {
    if intersection_factors.is_empty() {
        return Err(fail("empty algebraic factor witness"));
    }
    construct(upper, lower, multiplier, intersection_factors, selection)
}
struct AlignedCarriers {
    powers: [[Bi; 4]; 2],
    maps: [[[Q; 3]; 2]; 2],
    xy: [[Q; 3]; 2],
    lower_determinant: Q,
}
fn align(upper: &Surface, lower: &Surface, work: &mut Work) -> Result<AlignedCarriers> {
    let (a, xy) = input(upper, work)?;
    let (raw_b, other) = input(lower, work)?;
    let determinant = &other[0][1] * &other[1][2] - &other[0][2] * &other[1][1];
    let mut lower_map: [[Q; 3]; 2] = std::array::from_fn(|_| std::array::from_fn(|_| Q::zero()));
    for j in 0..3 {
        let dx = &xy[0][j]
            - if j == 0 {
                other[0][0].clone()
            } else {
                Q::zero()
            };
        let dy = &xy[1][j]
            - if j == 0 {
                other[1][0].clone()
            } else {
                Q::zero()
            };
        lower_map[0][j] = (&dx * &other[1][2] - &dy * &other[0][2]) / &determinant;
        lower_map[1][j] = (&other[0][1] * &dy - &other[1][1] * &dx) / &determinant;
    }
    for affine in &lower_map {
        for value in [
            affine[0].clone(),
            &affine[0] + &affine[1],
            &affine[0] + &affine[2],
        ] {
            if value < Q::zero() || value > Q::one() {
                return Err(fail(
                    "common triangle leaves the independent lower carrier's original cell",
                ));
            }
        }
    }
    let upper_map = [
        [Q::zero(), Q::one(), Q::zero()],
        [Q::zero(), Q::zero(), Q::one()],
    ];
    let maps = [upper_map, lower_map.clone()];
    let mut b: [Bi; 4] = std::array::from_fn(|_| vec![]);
    for axis in 0..4 {
        b[axis] = polynomial::compose(&raw_b[axis], &lower_map, work)?;
    }
    Ok(AlignedCarriers {
        powers: [a, b],
        maps,
        xy,
        lower_determinant: determinant,
    })
}

fn construct(
    upper: &Surface,
    lower: &Surface,
    multiplier: &DomainPolynomial,
    intersection_factors: &[DomainPolynomial],
    selection: Option<&[u32]>,
) -> Result<AlgebraicRegionBrep> {
    if intersection_factors.len() > 13 {
        return Err(fail("algebraic region factor count exceeded"));
    }
    let mut work = Work::default();
    let aligned = align(upper, lower, &mut work)?;
    let [a, b] = aligned.powers;
    let maps = aligned.maps;
    let xy = aligned.xy;
    let determinant = aligned.lower_determinant;
    let equation = sum(
        &product(&a[2], &b[3], &mut work)?,
        &scale(&product(&b[2], &a[3], &mut work)?, &-Q::one()),
    )?;
    let encoded = encode(&equation);
    let equation = decode(&encoded)?;
    let multiplier_decoded = decode(multiplier)?;
    if !polynomial::negative_boundary(&equation, &mut work)? {
        return Err(fail(
            "graph region reaches the source triangle boundary; independent side caps are required",
        ));
    }
    let du = derivative(&equation, 0);
    let dv = derivative(&equation, 1);
    let gradient = sum(
        &product(&du, &du, &mut work)?,
        &product(&dv, &dv, &mut work)?,
    )?;
    let regular = sum(
        &gradient,
        &product(&multiplier_decoded, &equation, &mut work)?,
    )?;
    if !polynomial::positive(&regular, &mut work)? {
        return Err(fail(
            "algebraic seam regularity is not proved by the supplied multiplier",
        ));
    }
    let partition = if intersection_factors.is_empty() {
        domains::partition(&[encoded.clone()])?
    } else {
        domains::partition_factorized(&encoded, intersection_factors)?
    };
    let mut selected = selection.map(|s| s.to_vec()).unwrap_or_else(|| {
        partition
            .faces
            .iter()
            .enumerate()
            .filter(|(_, f)| f.signs[0] == 1)
            .map(|(i, _)| i as u32)
            .collect()
    });
    selected.sort();
    let orientation = (&xy[0][1] * &xy[1][2] - &xy[0][2] * &xy[1][1]).is_negative();
    let (faces, seams, components) = topology::assemble(
        &partition,
        &selected,
        orientation,
        determinant.is_negative(),
    )?;
    let certificate = AlgebraicRegionCertificate {
        version: 1,
        sources: [source(upper)?, source(lower)?],
        normalized_parameter_maps: maps
            .each_ref()
            .map(|m| m.each_ref().map(|a| a.each_ref().map(roots::ratio))),
        regularity_multiplier: multiplier.clone(),
        intersection_factors: intersection_factors.to_vec(),
        height_equation: encoded,
        partition,
        selected,
        components,
        faces,
        seams,
    };
    Ok(AlgebraicRegionBrep {
        carriers: [upper.clone(), lower.clone()],
        powers: [a, b],
        certificate,
    })
}
pub fn verify_graph_region_brep(
    upper: &Surface,
    lower: &Surface,
    cert: &AlgebraicRegionCertificate,
) -> Result<AlgebraicRegionBrep> {
    if cert.sources != [source(upper)?, source(lower)?] {
        return Err(fail("algebraic B-rep source binding mismatch"));
    }
    if cert
        .seams
        .iter()
        .any(|s| s.oriented_uses[0] != -s.oriented_uses[1])
    {
        return Err(fail("algebraic B-rep seam orientation mismatch"));
    }
    if cert.version != 1 {
        return Err(fail("algebraic region B-rep certificate version mismatch"));
    }
    let expected = construct(
        upper,
        lower,
        &cert.regularity_multiplier,
        &cert.intersection_factors,
        Some(&cert.selected),
    )?;
    if expected.certificate != *cert {
        return Err(fail(
            "algebraic B-rep source, region, branch or cap certificate mismatch",
        ));
    }
    Ok(expected)
}
#[cfg(test)]
mod tests;

mod model_pocket;
pub use model_pocket::{
    certify_model_pocket, verify_model_pocket, AlgebraicModelPocket, ModelPocketCertificate,
    PocketEdge, PocketEdgeUse, PocketFace, PocketFaceKind, PocketVertex,
};

pub use model_pocket::{
    certify_pocket_boolean_model, certify_pocket_boolean_model_with_arcs,
    verify_pocket_boolean_model, BoundaryArcWitness, CertifiedPocketModel, PocketModelCertificate,
};

pub use model_pocket::{
    certify_framed_pocket_boolean_model, certify_model_pocket_batch,
    certify_oriented_pocket_boolean_model, verify_framed_pocket_boolean_model,
    verify_model_pocket_batch, CertifiedFramedPocketModel, CertifiedPocketModelBatch,
    FramedPocketModelCertificate, ModelPocketRequest, PocketCoordinateFrame,
    PocketModelBatchCertificate,
};

pub use model_pocket::{CertifiedGraphBoxBoolean, GraphBoxBooleanCertificate, certify_graph_box_boolean, verify_graph_box_boolean};
pub(crate) use model_pocket::try_graph_box_boolean;
