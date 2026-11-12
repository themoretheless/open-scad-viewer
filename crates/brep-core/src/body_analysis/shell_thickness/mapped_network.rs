//! Closed normal-offset skins with independently based, rectangularly trimmed
//! caps. Original UV-window correspondence is explicit and continuously proved.
pub use super::planar_network::{Boundary, Cut, CutKind, Limits, OffsetSettings};
use crate::feature_ops::region_pattern::Verification;
use crate::{Error, Model, Result};
use nurbs_core::{
    surface::Surface,
    surface_offset::{self, CandidateCertificate, CandidateStatus},
};
/// Increasing original U/V intervals corresponding to a complete candidate cap.
pub type SourceWindow = [[f64; 2]; 2];
fn refused(s: &str) -> Error {
    Error::new("BREP_MAPPED_OFFSET_NETWORK_REFUSED", s)
}
pub struct Thickness {
    sources: Vec<Surface>,
    caps: Vec<Surface>,
    windows: Vec<SourceWindow>,
    reversed: Vec<bool>,
    distance: f64,
    normal_tolerance: f64,
    normal_limit: usize,
    sewing_tolerance: f64,
    limits: Limits,
    certificates: Vec<CandidateCertificate>,
    normal_cells: usize,
    built: super::network::Built,
}
impl Thickness {
    pub fn model(&self) -> &Model {
        &self.built.model
    }
    pub fn source_patches(&self) -> &[Surface] {
        &self.sources
    }
    pub fn cap_patches(&self) -> &[Surface] {
        &self.caps
    }
    pub fn source_windows(&self) -> &[SourceWindow] {
        &self.windows
    }
    pub fn exposed_boundaries(&self) -> &[Boundary] {
        &self.built.boundaries
    }
    pub fn cuts(&self) -> &[Cut] {
        &self.built.cuts
    }
    pub fn evidence(&self) -> &Verification {
        &self.built.evidence
    }
    pub fn normal_certificates(&self) -> &[CandidateCertificate] {
        &self.certificates
    }
    pub fn normal_cells(&self) -> usize {
        self.normal_cells
    }
    /// Whole saved caps compared to normal offsets at the corresponding
    /// original UV windows. Does not cover omitted original source parameters,
    /// or certify closest thickness, support extension, or global embedding.
    pub fn mapped_normal_offset_error_upper(&self) -> f64 {
        self.certificates
            .iter()
            .map(|c| c.error_upper.unwrap())
            .fold(0., f64::max)
    }
    pub fn exact_work(&self) -> u64 {
        self.built.exact_work
    }
    pub fn reverify(&self) -> Result<Verification> {
        let actual = thicken_with_caps(
            &self.sources,
            &self.caps,
            &self.windows,
            &self.reversed,
            OffsetSettings {
                signed_thickness: self.distance,
                normal_tolerance: self.normal_tolerance,
                normal_cells: self.normal_limit,
                sewing_tolerance: self.sewing_tolerance,
            },
            self.limits,
        )?;
        if actual.built.model != self.built.model
            || actual.built.boundaries != self.built.boundaries
            || actual.built.cuts != self.built.cuts
        {
            return Err(refused(
                "Saved mapped skin differs from original geometry/UV reconstruction",
            ));
        }
        Ok(actual.built.evidence)
    }
}
/// Independently based full Bezier caps may map onto proper rectangular source
/// subdomains. Shared cap intersections are authored proposals; each full cap
/// needs continuous original normal-offset agreement. Ruled walls align both
/// original boundary bases using the native loft owner and receive fresh full
/// lift qualification. Source caps stay literal; only exposed boundaries get
/// walls. Original/cap cuts must have compatible boundary clocks; arbitrary
/// moving T cuts and nonrectangular cap trim regions require a separate path.
pub fn thicken_with_caps(
    sources: &[Surface],
    caps: &[Surface],
    windows: &[SourceWindow],
    reversed: &[bool],
    offset: OffsetSettings,
    limits: Limits,
) -> Result<Thickness> {
    let OffsetSettings {
        signed_thickness,
        normal_tolerance,
        normal_cells,
        sewing_tolerance,
    } = offset;
    super::freeform_network::validate(sources, caps, reversed, offset, limits)?;
    if windows.len() != sources.len() {
        return Err(refused("Each cap requires its explicit original UV window"));
    }
    let mut used = 0;
    let mut certificates = Vec::with_capacity(sources.len());
    for (((source, cap), window), reverse) in sources.iter().zip(caps).zip(windows).zip(reversed) {
        let remaining = normal_cells
            .checked_sub(used)
            .filter(|n| *n > 0)
            .ok_or_else(|| refused("Shared mapped normal-offset allowance exhausted"))?;
        let certificate = surface_offset::certify_mapped_candidate(
            source,
            cap,
            *window,
            if *reverse {
                -signed_thickness
            } else {
                signed_thickness
            },
            normal_tolerance,
            remaining,
        )?;
        used += certificate.cells;
        if certificate.status != CandidateStatus::Within {
            return Err(refused(
                "Original-window normal offset exceeds tolerance or remains unresolved",
            ));
        }
        certificates.push(certificate);
    }
    let built = super::network::construct(
        sources,
        caps,
        reversed,
        signed_thickness,
        sewing_tolerance,
        limits,
        &mut 0,
    )?;
    Ok(Thickness {
        sources: sources.to_vec(),
        caps: caps.to_vec(),
        windows: windows.to_vec(),
        reversed: reversed.to_vec(),
        distance: signed_thickness,
        normal_tolerance,
        normal_limit: normal_cells,
        sewing_tolerance,
        limits,
        certificates,
        normal_cells: used,
        built,
    })
}

