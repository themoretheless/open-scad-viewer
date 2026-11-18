//! Bind normalized solver hints to actual indexed source coedges. Source
//! geometry is never normalized: reversal is exact t -> -t, and interval
//! transfer uses rational arithmetic. The hint is only a proposal, not proof
//! of exact geometric equality after floating knot normalization.
use super::*;
use crate::profile_certificate::{
    ExactRatio,
    original_restriction::{self, Restriction},
};
use num_rational::BigRational as Q;
use num_traits::FromPrimitive;
#[derive(Clone, Debug)]
pub struct RawProfileCurve {
    face: usize,
    face_reversed: bool,
    coedge: [usize; 2],
    edge: usize,
    reversed: bool,
    trim: Option<brep_topology::CoedgeTrim>,
    z: f64,
    curve: Curve,
    hint_domain: [f64; 2],
}
pub(super) fn oriented(model: &Model, edge: usize, reversed: bool, z: f64) -> Result<Curve> {
    let mut curve = model
        .edges
        .get(edge)
        .ok_or_else(|| error("Unknown raw profile edge"))?
        .curve
        .clone();
    if curve
        .control_points
        .iter()
        .any(|p| p.len() != 3 || p[2] != z)
    {
        return Err(error("Raw source rim is not on its actual cap plane"));
    }
    for p in &mut curve.control_points {
        p.truncate(2);
    }
    if reversed {
        curve.control_points.reverse();
        curve.weights.reverse();
        curve.knots = curve.knots.iter().rev().map(|k| -k).collect();
    }
    Ok(curve)
}
fn rational(v: f64) -> Result<Q> {
    Q::from_f64(v).ok_or_else(|| error("Nonfinite source parameter map"))
}
fn ratio(q: &Q) -> ExactRatio {
    [q.numer().to_string(), q.denom().to_string()]
}
impl RawProfileCurve {
    pub fn edge(&self) -> usize {
        self.edge
    }
    pub fn coedge(&self) -> [usize; 2] {
        self.coedge
    }
    pub fn face(&self) -> usize {
        self.face
    }
    pub fn reversed(&self) -> bool {
        self.reversed
    }
    pub fn curve(&self) -> &Curve {
        &self.curve
    }
    pub fn hint_domain(&self) -> [f64; 2] {
        self.hint_domain
    }
    pub fn belongs_to_face(&self, model: &Model, face: usize) -> Result<bool> {
        let f = model
            .faces
            .get(face)
            .ok_or_else(|| error("Unknown Full Round selected face"))?;
        for w in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            let w = model
                .loops
                .get(w)
                .ok_or_else(|| error("Unknown source face loop"))?;
            if w.coedges.iter().any(|c| c.edge == self.edge) {
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub fn reverify(&self, model: &Model) -> Result<()> {
        let f = model
            .faces
            .get(self.face)
            .ok_or_else(|| error("Unknown bound source cap"))?;
        if !std::iter::once(f.outer)
            .chain(f.holes.iter().copied())
            .any(|w| w == self.coedge[0])
            || !model
                .shells
                .iter()
                .flat_map(|s| &s.faces)
                .any(|u| u.face == self.face && u.reversed == self.face_reversed)
        {
            return Err(error("Raw source coedge cap ownership changed"));
        }
        let c = model
            .loops
            .get(self.coedge[0])
            .and_then(|w| w.coedges.get(self.coedge[1]))
            .ok_or_else(|| error("Unknown bound raw coedge"))?;
        if c.edge != self.edge
            || (c.reversed ^ self.face_reversed) != self.reversed
            || c.trim() != self.trim
            || oriented(model, self.edge, self.reversed, self.z)? != self.curve
        {
            return Err(error("Raw source rim definition or traversal changed"));
        }
        Ok(())
    }
    pub fn restrict_hint(&self, range: [f64; 2]) -> Result<Restriction> {
        let h = self
            .hint_domain
            .map(rational)
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        let d = self
            .curve
            .domain()
            .map(rational)
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        if h[0] >= h[1] {
            return Err(error("Invalid solver hint domain"));
        }
        let mut mapped = Vec::new();
        for t in range {
            let t = rational(t)?;
            if t < h[0] || t > h[1] {
                return Err(error("Contact hint leaves its original parameter interval"));
            }
            mapped.push(ratio(
                &(&d[0] + (t - &h[0]) / (&h[1] - &h[0]) * (&d[1] - &d[0])),
            ));
        }
        original_restriction::restrict(&self.curve, [mapped.remove(0), mapped.remove(0)])
    }
    pub fn full(&self) -> Result<Restriction> {
        original_restriction::restrict_binary64(&self.curve, self.curve.domain())
    }
}
pub(super) fn bind(
    model: &Model,
    z: f64,
    hints: &[Vec<Curve>],
) -> Result<Vec<Vec<RawProfileCurve>>> {
    let caps = model
        .shells
        .iter()
        .flat_map(|s| &s.faces)
        .filter(|u| {
            model.faces[u.face]
                .surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == z)
        })
        .collect::<Vec<_>>();
    if caps.len() != 1 {
        return Err(error("Raw source binding needs one actual top cap"));
    }
    let use_ = caps[0];
    let face = &model.faces[use_.face];
    let loops = std::iter::once(face.outer)
        .chain(face.holes.iter().copied())
        .collect::<Vec<_>>();
    if loops.len() != hints.len() {
        return Err(error("Recognized/source cap loop census differs"));
    }
    let mut out = Vec::new();
    for (w, hints) in loops.into_iter().zip(hints) {
        let mut coedges = model.loops[w]
            .coedges
            .iter()
            .enumerate()
            .collect::<Vec<_>>();
        if use_.reversed {
            coedges.reverse();
        }
        if coedges.len() != hints.len() {
            return Err(error("Recognized/source coedge census differs"));
        }
        let mut bound = Vec::new();
        for ((ci, c), hint) in coedges.into_iter().zip(hints) {
            let reversed = c.reversed ^ use_.reversed;
            let curve = oriented(model, c.edge, reversed, z)?;
            if curve.degree != hint.degree
                || curve.control_points != hint.control_points
                || curve.weights != hint.weights
                || curve.knots.len() != hint.knots.len()
                || curve.periodic != hint.periodic
            {
                return Err(error("Solver hint does not own the indexed original curve"));
            }
            let owner = RawProfileCurve {
                face: use_.face,
                face_reversed: use_.reversed,
                coedge: [w, ci],
                edge: c.edge,
                reversed,
                trim: c.trim(),
                z,
                curve,
                hint_domain: hint.domain(),
            };
            owner.reverify(model)?;
            bound.push(owner);
        }
        out.push(bound);
    }
    Ok(out)
}
