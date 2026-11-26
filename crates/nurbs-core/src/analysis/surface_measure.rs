//! Parametrized surface area with outward-rounded bounds of the original net.
//! Integrates |S_u cross S_v| over every active knot rectangle, with multiplicity.
use crate::{
    Result, check,
    distance_bounds::Interval,
    numerics::interval_vec3::{cross, norm},
    numeric, resource,
    surface::Surface,
    trim_domain::{Location, TrimDomain},
};
pub(crate) mod jets;
pub(crate) mod first_jets;
use std::{cmp::Ordering, collections::BinaryHeap};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    Tolerance,
    WorkLimit,
    PrecisionLimit,
}
#[derive(Clone, Debug)]
pub struct AreaReport {
    pub value: f64,
    pub bounds: [f64; 2],
    pub error_upper: f64,
    pub within_tolerance: bool,
    /// Includes discarded subdivision trials and replaced parent cells.
    pub cells: usize,
    pub stop_reason: StopReason,
}
struct Cell {
    span: [usize; 2],
    domain: [[f64; 2]; 2],
    area: Interval,
    serial: usize,
    trim_location: Location,
}
impl Cell {
    fn gap(&self) -> f64 {
        self.area.hi - self.area.lo
    }
}
impl PartialEq for Cell {
    fn eq(&self, b: &Self) -> bool {
        self.gap() == b.gap() && self.serial == b.serial
    }
}
impl Eq for Cell {}
impl PartialOrd for Cell {
    fn partial_cmp(&self, b: &Self) -> Option<Ordering> {
        Some(self.cmp(b))
    }
}
impl Ord for Cell {
    fn cmp(&self, b: &Self) -> Ordering {
        self.gap()
            .total_cmp(&b.gap())
            .then(self.serial.cmp(&b.serial))
    }
}
fn add(a: [Interval; 3], b: [Interval; 3], scale: f64) -> Result<[Interval; 3]> {
    let mut out = a;
    for k in 0..3 {
        out[k] = a[k].add(b[k].mul(Interval::point(scale))?)?;
    }
    Ok(out)
}
fn cell(s: &Surface, span: [usize; 2], domain: [[f64; 2]; 2], serial: usize) -> Result<Cell> {
    let j = jets::calculate(s, span, domain, None)?;
    let normal = cross(j[1][0], j[0][1])?;
    let jacobian = norm(normal)?;
    let mut lo = jacobian.lo;
    let mut hi = jacobian.hi;
    if jacobian.lo > 0. {
        let nu = add(cross(j[2][0], j[0][1])?, cross(j[1][0], j[1][1])?, 1.)?;
        let nv = add(cross(j[1][1], j[0][1])?, cross(j[1][0], j[0][2])?, 1.)?;
        let nuu = add(
            add(cross(j[3][0], j[0][1])?, cross(j[2][0], j[1][1])?, 2.)?,
            cross(j[1][0], j[2][1])?,
            1.,
        )?;
        let nvv = add(
            add(cross(j[1][2], j[0][1])?, cross(j[1][1], j[0][2])?, 2.)?,
            cross(j[1][0], j[0][3])?,
            1.,
        )?;
        let remainder = |first, second| -> Result<Interval> {
            let first = norm(first)?;
            norm(second)?
                .add(first.mul(first)?.div(Interval::point(jacobian.lo))?)?
                .div(Interval::point(12.))
        };
        let error = remainder(nu, nuu)?.add(remainder(nv, nvv)?)?.hi;
        let mut corner_sum = Interval::point(0.);
        for u in 0..2 {
            for v in 0..2 {
                let c = jets::calculate(s, span, domain, Some([u, v]))?;
                corner_sum = corner_sum.add(norm(cross(c[1][0], c[0][1])?)?)?;
            }
        }
        let trapezoid = corner_sum.mul(Interval::point(0.25))?;
        lo = lo.max((trapezoid.lo - error).next_down().max(0.));
        hi = hi.min((trapezoid.hi + error).next_up());
    }
    Ok(Cell {
        span,
        domain,
        area: Interval::new(lo, hi)?,
        serial,
        trim_location: Location::Unresolved,
    })
}

fn sum(heap: &BinaryHeap<Cell>) -> Result<Interval> {
    let mut total = Interval::point(0.);
    for c in heap {
        total = total.add(c.area)?;
    }
    Interval::new(total.lo.max(0.), total.hi)
}
fn report(total: Interval, cells: usize, tolerance: f64, reason: StopReason) -> Result<AreaReport> {
    let value = total.lo + (total.hi - total.lo) / 2.;
    let error_upper = (value - total.lo).max(total.hi - value).next_up();
    numeric(
        value.is_finite() && error_upper.is_finite(),
        "Surface area exhausted numeric range",
    )?;
    let within_tolerance = error_upper <= tolerance;
    Ok(AreaReport {
        value,
        bounds: [total.lo, total.hi],
        error_upper,
        within_tolerance,
        cells,
        stop_reason: if within_tolerance {
            StopReason::Tolerance
        } else {
            reason
        },
    })
}
/// Absolute tolerance is in squared model units. A budget stop returns valid
/// area bounds without asserting the requested accuracy. No trim-loop removal
/// or union-of-images area is implied; folds/overlaps count with multiplicity.
pub fn area(s: &Surface, tolerance: f64, max_cells: usize) -> Result<AreaReport> {
    s.validate()?;
    check(
        tolerance.is_finite() && tolerance > 0.,
        "Area tolerance must be finite and positive",
    )?;
    check(
        (1..=100_000).contains(&max_cells),
        "Area work budget must be 1..100000 cell enclosures",
    )?;
    let mut heap = BinaryHeap::new();
    let mut cells = 0;
    for u in s.degree_u..s.control_points.len() {
        for v in s.degree_v..s.control_points[0].len() {
            if s.knots_u[u] < s.knots_u[u + 1] && s.knots_v[v] < s.knots_v[v + 1] {
                if cells == max_cells {
                    return Err(resource(
                        "Initial area knot rectangles exceed the work budget",
                    ));
                }
                heap.push(cell(
                    s,
                    [u, v],
                    [
                        [s.knots_u[u], s.knots_u[u + 1]],
                        [s.knots_v[v], s.knots_v[v + 1]],
                    ],
                    cells,
                )?);
                cells += 1;
            }
        }
    }
    let mut total = sum(&heap)?;
    loop {
        let r = report(total, cells, tolerance, StopReason::WorkLimit)?;
        if r.within_tolerance || cells + 2 > max_cells {
            total = sum(&heap)?;
            let recomputed = report(total, cells, tolerance, StopReason::WorkLimit)?;
            if recomputed.within_tolerance || cells + 2 > max_cells {
                return Ok(recomputed);
            }
        }
        let parent = heap.pop().unwrap();
        let mut best: Option<(Cell, Cell, Interval)> = None;
        // Try both axes, charging every constructed cell to the global budget.
        // Choose the lower uncertainty rather than splitting a constant axis.
        for axis in 0..2 {
            let [a, b] = parent.domain[axis];
            let mid = a + (b - a) / 2.;
            if !(a < mid && mid < b) || cells + 2 > max_cells {
                continue;
            }
            let mut left = parent.domain;
            let mut right = left;
            left[axis][1] = mid;
            right[axis][0] = mid;
            let left = cell(s, parent.span, left, cells)?;
            let right = cell(s, parent.span, right, cells + 1)?;
            cells += 2;
            let combined = left.area.add(right.area)?;
            if best
                .as_ref()
                .is_none_or(|(_, _, old)| combined.hi - combined.lo < old.hi - old.lo)
            {
                best = Some((left, right, combined));
            }
        }
        let Some((left, right, _)) = best else {
            heap.push(parent);
            return report(sum(&heap)?, cells, tolerance, StopReason::PrecisionLimit);
        };
        total = Interval::new(
            (total.lo - parent.area.lo).next_down(),
            (total.hi - parent.area.hi).next_up(),
        )?
        .add(left.area)?
        .add(right.area)?;
        total.lo = total.lo.max(0.);
        heap.push(left);
        heap.push(right);
    }
}

#[derive(Clone, Debug)]
pub struct TrimmedAreaReport {
    /// Bounds on the area of the retained parameter region, counted with multiplicity.
    pub area: Option<AreaReport>,
    pub minimum_surface_cells: usize,
    pub trim_cells: usize,
}

/// Enclose the true surface area of a periodic trim patch, including holes.
/// A UV rectangle unresolved by the trim classifier contributes zero to the
/// lower bound and its entire surface area to the upper bound.
pub fn periodic_trim_patch_area(
    patch: &crate::trim::periodic_patch::PeriodicTrimPatch,
    tolerance: f64,
    tolerance_uv: f64,
    max_surface_cells: usize,
    max_trim_cells: usize,
) -> Result<TrimmedAreaReport> {
    let trim = TrimDomain::new(&patch.trim.loops, tolerance_uv)?;
    trimmed_surface_area(
        &patch.surface,
        &trim,
        tolerance,
        max_surface_cells,
        max_trim_cells,
    )
}

/// Integrate a previously prepared trim domain on a NURBS surface.
/// During subdivision, determinate parent classifications apply to both children.
pub fn trimmed_surface_area(
    surface: &Surface,
    trim: &TrimDomain,
    tolerance: f64,
    max_surface_cells: usize,
    max_trim_cells: usize,
) -> Result<TrimmedAreaReport> {
    trimmed_surface_area_in_domain(
        surface,
        trim,
        None,
        tolerance,
        max_surface_cells,
        max_trim_cells,
    )
    .map(|(report, _)| report)
}

/// Enclose the retained surface area inside a finite UV rectangle.
/// The window is intersected with the active surface domain.
pub fn trimmed_surface_area_in_window(
    surface: &Surface,
    trim: &TrimDomain,
    window: [[f64; 2]; 2],
    tolerance: f64,
    max_surface_cells: usize,
    max_trim_cells: usize,
) -> Result<TrimmedAreaReport> {
    validate_trim_window(window)?;
    trimmed_surface_area_in_domain(
        surface,
        trim,
        Some(window),
        tolerance,
        max_surface_cells,
        max_trim_cells,
    )
    .map(|(report, _)| report)
}

fn validate_trim_window(window: [[f64; 2]; 2]) -> Result<()> {
    check(
        window
            .iter()
            .all(|&[lo, hi]| lo.is_finite() && hi.is_finite() && lo < hi),
        "Trimmed area window needs finite increasing UV intervals",
    )
}

