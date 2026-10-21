//! Checkable certificates of the *represented* positive-weight planar profile.
//!
//! The verifier does not trust the profile validator, intersection search,
//! floating-point area, a hash, or a caller-supplied "certified" flag. It
//! reconstructs exact rational homogeneous subdivision from every source bit
//! and checks every span and every unordered pair. Hull witnesses prove a
//! simultaneous homotopy to a simple polygon, preserving orientation/nesting.
//! This certifies neither construction history nor a three-dimensional solid.
use crate::{Curve, Error, Result};
use num_rational::BigRational as Q;
use num_traits::{One, Zero};

mod agreement;
pub use agreement::{
    ProjectiveCurveAgreementCertificate, certify_projective_curve_agreement,
    verify_projective_curve_agreement,
};
mod algebraic_intersections;
mod arithmetic;
mod intersections;
pub use algebraic_intersections::{
    AlgebraicFactorWitness, AlgebraicIntersectionCertificate, AlgebraicRootWitness,
    VerifiedAlgebraicIntersections, certify_algebraic_intersections,
    verify_algebraic_intersections,
};
mod multispan;
mod image_agreement;
pub use image_agreement::{CurveImageCertificate,ImageCoverageWitness,ImageRelationWitness,VerifiedCurveImage,certify_curve_image_agreement,verify_curve_image_agreement};
mod span_intersections;
mod branches;
pub use branches::{ParameterBranchDecomposition,BranchCut,BranchSlab};
pub(crate) use branches::critical_samples;
pub(crate) fn exact_polynomial_nonnegative(p:&[Q])->Result<bool>{
    if arithmetic::at(p,&Q::zero())<Q::zero()||arithmetic::at(p,&Q::one())<Q::zero(){return Ok(false)}
    Ok(critical_samples(&[p.to_vec()])?.iter().all(|x|arithmetic::at(p,x)>=Q::zero()))
}
pub use span_intersections::{SpanPairCertificate,SpanIntersectionCertificate,VerifiedSpanIntersections,VerifiedCurveOverlap,VerifiedParameterLocus,VerifiedBranchGraph,certify_span_intersections,verify_span_intersections};
mod nonlinear;
pub use agreement::{
    CurveAgreementCertificate, VerifiedCurveAgreement, certify_curve_agreement,
    verify_curve_agreement,
};
pub use intersections::{
    CurveIntersectionCertificate, VerifiedCurveIntersections, certify_curve_intersections,
    verify_curve_intersections,
};
pub use multispan::{
    MultispanAgreementCertificate, SpanAgreementWitness, certify_multispan_agreement,
    verify_multispan_agreement,
};
pub use nonlinear::{
    NonlinearCurveAgreementCertificate, certify_nonlinear_curve_agreement,
    verify_nonlinear_curve_agreement,
};
mod contact;
mod extraction;
mod measure;
mod offset;
mod offset_regions;
mod minkowski;
pub use minkowski::{MinkowskiRegionCertificate,VerifiedMinkowskiRegion,ExactRegionRings,certify_minkowski_sum,certify_minkowski_disk_offset,verify_minkowski_region};
mod offset_wire;
pub use contact::{
    EndpointContactCertificate, VerifiedEndpointContacts, certify_endpoint_contacts,
    verify_endpoint_contacts,
};
pub use offset::{
    OffsetCurveCertificate, VerifiedOffsetCurve, certify_offset_curve, verify_offset_curve,
};
pub use offset_regions::{
    OffsetRegionCertificate, VerifiedOffsetRegions, certify_offset_regions, verify_offset_regions,
};
pub use offset_wire::{
    OffsetWireCertificate, OffsetWirePart, VerifiedOffsetWire, certify_offset_wire,
    verify_offset_wire,
};

