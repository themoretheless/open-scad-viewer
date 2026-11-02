//! Reference-metric, small-displacement isotropic shell energy with dead loads.
//! Fixed trial rational basis/weights; ORIGINAL reference metric and loads.
//! Quadrature and stationarity are numerical diagnostics, not continuous energy,
//! nonlinear elasticity, whole-surface regularity, embedding or solid proofs.
use super::{Constraint, Report, Result, Surface, dot, error, norm};
mod assembly;
mod mechanics;
mod quadrature;

#[derive(Clone, Copy, Debug)]
pub struct Material {
    /// Plane-stress membrane stiffness (e.g. Young modulus times thickness).
    pub membrane: f64,
    /// Bending stiffness (e.g. Young modulus times thickness cubed / 12).
    pub bending: f64,
    pub poisson: f64,
}
impl Default for Material {
    fn default() -> Self {
        Self {
            membrane: 1.,
            bending: 0.01,
            poisson: 0.3,
        }
    }
}
#[derive(Clone, Debug)]
pub struct PointLoad {
    pub uv: [f64; 2],
    pub force: [f64; 3],
}
#[derive(Clone, Debug)]
pub struct Model {
    pub material: Material,
    /// Constant world-vector force per ORIGINAL surface area.
    pub traction: [f64; 3],
    /// Dead pressure along ORIGINAL unit normals, not a follower load.
    pub pressure: f64,
    pub points: Vec<PointLoad>,
}
impl Default for Model {
    fn default() -> Self {
        Self {
            material: Material::default(),
            traction: [0.; 3],
            pressure: 0.,
            points: Vec::new(),
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// Positive control-displacement anchor and actual hard-constraint tolerances.
    /// Legacy index-space fairness must be zero for this physical objective.
    pub fitting: super::Options,
    /// Tensor Gauss order 2..=8 on EVERY original AND trial nonempty knot span.
    pub quadrature_order: usize,
    pub subdivisions: usize,
    pub max_sites: usize,
    pub max_assembly_terms: u64,
    pub stationarity_tolerance: f64,
}
impl Default for Options {
    fn default() -> Self {
        let mut fitting = super::Options::default();
        fitting.fairness_weight = 0.;
        Self {
            fitting,
            quadrature_order: 4,
            subdivisions: 1,
            max_sites: 4096,
            max_assembly_terms: 50000000,
            stationarity_tolerance: 1e-7,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Diagnostics {
    pub elastic_energy: f64,
    pub anchor_energy: f64,
    pub load_work: f64,
    pub total_potential: f64,
    /// Relative Euclidean gradient projected onto the hard-equation nullspace.
    pub free_gradient_residual: f64,
    pub quadrature_sites: usize,
    pub assembly_terms: u64,
    pub numerical_quadrature_only: bool,
}
struct BoundaryState {
    sides: Vec<crate::surface_join::Boundary>,
    tolerances: [f64; 3],
}
pub struct Qualified {
    source: Surface,
    anchor: Surface,
    constraints: Vec<Constraint>,
    model: Model,
    options: Options,
    solution: Report,
    diagnostics: Diagnostics,
    boundary: Option<BoundaryState>,
    seams: Vec<(crate::surface_join::Boundary, crate::surface_g2::Report)>,
}
impl Qualified {
    /// Regenerate a nonlinear caller's final tangent equations from its owned
    /// originals, instead of borrowing this owner's earlier linearization rows.
    pub(super) fn reverify_with_constraints(
        &self,
        constraints: &[Constraint],
    ) -> Result<Diagnostics> {
        Ok(admit_actual(
            &self.anchor,
            &self.source,
            self.surface().clone(),
            constraints,
            &self.model,
            self.options,
        )?
        .diagnostics)
    }
    pub fn surface(&self) -> &Surface {
        &self.solution.surface
    }
    pub fn solution(&self) -> &Report {
        &self.solution
    }
    /// Immutable physical reference, independent of the trial coefficient basis.
    pub fn source(&self) -> &Surface {
        &self.source
    }
    /// Trial-basis control regularizer anchor; never the physical rest reference.
    pub fn anchor(&self) -> &Surface {
        &self.anchor
    }
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn constraints(&self) -> &[Constraint] {
        &self.constraints
    }
    pub fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }
    pub fn seams(&self) -> &[(crate::surface_join::Boundary, crate::surface_g2::Report)] {
        &self.seams
    }
    /// Fresh assembly, actual hard equations and constrained stationarity.
    /// Optional original boundary G2 must also pass within this cell allowance.
    pub fn reverify(&self, max_boundary_cells: usize) -> Result<Diagnostics> {
        validate_reference(
            &self.anchor,
            &self.source,
            &self.constraints,
            &self.model,
            self.options,
        )?;
        super::check_solution(
            self.surface().clone(),
            &self.constraints,
            self.options.fitting,
            self.solution.equations,
            self.solution.constraint_rank,
        )?;
        let a = assembly::assemble(&self.anchor, &self.source, &self.model, self.options)?;
        let (rows, _) = super::equations(&self.anchor, &self.constraints, self.options.fitting)?;
        let d = inspect(&self.anchor, self.surface(), &a, &rows, self.options)?;
        if let Some(b) = &self.boundary {
            seams(
                self.surface(),
                &self.source,
                &b.sides,
                b.tolerances,
                max_boundary_cells,
            )?;
        }
        Ok(d)
    }
}
fn validate(s: &Surface, c: &[Constraint], m: &Model, o: Options) -> Result<()> {
    s.validate()?;
    let n = s.control_points.len() * s.control_points[0].len();
    if n > 256
        || c.len() > 384
        || s.periodic_u
        || s.periodic_v
        || s.control_points.iter().flatten().any(|p| p.len() != 3)
        || m.points.len() > 256
        || !(2..=8).contains(&o.quadrature_order)
        || !(1..=8).contains(&o.subdivisions)
        || !(1..=8192).contains(&o.max_sites)
        || !(1..=100000000).contains(&o.max_assembly_terms)
        || !o.stationarity_tolerance.is_finite()
        || o.stationarity_tolerance <= 0.
        || !o.fitting.anchor_weight.is_finite()
        || o.fitting.anchor_weight <= 0.
        || o.fitting.fairness_weight != 0.
        || [
            o.fitting.position_tolerance,
            o.fitting.derivative_tolerance,
            o.fitting.normal_tolerance,
        ]
        .iter()
        .any(|v| !v.is_finite() || *v <= 0.)
        || [m.material.membrane, m.material.bending]
            .iter()
            .any(|v| !v.is_finite() || *v < 0.)
        || !m.material.poisson.is_finite()
        || m.material.poisson <= -1.
        || m.material.poisson >= 0.5
        || m.traction
            .iter()
            .chain(std::iter::once(&m.pressure))
            .any(|v| !v.is_finite())
        || m.points
            .iter()
            .flat_map(|p| p.force)
            .any(|v| !v.is_finite())
    {
        return Err(error(
            "Variational shell needs bounded nonperiodic 3D data, positive anchor, zero index fairness and finite stable material/load/solver parameters",
        ));
    }
    // A bending density alone cannot account for a derivative jump (crease)
    // concentrated on a knot line. Require a C1 trial space instead of silently
    // dropping its singular energy; membrane-only fields require C0.
    for (knots, degree, count) in [
        (&s.knots_u, s.degree_u, s.control_points.len()),
        (&s.knots_v, s.degree_v, s.control_points[0].len()),
    ] {
        let mut i = degree + 1;
        while i < count {
            let mut end = i + 1;
            while end < count && knots[end] == knots[i] {
                end += 1;
            }
            if knots[i] > knots[degree]
                && knots[i] < knots[count]
                && end - i > degree.saturating_sub(usize::from(m.material.bending > 0.))
            {
                return Err(error(
                    "Shell knot space lacks C0 membrane/C1 bending continuity; concentrated crease energy is unsupported",
                ));
            }
            i = end;
        }
    }
    Ok(())
}
pub(super) fn validate_reference(
    trial: &Surface,
    reference: &Surface,
    c: &[Constraint],
    m: &Model,
    o: Options,
) -> Result<()> {
    validate(trial, c, m, o)?;
    validate(reference, &[], m, o)?;
    let domain = |s: &Surface| {
        [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ]
    };
    if domain(trial) != domain(reference) {
        return Err(error(
            "Trial and ORIGINAL physical reference need identical authored UV domains",
        ));
    }
    Ok(())
}

pub fn solve(
    seed: &Surface,
    constraints: &[Constraint],
    model: &Model,
    options: Options,
) -> Result<Qualified> {
    solve_with_reference(seed, seed, constraints, model, options)
}
/// Trial coefficients/regularizer anchor may differ from the immutable physical
/// reference. Both charts use the SAME authored UV domain; no map is inferred.
pub fn solve_with_reference(
    seed: &Surface,
    reference: &Surface,
    constraints: &[Constraint],
    model: &Model,
    options: Options,
) -> Result<Qualified> {
    validate_reference(seed, reference, constraints, model, options)?;
    let a = assembly::assemble(seed, reference, model, options)?;
    let l = super::cholesky(&a.h)?;
    let mut displacement = super::triangular_solve(&l, &a.rhs);
    let original = controls(seed);
    let (rows, targets) = super::equations(seed, constraints, options.fitting)?;
    let rhs = rows
        .iter()
        .zip(targets)
        .map(|(row, target)| target - dot(row, &original) - dot(row, &displacement))
        .collect::<Vec<_>>();
    let (correction, rank) = super::linear::dense_correction(&l, &rows, &rhs)?;
    for (u, du) in displacement.iter_mut().zip(correction) {
        *u += du;
    }
    let mut surface = seed.clone();
    let (nu, nv) = (
        surface.control_points.len(),
        surface.control_points[0].len(),
    );
    let n = nu * nv;
    for i in 0..nu {
        for j in 0..nv {
            for k in 0..3 {
                surface.control_points[i][j][k] =
                    seed.control_points[i][j][k] + displacement[k * n + i * nv + j];
            }
        }
    }
    for c in constraints {
        if let Constraint::ControlPoint {
            index: [i, j],
            point,
        } = c
        {
            surface.control_points[*i][*j] = point.to_vec();
        }
    }
    let solution = super::check_solution(surface, constraints, options.fitting, rows.len(), rank)?;
    let diagnostics = inspect(seed, &solution.surface, &a, &rows, options)?;
    Ok(Qualified {
        source: reference.clone(),
        anchor: seed.clone(),
        constraints: constraints.to_vec(),
        model: model.clone(),
        options,
        solution,
        diagnostics,
        boundary: None,
        seams: Vec::new(),
    })
}
/// Fresh actual-geometry admission for a nonlinear caller's FINAL linearization.
/// The caller owns any complete-curve or original-boundary geometric proofs.
pub(super) fn admit_actual(
    seed: &Surface,
    reference: &Surface,
    actual: Surface,
    constraints: &[Constraint],
    model: &Model,
    options: Options,
) -> Result<Qualified> {
    validate_reference(seed, reference, constraints, model, options)?;
    let a = assembly::assemble(seed, reference, model, options)?;
    let (rows, _) = super::equations(seed, constraints, options.fitting)?;
    let l = super::cholesky(&a.h)?;
    let (_, rank) = super::linear::dense_correction(&l, &rows, &vec![0.; rows.len()])?;
    let solution = super::check_solution(actual, constraints, options.fitting, rows.len(), rank)?;
    let diagnostics = inspect(seed, &solution.surface, &a, &rows, options)?;
    Ok(Qualified {
        source: reference.clone(),
        anchor: seed.clone(),
        constraints: constraints.to_vec(),
        model: model.clone(),
        options,
        solution,
        diagnostics,
        boundary: None,
        seams: Vec::new(),
    })
}

fn controls(s: &Surface) -> Vec<f64> {
    (0..3)
        .flat_map(|k| s.control_points.iter().flatten().map(move |p| p[k]))
        .collect()
}
fn inspect(
    seed: &Surface,
    actual: &Surface,
    a: &assembly::Assembly,
    rows: &[Vec<f64>],
    o: Options,
) -> Result<Diagnostics> {
    if actual.weights != seed.weights
        || actual.knots_u != seed.knots_u
        || actual.knots_v != seed.knots_v
        || actual.degree_u != seed.degree_u
        || actual.degree_v != seed.degree_v
    {
        return Err(error(
            "Actual variational geometry changed its original rational basis",
        ));
    }
    let u = controls(actual)
        .iter()
        .zip(controls(seed))
        .map(|(a, b)| a - b)
        .collect::<Vec<_>>();
    let hu = a.h.iter().map(|r| dot(r, &u)).collect::<Vec<_>>();
    let gradient = hu
        .iter()
        .zip(&a.rhs)
        .map(|(h, f)| h - f)
        .collect::<Vec<_>>();
    let mut identity = vec![vec![0.; u.len()]; u.len()];
    for (i, r) in identity.iter_mut().enumerate() {
        r[i] = 1.;
    }
    let rhs = rows.iter().map(|r| dot(r, &gradient)).collect::<Vec<_>>();
    let (normal, _) = super::linear::dense_correction(&identity, rows, &rhs)?;
    let residual = norm(
        &gradient
            .iter()
            .zip(normal)
            .map(|(g, n)| g - n)
            .collect::<Vec<_>>(),
    ) / norm(&a.rhs).max(norm(&hu)).max(1.);
    let anchor = 0.5 * o.fitting.anchor_weight * dot(&u, &u);
    let elastic = 0.5
        * a.elastic
            .iter()
            .map(|r| dot(r, &u))
            .zip(&u)
            .map(|(a, b)| a * b)
            .sum::<f64>()
        - dot(&a.rest, &u)
        + a.elastic_constant;
    let work = dot(&a.force, &u) + a.load_constant;
    let total = elastic + anchor - work;
    if [residual, anchor, elastic, work, total]
        .iter()
        .any(|v| !v.is_finite())
        || residual > o.stationarity_tolerance
        || elastic < -1e-10 * anchor.max(1.)
    {
        return Err(error(
            "Actual constrained shell equilibrium/energy remains numerically unresolved",
        ));
    }
    Ok(Diagnostics {
        elastic_energy: elastic,
        anchor_energy: anchor,
        load_work: work,
        total_potential: total,
        free_gradient_residual: residual,
        quadrature_sites: a.sites,
        assembly_terms: a.terms,
        numerical_quadrature_only: true,
    })
}
fn seams(
    s: &Surface,
    reference: &Surface,
    sides: &[crate::surface_join::Boundary],
    t: [f64; 3],
    max: usize,
) -> Result<Vec<(crate::surface_join::Boundary, crate::surface_g2::Report)>> {
    if max < sides.len() || max > 100000 || t.iter().any(|v| !v.is_finite() || *v < 0.) || t[1] > 2.
    {
        return Err(error(
            "Original shell G2 seams need bounded cells and finite tolerances",
        ));
    }
    let mut remaining = max;
    let mut output = Vec::new();
    for (i, &side) in sides.iter().enumerate() {
        let r = crate::surface_g2::inspect(
            s,
            side,
            reference,
            side,
            false,
            t[0],
            t[1],
            t[2],
            remaining / (sides.len() - i),
        )?;
        remaining = remaining.saturating_sub(r.cells);
        if r.decision != crate::surface_join::Decision::WithinTolerance || !r.regularity_proven {
            return Err(error(
                "Actual loaded shell original boundary G2 remains unproven",
            ));
        }
        output.push((side, r));
    }
    Ok(output)
}
pub fn solve_preserving_g2(
    seed: &Surface,
    boundaries: &[crate::surface_join::Boundary],
    constraints: &[Constraint],
    model: &Model,
    options: Options,
    tolerances: [f64; 3],
    max_cells: usize,
) -> Result<Qualified> {
    validate_reference(seed, seed, constraints, model, options)?;
    let pins = super::boundary::control_rings(seed, boundaries)?;
    if pins.len() + constraints.len() > 384 {
        return Err(error("Loaded G2 lock allowance exhausted"));
    }
    let mut all = constraints.to_vec();
    for (i, j) in pins {
        let p = &seed.control_points[i][j];
        all.push(Constraint::ControlPoint {
            index: [i, j],
            point: [p[0], p[1], p[2]],
        });
    }
    let mut result = solve(seed, &all, model, options)?;
    result.seams = seams(result.surface(), seed, boundaries, tolerances, max_cells)?;
    result.boundary = Some(BoundaryState {
        sides: boundaries.to_vec(),
        tolerances,
    });
    Ok(result)
}