fn trimmed_surface_area_in_domain(
    surface: &Surface,
    trim: &TrimDomain,
    window: Option<[[f64; 2]; 2]>,
    tolerance: f64,
    max_surface_cells: usize,
    max_trim_cells: usize,
) -> Result<(TrimmedAreaReport, BinaryHeap<Cell>)> {
    surface.validate()?;
    check(
        tolerance.is_finite()
            && tolerance > 0.
            && (1..=100_000).contains(&max_surface_cells)
            && (1..=1_000_000).contains(&max_trim_cells),
        "Trimmed area needs positive finite tolerance and valid surface and trim cell limits",
    )?;
    let active = |knots: &[f64], degree: usize, count: usize, axis: usize| {
        (degree..count)
            .filter_map(|i| {
                let [start, end] = [knots[i], knots[i + 1]];
                let [lo, hi] = window.map_or([start, end], |window| {
                    [start.max(window[axis][0]), end.min(window[axis][1])]
                });
                (lo < hi).then_some((i, [lo, hi]))
            })
            .collect::<Vec<_>>()
    };
    let u = active(
        &surface.knots_u,
        surface.degree_u,
        surface.control_points.len(),
        0,
    );
    let v = active(
        &surface.knots_v,
        surface.degree_v,
        surface.control_points[0].len(),
        1,
    );
    let minimum_surface_cells = u.len() * v.len();
    if minimum_surface_cells == 0 {
        return Ok((
            TrimmedAreaReport {
                area: Some(AreaReport {
                    value: 0.,
                    bounds: [0.; 2],
                    error_upper: 0.,
                    within_tolerance: true,
                    cells: 0,
                    stop_reason: StopReason::Tolerance,
                }),
                minimum_surface_cells,
                trim_cells: 0,
            },
            BinaryHeap::new(),
        ));
    }
    if minimum_surface_cells > max_surface_cells {
        return Ok((
            TrimmedAreaReport {
                area: None,
                minimum_surface_cells,
                trim_cells: 0,
            },
            BinaryHeap::new(),
        ));
    }

    let mut trim_cells = 0;
    let mut make_cell = |span, domain, serial, parent_location| -> Result<Cell> {
        let remaining = max_trim_cells - trim_cells;
        let location = if parent_location != Location::Unresolved || remaining == 0 {
            parent_location
        } else {
            let report = trim.classify(domain, remaining.min(100_000))?;
            trim_cells += report.cells;
            report.location
        };
        if location == Location::Outside {
            return Ok(Cell {
                span,
                domain,
                area: Interval::point(0.),
                serial,
                trim_location: location,
            });
        }
        let mut candidate = cell(surface, span, domain, serial)?;
        candidate.trim_location = location;
        if location == Location::Unresolved {
            candidate.area.lo = 0.;
        }
        Ok(candidate)
    };

    let mut cells = 0;
    let mut heap = BinaryHeap::new();
    for (u, u_domain) in u {
        for &(v, v_domain) in &v {
            heap.push(make_cell(
                [u, v],
                [u_domain, v_domain],
                cells,
                Location::Unresolved,
            )?);
            cells += 1;
        }
    }
    let mut total = sum(&heap)?;
    let area = loop {
        let current = report(total, cells, tolerance, StopReason::WorkLimit)?;
        if current.within_tolerance || cells + 2 > max_surface_cells {
            let recomputed = report(sum(&heap)?, cells, tolerance, StopReason::WorkLimit)?;
            if recomputed.within_tolerance || cells + 2 > max_surface_cells {
                break recomputed;
            }
        }
        let parent = heap
            .pop()
            .ok_or_else(|| crate::numeric_err("Trimmed area has no active cells"))?;
        let width = parent.domain.map(|[start, end]| end - start);
        let preferred = usize::from(width[1] > width[0]);
        let split = [preferred, 1 - preferred].into_iter().find_map(|axis| {
            let [lo, hi] = parent.domain[axis];
            let mid = crate::distance_bounds::midpoint(lo, hi);
            (lo < mid && mid < hi).then_some((axis, mid))
        });
        let Some((axis, mid)) = split else {
            heap.push(parent);
            break report(sum(&heap)?, cells, tolerance, StopReason::PrecisionLimit)?;
        };
        let mut left = parent.domain;
        let mut right = parent.domain;
        left[axis][1] = mid;
        right[axis][0] = mid;
        let left = make_cell(parent.span, left, cells, parent.trim_location)?;
        let right = make_cell(parent.span, right, cells + 1, parent.trim_location)?;
        cells += 2;
        total = Interval::new(
            (total.lo - parent.area.lo).next_down(),
            (total.hi - parent.area.hi).next_up(),
        )?
        .add(left.area)?
        .add(right.area)?;
        total.lo = total.lo.max(0.);
        heap.push(left);
        heap.push(right);
    };
    Ok((
        TrimmedAreaReport {
            area: Some(area),
            minimum_surface_cells,
            trim_cells,
        },
        heap,
    ))
}

#[derive(Clone, Debug)]
pub struct TrimmedCentroidReport {
    pub measure: TrimmedAreaReport,
    /// World-space bounds, present when the retained area is proved positive.
    pub centroid_bounds: Option<[[f64; 2]; 3]>,
}

/// Bound the uniform-area centroid in world coordinates. The tolerance
/// controls the area refinement; the centroid enclosure reports its own width.
pub fn trimmed_surface_centroid(
    surface: &Surface,
    trim: &TrimDomain,
    area_tolerance: f64,
    max_surface_cells: usize,
    max_trim_cells: usize,
) -> Result<TrimmedCentroidReport> {
    let (measure, cells) = trimmed_surface_area_in_domain(
        surface,
        trim,
        None,
        area_tolerance,
        max_surface_cells,
        max_trim_cells,
    )?;
    let centroid_bounds = measure
        .area
        .as_ref()
        .filter(|area| area.bounds[0] > 0.)
        .map(|area| enclose_trimmed_centroid(surface, &cells, area))
        .transpose()?;
    Ok(TrimmedCentroidReport {
        measure,
        centroid_bounds,
    })
}

fn enclose_trimmed_centroid(
    surface: &Surface,
    cells: &BinaryHeap<Cell>,
    area: &AreaReport,
) -> Result<[[f64; 2]; 3]> {
    let origin = [0, 1, 2].map(|axis| Interval::point(surface.control_points[0][0][axis]));
    let mut moments = TrimmedFirstMoments::new(origin);
    moments.accumulate(surface, cells)?;
    moments.centroid(area)
}

struct TrimmedFirstMoments {
    origin: [Interval; 3],
    moment: [Interval; 3],
    support: [[f64; 2]; 3],
}

impl TrimmedFirstMoments {
    fn new(origin: [Interval; 3]) -> Self {
        Self {
            origin,
            moment: [Interval::point(0.); 3],
            support: [[f64::INFINITY, f64::NEG_INFINITY]; 3],
        }
    }

    fn accumulate(&mut self, surface: &Surface, cells: &BinaryHeap<Cell>) -> Result<()> {
        for cell in cells {
            if cell.area.hi == 0. {
                continue;
            }
            let position = cell_position_relative_to(surface, cell, self.origin)?;
            for (axis, relative) in position.into_iter().enumerate() {
                self.support[axis][0] = self.support[axis][0].min(relative.lo);
                self.support[axis][1] = self.support[axis][1].max(relative.hi);
                self.moment[axis] = self.moment[axis].add(relative.mul(cell.area)?)?;
            }
        }
        Ok(())
    }

    fn centroid(&self, area: &AreaReport) -> Result<[[f64; 2]; 3]> {
        let denominator = Interval::new(area.bounds[0], area.bounds[1])?;
        let mut bounds = [[0.; 2]; 3];
        for (axis, output) in bounds.iter_mut().enumerate() {
            let bound = self.moment[axis]
                .div(denominator)?
                .intersect(self.support[axis][0], self.support[axis][1])?
                .add(self.origin[axis])?;
            *output = [bound.lo, bound.hi];
        }
        Ok(bounds)
    }
}

fn cell_position_relative_to(
    surface: &Surface,
    cell: &Cell,
    origin: [Interval; 3],
) -> Result<[Interval; 3]> {
    let mut point = first_jets::calculate(surface, cell.span, cell.domain)?[0];
    let anchor =
        &surface.control_points[cell.span[0] - surface.degree_u][cell.span[1] - surface.degree_v];
    for axis in 0..3 {
        point[axis] = point[axis].add(Interval::point(anchor[axis]).sub(origin[axis])?)?;
    }
    Ok(point)
}

#[derive(Clone, Debug)]
pub struct TrimmedInertiaReport {
    pub measure: TrimmedAreaReport,
    /// Symmetric inertia tensor for unit areal density, with an interval per entry.
    pub inertia_bounds: Option<[[[f64; 2]; 3]; 3]>,
}

/// Bound the uniform-area inertia tensor of a trimmed surface about a fixed
/// world point. Area tolerance controls subdivision. Tensor widths describe
/// the remaining uncertainty in each entry.
pub fn trimmed_surface_area_inertia(
    surface: &Surface,
    trim: &TrimDomain,
    origin: [f64; 3],
    area_tolerance: f64,
    max_surface_cells: usize,
    max_trim_cells: usize,
) -> Result<TrimmedInertiaReport> {
    check(
        origin.iter().all(|x| x.is_finite()),
        "Inertia reference point must be finite",
    )?;
    let (measure, cells) = trimmed_surface_area_in_domain(
        surface,
        trim,
        None,
        area_tolerance,
        max_surface_cells,
        max_trim_cells,
    )?;
    if measure.area.is_none() {
        return Ok(TrimmedInertiaReport {
            measure,
            inertia_bounds: None,
        });
    }
    let inertia_bounds = enclose_trimmed_inertia(
        cells.iter().map(|cell| (surface, cell)),
        origin.map(Interval::point),
    )?;
    Ok(TrimmedInertiaReport {
        measure,
        inertia_bounds: Some(inertia_bounds),
    })
}

#[derive(Clone, Debug)]
pub struct TrimmedCentralInertiaReport {
    pub centroid: TrimmedCentroidReport,
    /// Symmetric tensor about the true area centroid, for unit density.
    pub central_inertia_bounds: Option<[[[f64; 2]; 3]; 3]>,
}

/// Enclose the centroid and its central inertia tensor on one certified
/// trim partition. A positive area proof is required for both quantities.
pub fn trimmed_surface_central_inertia(
    surface: &Surface,
    trim: &TrimDomain,
    area_tolerance: f64,
    max_surface_cells: usize,
    max_trim_cells: usize,
) -> Result<TrimmedCentralInertiaReport> {
    trimmed_surface_central_inertia_in_domain(
        surface,
        trim,
        None,
        area_tolerance,
        max_surface_cells,
        max_trim_cells,
    )
}

/// Enclose centroid and central inertia of the retained surface inside a UV window.
pub fn trimmed_surface_central_inertia_in_window(
    surface: &Surface,
    trim: &TrimDomain,
    window: [[f64; 2]; 2],
    area_tolerance: f64,
    max_surface_cells: usize,
    max_trim_cells: usize,
) -> Result<TrimmedCentralInertiaReport> {
    validate_trim_window(window)?;
    trimmed_surface_central_inertia_in_domain(
        surface,
        trim,
        Some(window),
        area_tolerance,
        max_surface_cells,
        max_trim_cells,
    )
}

fn trimmed_surface_central_inertia_in_domain(
    surface: &Surface,
    trim: &TrimDomain,
    window: Option<[[f64; 2]; 2]>,
    area_tolerance: f64,
    max_surface_cells: usize,
    max_trim_cells: usize,
) -> Result<TrimmedCentralInertiaReport> {
    let (measure, cells) = trimmed_surface_area_in_domain(
        surface,
        trim,
        window,
        area_tolerance,
        max_surface_cells,
        max_trim_cells,
    )?;
    let Some(area) = measure.area.as_ref().filter(|area| area.bounds[0] > 0.) else {
        return Ok(TrimmedCentralInertiaReport {
            centroid: TrimmedCentroidReport {
                measure,
                centroid_bounds: None,
            },
            central_inertia_bounds: None,
        });
    };
    let centroid_bounds = enclose_trimmed_centroid(surface, &cells, area)?;
    let center = centroid_intervals(centroid_bounds)?;
    let central_inertia_bounds =
        enclose_trimmed_inertia(cells.iter().map(|cell| (surface, cell)), center)?;
    Ok(TrimmedCentralInertiaReport {
        centroid: TrimmedCentroidReport {
            measure,
            centroid_bounds: Some(centroid_bounds),
        },
        central_inertia_bounds: Some(central_inertia_bounds),
    })
}

fn centroid_intervals([[x0, x1], [y0, y1], [z0, z1]]: [[f64; 2]; 3]) -> Result<[Interval; 3]> {
    Ok([
        Interval::new(x0, x1)?,
        Interval::new(y0, y1)?,
        Interval::new(z0, z1)?,
    ])
}

fn enclose_trimmed_inertia<'a>(
    cells: impl Iterator<Item = (&'a Surface, &'a Cell)>,
    origin: [Interval; 3],
) -> Result<[[[f64; 2]; 3]; 3]> {
    let mut tensor = [[Interval::point(0.); 3]; 3];
    for (surface, cell) in cells {
        if cell.area.hi == 0. {
            continue;
        }
        let position = cell_position_relative_to(surface, cell, origin)?;
        let mut squares = [Interval::point(0.); 3];
        for (output, x) in squares.iter_mut().zip(position) {
            let square = x.mul(x)?;
            *output = square.intersect(0., square.hi)?;
        }
        for (i, row) in tensor.iter_mut().enumerate() {
            let diagonal = squares[(i + 1) % 3].add(squares[(i + 2) % 3])?;
            row[i] = row[i].add(diagonal.mul(cell.area)?)?;
            for j in i + 1..3 {
                let product = position[i].mul(position[j])?.mul(cell.area)?;
                row[j] = row[j].sub(product)?;
            }
        }
    }
    let mut bounds = [[[0.; 2]; 3]; 3];
    for (i, row) in tensor.into_iter().enumerate() {
        bounds[i][i] = [row[i].lo.max(0.), row[i].hi];
        for j in i + 1..3 {
            let interval = [row[j].lo, row[j].hi];
            bounds[i][j] = interval;
            bounds[j][i] = interval;
        }
    }
    Ok(bounds)
}