pub(crate) fn nonstationary_span(c: &Curve, axis: usize) -> Result<bool> {
    source_bits(&[vec![c.clone()]], 1e-7)?;
    if axis > 1 || c.control_points.len() != c.degree + 1 {
        return Ok(false);
    }
    let nets = extraction::nets(&[vec![c.clone()]])?;
    Ok(nets.len() == 1 && arithmetic::no_stationary(&nets[0].1, axis))
}
pub use measure::{
    ExactAreaInterval, ProfileMeasureCertificate, VerifiedMeasuredProfile, certify_measure,
    verify_measure,
};
#[cfg(test)]
mod extended_tests;
#[cfg(test)]
mod measure_tests;
mod orientation;
mod producer;
mod winding;
mod point_winding;
pub(crate) mod topology_prisms;
mod linear_arrangement;
pub use linear_arrangement::{LinearWindingArrangement, regularize_linear_winding_arrangement};
pub use point_winding::{PointWindingCertificate, VerifiedPointWinding, RayCellWitness, RayRootWitness, certify_point_winding, verify_point_winding};
pub use winding::{LinearPeriodicContactRegions, regularize_linear_periodic_contacts, MultispanSurfaceRegions, OwnedMultispanPeriodicFaces, bind_multispan_regions_to_surface, assemble_multispan_periodic_surface_faces, MultispanChartAlias, MultispanPeriodicRegions, regularize_multispan_periodic_winding, OwnedClockMappedFaces, assemble_clock_mapped_surface_faces, ClockMappedSurfaceRegions, bind_clock_mapped_regions_to_surface, ClockMappedPeriodicRegions, regularize_clock_mapped_periodic_winding, ExactPeriodicContactRegions, regularize_exact_periodic_contacts,OwnedPeriodicFaces, PeriodicFaceLimits, assemble_periodic_surface_faces};
pub use winding::{
    ApproximatePeriodicWindingRegions, CoherentPeriodicWindingRegions, PeriodicWindingRegions,
    SurfacePeriodicRegions, WindingComponent, WindingRegularization,
    bind_periodic_regions_to_surface, regularize_approximated_periodic_winding,
    regularize_projective_coherent_periodic_winding, regularize_coherent_periodic_winding, regularize_nonzero_winding, regularize_periodic_winding,
};
#[cfg(feature = "codec")]
mod serialization;
pub use producer::certify;

pub const VERSION: u32 = 1;
const MAX_SPANS: usize = 128;
const MAX_PIECES: usize = 256;
const MAX_DEPTH: u8 = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileCertificate {
    pub version: u32,
    /// Lossless canonical source bits, including layout, knots and tolerance.
    /// Hex strings survive JSON/JavaScript without 64-bit integer rounding.
    pub source: String,
    pub subdivision_depth: u8,
    /// One independently checked strictly monotone coordinate per exact piece.
    pub monotone_axes: Vec<u8>,
    /// Complete lexicographic unordered-pair census; missing entries fail.
    /// 0/1 = coordinate normal; 2+8*i+j = legacy combined-hull edge normal.
    /// 256+32*i+j covers the larger nets of degree 4..8. 65535 is algebraic.
    pub separating_normals: Vec<u16>,
    pub orientation: Vec<i8>,
    pub nesting_depth: Vec<usize>,
    /// None uses the proved hull homotopy; Some(d) checks exact Green bounds
    /// after d dyadic subdivisions. Polynomial integrals are always exact.
    pub orientation_depth: Option<u8>,
}

/// Only returned after independent verification of the supplied source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedProfile {
    spans: usize,
    pieces: usize,
    nesting_depth: Vec<usize>,
}
impl VerifiedProfile {
    pub fn spans(&self) -> usize {
        self.spans
    }
    pub fn pieces(&self) -> usize {
        self.pieces
    }
    pub fn nesting_depth(&self) -> &[usize] {
        &self.nesting_depth
    }
}

fn refused(message: &str) -> Error {
    Error::new("BREP_PROFILE_CERTIFICATE_UNPROVEN", message)
}

