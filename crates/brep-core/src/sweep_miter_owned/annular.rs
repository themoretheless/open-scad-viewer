//! Source-owned annular cap mode. The original section/wall constructor is
//! retained; new filled-cap error and logical face groups belong to this result.
use super::{BoundaryBody, DirectRequest, Limits, Request};
use crate::{
    Model, Result, invalid,
    retained_annular_cap::{self, source_filling},
};
use nurbs_core::{
    curve::Curve,
    progressive_miter::{self, Sweep},
    sweeps::filled_cap_error::{self, BoundaryCertificate},
};

pub struct Boundary {
    model: Model,
    source: BoundaryBody,
    cap_face_groups: [Vec<usize>; 2],
    filled_caps: Option<source_filling::Report>,
    boundary: BoundaryCertificate,
    cap_construction_work: u64,
}
pub struct Smoothness {
    pub walls: crate::sweep_smoothness::ProfileReport,
    /// Applicable internal seams of each multi-face end cap. Cap/wall joins
    /// remain explicit C0 and are not included in these smoothness claims.
    pub cap_seams: Vec<crate::sweep_smoothness::AxisReport>,
    pub exact_work: u64,
}
impl Boundary {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn source(&self) -> &BoundaryBody {
        &self.source
    }
    pub fn cap_face_groups(&self) -> &[Vec<usize>; 2] {
        &self.cap_face_groups
    }
    pub fn cap_faces(&self) -> Vec<usize> {
        self.cap_face_groups.iter().flatten().copied().collect()
    }
    pub fn filled_caps(&self) -> Option<&source_filling::Report> {
        self.filled_caps.as_ref()
    }
    pub fn cap_construction_work(&self) -> u64 {
        self.cap_construction_work
    }
    pub fn annular_exact_work(&self) -> u64 {
        self.cap_construction_work + self.filled_caps.as_ref().map_or(0, |r| r.exact_work)
    }
    pub fn boundary(&self) -> &BoundaryCertificate {
        &self.boundary
    }
    /// A source-error certificate is not a global material/Solid certificate.
    pub fn complete_boundary(&self) -> Option<&BoundaryCertificate> {
        (self.boundary.continuous_bound && self.boundary.within_budget == Some(true))
            .then_some(&self.boundary)
    }
    /// Audit actual retained wall and internal cap jets with one caller-owned
    /// work budget. Source frame regularity never substitutes for these jets.
    pub fn inspect_smoothness(&self, max_work: u64) -> Result<Smoothness> {
        let walls =
            crate::sweep_smoothness::inspect_profile(&self.model, &self.cap_faces(), max_work)?;
        let mut work = walls.total_exact_work;
        let mut cap_seams = Vec::new();
        for group in &self.cap_face_groups {
            if group.len() < 2 {
                continue;
            }
            let excluded = (0..self.model.faces.len())
                .filter(|i| !group.contains(i))
                .collect::<Vec<_>>();
            let report = crate::sweep_smoothness::inspect_axis(
                &self.model,
                &excluded,
                crate::sweep_smoothness::Axis::Profile,
                max_work - work,
            )?;
            work += report.exact_work;
            cap_seams.push(report);
        }
        Ok(Smoothness {
            walls,
            cap_seams,
            exact_work: work,
        })
    }
}

