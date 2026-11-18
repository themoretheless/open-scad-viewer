//! Source-exact removal of artificial seams on one identical NURBS carrier.
//! Different carriers or nonlinear changes of UV coordinates are not inferred.
use super::*;
mod plan;
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

#[derive(Clone, Debug, PartialEq)]
pub enum BoundedCarrierProof {
    Identical,
    EquivalentUv(crate::boolean_robustness::UvEquivalence),
    SameParameterization { max_pole_deviation_mm: f64 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct BoundedCarrierMergeCertificate {
    pub version: u32,
    pub source: String,
    pub carriers: Vec<(usize, BoundedCarrierProof)>,
    pub exact: SameCarrierMergeCertificate,
}

#[derive(Clone, Debug)]
pub struct CertifiedBoundedCarrierMerge {
    model: Model,
    certificate: BoundedCarrierMergeCertificate,
}

impl CertifiedBoundedCarrierMerge {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn certificate(&self) -> &BoundedCarrierMergeCertificate {
        &self.certificate
    }
    pub fn into_model(self) -> Model {
        self.model
    }
}

fn same_parameterization(a: &crate::Surface, b: &crate::Surface, tolerance: f64) -> Option<f64> {
    if a.degree_u != b.degree_u
        || a.degree_v != b.degree_v
        || a.knots_u != b.knots_u
        || a.knots_v != b.knots_v
        || a.weights != b.weights
        || a.periodic_u != b.periodic_u
        || a.periodic_v != b.periodic_v
        || a.control_points.len() != b.control_points.len()
        || a.control_points
            .iter()
            .zip(&b.control_points)
            .any(|(x, y)| x.len() != y.len())
    {
        return None;
    }
    let deviation = a
        .control_points
        .iter()
        .flatten()
        .zip(b.control_points.iter().flatten())
        .map(|(x, y)| {
            if x.len() != y.len() || x.iter().chain(y).any(|v| !v.is_finite()) {
                return f64::INFINITY;
            }
            x.iter()
                .zip(y)
                .map(|(x, y)| (x - y) * (x - y))
                .sum::<f64>()
                .sqrt()
        })
        .fold(0., f64::max);
    (deviation <= tolerance).then_some(deviation)
}

fn stage_equivalent_carriers(
    source: &Model,
    faces: &[usize],
) -> Result<(Model, Vec<(usize, BoundedCarrierProof)>)> {
    source.validate()?;
    let first = *faces
        .first()
        .ok_or_else(|| fail("bounded carrier merge needs faces"))?;
    let canonical = source
        .faces
        .get(first)
        .ok_or_else(|| fail("missing bounded carrier"))?
        .surface
        .clone();
    let mut staged = source.clone();
    let mut proofs = Vec::new();
    for &index in faces {
        let face = source
            .faces
            .get(index)
            .ok_or_else(|| fail("missing bounded carrier"))?;
        if face.surface == canonical {
            proofs.push((index, BoundedCarrierProof::Identical));
            continue;
        }
        if let Some(map) =
            crate::boolean_robustness::certify_nurbs_uv_equivalence(&canonical, &face.surface)
        {
            for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                for coedge in &mut staged.loops[wire].coedges {
                    for uv in &mut coedge.pcurve.control_points {
                        let point = <[f64; 2]>::try_from(uv.as_slice()).map_err(|_| {
                            fail("equivalent-carrier pcurve is not two-dimensional")
                        })?;
                        *uv = map
                            .second_to_first(&face.surface, &canonical, point)
                            .to_vec();
                    }
                }
            }
            staged.faces[index].surface = canonical.clone();
            proofs.push((index, BoundedCarrierProof::EquivalentUv(map)));
            continue;
        }
        let tolerance = source.tolerance_mm;
        if let Some(deviation) = same_parameterization(&canonical, &face.surface, tolerance) {
            // Corresponding poles bound every rational surface point by the
            // convex-hull property. Parameters and weights are literally equal.
            staged.faces[index].surface = canonical.clone();
            proofs.push((
                index,
                BoundedCarrierProof::SameParameterization {
                    max_pole_deviation_mm: deviation,
                },
            ));
            continue;
        }
        return Err(fail(
            "bounded carrier merge lacks an exact UV map or bounded identical parameterization",
        ));
    }
    Ok((staged, proofs))
}

pub fn bounded_component(source: &Model, seed: usize) -> Result<Vec<usize>> {
    let face = source
        .faces
        .get(seed)
        .ok_or_else(|| fail("missing bounded carrier seed"))?;
    let owned = plan::owner(source, seed)?;
    let candidates = (0..source.faces.len())
        .filter(|&i| {
            plan::owner(source, i).ok() == Some(owned)
                && (source.faces[i].surface == face.surface
                    || crate::boolean_robustness::certify_nurbs_uv_equivalence(
                        &face.surface,
                        &source.faces[i].surface,
                    )
                    .is_some()
                    || same_parameterization(
                        &face.surface,
                        &source.faces[i].surface,
                        source.tolerance_mm,
                    )
                    .is_some())
        })
        .collect::<Vec<_>>();
    let edges = |i: usize| {
        std::iter::once(source.faces[i].outer)
            .chain(source.faces[i].holes.iter().copied())
            .flat_map(|w| source.loops[w].coedges.iter().map(|c| c.edge))
            .collect::<BTreeSet<_>>()
    };
    let mut seen = BTreeSet::new();
    let mut pending = vec![seed];
    while let Some(i) = pending.pop() {
        if seen.insert(i) {
            let own = edges(i);
            pending.extend(
                candidates
                    .iter()
                    .copied()
                    .filter(|&j| !seen.contains(&j) && !own.is_disjoint(&edges(j))),
            );
        }
    }
    Ok(seen.into_iter().collect())
}

pub fn certify_bounded_same_carrier_merge(
    source: &Model,
    faces: &[usize],
    depth: u8,
) -> Result<CertifiedBoundedCarrierMerge> {
    let (staged, carriers) = stage_equivalent_carriers(source, faces)?;
    let exact = certify_same_carrier_merge(&staged, faces, depth)?;
    let certificate = BoundedCarrierMergeCertificate {
        version: 1,
        source: source_id(&Source::Brep(source.clone()))?,
        carriers,
        exact: exact.certificate().clone(),
    };
    Ok(CertifiedBoundedCarrierMerge {
        model: exact.into_model(),
        certificate,
    })
}

pub fn verify_bounded_same_carrier_merge(
    source: &Model,
    result: &Model,
    certificate: &BoundedCarrierMergeCertificate,
) -> Result<()> {
    if certificate.version != 1 {
        return Err(fail("bounded-carrier certificate version mismatch"));
    }
    let fresh = certify_bounded_same_carrier_merge(
        source,
        &certificate.exact.selected,
        certificate.exact.depth,
    )?;
    if fresh.model != *result || fresh.certificate != *certificate {
        return Err(fail(
            "bounded-carrier source, UV evidence, merge or result changed",
        ));
    }
    Ok(())
}

pub(crate) fn seam_model_id(m: &Model) -> Result<String> {
    source_id(&Source::Brep(m.clone()))
}