fn source_bits(loops: &[Vec<Curve>], tolerance: f64) -> Result<String> {
    use std::fmt::Write;
    if !tolerance.is_finite()
        || tolerance <= 0.
        || loops.len() > 32
        || loops.iter().any(Vec::is_empty)
        || loops.iter().map(Vec::len).sum::<usize>() > MAX_SPANS
    {
        return Err(refused(
            "Certificate needs 1..32 nonempty closed wires, at most 128 spans and a positive tolerance",
        ));
    }
    let mut out = format!(
        "represented-profile-v1/{:016x}/{}",
        tolerance.to_bits(),
        loops.len()
    );
    for wire in loops {
        write!(out, "/w{}", wire.len()).unwrap();
        for c in wire {
            c.validate()?;
            if !(1..=8).contains(&c.degree)
                || c.control_points
                    .iter()
                    .any(|p| p.len() != 2 || p.iter().any(|x| x.abs() > 1e6))
            {
                return Err(refused(
                    "Certificate requires positive-weight planar curves of degree 1..8",
                ));
            }
            write!(
                out,
                "/c{}/{}/{}/{}",
                c.degree,
                c.control_points.len(),
                c.knots.len(),
                u8::from(c.periodic)
            )
            .unwrap();
            for x in c
                .control_points
                .iter()
                .flatten()
                .chain(&c.weights)
                .chain(&c.knots)
            {
                write!(out, "/{:016x}", x.to_bits()).unwrap();
            }
            if out.len() > 65536 {
                return Err(refused("Certificate exact source exceeds its size budget"));
            }
        }
    }
    Ok(out)
}

#[derive(Clone)]
struct Piece {
    wire: usize,
    net: Vec<[Q; 3]>,
    points: Vec<[Q; 2]>,
}
impl Piece {
    fn from_net(wire: usize, net: Vec<[Q; 3]>) -> Self {
        let points = net.iter().map(|p| [&p[0] / &p[2], &p[1] / &p[2]]).collect();
        Self { wire, net, points }
    }
}

fn split(net: &[[Q; 3]]) -> (Vec<[Q; 3]>, Vec<[Q; 3]>) {
    let mut level = net.to_vec();
    let mut left = vec![level[0].clone()];
    let mut right = vec![level.last().unwrap().clone()];
    let two = Q::from_integer(2.into());
    while level.len() > 1 {
        level = level
            .windows(2)
            .map(|p| std::array::from_fn(|k| (&p[0][k] + &p[1][k]) / &two))
            .collect();
        left.push(level[0].clone());
        right.push(level.last().unwrap().clone());
    }
    right.reverse();
    (left, right)
}

fn pieces(loops: &[Vec<Curve>], depth: u8) -> Result<Vec<Piece>> {
    if depth > MAX_DEPTH {
        return Err(refused(
            "Exact subdivision certificate exceeds its piece budget",
        ));
    }
    let source_nets = extraction::nets(loops)?;
    if source_nets.len() * (1usize << depth) > MAX_PIECES {
        return Err(refused(
            "Exact subdivision certificate exceeds its piece budget",
        ));
    }
    let mut out = Vec::new();
    for (wire, net) in source_nets {
        let mut nets = vec![net];
        for _ in 0..depth {
            nets = nets
                .into_iter()
                .flat_map(|n| {
                    let (a, b) = split(&n);
                    [a, b]
                })
                .collect();
        }
        out.extend(nets.into_iter().map(|n| Piece::from_net(wire, n)));
    }
    // These are exact rational equalities, including joins across source spans.
    for w in 0..loops.len() {
        let ring: Vec<_> = out.iter().filter(|p| p.wire == w).collect();
        for i in 0..ring.len() {
            if ring[i].points.last().unwrap() != &ring[(i + 1) % ring.len()].points[0] {
                return Err(refused("Certificate boundary is not exactly closed"));
            }
        }
    }
    Ok(out)
}

