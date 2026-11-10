//! Exact enclosed-body removal: strict complete containment supplies a closed
//! cavity without inventing caps along an unproved intersection.
use super::*;
mod supports;
use crate::profile_certificate::{atlas_roots as roots, ExactRatio};
#[derive(Clone, Debug, PartialEq)]
pub struct EnclosedDifferenceCertificate {
    pub version: u32,
    pub container: String,
    pub tool: String,
    pub depth: u8,
    pub convex_supports: Vec<[ExactRatio; 4]>,
    pub nominal_volumes: [ExactRatio; 2],
    pub result_atlas: Certificate,
}
#[derive(Clone, Debug)]
pub struct CertifiedEnclosedDifference {
    model: Model,
    certificate: EnclosedDifferenceCertificate,
}
impl CertifiedEnclosedDifference {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn certificate(&self) -> &EnclosedDifferenceCertificate {
        &self.certificate
    }
    pub fn into_model(self) -> Model {
        self.model
    }
}
pub(super) fn cavity(container: &Model, tool: &Model) -> Result<Model> {
    let mut out = container.clone();
    let (v, e, l, f, s) = (
        out.vertices.len(),
        out.edges.len(),
        out.loops.len(),
        out.faces.len(),
        out.shells.len(),
    );
    out.vertices.extend(tool.vertices.iter().cloned());
    out.edges.extend(tool.edges.iter().cloned().map(|mut x| {
        x.vertices = x.vertices.map(|id| id + v);
        x
    }));
    out.loops.extend(tool.loops.iter().cloned().map(|mut x| {
        for use_ in &mut x.coedges {
            use_.edge += e;
        }
        x
    }));
    out.faces.extend(tool.faces.iter().cloned().map(|mut x| {
        x.outer += l;
        for id in &mut x.holes {
            *id += l;
        }
        x
    }));
    out.shells.extend(tool.shells.iter().cloned().map(|mut x| {
        for use_ in &mut x.faces {
            use_.face += f;
            use_.reversed = !use_.reversed;
        }
        x
    }));
    out.bodies[0].inner_shells.push(s);
    out.rebuild_topology_ids();
    out.validate()?;
    Ok(out)
}
pub fn certify_enclosed_difference(
    container: &Model,
    tool: &Model,
    depth: u8,
) -> Result<CertifiedEnclosedDifference> {
    let (outer, outer_maps) = checked_charts(container, depth)?;
    let (inner, inner_maps) = checked_charts(tool, depth)?;
    let volumes = [
        supports::shell(container, &outer)?,
        supports::shell(tool, &inner)?,
    ];
    let planes = supports::containment(&outer_maps, &inner_maps, &inner, &mut Work::default())?;
    let model = cavity(container, tool)?;
    // A fresh global proof checks the new closed oriented boundary, including
    // the reversed cavity shell and all its interactions with the outer shell.
    let result_atlas = derive(&model, depth)?;
    let certificate = EnclosedDifferenceCertificate {
        version: 1,
        container: source_id(&Source::Brep(container.clone()))?,
        tool: source_id(&Source::Brep(tool.clone()))?,
        depth,
        convex_supports: planes
            .iter()
            .map(|p| p.each_ref().map(roots::ratio))
            .collect(),
        nominal_volumes: volumes.each_ref().map(roots::ratio),
        result_atlas,
    };
    Ok(CertifiedEnclosedDifference { model, certificate })
}
pub fn verify_enclosed_difference(
    container: &Model,
    tool: &Model,
    model: &Model,
    cert: &EnclosedDifferenceCertificate,
) -> Result<()> {
    if cert.version != 1
        || cert.depth > 3
        || cert.container != source_id(&Source::Brep(container.clone()))?
        || cert.tool != source_id(&Source::Brep(tool.clone()))?
        || cert.result_atlas.source != source_id(&Source::Brep(model.clone()))?
    {
        return Err(fail("enclosed difference source/result binding is false"));
    }
    let expected = certify_enclosed_difference(container, tool, cert.depth)?;
    if *cert != expected.certificate || *model != expected.model {
        return Err(fail(
            "enclosed difference supports, cavity topology or result proof is false",
        ));
    }
    Ok(())
}
#[cfg(test)]
mod tests;

pub(in crate::minkowski::minkowski3d::curved) fn connected_outer_volume(m:&Model,c:&Certificate)->Result<Q>{supports::shell(m,c)}
