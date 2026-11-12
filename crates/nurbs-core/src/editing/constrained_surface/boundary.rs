//! Complete rational boundary two-jets, with fresh continuous G2 admission.
use super::{Constraint, Options, Report, check_solution, error, solve};
use crate::{
    Result,
    surface::Surface,
    surface_g2,
    surface_join::{Boundary, Decision},
};
use std::collections::BTreeSet;

/// Immutable ownership of the actual solved surface and its whole-boundary proof.
/// Does not certify interior injectivity or a sewn solid.
pub struct Qualified {
    solution: Report,
    seams: Vec<(Boundary, surface_g2::Report)>,
    reference: Surface,
}

/// Solved interior plus fresh proof on four immutable ORIGINAL mapped supports.
pub struct MappedQualified {
    solution: Report,
    admission: crate::corner_blend::mapped::Report,
    originals: [(Surface, crate::curve::Curve, [f64; 2]); 4],
    corner_options: crate::corner_blend::Options,
    tolerances: [f64; 3],
}
impl MappedQualified {
    pub fn surface(&self) -> &Surface {
        &self.solution.surface
    }
    pub fn solution(&self) -> &Report {
        &self.solution
    }
    pub fn admission(&self) -> &crate::corner_blend::mapped::Report {
        &self.admission
    }
    pub fn original_supports(&self) -> [crate::corner_blend::mapped::Support<'_>; 4] {
        std::array::from_fn(|i| crate::corner_blend::mapped::Support {
            surface: &self.originals[i].0,
            pcurve: &self.originals[i].1,
            direction: self.originals[i].2,
        })
    }
    pub fn reverify(
        &self,
        max_cells: usize,
        max_coefficients: usize,
        max_exact: u64,
    ) -> Result<crate::corner_blend::mapped::Report> {
        crate::corner_blend::mapped::inspect_candidate(
            self.surface(),
            self.original_supports(),
            self.corner_options,
            self.tolerances,
            max_cells,
            max_coefficients,
            max_exact,
        )
    }
}