fn normal(a: &Piece, b: &Piece, witness: u16) -> Option<[Q; 2]> {
    match witness {
        0 => Some([Q::one(), Q::zero()]),
        1 => Some([Q::zero(), Q::one()]),
        _ => {
            // Preserve legacy cubic codes 2..65. Higher-degree nets use a
            // disjoint 32-wide table; unused/invalid codes are refused.
            let (i, j) = if (2..66).contains(&witness) {
                (((witness - 2) / 8) as usize, ((witness - 2) % 8) as usize)
            } else if (256..1280).contains(&witness) {
                (
                    ((witness - 256) / 32) as usize,
                    ((witness - 256) % 32) as usize,
                )
            } else {
                return None;
            };
            let points: Vec<_> = a.points.iter().chain(&b.points).collect();
            let (p, q) = (*points.get(i)?, *points.get(j)?);
            let n = [&p[1] - &q[1], &q[0] - &p[0]];
            (!n.iter().all(Zero::is_zero)).then_some(n)
        }
    }
}

fn normal_candidates(a: &Piece, b: &Piece) -> impl Iterator<Item = u16> {
    let count = a.points.len() + b.points.len();
    (0..2).chain((0..count).flat_map(move |i| {
        (i + 1..count).map(move |j| {
            if count <= 8 {
                2 + 8 * i as u16 + j as u16
            } else {
                256 + 32 * i as u16 + j as u16
            }
        })
    }))
}

fn adjacent(p: &[Piece], i: usize, j: usize) -> bool {
    if p[i].wire != p[j].wire {
        return false;
    }
    j == i + 1
        || (p[..i].iter().all(|q| q.wire != p[i].wire)
            && p[j + 1..].iter().all(|q| q.wire != p[i].wire))
}

// Exact positive-weight hull proof. All open Bernstein coefficients are
// positive, so a nonflat net strictly leaves a supporting plane in its entire
// open parameter interval, even at a repeated endpoint or stationary cusp.
fn separated(a: &Piece, b: &Piece, n: &[Q; 2], shared: bool) -> bool {
    let project = |p: &[Q; 2]| &p[0] * &n[0] + &p[1] * &n[1];
    let x: Vec<_> = a.points.iter().map(project).collect();
    let y: Vec<_> = b.points.iter().map(project).collect();
    fn ordered(a: &Piece, b: &Piece, x: &[Q], y: &[Q], shared: bool) -> bool {
        let hi = x.iter().max().unwrap();
        let lo = y.iter().min().unwrap();
        if hi < lo {
            return !shared;
        }
        if !shared || hi != lo {
            return false;
        }
        let ae = [&a.points[0], a.points.last().unwrap()];
        let be = [&b.points[0], b.points.last().unwrap()];
        let common = ae
            .iter()
            .flat_map(|p| be.iter().map(move |q| (*p, *q)))
            .filter(|(p, q)| p == q)
            .count();
        if common == 0 || common > 2 {
            return false;
        }
        let flat_a = x.iter().all(|q| q == hi);
        let flat_b = y.iter().all(|q| q == lo);
        if flat_a && flat_b {
            return false;
        }
        // A flat span occupies the plane. The other span must have only its
        // common endpoint on it; a nonshared endpoint might otherwise hit it.
        if flat_a {
            for (p, q) in [(be[0], &y[0]), (be[1], y.last().unwrap())] {
                if !ae.contains(&p) && q == lo {
                    return false;
                }
            }
        }
        if flat_b {
            for (p, q) in [(ae[0], &x[0]), (ae[1], x.last().unwrap())] {
                if !be.contains(&p) && q == hi {
                    return false;
                }
            }
        }
        true
    }
    ordered(a, b, &x, &y, shared) || ordered(b, a, &y, &x, shared)
}

