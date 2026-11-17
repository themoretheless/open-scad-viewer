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
    patch: &crate::editing::exact_periodic_surface::PeriodicTrimPatch,
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
    let denominator = Interval::new(area.bounds[0], area.bounds[1])?;
    let origin = [0, 1, 2].map(|axis| Interval::point(surface.control_points[0][0][axis]));
    let mut moment = [Interval::point(0.); 3];
    let mut support = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for cell in cells {
        if cell.area.hi == 0. {
            continue;
        }
        let position = cell_position_relative_to(surface, cell, origin)?;
        for (axis, relative) in position.into_iter().enumerate() {
            support[axis][0] = support[axis][0].min(relative.lo);
            support[axis][1] = support[axis][1].max(relative.hi);
            moment[axis] = moment[axis].add(relative.mul(cell.area)?)?;
        }
    }
    let mut centroid_bounds = [[0.; 2]; 3];
    for (axis, output) in centroid_bounds.iter_mut().enumerate() {
        let bound = moment[axis]
            .div(denominator)?
            .intersect(support[axis][0], support[axis][1])?
            .add(origin[axis])?;
        *output = [bound.lo, bound.hi];
    }
    Ok(centroid_bounds)
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
    let inertia_bounds = enclose_trimmed_inertia(surface, &cells, origin.map(Interval::point))?;
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
    let [[x0, x1], [y0, y1], [z0, z1]] = centroid_bounds;
    let center = [
        Interval::new(x0, x1)?,
        Interval::new(y0, y1)?,
        Interval::new(z0, z1)?,
    ];
    let central_inertia_bounds = enclose_trimmed_inertia(surface, &cells, center)?;
    Ok(TrimmedCentralInertiaReport {
        centroid: TrimmedCentroidReport {
            measure,
            centroid_bounds: Some(centroid_bounds),
        },
        central_inertia_bounds: Some(central_inertia_bounds),
    })
}

fn enclose_trimmed_inertia(
    surface: &Surface,
    cells: &BinaryHeap<Cell>,
    origin: [Interval; 3],
) -> Result<[[[f64; 2]; 3]; 3]> {
    let mut tensor = [[Interval::point(0.); 3]; 3];
    for cell in cells {
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
        check(
            moment_tolerance.is_finite() && moment_tolerance > 0.,
            "Principal moment tolerance must be positive and finite",
        )?;
        let principal_bounds = self
            .central_inertia_bounds
            .map(|bounds| principal_moment_intervals(bounds, moment_tolerance))
            .transpose()?;
        let within_moment_tolerance = principal_bounds.is_some_and(|bounds| {
            bounds
                .into_iter()
                .all(|[lo, hi]| (hi - lo).next_up() <= moment_tolerance)
        });
        Ok(TrimmedPrincipalInertiaReport {
            central: self,
            principal_bounds,
            within_moment_tolerance,
        })
    }
}

fn principal_moment_intervals(bounds: [[[f64; 2]; 3]; 3], tolerance: f64) -> Result<[[f64; 2]; 3]> {
    let mut matrix = [[Interval::point(0.); 3]; 3];
    let mut low = f64::INFINITY;
    let mut high = f64::NEG_INFINITY;
    for (i, row) in matrix.iter_mut().enumerate() {
        let mut radius = 0.0_f64;
        for (j, value) in row.iter_mut().enumerate() {
            let [lo, hi] = bounds[i][j];
            check(
                bounds[i][j] == bounds[j][i],
                "Principal moment tensor bounds must be symmetric",
            )?;
            *value = Interval::new(lo, hi)?;
            if i != j {
                radius = (radius + lo.abs().max(hi.abs())).next_up();
            }
        }
        low = low.min((row[i].lo - radius).next_down());
        high = high.max((row[i].hi + radius).next_up());
    }
    Interval::new(low, high)?;
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
        *moment = bounds;
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