#[derive(Clone, Debug)]
pub struct TrimmedPrincipalInertiaReport {
    pub central: TrimmedCentralInertiaReport,
    /// Enclosures in increasing eigenvalue order, including repeated moments.
    pub principal_bounds: Option<[[f64; 2]; 3]>,
    pub within_moment_tolerance: bool,
}

impl TrimmedCentralInertiaReport {
    /// Bound the sorted principal moments without choosing axes for repeated
    /// eigenvalues. A spectral comparison is used only if its sign is proved.
    pub fn bound_principal_moments(
        self,
        moment_tolerance: f64,
    ) -> Result<TrimmedPrincipalInertiaReport> {
        let (principal_bounds, within_moment_tolerance) =
            bound_optional_principal_moments(self.central_inertia_bounds, moment_tolerance)?;
        Ok(TrimmedPrincipalInertiaReport {
            central: self,
            principal_bounds,
            within_moment_tolerance,
        })
    }
}

/// Enclose the moment about an axis through the reference point of a
/// symmetric tensor. The direction need not be normalized.
pub fn bound_directional_inertia(
    bounds: [[[f64; 2]; 3]; 3],
    direction: [f64; 3],
) -> Result<[f64; 2]> {
    directional_inertia(bounds, &InertiaAxis::new(direction)?)
}

fn directional_inertia(
    bounds: [[[f64; 2]; 3]; 3],
    axis: &InertiaAxis,
) -> Result<[f64; 2]> {
    let matrix = symmetric_tensor_intervals(bounds)?;
    if let Some(i) = axis.coordinate {
        return Ok(bounds[i][i]);
    }
    let mut moment = Interval::point(0.);
    for (i, row) in matrix.iter().enumerate() {
        if axis.components[i] == Interval::point(0.) {
            continue;
        }
        let diagonal_weight = axis.squares[i].div(axis.norm_squared)?;
        moment = moment.add(row[i].mul(diagonal_weight)?)?;
        for (j, entry) in row.iter().copied().enumerate().skip(i + 1) {
            if axis.components[j] == Interval::point(0.) {
                continue;
            }
            let weight = axis.components[i]
                .mul(axis.components[j])?
                .mul(Interval::point(2.))?;
            moment = moment.add(entry.mul(weight.div(axis.norm_squared)?)?)?;
        }
    }
    Ok([moment.lo, moment.hi])
}

struct InertiaAxis {
    components: [Interval; 3],
    squares: [Interval; 3],
    norm_squared: Interval,
    coordinate: Option<usize>,
}

impl InertiaAxis {
    fn new(direction: [f64; 3]) -> Result<Self> {
        let scale = direction.into_iter().map(f64::abs).fold(0., f64::max);
        check(
            scale > 0. && direction.iter().all(|x| x.is_finite()),
            "Inertia axis must have a finite nonzero direction",
        )?;

        let mut axis = [Interval::point(0.); 3];
        let mut squares = axis;
        let mut norm_squared = Interval::point(0.);
        for (i, component) in direction.into_iter().enumerate() {
            if component == 0. {
                continue;
            }
            let scaled = component / scale;
            axis[i] = if component.abs() == scale {
                Interval::point(scaled)
            } else {
                Interval::new(scaled.next_down(), scaled.next_up())?
            };
            let square = axis[i].mul(axis[i])?;
            squares[i] = square.intersect(0., square.hi)?;
            norm_squared = norm_squared.add(squares[i])?;
        }

        let coordinate = direction
            .iter()
            .position(|&x| x != 0.)
            .filter(|&i| direction.iter().skip(i + 1).all(|&x| x == 0.));
        Ok(Self {
            components: axis,
            squares,
            norm_squared,
            coordinate,
        })
    }
}

/// Enclose inertia around the line defined by a point and a nonzero
/// direction. The parallel-axis correction uses squared perpendicular
/// distance from the centroid to the line.
pub fn bound_inertia_about_line(
    central: [[[f64; 2]; 3]; 3],
    mass_bounds: [f64; 2],
    centroid_bounds: [[f64; 2]; 3],
    point: [f64; 3],
    direction: [f64; 3],
) -> Result<[f64; 2]> {
    inertia_about_line(
        central,
        mass_bounds,
        centroid_bounds,
        point,
        &InertiaAxis::new(direction)?,
    )
}

fn inertia_about_line(
    central: [[[f64; 2]; 3]; 3],
    mass_bounds: [f64; 2],
    centroid_bounds: [[f64; 2]; 3],
    point: [f64; 3],
    axis: &InertiaAxis,
) -> Result<[f64; 2]> {
    let centroid = centroid_intervals(centroid_bounds)?;
    let mass = Interval::new(mass_bounds[0], mass_bounds[1])?;
    check(mass.lo >= 0., "Inertia mass bounds must be nonnegative")?;
    check(
        point.iter().all(|x| x.is_finite()),
        "Inertia axis point must be finite",
    )?;
    let moment = directional_inertia(central, axis)?;
    if mass.hi == 0. {
        return Ok(moment);
    }

    let mut offset = [Interval::point(0.); 3];
    for (i, coordinate) in centroid.into_iter().enumerate() {
        if axis.coordinate == Some(i) || (coordinate.lo == point[i] && coordinate.hi == point[i]) {
            continue;
        }
        offset[i] = coordinate.sub(Interval::point(point[i]))?;
    }
    if offset == [Interval::point(0.); 3] {
        return Ok(moment);
    }
    let perpendicular = if axis.coordinate.is_some() {
        offset
    } else {
        cross(offset, axis.components)?
    };
    let mut distance_numerator = Interval::point(0.);
    for coordinate in perpendicular {
        let square = coordinate.mul(coordinate)?;
        distance_numerator = distance_numerator.add(square.intersect(0., square.hi)?)?;
    }
    let distance_squared = if axis.coordinate.is_some() {
        distance_numerator
    } else {
        distance_numerator.div(axis.norm_squared)?
    };
    let correction = distance_squared.mul(mass)?;
    let result = Interval::new(moment[0], moment[1])?.add(correction)?;
    Ok([result.lo.max(moment[0]), result.hi])
}

/// Enclose the radius of gyration from a polar moment and the strictly
/// positive mass of the same distribution.
pub fn bound_radius_of_gyration(
    moment_bounds: [f64; 2],
    mass_bounds: [f64; 2],
) -> Result<[f64; 2]> {
    let moment = Interval::new(moment_bounds[0], moment_bounds[1])?;
    let mass = Interval::new(mass_bounds[0], mass_bounds[1])?;
    check(
        moment.hi >= 0.,
        "Radius of gyration requires a nonnegative moment",
    )?;
    check(
        mass.lo > 0.,
        "Radius of gyration requires strictly positive mass",
    )?;
    if moment.hi == 0. {
        return Ok([0.; 2]);
    }

    // Take square roots before dividing so the intermediate moment/mass
    // ratio need not fit in binary64.
    let numerator = Interval::new(
        moment.lo.max(0.).sqrt().next_down().max(0.),
        moment.hi.sqrt().next_up(),
    )?;
    let denominator = Interval::new(mass.lo.sqrt().next_down(), mass.hi.sqrt().next_up())?;
    let radius = numerator.div(denominator)?;
    Ok([radius.lo.max(0.), radius.hi])
}

/// Translate a central inertia enclosure to a fixed reference point using
/// the parallel-axis theorem. Tensor, mass and centroid must describe the
/// same distribution.
pub fn bound_inertia_about_point(
    central: [[[f64; 2]; 3]; 3],
    mass_bounds: [f64; 2],
    centroid_bounds: [[f64; 2]; 3],
    point: [f64; 3],
) -> Result<[[[f64; 2]; 3]; 3]> {
    let mass = Interval::new(mass_bounds[0], mass_bounds[1])?;
    let centroid = centroid_intervals(centroid_bounds)?;
    let reference = centroid_intervals(point.map(|coordinate| [coordinate; 2]))?;
    translate_inertia(central, mass, centroid, reference)
}

fn translate_inertia(
    central: [[[f64; 2]; 3]; 3],
    mass: Interval,
    centroid: [Interval; 3],
    reference: [Interval; 3],
) -> Result<[[[f64; 2]; 3]; 3]> {
    check(mass.lo >= 0., "Inertia mass bounds must be nonnegative")?;
    let matrix = symmetric_tensor_intervals(central)?;
    if mass.hi == 0. {
        return Ok(central);
    }
    let mut offset = [Interval::point(0.); 3];
    let mut squares = offset;
    for (i, interval) in centroid.into_iter().enumerate() {
        if interval.lo == interval.hi && interval == reference[i] {
            continue;
        }
        let delta = interval.sub(reference[i])?;
        offset[i] = delta;
        let square = delta.mul(delta)?;
        squares[i] = square.intersect(0., square.hi)?;
    }
    if offset == [Interval::point(0.); 3] {
        return Ok(central);
    }
    let mut translated = central;
    for (i, row) in matrix.iter().enumerate() {
        let perpendicular_squared = squares[(i + 1) % 3].add(squares[(i + 2) % 3])?;
        let correction = perpendicular_squared.mul(mass)?;
        let diagonal = row[i].add(correction.intersect(0., correction.hi)?)?;
        translated[i][i] = [diagonal.lo.max(row[i].lo), diagonal.hi];
        for (j, entry) in row.iter().copied().enumerate().skip(i + 1) {
            if offset[i] == Interval::point(0.) || offset[j] == Interval::point(0.) {
                continue;
            }
            let term = entry.sub(offset[i].mul(offset[j])?.mul(mass)?)?;
            translated[i][j] = [term.lo, term.hi];
            translated[j][i] = translated[i][j];
        }
    }
    Ok(translated)
}

/// Area and inertia at unit areal density in a shared Cartesian frame.
#[derive(Clone, Copy, Debug)]
pub struct SurfaceMassPropertiesBounds {
    pub area_bounds: [f64; 2],
    pub centroid_bounds: [[f64; 2]; 3],
    pub central_inertia_bounds: [[[f64; 2]; 3]; 3],
}

/// Add the measures of complete surface pieces, counting each piece once.
/// Each input must certify strictly positive area.
pub fn combine_surface_mass_properties(
    pieces: &[SurfaceMassPropertiesBounds],
) -> Result<SurfaceMassPropertiesBounds> {
    let combined = combine_mass_properties(pieces.iter().map(|piece| {
        Ok(SurfaceAssemblyMassBounds {
            mass_bounds: piece.area_bounds,
            centroid_bounds: piece.centroid_bounds,
            central_inertia_bounds: piece.central_inertia_bounds,
        })
    }))?;
    Ok(SurfaceMassPropertiesBounds {
        area_bounds: combined.mass_bounds,
        centroid_bounds: combined.centroid_bounds,
        central_inertia_bounds: combined.central_inertia_bounds,
    })
}

/// Certified mass properties of a surface assembly with density applied.
#[derive(Clone, Copy, Debug)]
pub struct SurfaceAssemblyMassBounds {
    pub mass_bounds: [f64; 2],
    pub centroid_bounds: [[f64; 2]; 3],
    pub central_inertia_bounds: [[[f64; 2]; 3]; 3],
}

/// Combine pieces with independent, uniform areal density intervals.
/// Every area and density interval must be strictly positive.
pub fn combine_surface_mass_properties_with_density(
    pieces: &[(SurfaceMassPropertiesBounds, [f64; 2])],
) -> Result<SurfaceAssemblyMassBounds> {
    combine_mass_properties(
        pieces
            .iter()
            .map(|(piece, density)| apply_surface_density(piece, *density)),
    )
}

fn apply_surface_density(
    piece: &SurfaceMassPropertiesBounds,
    [lo, hi]: [f64; 2],
) -> Result<SurfaceAssemblyMassBounds> {
    let density = Interval::new(lo, hi)?;
    let area = Interval::new(piece.area_bounds[0], piece.area_bounds[1])?;
    check(
        density.lo > 0. && area.lo > 0.,
        "Surface area and areal density must be strictly positive",
    )?;
    let mass = area.mul(density)?;
    let mut inertia = symmetric_tensor_intervals(piece.central_inertia_bounds)?;
    for row in &mut inertia {
        for moment in row {
            *moment = moment.mul(density)?;
        }
    }
    Ok(SurfaceAssemblyMassBounds {
        mass_bounds: [mass.lo, mass.hi],
        centroid_bounds: piece.centroid_bounds,
        central_inertia_bounds: inertia.map(|row| row.map(|moment| [moment.lo, moment.hi])),
    })
}