fn cross(a: &[Q; 2], b: &[Q; 2], p: &[Q; 2]) -> Q {
    (&b[0] - &a[0]) * (&p[1] - &a[1]) - (&b[1] - &a[1]) * (&p[0] - &a[0])
}
fn polygon_winding(poly: &[&[Q; 2]], point: &[Q; 2]) -> Result<i32> {
    let mut winding = 0i32;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        let c = cross(a, b, point);
        if c.is_zero()
            && (a[0].clone().min(b[0].clone()) <= point[0]
                && point[0] <= a[0].clone().max(b[0].clone()))
            && (a[1].clone().min(b[1].clone()) <= point[1]
                && point[1] <= a[1].clone().max(b[1].clone()))
        {
            return Err(refused(
                "Certificate nesting query lies on a polygon boundary",
            ));
        }
        if a[1] <= point[1] && point[1] < b[1] && c > Q::zero() {
            winding += 1;
        }
        if b[1] <= point[1] && point[1] < a[1] && c < Q::zero() {
            winding -= 1;
        }
    }
    if winding.abs() > 1 {
        return Err(refused("Certificate polygon is not simple"));
    }
    Ok(winding)
}
fn polygon_claims(pieces: &[Piece], count: usize) -> Result<(Vec<i8>, Vec<usize>)> {
    let rings: Vec<Vec<_>> = (0..count)
        .map(|w| {
            pieces
                .iter()
                .filter(|p| p.wire == w)
                .map(|p| &p.points[0])
                .collect()
        })
        .collect();
    let mut signs = Vec::new();
    let mut depths = Vec::new();
    for (w, ring) in rings.iter().enumerate() {
        let area = (0..ring.len()).fold(Q::zero(), |s, i| {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            s + &a[0] * &b[1] - &a[1] * &b[0]
        });
        signs.push(if area > Q::zero() {
            1
        } else if area < Q::zero() {
            -1
        } else {
            0
        });
        let point = ring[0];
        let mut depth = 0;
        for (other, poly) in rings.iter().enumerate() {
            if other == w {
                continue;
            }
            let winding = polygon_winding(poly, point)?;
            depth += usize::from(winding != 0);
        }
        depths.push(depth);
    }
    Ok((signs, depths))
}

