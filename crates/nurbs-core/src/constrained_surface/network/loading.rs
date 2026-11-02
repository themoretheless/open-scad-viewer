//! Fixed ORIGINAL reference shell objective under complete original curve
//! constraints. Final G2 tangent equations are rebuilt on ACTUAL saved geometry.
use super::*;
use crate::constrained_surface::variational;
#[derive(Clone, Copy)]
pub struct Options {
    /// Shared curve/boundary cells, exact work, collocation and nonlinear limits.
    /// The physical objective requires fitting.fairness_weight == 0.
    pub curves: CurvatureOptions,
    pub quadrature_order: usize,
    pub subdivisions: usize,
    pub max_quadrature_sites: usize,
    pub max_assembly_terms_per_call: u64,
    /// Shared across proposals AND final actual-geometry equilibrium admissions.
    pub max_total_assembly_terms: u64,
    pub stationarity_tolerance: f64,
}
impl Default for Options {
    fn default() -> Self {
        let v = variational::Options::default();
        let mut curves = CurvatureOptions::default();
        curves.network.fitting = v.fitting;
        Self {
            curves,
            quadrature_order: v.quadrature_order,
            subdivisions: v.subdivisions,
            max_quadrature_sites: v.max_sites,
            max_assembly_terms_per_call: v.max_assembly_terms,
            max_total_assembly_terms: 100000000,
            stationarity_tolerance: v.stationarity_tolerance,
        }
    }
}
pub(super) struct Settings<'a> {
    model: &'a variational::Model,
    reference: &'a Surface,
    physical: variational::Options,
    total: u64,
}
pub fn solve(
    seed: &Surface,
    guides: &[CurveConstraint<'_>],
    curvatures: &[CurvatureConstraint<'_>],
    constraints: &[Constraint],
    locks: Option<BoundaryLocks<'_>>,
    model: &variational::Model,
    options: Options,
) -> Result<Qualified> {
    solve_with_reference(
        seed,
        seed,
        guides,
        curvatures,
        constraints,
        locks,
        model,
        options,
    )
}
/// Keep ORIGINAL shell metric/loads while the trial fitting basis changes.
pub fn solve_with_reference(
    seed: &Surface,
    reference: &Surface,
    guides: &[CurveConstraint<'_>],
    curvatures: &[CurvatureConstraint<'_>],
    constraints: &[Constraint],
    locks: Option<BoundaryLocks<'_>>,
    model: &variational::Model,
    options: Options,
) -> Result<Qualified> {
    if !(1..=500000000).contains(&options.max_total_assembly_terms)
        || !options.curves.curvature_tolerance.is_finite()
        || options.curves.curvature_tolerance <= 0.
    {
        return Err(error(
            "Loaded network needs bounded shared assembly work and positive curvature tolerance",
        ));
    }
    let settings = Settings {
        model,
        reference,
        physical: variational::Options {
            fitting: options.curves.network.fitting,
            quadrature_order: options.quadrature_order,
            subdivisions: options.subdivisions,
            max_sites: options.max_quadrature_sites,
            max_assembly_terms: options.max_assembly_terms_per_call,
            stationarity_tolerance: options.stationarity_tolerance,
        },
        total: options.max_total_assembly_terms,
    };
    variational::validate_reference(seed, reference, constraints, model, settings.physical)?;
    super::solve_impl(
        seed,
        guides,
        curvatures,
        constraints,
        locks,
        options.curves.network,
        (!curvatures.is_empty()).then_some(options.curves.curvature_tolerance),
        Some(&settings),
    )
}
fn options(
    settings: &Settings<'_>,
    fitting: FitOptions,
    used: u64,
) -> Result<variational::Options> {
    let remaining = settings.total.saturating_sub(used);
    if remaining == 0 {
        return Err(error("Loaded network shared assembly work exhausted"));
    }
    let mut o = settings.physical;
    o.fitting = fitting;
    o.max_assembly_terms = o.max_assembly_terms.min(remaining);
    Ok(o)
}
pub(super) fn propose(
    seed: &Surface,
    all: &[Constraint],
    locks: Option<BoundaryLocks<'_>>,
    fitting: FitOptions,
    cells: usize,
    settings: &Settings<'_>,
    used: &mut u64,
) -> Result<(Report, Option<boundary::Qualified>, variational::Qualified)> {
    let constraints = if let Some(b) = locks {
        boundary::locked_constraints(seed, b.reference, b.boundaries, all, b.tolerances, cells)?.0
    } else {
        all.to_vec()
    };
    let owner = variational::solve_with_reference(
        seed,
        settings.reference,
        &constraints,
        settings.model,
        options(settings, fitting, *used)?,
    )?;
    *used += owner.diagnostics().assembly_terms;
    let boundary = if let Some(b) = locks {
        Some(boundary::admit_solution(
            seed,
            b.reference,
            b.boundaries,
            all,
            owner.solution().clone(),
            fitting,
            b.tolerances,
            cells,
        )?)
    } else {
        None
    };
    if boundary
        .as_ref()
        .is_some_and(|b| b.surface() != owner.surface())
    {
        return Err(error(
            "Loaded boundary admission changed actual equilibrium geometry",
        ));
    }
    Ok((owner.solution().clone(), boundary, owner))
}
pub(super) fn admit_current(
    seed: &Surface,
    actual: &Surface,
    owned: &[OwnedGuide],
    sites: &[Vec<f64>],
    constraints: &[Constraint],
    locks: Option<BoundaryLocks<'_>>,
    fitting: FitOptions,
    settings: &Settings<'_>,
    used: &mut u64,
    cost: u64,
) -> Result<std::result::Result<variational::Qualified, String>> {
    let mut all = constraints.to_vec();
    for (g, t) in owned.iter().zip(sites) {
        super::sites::constraints(g, t, Some(actual), &mut all)?;
    }
    if let Some(b) = locks {
        all = boundary::locked_constraints(
            seed,
            b.reference,
            b.boundaries,
            &all,
            b.tolerances,
            100000,
        )?
        .0;
    }
    // Identical original metric/basis, material and quadrature produce exactly
    // the same assembly cost, independent of final hard-equation linearization.
    // Reserve/charge this known cost even when fresh actual admission refuses.
    let mut o = options(settings, fitting, *used)?;
    if cost > o.max_assembly_terms {
        return Err(error(
            "Final loaded equilibrium shared assembly allowance exhausted",
        ));
    }
    o.max_assembly_terms = cost.max(1);
    *used += cost;
    Ok(variational::admit_actual(
        seed,
        settings.reference,
        actual.clone(),
        &all,
        settings.model,
        o,
    )
    .map_err(|e| e.to_string()))
}

/// Derive the FINAL tangent equations anew from unchanged original curves and
/// supports on every replay; stored JetEquation receipts grant no authority.
pub(super) fn reverify(q: &Qualified) -> Result<variational::Diagnostics> {
    let owner = q
        .loading
        .as_ref()
        .ok_or_else(|| error("Loaded network lost its physical owner"))?;
    if owner.surface() != q.surface() {
        return Err(error(
            "Saved loaded curve and equilibrium geometries differ",
        ));
    }
    let mut all = q.constraints.clone();
    for (g, t) in q.guides.iter().zip(&q.sites) {
        super::sites::constraints(g, t, Some(q.surface()), &mut all)?;
    }
    if let Some(b) = &q.boundary {
        let sides = b.seams().iter().map(|(s, _)| *s).collect::<Vec<_>>();
        all = boundary::locked_constraints(
            owner.anchor(),
            b.reference(),
            &sides,
            &all,
            q.boundary_tolerances
                .ok_or_else(|| error("Loaded boundary tolerances missing"))?,
            100000,
        )?
        .0;
    }
    owner.reverify_with_constraints(&all)
}
