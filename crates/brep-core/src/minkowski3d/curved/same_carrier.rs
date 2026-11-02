//! Source-exact removal of artificial seams on one identical NURBS carrier.
//! Different carriers or nonlinear changes of UV coordinates are not inferred.
use super::*;
mod plan;
pub(crate) use plan::component as same_carrier_component;
#[derive(Clone, Debug, PartialEq)]
pub struct SameCarrierMergeCertificate {
    pub version: u32,
    pub source: String,
    pub result: String,
    pub selected: Vec<usize>,
    pub removed_edges: Vec<usize>,
    pub boundary_edges: Vec<Vec<usize>>,
    pub original_profiles: Vec<crate::profile_certificate::ProfileCertificate>,
    pub merged_profile: crate::profile_certificate::ProfileCertificate,
    pub source_atlas: SourceAtlasCertificate,
    pub result_atlas: SourceAtlasCertificate,
    pub boundary_uses: usize,
    pub boundary_cells: usize,
    pub replacement_cells: usize,
    pub exact_work: u64,
    pub result_face: usize,
    pub depth: u8,
}
#[derive(Clone, Debug)]
pub struct CertifiedSameCarrierMerge {
    model: Model,
    certificate: SameCarrierMergeCertificate,
}
impl CertifiedSameCarrierMerge {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn certificate(&self) -> &SameCarrierMergeCertificate {
        &self.certificate
    }
    pub fn into_model(self) -> Model {
        self.model
    }
}
/// Proves equality of filled regions by exact oriented UV-chain cancellation.
/// Input/result atlases prove embedding; complete original profile certificates
/// prove material-left winding. A canceled pair has the SAME entire rational UV
/// image with opposite orientation, not merely nearby endpoints. Consequently
/// the new winding is exactly the sum of original windings at every off-boundary
/// point. No carrier, remaining edge curve, tolerance or material is modified.
pub fn certify_same_carrier_merge(
    source: &Model,
    faces: &[usize],
    depth: u8,
) -> Result<CertifiedSameCarrierMerge> {
    source.validate()?;
    let selected = faces
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if selected.len() != faces.len()
        || !(2..=64).contains(&selected.len())
        || source.faces.len() > 256
        || selected.iter().any(|&i| i >= source.faces.len())
        || depth > 3
    {
        return Err(fail(
            "same-carrier merge needs 2..64 distinct faces and bounded source/depth",
        ));
    }
    let boundary = atlas::boolean::source_boundary(source)?;
    let source_atlas = certify_source_atlas(source, depth)?;
    let tol = 1e-8;
    let curves = |c: &[crate::Coedge]| c.iter().map(|c| c.pcurve.clone()).collect::<Vec<_>>();
    let mut original_profiles = Vec::new();
    for &i in &selected {
        let f = &source.faces[i];
        let loops = std::iter::once(f.outer)
            .chain(f.holes.iter().copied())
            .map(|w| curves(&source.loops[w].coedges))
            .collect::<Vec<_>>();
        let proof = crate::profile_certificate::certify(&loops, tol)?;
        if proof.orientation[0] != 1
            || proof.nesting_depth[0] != 0
            || proof.orientation.iter().skip(1).any(|&o| o != -1)
            || proof.nesting_depth.iter().skip(1).any(|&d| d != 1)
        {
            return Err(fail("original face UV material-left regions are unproved"));
        }
        original_profiles.push(proof);
    }
    let (mut loops, removed_edges) = plan::boundary(source, &selected)?;
    let profile = crate::profile_certificate::certify(
        &loops.iter().map(|c| curves(c)).collect::<Vec<_>>(),
        tol,
    )?;
    let outer = profile
        .nesting_depth
        .iter()
        .enumerate()
        .filter(|(_, d)| **d == 0)
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    if outer.len() != 1 || profile.nesting_depth.iter().any(|&d| d > 1) {
        return Err(fail(
            "merged UV domain needs exactly one outer ring and disjoint holes",
        ));
    }
    loops.swap(0, outer[0]);
    let merged_profile = crate::profile_certificate::certify(
        &loops.iter().map(|c| curves(c)).collect::<Vec<_>>(),
        tol,
    )?;
    let boundary_edges = loops
        .iter()
        .map(|l| l.iter().map(|c| c.edge).collect())
        .collect();
    let reversed = plan::owner(source, selected[0])?.1;
    let replaced = crate::region_replacement::replace(
        source,
        &selected,
        &source.faces[selected[0]].surface,
        &loops,
        reversed,
        tol,
        100000,
        1000000,
    )?;
    let result_atlas = certify_source_atlas(&replaced.model, depth)?;
    let certificate = SameCarrierMergeCertificate {
        version: 1,
        source: source_id(&Source::Brep(source.clone()))?,
        result: source_id(&Source::Brep(replaced.model.clone()))?,
        selected,
        removed_edges,
        boundary_edges,
        original_profiles,
        merged_profile,
        source_atlas,
        result_atlas,
        boundary_uses: boundary.0,
        boundary_cells: boundary.1,
        replacement_cells: replaced.cells,
        exact_work: replaced.exact_work,
        result_face: replaced.face,
        depth,
    };
    Ok(CertifiedSameCarrierMerge {
        model: replaced.model,
        certificate,
    })
}
pub fn verify_same_carrier_merge(
    source: &Model,
    result: &Model,
    cert: &SameCarrierMergeCertificate,
) -> Result<()> {
    if cert.version != 1 {
        return Err(fail("same-carrier certificate version mismatch"));
    }
    let fresh = certify_same_carrier_merge(source, &cert.selected, cert.depth)?;
    if fresh.model != *result || fresh.certificate != *cert {
        return Err(fail(
            "same-carrier source, boundary cancellation or material certificate changed",
        ));
    }
    Ok(())
}

pub(crate) fn seam_model_id(m: &Model) -> Result<String> {
    source_id(&Source::Brep(m.clone()))
}