/// Search-free independent checker. No numerical geometry implementation is
/// called. Every supplied witness is checked using exact integer/rational
/// arithmetic; an unproved or malformed claim returns an error.
pub fn verify(
    loops: &[Vec<Curve>],
    tolerance: f64,
    c: &ProfileCertificate,
) -> Result<VerifiedProfile> {
    let (verified, orientation) = verify_geometry(loops, tolerance, c)?;
    if orientation
        .iter()
        .zip(verified.nesting_depth())
        .any(|(s, d)| (*s == 1) != (*d % 2 == 0))
    {
        return Err(refused(
            "Certificate orientation or material-left nesting claim is false",
        ));
    }
    Ok(verified)
}
fn verify_geometry(
    loops: &[Vec<Curve>],
    tolerance: f64,
    c: &ProfileCertificate,
) -> Result<(VerifiedProfile, Vec<i8>)> {
    if c.version != VERSION || c.source != source_bits(loops, tolerance)? {
        return Err(refused(
            "Certificate version or exact source bits do not match",
        ));
    }
    let p = pieces(loops, c.subdivision_depth)?;
    if c.monotone_axes.len() != p.len()
        || c.separating_normals.len() != p.len() * p.len().saturating_sub(1) / 2
    {
        return Err(refused("Certificate omits or adds span/pair claims"));
    }
    for (piece, axis) in p.iter().zip(&c.monotone_axes) {
        if *axis > 1 || !arithmetic::monotone(&piece.net, *axis as usize)? {
            return Err(refused("Certificate monotonicity claim is false"));
        }
    }
    let mut pair = 0;
    let mut algebraic = false;
    for i in 0..p.len() {
        for j in i + 1..p.len() {
            if c.separating_normals[pair] == u16::MAX {
                // Other-wire hull separation also proves that nesting query
                // points remain outside every span throughout its homotopy.
                if p[i].wire != p[j].wire
                    || !arithmetic::endpoint_only(&p[i].net, &p[j].net, adjacent(&p, i, j))?
                {
                    return Err(refused("Certificate algebraic separation claim is false"));
                }
                algebraic = true;
            } else if c.separating_normals[pair] == u16::MAX - 1 {
                if p[i].wire != p[j].wire
                    || !arithmetic::endpoint_census(&p[i].net, &p[j].net, adjacent(&p, i, j))?
                {
                    return Err(refused("General algebraic endpoint census is false"));
                }
                algebraic = true;
            } else {
                let n = normal(&p[i], &p[j], c.separating_normals[pair])
                    .ok_or_else(|| refused("Invalid separator witness"))?;
                if !separated(&p[i], &p[j], &n, adjacent(&p, i, j)) {
                    return Err(refused("Certificate separation claim is false"));
                }
            }
            pair += 1;
        }
    }
    let (mut orientation, nesting_depth) = polygon_claims(&p, loops.len())?;
    if let Some(depth) = c.orientation_depth {
        orientation = orientation::check(loops, depth)?;
    } else if algebraic || orientation.contains(&0) {
        return Err(refused(
            "Algebraic contacts require a separate exact orientation proof",
        ));
    }
    if c.orientation != orientation || c.nesting_depth != nesting_depth {
        return Err(refused(
            "Certificate orientation or material-left nesting claim is false",
        ));
    }
    Ok((
        VerifiedProfile {
            spans: p.len() / (1usize << c.subdivision_depth),
            pieces: p.len(),
            nesting_depth,
        },
        orientation,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rect(a: [f64; 2], b: [f64; 2]) -> Vec<Curve> {
        crate::sketch::polygon_wire(vec![a, [b[0], a[1]], b, [a[0], b[1]]]).unwrap()
    }
    fn reverse(w: Vec<Curve>) -> Vec<Curve> {
        w.into_iter().rev().map(|c| c.reverse().unwrap()).collect()
    }
    #[test]
    fn exact_nested_profile_roundtrip_and_claim_tampering() {
        let loops = vec![
            rect([0., 0.], [10., 10.]),
            reverse(rect([2., 2.], [8., 8.])),
            rect([4., 4.], [6., 6.]),
        ];
        let c = certify(&loops, 1e-7).unwrap();
        assert_eq!(c.nesting_depth, vec![0, 1, 2]);
        assert_eq!(c.orientation, vec![1, -1, 1]);
        let v = verify(&loops, 1e-7, &c).unwrap();
        assert_eq!(v.spans(), 12);
        let mut bad = c.clone();
        bad.version += 1;
        assert!(verify(&loops, 1e-7, &bad).is_err());
        let mut bad = c.clone();
        bad.source.push('0');
        assert!(verify(&loops, 1e-7, &bad).is_err());
        let mut bad = c.clone();
        bad.monotone_axes[0] = 2;
        assert!(verify(&loops, 1e-7, &bad).is_err());
        let mut bad = c.clone();
        bad.separating_normals.pop();
        assert!(verify(&loops, 1e-7, &bad).is_err());
        let mut bad = c.clone();
        bad.separating_normals[0] = 65000;
        assert!(verify(&loops, 1e-7, &bad).is_err());
        let mut bad = c.clone();
        bad.orientation[0] = -1;
        assert!(verify(&loops, 1e-7, &bad).is_err());
        let mut bad = c.clone();
        bad.nesting_depth[1] = 0;
        assert!(verify(&loops, 1e-7, &bad).is_err());
        let mut bad = c.clone();
        bad.subdivision_depth = 255;
        assert!(verify(&loops, 1e-7, &bad).is_err());
        #[cfg(feature = "codec")]
        {
            let json = value_codec::to_string(&c).unwrap();
            let decoded: ProfileCertificate = value_codec::from_str(&json).unwrap();
            assert_eq!(c, decoded);
            verify(&loops, 1e-7, &decoded).unwrap();
            let mut value = value_codec::to_value(&c).unwrap();
            value["solidValid"] = value_codec::Value::Bool(true);
            assert!(value_codec::from_value::<ProfileCertificate>(value).is_err());
        }
    }
    #[test]
    fn source_binding_includes_weights_knots_tolerance_signed_zero_and_layout() {
        let loops = vec![rect([0., 0.], [10., 10.])];
        let c = certify(&loops, 1e-7).unwrap();
        let mut changed = loops.clone();
        changed[0][0].weights[0] = 2.;
        assert!(verify(&changed, 1e-7, &c).is_err());
        let mut changed = loops.clone();
        changed[0][0].knots[2] = 2.;
        changed[0][0].knots[3] = 2.;
        assert!(verify(&changed, 1e-7, &c).is_err());
        let mut changed = loops.clone();
        changed[0][0].control_points[0][0] = -0.;
        assert!(verify(&changed, 1e-7, &c).is_err());
        let mut changed = loops.clone();
        changed[0].rotate_left(1);
        assert!(verify(&changed, 1e-7, &c).is_err());
        assert!(verify(&loops, 1e-6, &c).is_err());
    }
    #[test]
    fn rational_circle_and_affine_ellipse_retain_original_definitions() {
        let mut loops = vec![crate::sketch::circle_wire(2.).unwrap()];
        for c in &mut loops[0] {
            for p in &mut c.control_points {
                p[0] = 3. * p[0] + p[1];
                p[1] *= 2.;
            }
        }
        let before = loops.clone();
        let c = certify(&loops, 1e-7).unwrap();
        verify(&loops, 1e-7, &c).unwrap();
        assert_eq!(loops, before);
    }
    #[test]
    fn empty_material_has_a_complete_empty_certificate() {
        let c = certify(&[], 1e-7).unwrap();
        assert_eq!(verify(&[], 1e-7, &c).unwrap().pieces(), 0);
        assert!(verify(&[rect([0., 0.], [1., 1.])], 1e-7, &c).is_err());
    }
    #[test]
    fn crossing_overlap_open_wire_and_wrong_winding_never_certify() {
        let bow =
            crate::sketch::polygon_wire(vec![[0., 0.], [4., 4.], [0., 4.], [4., 0.]]).unwrap();
        assert!(certify(&[bow], 1e-7).is_err());
        assert!(certify(&[rect([0., 0.], [4., 4.]), rect([2., 2.], [6., 6.])], 1e-7).is_err());
        let mut open = rect([0., 0.], [4., 4.]);
        open[0].control_points[0][0] = 1.;
        assert!(certify(&[open], 1e-7).is_err());
        assert!(certify(&[reverse(rect([0., 0.], [4., 4.]))], 1e-7).is_err());
    }
    #[test]
    fn same_side_curved_cusp_has_an_algebraic_certificate_in_both_frames() {
        let cubic = |points: Vec<Vec<f64>>| Curve {
            degree: 3,
            control_points: points,
            weights: vec![1.; 4],
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            periodic: false,
        };
        let a = cubic(vec![vec![0., 0.], vec![2., 0.], vec![4., 2.], vec![6., 6.]]);
        let b = cubic(vec![
            vec![0., 0.],
            vec![2., 0.],
            vec![6., 2.],
            vec![12., 12.],
        ]);
        for transformed in [false, true] {
            let mut wire = vec![
                b.clone(),
                Curve::from_polyline(vec![vec![12., 12.], vec![6., 6.]]).unwrap(),
                a.reverse().unwrap(),
            ];
            if transformed {
                for c in &mut wire {
                    for p in &mut c.control_points {
                        let (x, y) = (p[0], p[1]);
                        p[0] = 17. + x - y;
                        p[1] = -9. + x + y;
                    }
                }
            }
            let loops = vec![wire];
            let c = certify(&loops, 1e-7).unwrap();
            assert!(c.separating_normals.contains(&u16::MAX));
            assert_eq!(c.orientation, vec![1]);
            verify(&loops, 1e-7, &c).unwrap();
            let mut bad = c.clone();
            bad.orientation_depth = None;
            assert!(verify(&loops, 1e-7, &bad).is_err());
            if !transformed {
                let mut crossing = loops.clone();
                crossing[0][0].control_points[2][1] = 4.;
                // Simulate a malicious producer: update source binding too.
                // The algebraic census itself must reject the new crossing.
                let mut forged = c.clone();
                forged.source = source_bits(&crossing, 1e-7).unwrap();
                assert!(verify(&crossing, 1e-7, &forged).is_err());
            }
        }
    }
}