/// Construct complete G2 fields from four arbitrary original rational support
/// pcurves, solve interior point/jet constraints, and freshly admit the ACTUAL
/// result on the ORIGINAL supports. No sampled/collar receipt is accepted as
/// final authority. Corner compatibility and a globally injective candidate
/// chart remain required; impossible conditions refuse.
pub fn solve_from_mapped_supports(
    supports: [crate::corner_blend::mapped::Support<'_>; 4],
    corner_options: crate::corner_blend::Options,
    constraints: &[Constraint],
    options: Options,
    tolerances: [f64; 3],
    max_cells: usize,
    max_coefficients: usize,
    max_exact: u64,
) -> Result<MappedQualified> {
    if !(100..=100000).contains(&max_cells)
        || !(100..=2000000).contains(&max_coefficients)
        || max_exact > 1000000
    {
        return Err(error("Mapped G2 constraints need bounded shared work"));
    }
    let original = crate::corner_blend::mapped::g2(
        supports,
        corner_options,
        tolerances,
        max_cells / 3,
        max_coefficients / 3,
        max_exact / 3,
    )?;
    let mut remaining = max_cells.saturating_sub(original.cells);
    let coefficients = max_coefficients.saturating_sub(original.coefficient_work);
    let exact = max_exact.saturating_sub(original.corner.exact_work);
    let mut seed = original.corner.surface.clone();
    if seed.degree_u < 6 {
        seed = crate::surface_edit::elevate(&seed, crate::surface::Axis::U, 6)?;
    }
    if seed.degree_v < 6 {
        seed = crate::surface_edit::elevate(&seed, crate::surface::Axis::V, 6)?;
    }
    let locked = solve_preserving_g2(
        &seed,
        &seed,
        &[
            Boundary::UMin,
            Boundary::UMax,
            Boundary::VMin,
            Boundary::VMax,
        ],
        constraints,
        options,
        tolerances,
        remaining / 3,
    )?;
    remaining = remaining.saturating_sub(locked.seams.iter().map(|(_, r)| r.cells).sum::<usize>());
    let admission = crate::corner_blend::mapped::inspect_candidate(
        locked.surface(),
        supports,
        corner_options,
        tolerances,
        remaining,
        coefficients,
        exact,
    )?;
    if !admission.admitted || !admission.direct_admitted || !admission.corner.interior.certified {
        return Err(error(
            "Constrained patch G2 on complete original support curves remains unproven",
        ));
    }
    if admission.corner.surface != *locked.surface() {
        return Err(error(
            "Original support proof changed the constrained surface",
        ));
    }
    Ok(MappedQualified {
        solution: locked.solution,
        admission,
        originals: supports.map(|s| (s.surface.clone(), s.pcurve.clone(), s.direction)),
        corner_options,
        tolerances,
    })
}
impl Qualified {
    pub fn surface(&self) -> &Surface {
        &self.solution.surface
    }
    pub fn solution(&self) -> &Report {
        &self.solution
    }
    pub fn seams(&self) -> &[(Boundary, surface_g2::Report)] {
        &self.seams
    }
    pub fn reference(&self) -> &Surface {
        &self.reference
    }
}
/// Preserve the original rational 0/1/2-jets on entire selected boundaries while
/// solving interior constraints. The reference and seed must share the complete
/// clamped basis and the weights on the three boundary layers. Interior weights
/// may differ. Three boundary coefficient layers are fixed exactly;
/// this is stronger than geometric G2 wherever the boundary is regular.
/// Actual retained coefficients are independently qualified over whole seams.
pub fn solve_preserving_g2(
    seed: &Surface,
    reference: &Surface,
    boundaries: &[Boundary],
    constraints: &[Constraint],
    options: Options,
    tolerances: [f64; 3],
    max_cells: usize,
) -> Result<Qualified> {
    let (all, _) = locked_constraints(
        seed,
        reference,
        boundaries,
        constraints,
        tolerances,
        max_cells,
    )?;
    let result = solve(seed, &all, options)?;
    let locks = super::network::BoundaryLocks {
        reference,
        boundaries,
        tolerances,
    };
    admit_solution(seed, locks, constraints, result, options, max_cells)
}

/// Share original lock construction across numerical objectives.
pub(super) fn locked_constraints(
    seed: &Surface,
    reference: &Surface,
    boundaries: &[Boundary],
    constraints: &[Constraint],
    tolerances: [f64; 3],
    max_cells: usize,
) -> Result<(Vec<Constraint>, BTreeSet<(usize, usize)>)> {
    seed.validate()?;
    reference.validate()?;
    if boundaries.is_empty()
        || boundaries.len() > 4
        || max_cells > 100000
        || max_cells < boundaries.len()
        || tolerances.iter().any(|x| !x.is_finite() || *x < 0.)
        || tolerances[1] > 2.
        || seed.degree_u < 2
        || seed.degree_v < 2
        || seed.periodic_u
        || seed.periodic_v
        || seed.degree_u != reference.degree_u
        || seed.degree_v != reference.degree_v
        || seed.knots_u != reference.knots_u
        || seed.knots_v != reference.knots_v
    {
        return Err(error(
            "G2 locks require matching nonperiodic clamped bases, weights and bounded work",
        ));
    }
    let n = seed.control_points.len();
    let m = seed.control_points[0].len();
    for (k, p, count) in [
        (&seed.knots_u, seed.degree_u, n),
        (&seed.knots_v, seed.degree_v, m),
    ] {
        if k[..=p].iter().any(|x| *x != k[p]) || k[count..].iter().any(|x| *x != k[count]) {
            return Err(error("G2 coefficient locks require clamped endpoint bases"));
        }
    }
    let pins = control_rings(seed, boundaries)?;
    if pins
        .iter()
        .any(|&(i, j)| seed.weights[i][j] != reference.weights[i][j])
    {
        return Err(error(
            "G2 locks require identical original weights on the three boundary layers",
        ));
    }
    if pins.len() + constraints.len() > 384 {
        return Err(error("G2 lock constraint budget exceeded"));
    }
    let mut all = constraints.to_vec();
    for &(i, j) in &pins {
        let p = &reference.control_points[i][j];
        if p.len() != 3 {
            return Err(error("G2 locks require 3D controls"));
        }
        all.push(Constraint::ControlPoint {
            index: [i, j],
            point: [p[0], p[1], p[2]],
        });
    }
    Ok((all, pins))
}

/// Admit a saved solution without solving again or moving any coefficient.
pub(super) fn admit_solution(
    seed: &Surface,
    locks: super::network::BoundaryLocks<'_>,
    constraints: &[Constraint],
    result: Report,
    options: Options,
    max_cells: usize,
) -> Result<Qualified> {
    let super::network::BoundaryLocks {
        reference,
        boundaries,
        tolerances,
    } = locks;
    let (all, pins) = locked_constraints(
        seed,
        reference,
        boundaries,
        constraints,
        tolerances,
        max_cells,
    )?;

    let surface = result.surface;
    for &(i, j) in &pins {
        if surface.control_points[i][j]
            .iter()
            .map(|x| x.to_bits())
            .collect::<Vec<_>>()
            != reference.control_points[i][j]
                .iter()
                .map(|x| x.to_bits())
                .collect::<Vec<_>>()
        {
            return Err(error(
                "Actual G2 boundary control differs from its exact original lock",
            ));
        }
    }
    let solution = check_solution(
        surface,
        &all,
        options,
        result.equations,
        result.constraint_rank,
    )?;
    let mut remaining = max_cells;
    let mut seams = Vec::new();
    for (index, &side) in boundaries.iter().enumerate() {
        let budget = remaining / (boundaries.len() - index);
        let report = surface_g2::inspect(
            surface_g2::SurfaceSeam { a: &solution.surface, ea: side, b: reference, eb: side, reverse: false },
            surface_g2::G2Tolerance { position: tolerances[0], normal: tolerances[1], tensor: tolerances[2] },
            budget,
        )?;
        remaining = remaining.saturating_sub(report.cells);
        if report.decision != Decision::WithinTolerance || !report.regularity_proven {
            return Err(error("Whole-boundary G2 or regularity remains unproven"));
        }
        seams.push((side, report));
    }
    Ok(Qualified {
        solution,
        seams,
        reference: reference.clone(),
    })
}

/// Three original clamped control layers, including every original corner.
pub(super) fn control_rings(
    seed: &Surface,
    boundaries: &[Boundary],
) -> Result<BTreeSet<(usize, usize)>> {
    if seed.degree_u < 2
        || seed.degree_v < 2
        || boundaries.is_empty()
        || boundaries.len() > 4
        || seed.periodic_u
        || seed.periodic_v
    {
        return Err(error(
            "Boundary two-jets need complete original degree-two layers",
        ));
    }
    let (n, m) = (seed.control_points.len(), seed.control_points[0].len());
    for (knots, degree, count) in [
        (&seed.knots_u, seed.degree_u, n),
        (&seed.knots_v, seed.degree_v, m),
    ] {
        if knots[..=degree].iter().any(|k| *k != knots[degree])
            || knots[count..].iter().any(|k| *k != knots[count])
        {
            return Err(error(
                "Boundary control layers require original clamped endpoint knots",
            ));
        }
    }
    let mut pins = BTreeSet::new();
    let mut seen = BTreeSet::new();
    for &side in boundaries {
        let key = match side {
            Boundary::UMin => 0,
            Boundary::UMax => 1,
            Boundary::VMin => 2,
            Boundary::VMax => 3,
        };
        if !seen.insert(key) {
            return Err(error("Repeated G2 boundary"));
        }
        for layer in 0..3 {
            match side {
                Boundary::UMin => pins.extend((0..m).map(|j| (layer, j))),
                Boundary::UMax => pins.extend((0..m).map(|j| (n - 1 - layer, j))),
                Boundary::VMin => pins.extend((0..n).map(|i| (i, layer))),
                Boundary::VMax => pins.extend((0..n).map(|i| (i, m - 1 - layer))),
            }
        }
    }
    Ok(pins)
}