fn finish(
    source: BoundaryBody,
    sweep: &Sweep<'_>,
    ring_sizes: &[usize],
    limits: &Limits,
) -> Result<Boundary> {
    let raw = source.model();
    if source.closed() || ring_sizes.len() != 2 || !source.caps_deferred {
        let groups = if source.closed() {
            [vec![], vec![]]
        } else {
            [vec![raw.faces.len() - 2], vec![raw.faces.len() - 1]]
        };
        let boundary = filled_cap_error::compose_boundary(
            source.boundary().wall_error_upper,
            source.boundary().filled_cap_error_upper,
            source.closed(),
            Some(source.budget()),
        );
        let cap_construction_work = source.cap_probe_work;
        return Ok(Boundary {
            model: raw.clone(),
            source,
            cap_face_groups: groups,
            filled_caps: None,
            boundary,
            cap_construction_work,
        });
    }
    let first_id = raw.faces.len() - 2;
    let last_id = raw.faces.len() - 1;
    let first = retained_annular_cap::replace(
        raw,
        first_id,
        limits.cap_regions.max_exact_work - source.cap_probe_work,
    )?;
    let last = retained_annular_cap::replace(
        &first.model,
        first.source_face_map[last_id].unwrap(),
        limits.cap_regions.max_exact_work - source.cap_probe_work - first.exact_work,
    )?;
    let construction_work = source.cap_probe_work + first.exact_work + last.exact_work;
    let groups = [
        first
            .cap_faces
            .iter()
            .map(|&f| last.source_face_map[f].unwrap())
            .collect(),
        last.cap_faces,
    ];
    // Both replacements operate on the terminal source faces. Every source
    // wall has exactly the same coefficients, trim, index and boundary edges.
    if last.model.faces[..first_id] != raw.faces[..first_id]
        || last.model.edges[..raw.edges.len()] != raw.edges[..]
        || last.model.vertices != raw.vertices
    {
        return Err(invalid("Annular mode changed source-owned retained walls"));
    }
    let remaining = limits.cap_regions.max_exact_work - construction_work;
    let proof = source_filling::inspect(
        sweep,
        &last.model,
        &groups,
        ring_sizes,
        source.budget(),
        limits.domain_tolerance,
        limits.projection_cells,
        retained_annular_cap::reference_filling::Limits {
            exact_work: remaining.min(limits.domain_exact_work),
            products: limits.max_products.min(100_000),
            trim_pairs: limits.domain_pairs,
            trim_cells: limits.domain_cells,
            trim_domain_cells: limits.cap_regions.max_trim_domain_cells,
            spans: limits.max_faces,
            linear_cells: limits.wall_cells,
            contacts: crate::face_contacts::Limits {
                pairs: limits.cap_regions.max_trim_pairs,
                cells: limits.cap_regions.max_trim_cells,
                domain_cells: limits.cap_regions.max_trim_domain_cells,
                cells_per_pair: limits.cap_regions.max_chart_cells,
                domain_cells_per_pair: limits.cap_regions.max_trim_domain_cells.min(10_000),
            },
        },
    )?;
    let boundary = filled_cap_error::compose_boundary(
        source.boundary().wall_error_upper,
        proof.filled_cap_error_upper,
        false,
        Some(source.budget()),
    );
    Ok(Boundary {
        model: last.model,
        source,
        cap_face_groups: groups,
        filled_caps: Some(proof),
        boundary,
        cap_construction_work: construction_work,
    })
}

/// Progressive source ownership, including all affine/frame/guide laws. The
/// mode keeps legacy section correction and wall error; filled caps are freshly
/// compared with the original authored affine endpoint maps.
pub fn construct(request: &Request<'_>, limits: &Limits) -> Result<Boundary> {
    let source = super::construct_source_for_annular(request, limits)?;
    let profiles = request.loops.iter().flatten().cloned().collect::<Vec<_>>();
    let sizes = request.loops.iter().map(Vec::len).collect::<Vec<_>>();
    let mut options = request.options;
    options.max_steps = source.max_steps();
    options.initial_steps = options.initial_steps.min(options.max_steps);
    let mut sweep = Sweep::new(
        &profiles,
        request.points,
        request.scale,
        request.twist,
        options,
    )?;
    if let Some((a, c)) = request.affine {
        sweep = sweep.with_affine_laws(a, c)?;
    }
    if let Some((a, n)) = request.frames {
        sweep = sweep.with_frame_laws(a, n)?;
    }
    if let Some(g) = request.guide {
        sweep = sweep.with_orientation_guide(g)?;
    }
    finish(source, &sweep, &sizes, limits)
}
fn scalar(value: f64) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![value, 0., 0.]; 2],
        weights: vec![1.; 2],
        periodic: false,
    }
}
pub fn construct_direct(request: &DirectRequest<'_>, limits: &Limits) -> Result<Boundary> {
    construct_direct_impl(request, limits, false)
}
/// Keep exact existing terminal faces; replace only unproved terminal cap
/// identities. The probe and replacement share the cap exact-work allowance.
pub fn construct_direct_auto(request: &DirectRequest<'_>, limits: &Limits) -> Result<Boundary> {
    construct_direct_impl(request, limits, true)
}
fn construct_direct_impl(
    request: &DirectRequest<'_>,
    limits: &Limits,
    automatic: bool,
) -> Result<Boundary> {
    let source = if automatic {
        super::construct_direct_source_for_auto(request, limits)?
    } else {
        super::construct_direct_source_for_annular(request, limits)?
    };
    let profiles = request.loops.iter().flatten().cloned().collect::<Vec<_>>();
    let sizes = request.loops.iter().map(Vec::len).collect::<Vec<_>>();
    let scale = scalar(1.);
    let twist = scalar(0.);
    let sweep = Sweep::new_direct(
        &profiles,
        request.points,
        &scale,
        &twist,
        progressive_miter::Options {
            normal: request.normal,
            closed: request.closed,
            miter_limit: request.miter_limit,
            initial_steps: 1,
            max_steps: 1,
            max_deviation: request.max_deviation,
        },
    )?;
    finish(source, &sweep, &sizes, limits)
}
