use super::*;
mod periodic_grid;

pub(super) fn face_integral(
    model: &Model,
    use_: &FaceUse,
    divisions: [usize; 2],
    origin: [f64; 3],
    budget: &mut Budget,
    integration_spans: &BTreeMap<usize, Vec<Vec<[f64; 2]>>>,
    integration_grids: &[[Vec<f64>; 2]],
) -> Result<[f64; 11]> {
    let face = &model.faces[use_.face];
    let surface = &face.surface;
    // Constant homogeneous weights cancel to a polynomial patch; retain the
    // cheaper rule for these faces. Varying positive weights need the higher
    // order rational rule. Both routes use the same directional convergence
    // estimates, conditioning schedules and shared evaluation budget.
    let quadrature: &[(f64, f64)] = if surface.weights.iter().flatten()
        .all(|weight| *weight == surface.weights[0][0]) {
        &GAUSS_POLYNOMIAL
    } else {
        &GAUSS_RATIONAL
    };
    let u0 = if surface.periodic_u {
        integration_grids[use_.face][0][0]
    } else {
        surface.knots_u[surface.degree_u]
    };
    let mut result = [0.; 11];
    for &wire in std::iter::once(&face.outer).chain(&face.holes) {
        for (coedge_index, coedge) in model.loops[wire].coedges.iter().enumerate() {
            // Pcurves already follow their loop; coedge.reversed applies to the 3D carrier only.
            let curve = &coedge.pcurve;
            for &[start, end] in &integration_spans[&wire][coedge_index] {
                for part in 0..divisions[0] {
                    let a = start + (end - start) * part as f64 / divisions[0] as f64;
                    let b = start + (end - start) * (part + 1) as f64 / divisions[0] as f64;
                    for &(x, w) in quadrature {
                        let evaluation = curve.evaluate(((a + b) / 2. + x * (b - a) / 2.).clamp(a,b))?;
                        let dv = evaluation.d1.ok_or_else(|| {
                            Error::new("BREP_ANALYSIS_INDETERMINATE", "Undefined trim derivative")
                        })?[1];
                        if dv == 0. {
                            continue;
                        }
                        let mut uv = [evaluation.point[0], evaluation.point[1]];
                        // A positive rational trim stays in its control hull.
                        // World-coordinate charts can round an endpoint one ULP
                        // outside the surface. Clamp only when the complete
                        // original trim hull proves containment in that domain.
                        let domains=[[surface.knots_u[surface.degree_u],surface.knots_u[surface.knots_u.len()-surface.degree_u-1]],
                            [surface.knots_v[surface.degree_v],surface.knots_v[surface.knots_v.len()-surface.degree_v-1]]];
                        if curve.control_points.iter().all(|p|(0..2).all(|k|p[k]>=domains[k][0]&&p[k]<=domains[k][1])) {
                            for k in 0..2 {uv[k]=uv[k].clamp(domains[k][0],domains[k][1]);}
                        }
                        let [u,v]=uv;
                        for [low, high] in spans(&integration_grids[use_.face][0], [u0, u]) {
                            for inner in 0..divisions[1] {
                                let l = low + (high - low) * inner as f64 / divisions[1] as f64;
                                let h =
                                    low + (high - low) * (inner + 1) as f64 / divisions[1] as f64;
                                for &(ix, iw) in quadrature {
                                    let values = flux(
                                        surface,
                                        // Every quadrature node lies in its admitted interval.
                                        // Rounding on a narrow world-coordinate span may
                                        // otherwise place it one ULP before the lower knot.
                                        ((l + h) / 2. + ix * (h - l) / 2.).clamp(l,h),
                                        v,
                                        origin,
                                        if use_.reversed { -1. } else { 1. },
                                        budget,
                                    )?;
                                    let weight = w * (b - a) / 2. * dv * iw * (h - l) / 2.;
                                    for i in 0..11 {
                                        result[i] += weight * values[i];
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(result)
}
/// Number of accumulated integrals: area, volume, three first moments and six
/// second moments.
const INTEGRALS: usize = 11;
/// Admitted relative quadrature tolerances.
const RELATIVE_TOLERANCE_RANGE: std::ops::RangeInclusive<f64> = 1e-10..=1e-3;
/// Admitted surface-evaluation budgets.
const EVALUATION_BUDGET_RANGE: std::ops::RangeInclusive<usize> = 100..=2_000_000;
/// Initial per-direction subdivision count of each face.
const INITIAL_DIVISIONS: [usize; 2] = [2, 2];
/// Per-direction subdivision count at which a face is no longer refined.
const MAX_DIVISIONS: usize = 16;
/// Fraction of each integral's dimensional scale used as its absolute floor.
const ABSOLUTE_FLOOR_FRACTION: f64 = 1e-3;
/// Fraction of the cubic model scale below which the volume is unresolved.
const MIN_VOLUME_FRACTION: f64 = 1e-14;

type Integrals = [f64; INTEGRALS];
type Spans = BTreeMap<usize, Vec<Vec<[f64; 2]>>>;

/// Conservative axis-aligned box of every surface control point.
fn control_bounds(model: &Model) -> [[f64; 3]; 2] {
    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    for p in model
        .faces
        .iter()
        .flat_map(|f| f.surface.control_points.iter().flatten())
    {
        for i in 0..3 {
            bounds[0][i] = bounds[0][i].min(p[i]);
            bounds[1][i] = bounds[1][i].max(p[i]);
        }
    }
    bounds
}

/// Dimensional scale of each integral: area, volume, first and second moments.
fn integral_scales(bounds: &[[f64; 3]; 2]) -> Integrals {
    let scale = (0..3)
        .map(|i| bounds[1][i] - bounds[0][i])
        .fold(0., f64::max);
    std::array::from_fn(|i| {
        scale.powi(if i == 0 {
            2
        } else if i == 1 {
            3
        } else if i < 5 {
            4
        } else {
            5
        })
    })
}

/// Trim-parameter integration spans of every loop, keyed by loop index.
fn integration_spans(
    model: &Model,
    integration_grids: &[[Vec<f64>; 2]],
    relative_tolerance: f64,
    budget: &mut Budget,
) -> Result<Spans> {
    let mut integration_spans = BTreeMap::new();
    for (face_index, face) in model.faces.iter().enumerate() {
        for &wire in std::iter::once(&face.outer).chain(&face.holes) {
            let schedule = model.loops[wire]
                .coedges
                .iter()
                .map(|c| {
                    trim_intervals(
                        &c.pcurve,
                        &integration_grids[face_index],
                        relative_tolerance,
                        budget,
                    )
                })
                .collect::<Result<_>>()?;
            integration_spans.insert(wire, schedule);
        }
    }
    Ok(integration_spans)
}

/// Converts converged integrals into the reported mass properties.
fn finish(
    total: &Integrals,
    errors: &Integrals,
    scales: &Integrals,
    origin: [f64; 3],
    bounds: [[f64; 3]; 2],
    evaluations: usize,
) -> Result<MassProperties> {
    let volume = total[1];
    if volume <= scales[1] * MIN_VOLUME_FRACTION {
        return Err(Error::new(
            "BREP_ANALYSIS_INDETERMINATE",
            "Boundary has non-positive or numerically unresolved signed volume",
        ));
    }
    let c: [f64; 3] = std::array::from_fn(|i| total[2 + i] / volume);
    let moments: [[f64; 3]; 3] = [
        [total[5], total[8], total[9]],
        [total[8], total[6], total[10]],
        [total[9], total[10], total[7]],
    ];
    let central: [[f64; 3]; 3] = std::array::from_fn(|i| {
        std::array::from_fn(|j| moments[i][j] - volume * c[i] * c[j])
    });
    let trace = (0..3).map(|i| central[i][i]).sum::<f64>();
    let inertia = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            if i == j {
                trace - central[i][j]
            } else {
                -central[i][j]
            }
        })
    });
    if total
        .iter()
        .chain(inertia.iter().flatten())
        .any(|x| !x.is_finite())
    {
        return Err(Error::new(
            "BREP_ANALYSIS_INDETERMINATE",
            "Non-finite mass integral",
        ));
    }
    Ok(MassProperties {
        surface_area_mm2: total[0],
        signed_volume_mm3: volume,
        centroid: std::array::from_fn(|i| origin[i] + c[i]),
        inertia_mm5: inertia,
        conservative_bounds: bounds,
        area_error_estimate_mm2: errors[0],
        volume_error_estimate_mm3: errors[1],
        evaluations,
    })
}

fn difference(left: &Integrals, right: &Integrals) -> Integrals {
    std::array::from_fn(|i| (left[i] - right[i]).abs())
}

/// Adaptive per-face, per-direction refinement of the boundary quadrature.
struct Refinement<'a> {
    model: &'a Model,
    uses: Vec<&'a FaceUse>,
    origin: [f64; 3],
    scales: Integrals,
    integration_spans: Spans,
    integration_grids: Vec<[Vec<f64>; 2]>,
    divisions: Vec<[usize; 2]>,
    current: Vec<Integrals>,
    face_errors: Vec<[Integrals; 2]>,
}

impl Refinement<'_> {
    fn integrate(
        &self,
        index: usize,
        divisions: [usize; 2],
        budget: &mut Budget,
    ) -> Result<Integrals> {
        face_integral(
            self.model,
            self.uses[index],
            divisions,
            self.origin,
            budget,
            &self.integration_spans,
            &self.integration_grids,
        )
        .map_err(|error| {
            if error.code == "BREP_RESOURCE_LIMIT" {
                Error::new(
                    "BREP_RESOURCE_LIMIT",
                    format!(
                        "{} (face {}, subdivisions {:?}, evaluations {}/{})",
                        error.message, self.uses[index].face, divisions, budget.used, budget.limit
                    ),
                )
            } else {
                error
            }
        })
    }

    /// Integrates every face once and estimates both directional errors.
    fn seed(&mut self, budget: &mut Budget) -> Result<()> {
        let count = self.uses.len();
        self.divisions = vec![INITIAL_DIVISIONS; count];
        self.current = (0..count)
            .map(|i| self.integrate(i, INITIAL_DIVISIONS, budget))
            .collect::<Result<Vec<_>>>()?;
        self.face_errors = vec![[[0.; INTEGRALS]; 2]; count];
        for index in 0..count {
            for axis in 0..2 {
                let mut coarse_divisions = self.divisions[index];
                coarse_divisions[axis] /= 2;
                let coarse = self.integrate(index, coarse_divisions, budget)?;
                self.face_errors[index][axis] = difference(&self.current[index], &coarse);
            }
        }
        Ok(())
    }

    /// Summed error estimates and integrals over every face.
    fn totals(&self) -> (Integrals, Integrals) {
        let mut errors = [0.; INTEGRALS];
        let mut total = [0.; INTEGRALS];
        for (index, values) in self.current.iter().enumerate() {
            for i in 0..INTEGRALS {
                errors[i] += self.face_errors[index][0][i] + self.face_errors[index][1][i];
                total[i] += values[i];
            }
        }
        (errors, total)
    }

    fn floor(&self, total: &Integrals, i: usize) -> f64 {
        total[i].abs().max(self.scales[i] * ABSOLUTE_FLOOR_FRACTION)
    }

    fn converged(&self, errors: &Integrals, total: &Integrals, relative_tolerance: f64) -> bool {
        (0..INTEGRALS).all(|i| errors[i] <= relative_tolerance * self.floor(total, i))
    }

    /// Face direction with the largest relative error still below the cap.
    fn next(&self, total: &Integrals) -> Option<(usize, usize)> {
        let score = |index: usize, axis: usize| {
            (0..INTEGRALS)
                .map(|i| self.face_errors[index][axis][i] / self.floor(total, i))
                .fold(0_f64, f64::max)
        };
        (0..self.uses.len())
            .flat_map(|index| (0..2).map(move |axis| (index, axis)))
            .filter(|&(index, axis)| self.divisions[index][axis] < MAX_DIVISIONS)
            .max_by(|&(a, aa), &(b, bb)| score(a, aa).total_cmp(&score(b, bb)))
    }

    fn refine(&mut self, index: usize, axis: usize, budget: &mut Budget) -> Result<()> {
        let old = self.current[index];
        self.divisions[index][axis] *= 2;
        let refined = self.integrate(index, self.divisions[index], budget)?;
        self.face_errors[index][axis] = difference(&refined, &old);
        // Refresh the other direction at the new grid; mixed dependence must
        // not inherit a stale error estimate from the earlier face grid.
        let other = 1 - axis;
        let mut coarse_divisions = self.divisions[index];
        coarse_divisions[other] /= 2;
        let coarse = self.integrate(index, coarse_divisions, budget)?;
        self.face_errors[index][other] = difference(&refined, &coarse);
        self.current[index] = refined;
        Ok(())
    }
}

/// Relative quadrature convergence target; error estimates do not bound all
/// floating-point or geometry errors and must not certify solid validity.
pub fn mass_properties(
    model: &Model,
    relative_tolerance: f64,
    max_evaluations: usize,
) -> Result<MassProperties> {
    model.validate()?;
    if !RELATIVE_TOLERANCE_RANGE.contains(&relative_tolerance)
        || !EVALUATION_BUDGET_RANGE.contains(&max_evaluations)
    {
        return Err(Error::new(
            "BREP_INVALID_ANALYSIS_OPTIONS",
            "Relative tolerance must be 1e-10..1e-3 and evaluation budget 100..2000000",
        ));
    }
    if model.bodies.is_empty() || model.shells.iter().any(|s| !s.closed) {
        return Err(Error::new(
            "BREP_UNSUPPORTED_OPERATION",
            "Mass properties require closed body shells",
        ));
    }
    let bounds = control_bounds(model);
    let origin = std::array::from_fn(|i| (bounds[0][i] + bounds[1][i]) / 2.);
    let scales = integral_scales(&bounds);
    let mut budget = Budget {
        used: 0,
        limit: max_evaluations,
    };
    let integration_grids = model
        .faces
        .iter()
        .enumerate()
        .map(|(i, face)| {
            let grid = conditioned_surface_grid(&face.surface, &mut budget)?;
            periodic_grid::lift(model, i, grid, &mut budget)
        })
        .collect::<Result<Vec<_>>>()?;
    let integration_spans =
        integration_spans(model, &integration_grids, relative_tolerance, &mut budget)?;
    // Independently refine the Green boundary integral and its surface
    // antiderivative. Each face retains both directional error estimates;
    // accurate directions are not uniformly refined with harder neighbors.
    let mut refinement = Refinement {
        model,
        uses: model.shells.iter().flat_map(|s| s.faces.iter()).collect(),
        origin,
        scales,
        integration_spans,
        integration_grids,
        divisions: Vec::new(),
        current: Vec::new(),
        face_errors: Vec::new(),
    };
    refinement.seed(&mut budget)?;
    loop {
        let (errors, total) = refinement.totals();
        if refinement.converged(&errors, &total, relative_tolerance) {
            return finish(&total, &errors, &scales, origin, bounds, budget.used);
        }
        let Some((index, axis)) = refinement.next(&total) else {
            break;
        };
        refinement.refine(index, axis, &mut budget)?;
    }
    Err(Error::new(
        "BREP_ANALYSIS_INDETERMINATE",
        "Mass-property quadrature did not converge within 16 subdivisions",
    ))
}
