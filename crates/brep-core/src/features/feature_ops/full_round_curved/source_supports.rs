//! Whole source-side image equality, not merely top-rim coincidence.
//! Exact tensor factorization + complete UV rectangle + exact native profile
//! image agreement imply the entire side is a union of rim × height images.
//! Caps, normal orientation, source solid validity and material subtraction
//! remain separate obligations.
use super::*;
use crate::profile_certificate::{self as pc, ExactRatio};
use num_rational::BigRational as Q;
use num_traits::FromPrimitive;
use std::collections::{BTreeMap, BTreeSet};
pub mod lower_rims;
pub mod vertical_seams;
pub mod profiles;
mod rectangle;
fn q(v: f64) -> Result<Q> {
    Q::from_f64(v).ok_or_else(|| error("Nonfinite original support coefficient"))
}
fn ratio_value(v: &Q) -> ExactRatio {
    [v.numer().to_string(), v.denom().to_string()]
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum CurveProof {
    Parametric(pc::MultispanAgreementCertificate),
    Image(pc::CurveImageCertificate),
}
#[derive(Clone, Debug, PartialEq)]
pub struct RimBinding {
    source: [usize; 2],
    edge: usize,
    face: usize,
    window: [f64; 2],
    proof: CurveProof,
}
impl RimBinding {
    pub fn source_index(&self) -> [usize; 2] {
        self.source
    }
    pub fn edge(&self) -> usize {
        self.edge
    }
    pub fn face(&self) -> usize {
        self.face
    }
    pub fn profile_window(&self) -> [f64; 2] {
        self.window
    }
}
#[derive(Clone, Debug)]
pub struct SourceSupports {
    source: Model,
    raw: Vec<Vec<raw_profiles::RawProfileCurve>>,
    height: [f64; 2],
    faces: Vec<rectangle::FaceData>,
    bindings: Vec<RimBinding>,
}
fn contains(model: &Model, face: usize, edge: usize) -> bool {
    let f = &model.faces[face];
    std::iter::once(f.outer)
        .chain(f.holes.iter().copied())
        .any(|w| model.loops[w].coedges.iter().any(|c| c.edge == edge))
}
fn curve_proof(a: &Curve, b: &Curve, window: [f64; 2]) -> Result<CurveProof> {
    if let Ok(c) = pc::certify_multispan_agreement(a, b, a.domain(), window) {
        pc::verify_multispan_agreement(a, b, a.domain(), window, &c)?;
        return Ok(CurveProof::Parametric(c));
    }
    let c = pc::certify_curve_image_agreement(a, b, a.domain(), window)?;
    pc::verify_curve_image_agreement(a, b, a.domain(), window, &c)?;
    Ok(CurveProof::Image(c))
}
type Data = (Vec<rectangle::FaceData>, Vec<RimBinding>);
fn derive(
    model: &Model,
    raw: &[Vec<raw_profiles::RawProfileCurve>],
    height: [f64; 2],
) -> Result<Data> {
    model.validate()?;
    if model.bodies.len() != 1
        || model.shells.len() != 1
        || raw.is_empty()
        || raw.len() > 64
        || raw.iter().any(Vec::is_empty)
        || raw.iter().map(Vec::len).sum::<usize>() > 254
        || height[0] >= height[1]
        || height.iter().any(|z| !z.is_finite())
        || model
            .vertices
            .iter()
            .any(|v| v.point[2] != height[0] && v.point[2] != height[1])
    {
        return Err(error("Invalid complete source support census or height"));
    }
    let mut faces = BTreeMap::new();
    let mut bindings = Vec::new();
    let mut used = BTreeMap::<usize, BTreeSet<usize>>::new();
    for (wi, wire) in raw.iter().enumerate() {
        for (ci, rim) in wire.iter().enumerate() {
            rim.reverify(model)?;
            if model.edges[rim.edge()]
                .curve
                .control_points
                .iter()
                .any(|p| p[2] != height[1])
            {
                return Err(error("Bound rim is not on the source top height"));
            }
            let side = (0..model.faces.len())
                .filter(|&f| f != rim.face() && contains(model, f, rim.edge()))
                .collect::<Vec<_>>();
            if side.len() != 1 {
                return Err(error("Raw rim must have one unique original side owner"));
            }
            let side = side[0];
            if !faces.contains_key(&side) {
                faces.insert(side, rectangle::derive(model, side, height)?);
            }
            let data = &faces[&side];
            let f = &model.faces[side];
            let coedges = model.loops[f.outer]
                .coedges
                .iter()
                .filter(|c| c.edge == rim.edge())
                .collect::<Vec<_>>();
            if coedges.len() != 1 {
                return Err(error("Raw rim repeated on original side"));
            }
            let c = coedges[0];
            let a = &c.pcurve.control_points[0];
            let b = &c.pcurve.control_points[1];
            if a[data.axis] != data.top || b[data.axis] != data.top {
                return Err(error(
                    "Original rim pcurve is not on the actual top isocurve",
                ));
            }
            let mut window = [a[1 - data.axis], b[1 - data.axis]];
            if c.reversed ^ rim.reversed() {
                window.swap(0, 1);
            }
            let proof = curve_proof(rim.curve(), &data.profile, window)?;
            if !used.entry(side).or_default().insert(rim.edge()) {
                return Err(error("Original source rim is duplicated"));
            }
            bindings.push(RimBinding {
                source: [wi, ci],
                edge: rim.edge(),
                face: side,
                window,
                proof,
            });
        }
    }
    let mut caps = [0usize; 2];
    for (face, f) in model.faces.iter().enumerate() {
        let horizontal = f
            .surface
            .control_points
            .iter()
            .flatten()
            .all(|p| p[2] == f.surface.control_points[0][0][2]);
        if horizontal {
            let z = f.surface.control_points[0][0][2];
            let end = if z == height[0] {
                0
            } else if z == height[1] {
                1
            } else {
                return Err(error("Unexpected horizontal source carrier"));
            };
            caps[end] += 1;
        } else {
            let data = faces
                .get(&face)
                .ok_or_else(|| error("An original side surface is missing from the rim census"))?;
            if used.get(&face).unwrap().iter().copied().collect::<Vec<_>>() != data.top_edges {
                return Err(error(
                    "Source rims do not cover the whole original side profile window",
                ));
            }
        }
    }
    if caps != [1, 1] {
        return Err(error("Source support cover needs exactly two caps"));
    }
    Ok((faces.into_values().collect(), bindings))
}
pub fn qualify(
    source: &Model,
    raw: &[Vec<raw_profiles::RawProfileCurve>],
    height: [f64; 2],
) -> Result<SourceSupports> {
    let (faces, bindings) = derive(source, raw, height)?;
    Ok(SourceSupports {
        source: source.clone(),
        raw: raw.to_vec(),
        height,
        faces,
        bindings,
    })
}
impl SourceSupports {
    pub fn source(&self) -> &Model {
        &self.source
    }
    pub fn bindings(&self) -> &[RimBinding] {
        &self.bindings
    }
    pub fn side_count(&self) -> usize {
        self.faces.len()
    }
    pub fn extrusion_axis(&self, face: usize) -> Option<usize> {
        self.faces.iter().find(|f| f.face == face).map(|f| f.axis)
    }
    pub fn vertical_weight_ratio(&self, face: usize) -> Option<&ExactRatio> {
        self.faces.iter().find(|f| f.face == face).map(|f| &f.ratio)
    }
    pub fn reverify(&self) -> Result<()> {
        let (faces, bindings) = derive(&self.source, &self.raw, self.height)?;
        if faces != self.faces || bindings != self.bindings {
            return Err(error("Whole source support replay changed"));
        }
        Ok(())
    }
}
impl CurvedFullRound {
    pub fn qualify_source_supports(&self) -> Result<SourceSupports> {
        let low = self
            .source()
            .vertices
            .iter()
            .map(|v| v.point[2])
            .fold(f64::INFINITY, f64::min);
        let high = self
            .source()
            .vertices
            .iter()
            .map(|v| v.point[2])
            .fold(f64::NEG_INFINITY, f64::max);
        qualify(self.source(), self.raw_profile_sources(), [low, high])
    }
}
