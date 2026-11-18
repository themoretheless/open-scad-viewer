//! Complete algebraic branch strata of two independently parameterized graph
//! carriers. Original denominators and parameter-footprint guards are retained.
use super::*;
use crate::profile_certificate::{
    AlgebraicBranchCertificate, VerifiedAlgebraicBranches, certify_algebraic_branches,
};
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceAlgebraicBranchCertificate {
    pub version: u32,
    pub sources: [String; 2],
    pub normalized_parameter_maps: [[[ExactRatio; 3]; 2]; 2],
    pub branches: AlgebraicBranchCertificate,
}
#[derive(Clone, Debug)]
pub struct CertifiedSurfaceAlgebraicBranches {
    carriers: [Surface; 2],
    certificate: SurfaceAlgebraicBranchCertificate,
    branches: VerifiedAlgebraicBranches,
}
impl CertifiedSurfaceAlgebraicBranches {
    pub fn carriers(&self) -> &[Surface; 2] {
        &self.carriers
    }
    pub fn certificate(&self) -> &SurfaceAlgebraicBranchCertificate {
        &self.certificate
    }
    pub fn branches(&self) -> &VerifiedAlgebraicBranches {
        &self.branches
    }
    pub fn reverify(&self) -> Result<()> {
        verify_surface_algebraic_branches(&self.carriers[0], &self.carriers[1], &self.certificate)
            .map(|_| ())
    }
}
pub fn certify_surface_algebraic_branches(
    first: &Surface,
    second: &Surface,
) -> Result<CertifiedSurfaceAlgebraicBranches> {
    let mut work = Work::default();
    let aligned = align(first, second, &mut work)?;
    for affine in &aligned.maps[1] {
        let fourth = &affine[0] + &affine[1] + &affine[2];
        if fourth < Q::zero() || fourth > Q::one() {
            return Err(fail(
                "Full original surface product needs footprint clipping before algebraic branch ownership",
            ));
        }
    }
    let equation = sum(
        &product(&aligned.powers[0][2], &aligned.powers[1][3], &mut work)?,
        &scale(
            &product(&aligned.powers[1][2], &aligned.powers[0][3], &mut work)?,
            &-Q::one(),
        ),
    )?;
    let branches = certify_algebraic_branches(&encode(&equation))?;
    let certificate = SurfaceAlgebraicBranchCertificate {
        version: 1,
        sources: [source(first)?, source(second)?],
        normalized_parameter_maps: aligned
            .maps
            .each_ref()
            .map(|m| m.each_ref().map(|r| r.each_ref().map(roots::ratio))),
        branches: branches.certificate().clone(),
    };
    Ok(CertifiedSurfaceAlgebraicBranches {
        carriers: [first.clone(), second.clone()],
        certificate,
        branches,
    })
}
pub fn verify_surface_algebraic_branches(
    first: &Surface,
    second: &Surface,
    c: &SurfaceAlgebraicBranchCertificate,
) -> Result<CertifiedSurfaceAlgebraicBranches> {
    if c.version != 1 {
        return Err(fail("Surface branch certificate version mismatch"));
    }
    let fresh = certify_surface_algebraic_branches(first, second)?;
    if fresh.certificate != *c {
        return Err(fail(
            "Original NURBS carriers, maps, branch multiplicities or ports changed",
        ));
    }
    Ok(fresh)
}