fn combine_mass_properties(
    pieces: impl Iterator<Item = Result<SurfaceAssemblyMassBounds>>,
) -> Result<SurfaceAssemblyMassBounds> {
    let pieces = pieces.collect::<Result<Vec<_>>>()?;
    let (mass, centroid) = enclose_mass_weighted_centroid(&pieces)?;
    Ok(SurfaceAssemblyMassBounds {
        mass_bounds: [mass.lo, mass.hi],
        centroid_bounds: centroid.map(|coordinate| [coordinate.lo, coordinate.hi]),
        central_inertia_bounds: sum_translated_surface_inertia(&pieces, centroid)?,
    })
}

/// Enclose the complete tensor about a fixed reference point, translating
/// each component before summing its contribution.
pub fn bound_assembly_inertia_about_point(
    pieces: &[SurfaceAssemblyMassBounds],
    point: [f64; 3],
) -> Result<[[[f64; 2]; 3]; 3]> {
    check(
        !pieces.is_empty(),
        "Surface mass properties require at least one piece",
    )?;
    let reference = centroid_intervals(point.map(|coordinate| [coordinate; 2]))?;
    sum_translated_surface_inertia(pieces, reference)
}

fn sum_translated_surface_inertia(
    pieces: &[SurfaceAssemblyMassBounds],
    reference: [Interval; 3],
) -> Result<[[[f64; 2]; 3]; 3]> {
    let mut inertia = [[Interval::point(0.); 3]; 3];
    for piece in pieces {
        let translated = translate_inertia(
            piece.central_inertia_bounds,
            Interval::new(piece.mass_bounds[0], piece.mass_bounds[1])?,
            centroid_intervals(piece.centroid_bounds)?,
            reference,
        )?;
        for (i, row) in inertia.iter_mut().enumerate() {
            for (value, [lo, hi]) in row.iter_mut().zip(translated[i]) {
                *value = value.add(Interval::new(lo, hi)?)?;
            }
        }
    }
    Ok(inertia.map(|row| row.map(|value| [value.lo, value.hi])))
}

/// Enclose the moment of a surface assembly about a fixed spatial line by
/// adding each piece's moment about that same line.
pub fn bound_assembly_inertia_about_line(
    pieces: &[SurfaceAssemblyMassBounds],
    point: [f64; 3],
    direction: [f64; 3],
) -> Result<[f64; 2]> {
    check(
        !pieces.is_empty(),
        "Surface mass properties require at least one piece",
    )?;
    let axis = InertiaAxis::new(direction)?;
    let mut total = Interval::point(0.);
    for piece in pieces {
        let [lo, hi] = inertia_about_line(
            piece.central_inertia_bounds,
            piece.mass_bounds,
            piece.centroid_bounds,
            point,
            &axis,
        )?;
        total = add_component_masses(total, Interval::new(lo.max(0.), hi)?)?;
    }
    Ok([total.lo, total.hi])
}

