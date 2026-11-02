//! Complete original-carrier boundary authority for the extrusion family.
//! Cap UV auxiliaries are explicitly canonicalized; the original Model and
//! legacy pcurves remain unchanged. No rounded Full Round result is admitted.
use super::*;
pub mod material;
use crate::profile_certificate::{
    original_restriction::{self as exact, wire::regularity::SourceRegularity},
    ExactRatio,
};
use num_rational::BigRational as Q;
use source_caps::pullbacks::CanonicalPullbacks;
use source_supports::{profiles::NativeProfiles, vertical_seams::VerticalSeams, SourceSupports};
#[derive(Clone, Debug)]
pub struct SourceBoundary {
    profiles: NativeProfiles,
    vertical: VerticalSeams,
    caps: CanonicalPullbacks,
    cap_regularity: Vec<Vec<SourceRegularity>>,
    edge_uses: Vec<[[usize; 3]; 2]>,
}
fn q(x: f64) -> Result<Q> {
    Q::from_float(x).ok_or_else(|| error("Nonfinite source boundary coordinate"))
}
fn decode(x: &ExactRatio) -> Result<Q> {
    Ok(Q::new(
        x[0].parse()
            .map_err(|_| error("Invalid boundary numerator"))?,
        x[1].parse()
            .map_err(|_| error("Invalid boundary denominator"))?,
    ))
}
fn incidence(model: &Model) -> Result<Vec<[[usize; 3]; 2]>> {
    let mut uses = vec![Vec::new(); model.edges.len()];
    let mut faces = BTreeSet::new();
    for face_use in &model.shells[0].faces {
        if !faces.insert(face_use.face) {
            return Err(error("Repeated native shell face"));
        }
        let face = &model.faces[face_use.face];
        for w in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            for (ci, c) in model.loops[w].coedges.iter().enumerate() {
                uses[c.edge].push(([face_use.face, w, ci], c.reversed ^ face_use.reversed));
            }
        }
    }
    if faces != (0..model.faces.len()).collect() {
        return Err(error("Native shell face census incomplete"));
    }
    uses.into_iter()
        .map(|u| {
            if u.len() != 2 || u[0].1 == u[1].1 || u[0].0[0] == u[1].0[0] {
                return Err(error(
                    "Native boundary edge lacks two opposite distinct face uses",
                ));
            }
            Ok([u[0].0, u[1].0])
        })
        .collect()
}
fn cap_regularity(caps: &CanonicalPullbacks) -> Result<Vec<Vec<SourceRegularity>>> {
    let model = caps.original().source();
    let mut regularity = Vec::new();
    for cap in caps.caps() {
        let z = q(model.faces[cap.face()].surface.control_points[0][0][2])?;
        for b in cap.boundaries() {
            let [w, ci] = b.coedge();
            let c = &model.loops[w].coedges[ci];
            if c.edge != b.edge() {
                return Err(error("Canonical cap boundary index changed"));
            }
            let edge = &model.edges[c.edge];
            let curve = b.inverse().world_curve();
            let rim = exact::restrict_binary64(curve, curve.domain())?;
            for (i, t) in rim.parameter_range().into_iter().enumerate() {
                let xy = rim.evaluate_natural(t)?;
                let v = &model.vertices[edge.vertices[i ^ usize::from(c.reversed)]].point;
                if q(v[0])? != decode(&xy[0])? || q(v[1])? != decode(&xy[1])? || q(v[2])? != z {
                    return Err(error(
                        "Native horizontal vertex differs from exact rim endpoint",
                    ));
                }
            }
        }
        regularity.push(
            cap.world_region()
                .jordan()
                .sources()
                .iter()
                .map(|w| w.qualify_source_regularity())
                .collect::<Result<Vec<_>>>()?,
        );
    }
    Ok(regularity)
}
pub fn qualify(supports: &SourceSupports) -> Result<SourceBoundary> {
    let profiles = supports.qualify_native_profiles()?;
    let vertical = supports.qualify_lower_rims()?.qualify_vertical_seams()?;
    let caps = source_caps::qualify(supports.source())?.canonical_pullbacks()?;
    let regularity = cap_regularity(&caps)?;
    let mut faces = profiles
        .oriented_faces()
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    for cap in caps.caps() {
        if !faces.insert(cap.face()) {
            return Err(error("Native cap/side face census overlaps"));
        }
    }
    if faces != (0..supports.source().faces.len()).collect() {
        return Err(error("Native cap/side boundary face census incomplete"));
    }
    let edge_uses = incidence(supports.source())?;
    Ok(SourceBoundary {
        profiles,
        vertical,
        caps,
        cap_regularity: regularity,
        edge_uses,
    })
}
impl SourceBoundary {
    pub fn source(&self) -> &Model {
        self.profiles.supports().source()
    }
    pub fn profiles(&self) -> &NativeProfiles {
        &self.profiles
    }
    pub fn vertical_seams(&self) -> &VerticalSeams {
        &self.vertical
    }
    pub fn caps(&self) -> &CanonicalPullbacks {
        &self.caps
    }
    pub fn cap_regularity(&self) -> &[Vec<SourceRegularity>] {
        &self.cap_regularity
    }
    pub fn edge_uses(&self) -> &[[[usize; 3]; 2]] {
        &self.edge_uses
    }
    pub fn reverify(&self) -> Result<()> {
        self.profiles.reverify()?;
        self.vertical.reverify()?;
        self.caps.reverify()?;
        if self.vertical.lower_rims().supports().source() != self.source()
            || self.caps.original().source() != self.source()
            || incidence(self.source())? != self.edge_uses
        {
            return Err(error(
                "Original boundary source ownership/incidence changed",
            ));
        }
        let expected = cap_regularity(&self.caps)?;
        if expected.len() != self.cap_regularity.len() {
            return Err(error("Native cap regularity census changed"));
        }
        for (a, b) in self.cap_regularity.iter().zip(expected) {
            if a.len() != b.len() {
                return Err(error("Native cap loop census changed"));
            }
            for (a, b) in a.iter().zip(b) {
                a.reverify()?;
                if a.span_count() != b.span_count() || a.internal_joins() != b.internal_joins() {
                    return Err(error("Native world rim regularity changed"));
                }
            }
        }
        Ok(())
    }
}
impl CurvedFullRound {
    pub fn qualify_source_boundary(&self) -> Result<SourceBoundary> {
        qualify(&self.qualify_source_supports()?)
    }
}