/// Propose native offset caps restricted to explicit original source windows.
/// Window/root discovery is separate. Shared original-corner identities propose
/// endpoint coupling only within sewing tolerance; the complete changed caps,
/// source correspondence and all saved seams are independently verified below.
#[allow(clippy::too_many_arguments)]
pub fn thicken_from_windows(sources: &[Surface], windows: &[SourceWindow], reversed: &[bool], offset: crate::shell_thickness::planar_network::OffsetSettings, limits: Limits) -> Result<Thickness> {
        let crate::shell_thickness::planar_network::OffsetSettings { signed_thickness, normal_tolerance, normal_cells, sewing_tolerance } = offset;
    super::freeform_network::validate(
        sources,
        sources,
        reversed,
        crate::shell_thickness::planar_network::OffsetSettings {
            signed_thickness,
            normal_tolerance,
            normal_cells,
            sewing_tolerance,
        },
        limits,
    )?;
    if sources.len() != windows.len() {
        return Err(refused("Each source needs an offset window"));
    }
    let mut caps = Vec::with_capacity(sources.len());
    for ((s, w), reverse) in sources.iter().zip(windows).zip(reversed) {
        let domain = [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ];
        if !(0..2).all(|k| {
            w[k].iter().all(|x| x.is_finite())
                && w[k][0] < w[k][1]
                && w[k][0] >= domain[k][0]
                && w[k][1] <= domain[k][1]
        }) {
            return Err(refused("Offset window lies outside the original source"));
        }
        let distance = if *reverse {
            -signed_thickness
        } else {
            signed_thickness
        };
        let proposal = surface_offset::offset(s, distance, normal_tolerance * 0.125);
        let surface = match proposal {
            Ok(p)
                if crate::rounded_patch_network::clamped_single(
                    &p.surface.knots_u,
                    p.surface.degree_u,
                    p.surface.control_points.len(),
                ) && crate::rounded_patch_network::clamped_single(
                    &p.surface.knots_v,
                    p.surface.degree_v,
                    p.surface.control_points[0].len(),
                ) =>
            {
                p.surface
            }
            _ => surface_offset::offset_preserving_basis(s, distance)?.surface,
        };
        caps.push(surface.trim([w[0][0], w[0][1], w[1][0], w[1][1]])?);
    }
    couple_corners(sources, &mut caps, sewing_tolerance)?;
    thicken_with_caps(
        sources,
        &caps,
        windows,
        reversed,
        OffsetSettings {
            signed_thickness,
            normal_tolerance,
            normal_cells,
            sewing_tolerance,
        },
        limits,
    )
}
fn corners(s: &Surface) -> [(usize, usize); 4] {
    let u = s.control_points.len() - 1;
    let v = s.control_points[0].len() - 1;
    [(0, 0), (u, 0), (u, v), (0, v)]
}
pub(super) fn couple_corners(sources: &[Surface], caps: &mut [Surface], tolerance: f64) -> Result<()> {
    use nurbs_core::interval_eval::Interval as I;
    let mut vertices = std::collections::BTreeMap::<[u64; 3], Vec<f64>>::new();
    for (s, cap) in sources.iter().zip(caps) {
        for ((i, j), (a, b)) in corners(s).into_iter().zip(corners(cap)) {
            let old = &s.control_points[i][j];
            let new = &mut cap.control_points[a][b];
            let key = std::array::from_fn(|k| if old[k] == 0. { 0 } else { old[k].to_bits() });
            if let Some(canonical) = vertices.get(&key) {
                let mut error = I::point(0.);
                for k in 0..3 {
                    let e = I::point(new[k]).sub(I::point(canonical[k]))?;
                    error = error.add(I::point(e.lo.abs().max(e.hi.abs())))?;
                }
                if error.hi > tolerance {
                    return Err(refused("Offset-window corner proposals do not meet"));
                }
                new.clone_from(canonical);
            } else {
                vertices.insert(key, new.clone());
            }
        }
    }
    Ok(())
}