/// Enclose the common center of positive-mass pieces. The extremal mass
/// assignments account for the same weights in the numerator and denominator.
pub fn bound_mass_weighted_centroid(
    pieces: &[SurfaceAssemblyMassBounds],
) -> Result<[[f64; 2]; 3]> {
    let (_, centroid) = enclose_mass_weighted_centroid(pieces)?;
    Ok(centroid.map(|axis| [axis.lo, axis.hi]))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BalanceTolerance {
    Within,
    Outside,
    Undetermined,
}

/// Static unbalance in mass times length units.
#[derive(Clone, Copy, Debug)]
pub struct StaticUnbalanceBounds {
    pub vector_bounds: [[f64; 2]; 3],
    pub magnitude_bounds: [f64; 2],
}

/// Enclose the mass-weighted perpendicular displacement from a rotation axis.
/// Signed contributions are summed before taking the magnitude.
pub fn bound_assembly_static_unbalance(
    pieces: &[SurfaceAssemblyMassBounds],
    point: [f64; 3],
    direction: [f64; 3],
) -> Result<StaticUnbalanceBounds> {
    check(
        !pieces.is_empty(),
        "Surface mass properties require at least one piece",
    )?;
    let axis = InertiaAxis::new(direction)?;
    let origin = centroid_intervals(point.map(|coordinate| [coordinate; 2]))?;
    let mut moment = [Interval::point(0.); 3];
    for piece in pieces {
        let mass = positive_piece_mass(piece)?;
        let center = centroid_intervals(piece.centroid_bounds)?;
        let offset = crate::numerics::interval_vec3::sub(center, origin)?;
        let contribution = cross(offset, axis.components)?;
        for (sum, component) in moment.iter_mut().zip(contribution) {
            *sum = sum.add(component.mul(mass)?)?;
        }
    }
    let vector = crate::numerics::interval_vec3::div(
        cross(axis.components, moment)?,
        axis.norm_squared,
    )?;
    let magnitude = norm(moment)?.div(plane_normal_length(&axis)?)?;
    Ok(StaticUnbalanceBounds {
        vector_bounds: vector.map(|entry| [entry.lo, entry.hi]),
        magnitude_bounds: [magnitude.lo.max(0.), magnitude.hi],
    })
}

/// Couple unbalance in mass times squared length units.
#[derive(Clone, Copy, Debug)]
pub struct CoupleUnbalanceBounds {
    /// Components of the cross product of the unit axis and its inertia image.
    pub vector_bounds: [[f64; 2]; 3],
    pub magnitude_bounds: [f64; 2],
}

/// Enclose couple unbalance about an axis at a fixed axial reference point.
pub fn bound_assembly_couple_unbalance(
    pieces: &[SurfaceAssemblyMassBounds],
    point: [f64; 3],
    direction: [f64; 3],
) -> Result<CoupleUnbalanceBounds> {
    use crate::numerics::interval_vec3::{div, dot_tight};

    let axis = InertiaAxis::new(direction)?;
    let tensor = symmetric_tensor_intervals(bound_assembly_inertia_about_point(pieces, point)?)?;
    let mut image = [Interval::point(0.); 3];
    for (value, row) in image.iter_mut().zip(tensor) {
        *value = dot_tight(row, axis.components)?;
    }
    let vector = div(cross(axis.components, image)?, axis.norm_squared)?;
    let magnitude = norm(vector)?;
    Ok(CoupleUnbalanceBounds {
        vector_bounds: vector.map(|value| [value.lo, value.hi]),
        magnitude_bounds: [magnitude.lo, magnitude.hi],
    })
}

/// Enclose equivalent radial unbalance vectors at two ordered axial offsets
/// from the reference point. Corrections would use the opposite vectors.
pub fn bound_assembly_two_plane_unbalance(
    pieces: &[SurfaceAssemblyMassBounds],
    point: [f64; 3],
    direction: [f64; 3],
    plane_offsets: [f64; 2],
) -> Result<[[[f64; 2]; 3]; 2]> {
    use crate::numerics::interval_vec3::{div, scale, sub};

    let [first, second] = plane_offsets;
    check(
        first.is_finite() && second.is_finite() && first < second,
        "Balance planes require finite increasing axial offsets",
    )?;
    let axis = InertiaAxis::new(direction)?;
    let unit_axis = div(axis.components, plane_normal_length(&axis)?)?;
    let static_vector =
        bound_assembly_static_unbalance(pieces, point, direction)?.vector_bounds;
    let couple_vector =
        bound_assembly_couple_unbalance(pieces, point, direction)?.vector_bounds;
    // If U is static unbalance and D its axial first moment, a pair of
    // equivalent vectors satisfies U1 + U2 = U and a*U1 + b*U2 = D.
    let static_vector = centroid_intervals(static_vector)?;
    let axial_moment = cross(unit_axis, centroid_intervals(couple_vector)?)?;
    let separation = Interval::point(second).sub(Interval::point(first))?;
    let at_first = div(
        sub(scale(static_vector, Interval::point(second))?, axial_moment)?,
        separation,
    )?;
    let at_second = div(
        sub(axial_moment, scale(static_vector, Interval::point(first))?)?,
        separation,
    )?;
    Ok([at_first, at_second].map(|vector| vector.map(|value| [value.lo, value.hi])))
}

/// Certified unbalance magnitude and tolerance status in each correction plane.
#[derive(Clone, Copy, Debug)]
pub struct TwoPlaneBalanceReport {
    pub magnitude_bounds: [[f64; 2]; 2],
    pub plane_status: [BalanceTolerance; 2],
    pub overall_status: BalanceTolerance,
}

/// Compare equivalent radial unbalance vectors with separate magnitude
/// tolerances for the two correction planes.
pub fn classify_two_plane_unbalance(
    vectors: [[[f64; 2]; 3]; 2],
    tolerances: [f64; 2],
) -> Result<TwoPlaneBalanceReport> {
    check(
        tolerances.iter().all(|value| value.is_finite() && *value >= 0.),
        "Balance tolerances must be finite and nonnegative",
    )?;
    let mut magnitude_bounds = [[0.; 2]; 2];
    let mut plane_status = [BalanceTolerance::Undetermined; 2];
    for (index, vector) in vectors.into_iter().enumerate() {
        let magnitude = norm(centroid_intervals(vector)?)?;
        magnitude_bounds[index] = [magnitude.lo, magnitude.hi];
        plane_status[index] = classify_balance_tolerance(magnitude_bounds[index], tolerances[index]);
    }
    let overall_status = combine_plane_balance_status(plane_status);
    Ok(TwoPlaneBalanceReport {
        magnitude_bounds,
        plane_status,
        overall_status,
    })
}

/// Counterweight mass and radial placement at a specified mounting radius.
#[derive(Clone, Copy, Debug)]
pub struct PlaneCounterweightBounds {
    pub mass_bounds: [f64; 2],
    /// Offset from the rotation axis, in the same frame as the unbalance.
    /// Absent when the unbalance enclosure includes zero.
    pub radial_offset_bounds: Option<[[f64; 2]; 3]>,
}

/// Enclose one opposing counterweight in each of two correction planes.
pub fn bound_two_plane_counterweights(
    vectors: [[[f64; 2]; 3]; 2],
    radii: [f64; 2],
) -> Result<[PlaneCounterweightBounds; 2]> {
    let mut result = [PlaneCounterweightBounds {
        mass_bounds: [0.; 2],
        radial_offset_bounds: None,
    }; 2];
    for ((vector, radius), output) in vectors.into_iter().zip(radii).zip(&mut result) {
        check(
            radius.is_finite() && radius > 0.,
            "Counterweight radius must be strictly positive and finite",
        )?;
        let vector = centroid_intervals(vector)?;
        let magnitude = norm(vector)?;
        let mass = magnitude.div(Interval::point(radius))?;
        output.mass_bounds = [mass.lo.max(0.), mass.hi];
        if magnitude.lo > 0. {
            let mut offset = [[0.; 2]; 3];
            for (bounds, component) in offset.iter_mut().zip(vector) {
                let opposite = Interval::new(-component.hi, -component.lo)?;
                let coordinate = opposite
                    .div(magnitude)?
                    .intersect(-1., 1.)?
                    .mul(Interval::point(radius))?
                    .intersect(-radius, radius)?;
                *bounds = [coordinate.lo, coordinate.hi];
            }
            output.radial_offset_bounds = Some(offset);
        }
    }
    Ok(result)
}

/// Enclosure of an installed counterweight in a correction plane.
#[derive(Clone, Copy, Debug)]
pub struct InstalledCounterweightBounds {
    pub mass_bounds: [f64; 2],
    /// Position relative to the axis, in the same frame as the unbalance.
    /// The true offset must lie in the correction plane.
    pub radial_offset_bounds: [[f64; 2]; 3],
}

/// Enclose and classify residual unbalance after adding installed weights.
pub fn bound_two_plane_residual_unbalance(
    unbalance: [[[f64; 2]; 3]; 2],
    counterweights: [InstalledCounterweightBounds; 2],
    tolerances: [f64; 2],
) -> Result<([[[f64; 2]; 3]; 2], TwoPlaneBalanceReport)> {
    let mut residual = unbalance;
    for (plane, counterweight) in residual.iter_mut().zip(counterweights) {
        let [lo, hi] = counterweight.mass_bounds;
        let mass = Interval::new(lo, hi)?;
        check(lo >= 0., "Installed counterweight masses must be nonnegative")?;
        let radial = centroid_intervals(counterweight.radial_offset_bounds)?;
        for (bounds, offset) in plane.iter_mut().zip(radial) {
            let [lo, hi] = *bounds;
            let before = Interval::new(lo, hi)?;
            if mass.hi != 0. && offset != Interval::point(0.) {
                let after = before.add(mass.mul(offset)?)?;
                *bounds = [after.lo, after.hi];
            }
        }
    }
    let report = classify_two_plane_unbalance(residual, tolerances)?;
    Ok((residual, report))
}

/// Required mounting radius and mass capacity status for each correction plane.
#[derive(Clone, Copy, Debug)]
pub struct TwoPlaneCounterweightCapacity {
    pub minimum_radius_bounds: [[f64; 2]; 2],
    pub plane_status: [BalanceTolerance; 2],
    pub overall_status: BalanceTolerance,
}

/// Compare the mounting radii with the minimum radii needed to cancel each
/// plane's unbalance without exceeding its positive counterweight mass limit.
pub fn bound_two_plane_counterweight_capacity(
    vectors: [[[f64; 2]; 3]; 2],
    mounting_radii: [f64; 2],
    mass_limits: [f64; 2],
) -> Result<TwoPlaneCounterweightCapacity> {
    check(
        mounting_radii
            .into_iter()
            .chain(mass_limits)
            .all(|value| value.is_finite() && value > 0.),
        "Counterweight capacity requires finite positive radii and mass limits",
    )?;
    let mut minimum_radius_bounds = [[0.; 2]; 2];
    let mut plane_status = [BalanceTolerance::Undetermined; 2];
    for (index, vector) in vectors.into_iter().enumerate() {
        let magnitude = norm(centroid_intervals(vector)?)?;
        let radius = magnitude.div(Interval::point(mass_limits[index]))?;
        let bounds = [radius.lo.max(0.), radius.hi];
        minimum_radius_bounds[index] = bounds;
        plane_status[index] = classify_balance_tolerance(bounds, mounting_radii[index]);
    }
    Ok(TwoPlaneCounterweightCapacity {
        minimum_radius_bounds,
        plane_status,
        overall_status: combine_plane_balance_status(plane_status),
    })
}

fn combine_plane_balance_status(plane_status: [BalanceTolerance; 2]) -> BalanceTolerance {
    if plane_status.contains(&BalanceTolerance::Outside) {
        BalanceTolerance::Outside
    } else if plane_status == [BalanceTolerance::Within; 2] {
        BalanceTolerance::Within
    } else {
        BalanceTolerance::Undetermined
    }
}

fn classify_balance_tolerance([lo, hi]: [f64; 2], tolerance: f64) -> BalanceTolerance {
    if hi <= tolerance {
        BalanceTolerance::Within
    } else if lo > tolerance {
        BalanceTolerance::Outside
    } else {
        BalanceTolerance::Undetermined
    }
}

/// Enclose the perpendicular distance from the common center of mass to an
/// axis, then compare the entire enclosure with the allowed eccentricity.
pub fn bound_assembly_balance_about_line(
    pieces: &[SurfaceAssemblyMassBounds],
    point: [f64; 3],
    direction: [f64; 3],
    tolerance: f64,
) -> Result<([f64; 2], BalanceTolerance)> {
    check(
        !pieces.is_empty() && tolerance.is_finite() && tolerance >= 0.,
        "Balance requires surface pieces and a finite nonnegative tolerance",
    )?;
    let axis = InertiaAxis::new(direction)?;
    let origin = centroid_intervals(point.map(|coordinate| [coordinate; 2]))?;
    let mut projected = Vec::with_capacity(pieces.len());
    for piece in pieces {
        positive_piece_mass(piece)?;
        let center = centroid_intervals(piece.centroid_bounds)?;
        let offset = crate::numerics::interval_vec3::sub(center, origin)?;
        projected.push((cross(offset, axis.components)?, piece.mass_bounds));
    }

    // Project each piece before averaging so common motion along the axis
    // does not inflate the enclosure of the perpendicular displacement.
    let mut endpoints = Vec::with_capacity(projected.len());
    let mut perpendicular = [Interval::point(0.); 3];
    for (component, interval) in perpendicular.iter_mut().enumerate() {
        endpoints.extend(projected.iter().map(|(vector, mass)| (vector[component].lo, *mass)));
        let lo = extremal_weighted_coordinate(&mut endpoints, false)?;
        endpoints.clear();
        endpoints.extend(projected.iter().map(|(vector, mass)| (vector[component].hi, *mass)));
        let hi = extremal_weighted_coordinate(&mut endpoints, true)?;
        endpoints.clear();
        *interval = Interval::new(lo, hi)?;
    }
    let distance = norm(perpendicular)?.div(plane_normal_length(&axis)?)?;
    let lo = distance.lo.max(0.);
    let hi = distance.hi;
    let bounds = [lo, hi];
    Ok((bounds, classify_balance_tolerance(bounds, tolerance)))
}

/// Enclose the signed distance of the common center of mass from an
/// oriented plane, specified by a point and a finite nonzero normal.
pub fn bound_mass_weighted_centroid_plane_distance(
    pieces: &[SurfaceAssemblyMassBounds],
    plane_point: [f64; 3],
    plane_normal: [f64; 3],
) -> Result<[f64; 2]> {
    check(
        !pieces.is_empty(),
        "Surface mass properties require at least one piece",
    )?;
    let axis = InertiaAxis::new(plane_normal)?;
    let origin = centroid_intervals(plane_point.map(|value| [value; 2]))?;
    let mut lower = Vec::with_capacity(pieces.len());
    let mut upper = Vec::with_capacity(pieces.len());
    for piece in pieces {
        positive_piece_mass(piece)?;
        let projection = project_component_centroid(piece, origin, &axis)?;
        lower.push((projection.lo, piece.mass_bounds));
        upper.push((projection.hi, piece.mass_bounds));
    }
    let projection = Interval::new(
        extremal_weighted_coordinate(&mut lower, false)?,
        extremal_weighted_coordinate(&mut upper, true)?,
    )?;
    let distance = projection.div(plane_normal_length(&axis)?)?;
    Ok([distance.lo, distance.hi])
}

/// Enclose the sum of each component's mass times its signed distance
/// from the oriented plane.
pub fn bound_assembly_first_mass_moment_about_plane(
    pieces: &[SurfaceAssemblyMassBounds],
    plane_point: [f64; 3],
    plane_normal: [f64; 3],
) -> Result<[f64; 2]> {
    check(!pieces.is_empty(), "Surface mass properties require at least one piece")?;
    let axis = InertiaAxis::new(plane_normal)?;
    let origin = centroid_intervals(plane_point.map(|value| [value; 2]))?;
    let mut moment = Interval::point(0.);
    for piece in pieces {
        let weight = positive_piece_mass(piece)?;
        let projection = project_component_centroid(piece, origin, &axis)?;
        moment = moment.add(projection.mul(weight)?)?;
    }
    let signed = moment.div(plane_normal_length(&axis)?)?;
    Ok([signed.lo, signed.hi])
}

fn project_component_centroid(
    piece: &SurfaceAssemblyMassBounds,
    origin: [Interval; 3],
    axis: &InertiaAxis,
) -> Result<Interval> {
    let center = centroid_intervals(piece.centroid_bounds)?;
    let mut projection = Interval::point(0.);
    for (i, coefficient) in axis.components.into_iter().enumerate() {
        if coefficient != Interval::point(0.) {
            let offset = center[i].sub(origin[i])?;
            projection = projection.add(offset.mul(coefficient)?)?;
        }
    }
    Ok(projection)
}

fn plane_normal_length(axis: &InertiaAxis) -> Result<Interval> {
    Interval::new(
        axis.norm_squared.lo.sqrt().next_down(),
        axis.norm_squared.hi.sqrt().next_up(),
    )
}

fn positive_piece_mass(piece: &SurfaceAssemblyMassBounds) -> Result<Interval> {
    let weight = Interval::new(piece.mass_bounds[0], piece.mass_bounds[1])?;
    check(
        weight.lo > 0.,
        "Each surface piece must have strictly positive mass or area",
    )?;
    Ok(weight)
}

fn enclose_mass_weighted_centroid(
    pieces: &[SurfaceAssemblyMassBounds],
) -> Result<(Interval, [Interval; 3])> {
    check(!pieces.is_empty(), "Surface mass properties require at least one piece")?;
    let mut mass = Interval::point(0.);
    for piece in pieces {
        mass = add_component_masses(mass, positive_piece_mass(piece)?)?;
        centroid_intervals(piece.centroid_bounds)?;
    }

    let mut centroid = [Interval::point(0.); 3];
    let mut endpoints = Vec::with_capacity(pieces.len());
    for (axis, interval) in centroid.iter_mut().enumerate() {
        let mut bounds = [0.; 2];
        for (side, output) in bounds.iter_mut().enumerate() {
            endpoints.clear();
            endpoints.extend(pieces.iter().map(|piece| {
                (piece.centroid_bounds[axis][side], piece.mass_bounds)
            }));
            *output = extremal_weighted_coordinate(&mut endpoints, side == 1)?;
        }
        *interval = Interval::new(bounds[0], bounds[1])?;
    }
    Ok((mass, centroid))
}

fn extremal_weighted_coordinate(endpoints: &mut [(f64, [f64; 2])], maximize: bool) -> Result<f64> {
    endpoints.sort_by(|a, b| a.0.total_cmp(&b.0));
    let minimum = endpoints[0].0;
    let maximum = endpoints[endpoints.len() - 1].0;
    if minimum == maximum {
        return Ok(minimum);
    }
    let origin = Interval::point(crate::distance_bounds::midpoint(minimum, maximum));
    let mut samples = Vec::with_capacity(endpoints.len());
    for &(position, weights) in endpoints.iter() {
        samples.push((Interval::point(position).sub(origin)?, weights));
    }

    // Differentiating a weighted mean with respect to a weight gives
    // the sign of (position - mean). An extremizer therefore switches
    // between lower and upper weights at a single sorted position.
    let left_weight = usize::from(!maximize);
    let right_weight = usize::from(maximize);
    let mut suffixes = vec![(Interval::point(0.), Interval::point(0.)); samples.len() + 1];
    for (i, &(position, weights)) in samples.iter().enumerate().rev() {
        let (mass, moment) = suffixes[i + 1];
        let weight = Interval::point(weights[right_weight]);
        suffixes[i] = (add_component_masses(mass, weight)?, moment.add(position.mul(weight)?)?);
    }
    let mut prefix_mass = Interval::point(0.);
    let mut prefix_moment = Interval::point(0.);
    let mut extreme = if maximize { minimum } else { maximum };
    for (split, (right_mass, right_moment)) in suffixes.into_iter().enumerate() {
        let mass = add_component_masses(prefix_mass, right_mass)?;
        let moment = prefix_moment.add(right_moment)?;
        let coordinate = origin.add(moment.div(mass)?)?;
        extreme = if maximize {
            extreme.max(coordinate.hi.min(maximum))
        } else {
            extreme.min(coordinate.lo.max(minimum))
        };
        if let Some(&(position, weights)) = samples.get(split) {
            let weight = Interval::point(weights[left_weight]);
            prefix_mass = add_component_masses(prefix_mass, weight)?;
            prefix_moment = prefix_moment.add(position.mul(weight)?)?;
        }
    }
    Ok(extreme)
}

fn symmetric_tensor_intervals(bounds: [[[f64; 2]; 3]; 3]) -> Result<[[Interval; 3]; 3]> {
    let mut matrix = [[Interval::point(0.); 3]; 3];
    for (i, row) in matrix.iter_mut().enumerate() {
        for (j, value) in row.iter_mut().enumerate() {
            check(
                bounds[i][j] == bounds[j][i],
                "Inertia tensor bounds must be symmetric",
            )?;
            let [lo, hi] = bounds[i][j];
            *value = Interval::new(lo, hi)?;
        }
    }
    Ok(matrix)
}

type PrincipalMomentBounds = [[f64; 2]; 3];

fn bound_optional_principal_moments(
    central_inertia_bounds: Option<[[[f64; 2]; 3]; 3]>,
    moment_tolerance: f64,
) -> Result<(Option<PrincipalMomentBounds>, bool)> {
    let principal_bounds = match central_inertia_bounds {
        Some(bounds) => Some(bound_principal_moments_from_tensor(bounds, moment_tolerance)?),
        None => {
            validate_moment_tolerance(moment_tolerance)?;
            None
        }
    };
    Ok((
        principal_bounds,
        principal_moments_within_tolerance(principal_bounds, moment_tolerance),
    ))
}

fn principal_moments_within_tolerance(
    bounds: Option<PrincipalMomentBounds>,
    tolerance: f64,
) -> bool {
    bounds.is_some_and(|bounds| {
        bounds.into_iter().all(|[lo, hi]| (hi - lo).next_up() <= tolerance)
    })
}

/// Enclose ordered principal moments of a physical central inertia tensor.
/// The bounds must describe a nonnegative mass distribution.
pub fn bound_physical_principal_moments(
    tensor: [[[f64; 2]; 3]; 3],
    tolerance: f64,
) -> Result<[[f64; 2]; 3]> {
    let mut moments = bound_principal_moments_from_tensor(tensor, tolerance)?;
    for moment in &mut moments {
        moment[0] = moment[0].max(0.);
    }

    // Inertia is trace(C) I - C for the positive semidefinite second
    // moment matrix C. Thus the largest moment is at most half the trace.
    let trace_half_hi = (0..3).fold(0., |sum: f64, axis| {
        (sum + (tensor[axis][axis][1] * 0.5).next_up()).next_up()
    });
    let [[a_lo, a_hi], [b_lo, b_hi], [c_lo, c_hi]] = moments;
    moments[0][0] = a_lo.max((c_lo - b_hi).next_down());
    moments[1][0] = b_lo.max((c_lo - a_hi).next_down()).max(moments[0][0]);
    moments[2][0] = c_lo.max(moments[1][0]);
    moments[2][1] = c_hi.min(trace_half_hi).min((a_hi + b_hi).next_up());
    moments[1][1] = b_hi.min(moments[2][1]);
    moments[0][1] = a_hi.min(moments[1][1]);
    for [lo, hi] in moments {
        Interval::new(lo, hi)?;
    }
    Ok(moments)
}

/// Bound the two consecutive principal moment gaps in ascending moment order.
/// A strictly positive lower bound certifies separation of the two moments.
pub fn bound_physical_principal_moment_gaps(
    tensor: [[[f64; 2]; 3]; 3],
    tolerance: f64,
) -> Result<[[f64; 2]; 2]> {
    let [[a_lo, a_hi], [b_lo, b_hi], [c_lo, c_hi]] =
        bound_physical_principal_moments(tensor, tolerance)?;
    // For physical inertia, the largest moment cannot exceed the sum
    // of the two smaller ones, so the upper gap is at most the least moment.
    let first = Interval::new(
        (b_lo - a_hi).next_down().max(0.),
        (b_hi - a_lo).next_up(),
    )?;
    let second = Interval::new(
        (c_lo - b_hi).next_down().max(0.),
        (c_hi - b_lo).next_up().min(a_hi),
    )?;
    Ok([[first.lo, first.hi], [second.lo, second.hi]])
}

/// Bound the sine of the acute angle between a candidate direction and
/// the selected principal axis, indexed by ascending moment (0 through 2).
/// Returns None when the selected moment is not isolated or the bound is trivial.
pub fn certify_principal_axis_direction(
    tensor: [[[f64; 2]; 3]; 3],
    index: usize,
    direction: [f64; 3],
    moment_tolerance: f64,
) -> Result<Option<f64>> {
    check(index < 3, "Principal moment index must be in 0..3")?;
    let axis = InertiaAxis::new(direction)?;
    let moments = bound_physical_principal_moments(tensor, moment_tolerance)?;
    certify_principal_axis_residual(tensor, moments, index, &axis)
}

fn certify_principal_axis_residual(
    tensor: [[[f64; 2]; 3]; 3],
    moments: PrincipalMomentBounds,
    index: usize,
    axis: &InertiaAxis,
) -> Result<Option<f64>> {
    let [lo, hi] = moments[index];
    let shift = crate::distance_bounds::midpoint(lo, hi);
    let mut separation = f64::INFINITY;
    for (other, [other_lo, other_hi]) in moments.into_iter().enumerate() {
        if other == index {
            continue;
        }
        let gap = if other < index && other_hi < lo {
            (shift - other_hi).next_down()
        } else if other > index && hi < other_lo {
            (other_lo - shift).next_down()
        } else {
            return Ok(None);
        };
        separation = separation.min(gap);
    }
    if separation <= 0. {
        return Ok(None);
    }

    let mut matrix = symmetric_tensor_intervals(tensor)?;
    let mut residual = [Interval::point(0.); 3];
    for (i, row) in matrix.iter_mut().enumerate() {
        row[i] = row[i].sub(Interval::point(shift))?;
        residual[i] = crate::numerics::interval_vec3::dot_tight(*row, axis.components)?;
    }

    // In an eigenbasis every unwanted direction contributes its component
    // multiplied by an eigenvalue distance of at least separation.
    let length_lower = axis.norm_squared.lo.sqrt().next_down();
    let relative_residual = (norm(residual)?.hi / length_lower).next_up();
    let sin_upper = (relative_residual / separation).next_up();
    Ok((sin_upper < 1.).then_some(sin_upper))
}

#[derive(Clone, Copy, Debug)]
pub struct PrincipalAxisCertificate {
    /// A direction representative, scaled so its largest component has magnitude one.
    pub direction: [f64; 3],
    /// Upper bound on the sine of the acute angle to the true axis.
    pub sin_angle_upper: f64,
}

/// Enclose the entries of the orthogonal projector onto a certified axis.
pub fn bound_principal_axis_projector(
    certificate: PrincipalAxisCertificate,
) -> Result<[[[f64; 2]; 3]; 3]> {
    let error = certificate.sin_angle_upper;
    check(
        (0. ..=1.).contains(&error),
        "Principal axis sine bound must lie in [0, 1]",
    )?;
    let axis = InertiaAxis::new(certificate.direction)?;
    let uncertainty = Interval::new(-error, error)?;
    let mut projector = [[[0.; 2]; 3]; 3];
    // The spectral norm of the difference of two rank-one orthogonal
    // projectors is the sine of their acute angle. It bounds every entry.
    for i in 0..3 {
        for j in i..3 {
            let (product, minimum, maximum) = if i == j {
                (axis.squares[i], 0., 1.)
            } else {
                (axis.components[i].mul(axis.components[j])?, -0.5, 0.5)
            };
            let entry = product
                .div(axis.norm_squared)?
                .add(uncertainty)?
                .intersect(minimum, maximum)?;
            projector[i][j] = [entry.lo, entry.hi];
            projector[j][i] = projector[i][j];
        }
    }
    Ok(projector)
}

/// Enclose the distance from a point to a certified principal axis through
/// a reference point enclosed by the three Cartesian anchor intervals.
pub fn bound_distance_to_principal_axis(
    certificate: PrincipalAxisCertificate,
    anchor_bounds: [[f64; 2]; 3],
    point: [f64; 3],
) -> Result<[f64; 2]> {
    check(
        (0. ..=1.).contains(&certificate.sin_angle_upper),
        "Principal axis sine bound must lie in [0, 1]",
    )?;
    let axis = InertiaAxis::new(certificate.direction)?;
    let anchor = centroid_intervals(anchor_bounds)?;
    let point = centroid_intervals(point.map(|coordinate| [coordinate; 2]))?;
    let offset = crate::numerics::interval_vec3::sub(point, anchor)?;
    let magnitude = norm(offset)?;
    let approximate = norm(cross(offset, axis.components)?)?.div(plane_normal_length(&axis)?)?;

    // The difference of the two perpendicular projectors has operator
    // norm sin(angle). The reverse triangle inequality bounds the change
    // in the length of the projected offset.
    let error = magnitude.mul(Interval::point(certificate.sin_angle_upper))?.hi;
    let distance = approximate
        .add(Interval::new(-error, error)?)?
        .intersect(0., magnitude.hi)?;
    Ok([distance.lo, distance.hi])
}

/// Enclose the sine of the acute angle between a fixed direction and a
/// certified principal axis. Either direction may be reversed or rescaled.
pub fn bound_principal_axis_angle_sine(
    certificate: PrincipalAxisCertificate,
    direction: [f64; 3],
) -> Result<[f64; 2]> {
    let error = certificate.sin_angle_upper;
    check(
        (0. ..=1.).contains(&error),
        "Principal axis sine bound must lie in [0, 1]",
    )?;
    let axis = InertiaAxis::new(certificate.direction)?;
    let probe = InertiaAxis::new(direction)?;
    let length = plane_normal_length(&axis)?.mul(plane_normal_length(&probe)?)?;
    let sine = norm(cross(axis.components, probe.components)?)?.div(length)?;
    // For a unit probe the sine is the norm of its perpendicular projection.
    // Changing the axis changes the projector by at most the certificate error.
    let bounds = sine.add(Interval::new(-error, error)?)?.intersect(0., 1.)?;
    Ok([bounds.lo, bounds.hi])
}

/// Find and certify a direction for a selected principal moment. Candidate
/// construction uses midpoint data, and certification uses the full intervals.
pub fn find_certified_principal_axis(
    tensor: [[[f64; 2]; 3]; 3],
    index: usize,
    moment_tolerance: f64,
) -> Result<Option<PrincipalAxisCertificate>> {
    check(index < 3, "Principal moment index must be in 0..3")?;
    let moments = bound_physical_principal_moments(tensor, moment_tolerance)?;
    find_principal_axis_from_bounds(tensor, moments, index)
}

fn find_principal_axis_from_bounds(
    tensor: [[[f64; 2]; 3]; 3],
    moments: PrincipalMomentBounds,
    index: usize,
) -> Result<Option<PrincipalAxisCertificate>> {
    use crate::distance_bounds::midpoint;
    let shift = midpoint(moments[index][0], moments[index][1]);
    let mut matrix = tensor.map(|row| row.map(|[lo, hi]| midpoint(lo, hi)));
    let scale = matrix
        .iter()
        .flatten()
        .fold(shift.abs(), |maximum, value| maximum.max(value.abs()));
    if scale == 0. {
        return Ok(None);
    }
    for (i, row) in matrix.iter_mut().enumerate() {
        for value in row.iter_mut() {
            *value /= scale;
        }
        row[i] -= shift / scale;
    }

    let mut best: Option<PrincipalAxisCertificate> = None;
    // A null direction of the shifted midpoint tensor lies perpendicular
    // to its rows. Try every independent row pair and verify the result.
    for (a, b) in [(0, 1), (0, 2), (1, 2)] {
        let direction = [0, 1, 2].map(|i| {
            let j = (i + 1) % 3;
            let k = (i + 2) % 3;
            matrix[a][j] * matrix[b][k] - matrix[a][k] * matrix[b][j]
        });
        let magnitude = direction.into_iter().map(f64::abs).fold(0., f64::max);
        if magnitude == 0. {
            continue;
        }
        let direction = direction.map(|value| value / magnitude);
        let axis = InertiaAxis::new(direction)?;
        if let Some(sin_angle_upper) = certify_principal_axis_residual(tensor, moments, index, &axis)?
            && best.is_none_or(|current| sin_angle_upper < current.sin_angle_upper)
        {
            best = Some(PrincipalAxisCertificate { direction, sin_angle_upper });
        }
    }
    Ok(best)
}

fn validate_moment_tolerance(tolerance: f64) -> Result<()> {
    check(
        tolerance.is_finite() && tolerance > 0.,
        "Principal moment tolerance must be positive and finite",
    )
}

/// Enclose the ascending eigenvalues of every symmetric tensor inside the
/// supplied entry intervals. The tolerance controls spectral refinement.
pub fn bound_principal_moments_from_tensor(
    bounds: [[[f64; 2]; 3]; 3],
    tolerance: f64,
) -> Result<[[f64; 2]; 3]> {
    validate_moment_tolerance(tolerance)?;
    let matrix = symmetric_tensor_intervals(bounds)?;
    let mut low = f64::INFINITY;
    let mut high = f64::NEG_INFINITY;
    let mut off_diagonal_norm = 0.0_f64;
    for (i, row) in matrix.iter().enumerate() {
        let mut radius = 0.0_f64;
        for (j, value) in row.iter().enumerate() {
            if i != j {
                radius = (radius + value.lo.abs().max(value.hi.abs())).next_up();
            }
        }
        off_diagonal_norm = off_diagonal_norm.max(radius);
        low = low.min((row[i].lo - radius).next_down());
        high = high.max((row[i].hi + radius).next_up());
    }
    Interval::new(low, high)?;
    // Weyl bounds the ordered eigenvalue shift by the symmetric
    // off-diagonal matrix's maximum absolute row sum.
    let mut diagonal_lower = [0, 1, 2].map(|i| matrix[i][i].lo);
    let mut diagonal_upper = [0, 1, 2].map(|i| matrix[i][i].hi);
    diagonal_lower.sort_by(f64::total_cmp);
    diagonal_upper.sort_by(f64::total_cmp);
    let mut moments = [[low; 2]; 3];
    for (index, moment) in moments.iter_mut().enumerate() {
        let mut bounds = [low, high];
        for _ in 0..96 {
            let [lo, hi] = bounds;
            let width = hi - lo;
            if width.next_up() <= tolerance {
                break;
            }
            let mid = crate::distance_bounds::midpoint(lo, hi);
            for candidate in [
                crate::distance_bounds::midpoint(lo, mid),
                crate::distance_bounds::midpoint(mid, hi),
            ] {
                if !(bounds[0] < candidate && candidate < bounds[1]) {
                    continue;
                }
                if let Some(negative) = shifted_inertia(matrix, candidate) {
                    if negative <= index {
                        bounds[0] = candidate;
                    } else {
                        bounds[1] = candidate;
                    }
                }
            }
            if bounds == [lo, hi] {
                break;
            }
        }
        let ordered_lo = (diagonal_lower[index] - off_diagonal_norm).next_down();
        let ordered_hi = (diagonal_upper[index] + off_diagonal_norm).next_up();
        *moment = [bounds[0].max(ordered_lo), bounds[1].min(ordered_hi)];
    }
    Ok(moments)
}

// Interval LDLᵀ congruence proves an eigenvalue count when every pivot
// is separated from zero. Failure to prove a pivot does not move bounds.
fn shifted_inertia(mut matrix: [[Interval; 3]; 3], shift: f64) -> Option<usize> {
    for (i, row) in matrix.iter_mut().enumerate() {
        row[i] = row[i].sub(Interval::point(shift)).ok()?;
    }
    let mut negative = 0;
    for pivot in 0..3 {
        let index = (pivot..3).find(|&i| matrix[i][i].lo > 0. || matrix[i][i].hi < 0.)?;
        matrix.swap(pivot, index);
        for row in &mut matrix {
            row.swap(pivot, index);
        }
        let d = matrix[pivot][pivot];
        negative += usize::from(d.hi < 0.);
        for i in pivot + 1..3 {
            let factor = matrix[i][pivot].div_signed(d).ok()?;
            for j in i..3 {
                let correction = factor.mul(matrix[j][pivot]).ok()?;
                let entry = matrix[i][j].sub(correction).ok()?;
                matrix[i][j] = entry;
                matrix[j][i] = entry;
            }
        }
    }
    Some(negative)
}

#[derive(Clone, Debug)]
pub struct TrimmedPatchSetAreaReport {
    pub total: TrimmedAreaReport,
    /// Area of each patch in input order. Empty if full coverage is unavailable.
    pub components: Vec<AreaReport>,
}

/// Enclose the sum of the areas of a collection of periodic trim patches.
/// Each patch contributes its own retained area, so overlapping world images
/// count with multiplicity. Both cell budgets are shared across all patches.
pub fn periodic_trim_patch_set_area(
    patches: &[crate::trim::periodic_patch::PeriodicTrimPatch],
    tolerance: f64,
    tolerance_uv: f64,
    max_surface_cells: usize,
    max_trim_cells: usize,
) -> Result<TrimmedPatchSetAreaReport> {
    measure_periodic_patch_set(
        patches,
        tolerance,
        tolerance_uv,
        max_surface_cells,
        max_trim_cells,
        |_, _, _| Ok(()),
    )
}

#[derive(Clone, Debug)]
pub struct TrimmedPatchSetCentroidReport {
    pub measure: TrimmedPatchSetAreaReport,
    pub centroid_bounds: Option<[[f64; 2]; 3]>,
}

/// Enclose the common area centroid of all patches under shared work limits.
/// The combined area must be separated from zero.
pub fn periodic_trim_patch_set_centroid(
    patches: &[crate::trim::periodic_patch::PeriodicTrimPatch],
    tolerance: f64,
    tolerance_uv: f64,
    max_surface_cells: usize,
    max_trim_cells: usize,
) -> Result<TrimmedPatchSetCentroidReport> {
    measure_periodic_patch_set_centroid(
        patches,
        tolerance,
        tolerance_uv,
        max_surface_cells,
        max_trim_cells,
        |_| {},
    )
}

#[derive(Clone, Debug)]
pub struct TrimmedPatchSetCentralInertiaReport {
    pub centroid: TrimmedPatchSetCentroidReport,
    /// Inertia of all components about their common area centroid at unit density.
    pub central_inertia_bounds: Option<[[[f64; 2]; 3]; 3]>,
}

#[derive(Clone, Debug)]
pub struct TrimmedPatchSetPrincipalInertiaReport {
    pub central: TrimmedPatchSetCentralInertiaReport,
    /// Enclosures in increasing order for the inertia about the common centroid.
    pub principal_bounds: Option<[[f64; 2]; 3]>,
    pub within_moment_tolerance: bool,
}

impl TrimmedPatchSetCentralInertiaReport {
    /// Extract complete bounds for assembly with other surface pieces.
    pub fn mass_properties_bounds(&self) -> Option<SurfaceMassPropertiesBounds> {
        let area_bounds = self.centroid.measure.total.area.as_ref()?.bounds;
        (area_bounds[0] > 0.).then_some(SurfaceMassPropertiesBounds {
            area_bounds,
            centroid_bounds: self.centroid.centroid_bounds?,
            central_inertia_bounds: self.central_inertia_bounds?,
        })
    }

    /// Enclose the moment about an axis through the common area centroid.
    pub fn bound_axis_moment(&self, direction: [f64; 3]) -> Result<Option<[f64; 2]>> {
        self.central_inertia_bounds
            .map(|bounds| bound_directional_inertia(bounds, direction))
            .transpose()
    }

    /// Translate the common central tensor to a point in model coordinates.
    pub fn bound_inertia_about_point(&self, point: [f64; 3]) -> Result<Option<[[[f64; 2]; 3]; 3]>> {
        let (Some(central), Some(centroid), Some(area)) = (
            self.central_inertia_bounds,
            self.centroid.centroid_bounds,
            self.centroid.measure.total.area.as_ref(),
        ) else {
            return Ok(None);
        };
        bound_inertia_about_point(central, area.bounds, centroid, point).map(Some)
    }

    /// Enclose the inertia of all components around a fixed line.
    pub fn bound_inertia_about_line(
        &self,
        point: [f64; 3],
        direction: [f64; 3],
    ) -> Result<Option<[f64; 2]>> {
        let (Some(central), Some(centroid), Some(area)) = (
            self.central_inertia_bounds,
            self.centroid.centroid_bounds,
            self.centroid.measure.total.area.as_ref(),
        ) else {
            return Ok(None);
        };
        bound_inertia_about_line(central, area.bounds, centroid, point, direction).map(Some)
    }

    /// Enclose the radius of gyration around a fixed line for unit areal density.
    pub fn bound_line_radius_of_gyration(
        &self,
        point: [f64; 3],
        direction: [f64; 3],
    ) -> Result<Option<[f64; 2]>> {
        let Some(area) = self.centroid.measure.total.area.as_ref() else {
            return Ok(None);
        };
        if area.bounds[0] <= 0. {
            return Ok(None);
        }
        self.bound_inertia_about_line(point, direction)?
            .map(|moment| bound_radius_of_gyration(moment, area.bounds))
            .transpose()
    }

    /// Enclose the sorted principal moments of the complete patch set.
    /// Unavailable central inertia remains unavailable in the spectral report.
    pub fn bound_principal_moments(
        self,
        moment_tolerance: f64,
    ) -> Result<TrimmedPatchSetPrincipalInertiaReport> {
        let (principal_bounds, within_moment_tolerance) =
            bound_optional_principal_moments(self.central_inertia_bounds, moment_tolerance)?;
        Ok(TrimmedPatchSetPrincipalInertiaReport {
            central: self,
            principal_bounds,
            within_moment_tolerance,
        })
    }
}

/// Enclose the central inertia of the complete patch set on a shared
/// integration partition. Component offsets from the common centroid are included.
pub fn periodic_trim_patch_set_central_inertia(
    patches: &[crate::trim::periodic_patch::PeriodicTrimPatch],
    tolerance: f64,
    tolerance_uv: f64,
    max_surface_cells: usize,
    max_trim_cells: usize,
) -> Result<TrimmedPatchSetCentralInertiaReport> {
    let mut partitions = Vec::new();
    let centroid = measure_periodic_patch_set_centroid(
        patches,
        tolerance,
        tolerance_uv,
        max_surface_cells,
        max_trim_cells,
        |partition| partitions.push(partition),
    )?;
    let central_inertia_bounds = centroid
        .centroid_bounds
        .map(|bounds| {
            let cells = patches
                .iter()
                .zip(&partitions)
                .flat_map(|(patch, cells)| cells.iter().map(|cell| (&patch.surface, cell)));
            enclose_trimmed_inertia(cells, centroid_intervals(bounds)?)
        })
        .transpose()?;
    Ok(TrimmedPatchSetCentralInertiaReport {
        centroid,
        central_inertia_bounds,
    })
}

#[derive(Clone, Debug)]
pub struct TrimmedPatchSetDensityReport {
    pub measure: TrimmedPatchSetAreaReport,
    /// Component masses in patch order, empty when full area coverage is unavailable.
    pub component_mass_bounds: Vec<[f64; 2]>,
    /// Total mass can be enclosed even when no center of mass is certified.
    pub total_mass_bounds: Option<[f64; 2]>,
    pub mass_properties: Option<SurfaceAssemblyMassBounds>,
    weighted_components: Vec<SurfaceAssemblyMassBounds>,
}

impl TrimmedPatchSetDensityReport {
    /// Enclose the signed distance of the density-weighted center to a plane.
    pub fn center_of_mass_plane_distance(
        &self,
        point: [f64; 3],
        normal: [f64; 3],
    ) -> Result<Option<[f64; 2]>> {
        self.mass_properties
            .map(|_| {
                bound_mass_weighted_centroid_plane_distance(&self.weighted_components, point, normal)
            })
            .transpose()
    }

    /// Enclose the density-weighted tensor about a point in model coordinates.
    pub fn bound_inertia_about_point(&self, point: [f64; 3]) -> Result<Option<[[[f64; 2]; 3]; 3]>> {
        self.mass_properties
            .map(|_| bound_assembly_inertia_about_point(&self.weighted_components, point))
            .transpose()
    }

    /// Enclose inertia about a fixed line using the weighted component tensors.
    pub fn bound_inertia_about_line(
        &self,
        point: [f64; 3],
        direction: [f64; 3],
    ) -> Result<Option<[f64; 2]>> {
        self.mass_properties
            .map(|_| bound_assembly_inertia_about_line(&self.weighted_components, point, direction))
            .transpose()
    }

    /// Return the ordered principal moments and whether all bounds meet tolerance.
    pub fn bound_principal_moments(
        &self,
        tolerance: f64,
    ) -> Result<(Option<PrincipalMomentBounds>, bool)> {
        validate_moment_tolerance(tolerance)?;
        let moments = self
            .mass_properties
            .map(|mass| bound_physical_principal_moments(mass.central_inertia_bounds, tolerance))
            .transpose()?;
        Ok((moments, principal_moments_within_tolerance(moments, tolerance)))
    }

    /// Enclose adjacent spectral gaps of the density-weighted central tensor.
    pub fn bound_principal_moment_gaps(&self, tolerance: f64) -> Result<Option<[[f64; 2]; 2]>> {
        validate_moment_tolerance(tolerance)?;
        self.mass_properties
            .map(|mass| bound_physical_principal_moment_gaps(mass.central_inertia_bounds, tolerance))
            .transpose()
    }

    /// Find every certifiable principal axis, in ascending moment order.
    pub fn bound_principal_axes(
        &self,
        tolerance: f64,
    ) -> Result<Option<[Option<PrincipalAxisCertificate>; 3]>> {
        validate_moment_tolerance(tolerance)?;
        let Some(mass) = self.mass_properties else {
            return Ok(None);
        };
        let tensor = mass.central_inertia_bounds;
        let moments = bound_physical_principal_moments(tensor, tolerance)?;
        let mut axes = [None; 3];
        for (index, axis) in axes.iter_mut().enumerate() {
            *axis = find_principal_axis_from_bounds(tensor, moments, index)?;
        }
        Ok(Some(axes))
    }

    /// Enclose the density-weighted static unbalance about a rotation axis.
    pub fn bound_static_unbalance(
        &self,
        point: [f64; 3],
        direction: [f64; 3],
    ) -> Result<Option<StaticUnbalanceBounds>> {
        self.mass_properties
            .map(|_| bound_assembly_static_unbalance(&self.weighted_components, point, direction))
            .transpose()
    }

    /// Enclose couple unbalance about the supplied axial reference point.
    pub fn bound_couple_unbalance(
        &self,
        point: [f64; 3],
        direction: [f64; 3],
    ) -> Result<Option<CoupleUnbalanceBounds>> {
        self.mass_properties
            .map(|_| bound_assembly_couple_unbalance(&self.weighted_components, point, direction))
            .transpose()
    }

    /// Enclose equivalent unbalance vectors in the two requested planes.
    pub fn bound_two_plane_unbalance(
        &self,
        point: [f64; 3],
        direction: [f64; 3],
        offsets: [f64; 2],
    ) -> Result<Option<[[[f64; 2]; 3]; 2]>> {
        self.mass_properties
            .map(|_| {
                bound_assembly_two_plane_unbalance(
                    &self.weighted_components, point, direction, offsets,
                )
            })
            .transpose()
    }

    /// Check a fixed rotation axis against the density-weighted center of mass.
    pub fn bound_balance_about_line(
        &self,
        point: [f64; 3],
        direction: [f64; 3],
        tolerance: f64,
    ) -> Result<Option<([f64; 2], BalanceTolerance)>> {
        self.mass_properties
            .map(|_| {
                bound_assembly_balance_about_line(
                    &self.weighted_components,
                    point,
                    direction,
                    tolerance,
                )
            })
            .transpose()
    }

    /// Enclose the density-weighted first moment about an oriented plane.
    pub fn first_mass_moment_about_plane(
        &self,
        point: [f64; 3],
        normal: [f64; 3],
    ) -> Result<Option<[f64; 2]>> {
        self.mass_properties
            .map(|_| {
                bound_assembly_first_mass_moment_about_plane(
                    &self.weighted_components,
                    point,
                    normal,
                )
            })
            .transpose()
    }

    /// Enclose the fractional mass of each patch in input order.
    pub fn component_mass_fractions(&self) -> Result<Option<Vec<[f64; 2]>>> {
        bound_component_mass_fractions(&self.component_mass_bounds)
    }
}

/// Enclose each component's mass divided by the sum of all component
/// masses. Return no fractions when the total can be zero.
pub fn bound_component_mass_fractions(
    mass_bounds: &[[f64; 2]],
) -> Result<Option<Vec<[f64; 2]>>> {
    let mut prefix = Vec::with_capacity(mass_bounds.len());
    let mut total = Interval::point(0.);
    for &[lo, hi] in mass_bounds {
        let mass = Interval::new(lo, hi)?;
        check(mass.lo >= 0., "Component mass bounds must be nonnegative")?;
        prefix.push(total);
        total = add_component_masses(total, mass)?;
    }
    if total.lo == 0. {
        return Ok(None);
    }

    let mut fractions = Vec::with_capacity(mass_bounds.len());
    let mut suffix = Interval::point(0.);
    for (&[lo, hi], before) in mass_bounds.iter().zip(prefix).rev() {
        let other = add_component_masses(before, suffix)?;
        // The fraction increases with this mass and decreases with the
        // sum of the other masses.
        let lower = mass_fraction_endpoint(lo, other.hi)?;
        let upper = mass_fraction_endpoint(hi, other.lo)?;
        fractions.push([lower.lo, upper.hi]);
        suffix = add_component_masses(suffix, Interval::new(lo, hi)?)?;
    }
    fractions.reverse();
    Ok(Some(fractions))
}

fn add_component_masses(a: Interval, b: Interval) -> Result<Interval> {
    if a.hi == 0. {
        return Ok(b);
    }
    if b.hi == 0. {
        return Ok(a);
    }
    let sum = a.add(b)?;
    sum.intersect(a.lo.max(b.lo), sum.hi)
}

fn mass_fraction_endpoint(mass: f64, other: f64) -> Result<Interval> {
    if mass == 0. {
        return Ok(Interval::point(0.));
    }
    if other == 0. {
        return Ok(Interval::point(1.));
    }
    let scale = mass.max(other);
    let scaled = |value: f64| {
        if value == scale {
            return Ok(Interval::point(1.));
        }
        let quotient = value / scale;
        Interval::new(quotient.next_down().max(0.), quotient.next_up().min(1.))
    };
    let mass = scaled(mass)?;
    let other = scaled(other)?;
    mass.div(mass.add(other)?)?.intersect(0., 1.)
}

/// Measure the mass of periodic trim patches with one areal density interval
/// per patch in input order. The tolerance applies to geometric area, and the
/// two integration limits are shared across the entire set.
pub fn periodic_trim_patch_set_mass_with_density(
    patches: &[crate::trim::periodic_patch::PeriodicTrimPatch],
    densities: &[[f64; 2]],
    tolerance: f64,
    tolerance_uv: f64,
    max_surface_cells: usize,
    max_trim_cells: usize,
) -> Result<TrimmedPatchSetDensityReport> {
    check(
        densities.len() == patches.len()
            && densities
                .iter()
                .all(|&[lo, hi]| lo > 0. && lo <= hi && hi.is_finite()),
        "Each periodic trim patch requires one strictly positive finite density interval",
    )?;
    let mut pieces = Vec::new();
    let mut component_densities = densities.iter().copied();
    let measure = measure_periodic_patch_set(
        patches,
        tolerance,
        tolerance_uv,
        max_surface_cells,
        max_trim_cells,
        |surface, cells, area| {
            let density = component_densities
                .next()
                .ok_or_else(|| crate::numeric_err("Missing trim patch density"))?;
            if area.bounds[0] > 0. {
                let centroid_bounds = enclose_trimmed_centroid(surface, &cells, area)?;
                let central_inertia_bounds = enclose_trimmed_inertia(
                    cells.iter().map(|cell| (surface, cell)),
                    centroid_intervals(centroid_bounds)?,
                )?;
                pieces.push(apply_surface_density(
                    &SurfaceMassPropertiesBounds {
                        area_bounds: area.bounds,
                        centroid_bounds,
                        central_inertia_bounds,
                    },
                    density,
                )?);
            }
            Ok(())
        },
    )?;
    let mut total_mass = Interval::point(0.);
    let component_mass_bounds = measure
        .components
        .iter()
        .zip(densities)
        .map(|(area, density)| {
            let area = Interval::new(area.bounds[0], area.bounds[1])?;
            let mass = area.mul(Interval::new(density[0], density[1])?)?;
            let mass = mass.intersect(0., mass.hi)?;
            total_mass = total_mass.add(mass)?;
            Ok([mass.lo, mass.hi])
        })
        .collect::<Result<Vec<_>>>()?;
    let total_mass_bounds = measure
        .total
        .area
        .as_ref()
        .map(|_| [total_mass.lo.max(0.), total_mass.hi]);
    let mass_properties = (pieces.len() == patches.len())
        .then(|| combine_mass_properties(pieces.iter().copied().map(Ok)))
        .transpose()?;
    Ok(TrimmedPatchSetDensityReport {
        measure,
        component_mass_bounds,
        total_mass_bounds,
        mass_properties,
        weighted_components: pieces,
    })
}

fn measure_periodic_patch_set_centroid(
    patches: &[crate::trim::periodic_patch::PeriodicTrimPatch],
    tolerance: f64,
    tolerance_uv: f64,
    max_surface_cells: usize,
    max_trim_cells: usize,
    mut save_partition: impl FnMut(BinaryHeap<Cell>),
) -> Result<TrimmedPatchSetCentroidReport> {
    let mut moments = None;
    let measure = measure_periodic_patch_set(
        patches,
        tolerance,
        tolerance_uv,
        max_surface_cells,
        max_trim_cells,
        |surface, cells, _| {
            let moments = moments.get_or_insert_with(|| {
                let origin =
                    [0, 1, 2].map(|axis| Interval::point(surface.control_points[0][0][axis]));
                TrimmedFirstMoments::new(origin)
            });
            moments.accumulate(surface, &cells)?;
            save_partition(cells);
            Ok(())
        },
    )?;
    let centroid_bounds = match (measure.total.area.as_ref(), moments) {
        (Some(area), Some(moments)) if area.bounds[0] > 0. => Some(moments.centroid(area)?),
        _ => None,
    };
    Ok(TrimmedPatchSetCentroidReport {
        measure,
        centroid_bounds,
    })
}

fn measure_periodic_patch_set(
    patches: &[crate::trim::periodic_patch::PeriodicTrimPatch],
    tolerance: f64,
    tolerance_uv: f64,
    max_surface_cells: usize,
    max_trim_cells: usize,
    mut consume_partition: impl FnMut(&Surface, BinaryHeap<Cell>, &AreaReport) -> Result<()>,
) -> Result<TrimmedPatchSetAreaReport> {
    check(
        (1..=256).contains(&patches.len())
            && [tolerance, tolerance_uv]
                .iter()
                .all(|t| t.is_finite() && *t > 0.)
            && (1..=100_000).contains(&max_surface_cells)
            && (1..=1_000_000).contains(&max_trim_cells),
        "Trimmed patch set needs 1..256 patches, positive tolerances and valid cell limits",
    )?;
    let mut minima = Vec::with_capacity(patches.len());
    for patch in patches {
        let surface = &patch.surface;
        surface.validate()?;
        let count = |knots: &[f64], degree: usize, controls: usize| {
            knots[degree..=controls]
                .windows(2)
                .filter(|pair| pair[0] < pair[1])
                .count()
        };
        minima.push(
            count(
                &surface.knots_u,
                surface.degree_u,
                surface.control_points.len(),
            ) * count(
                &surface.knots_v,
                surface.degree_v,
                surface.control_points[0].len(),
            ),
        );
    }
    let minimum_surface_cells = minima.iter().sum();
    if minimum_surface_cells > max_surface_cells || max_trim_cells < patches.len() {
        return Ok(TrimmedPatchSetAreaReport {
            total: TrimmedAreaReport {
                area: None,
                minimum_surface_cells,
                trim_cells: 0,
            },
            components: Vec::new(),
        });
    }

    let mut remaining_minimum = minimum_surface_cells;
    let mut surface_cells = 0;
    let mut trim_cells = 0;
    let mut total = Interval::point(0.);
    let mut components = Vec::with_capacity(patches.len());
    let share = (tolerance / patches.len() as f64).max(f64::from_bits(1));
    let mut stop = StopReason::PrecisionLimit;
    for (index, (patch, minimum)) in patches.iter().zip(minima).enumerate() {
        remaining_minimum -= minimum;
        let trim = TrimDomain::new(&patch.trim.loops, tolerance_uv)?;
        let (measure, partition) = trimmed_surface_area_in_domain(
            &patch.surface,
            &trim,
            None,
            share,
            minimum
                + (max_surface_cells - surface_cells - remaining_minimum - minimum)
                    / (patches.len() - index),
            (max_trim_cells - trim_cells) / (patches.len() - index),
        )?;
        let area = measure
            .area
            .ok_or_else(|| crate::numeric_err("Missing prepared trim patch area"))?;
        consume_partition(&patch.surface, partition, &area)?;
        surface_cells += area.cells;
        trim_cells += measure.trim_cells;
        if area.stop_reason == StopReason::WorkLimit {
            stop = StopReason::WorkLimit;
        }
        total = total.add(Interval::new(area.bounds[0], area.bounds[1])?)?;
        components.push(area);
    }
    let area = report(
        Interval::new(total.lo.max(0.), total.hi)?,
        surface_cells,
        tolerance,
        stop,
    )?;
    Ok(TrimmedPatchSetAreaReport {
        total: TrimmedAreaReport {
            area: Some(area),
            minimum_surface_cells,
            trim_cells,
        },
        components,
    })
}
