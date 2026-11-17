//! Outward moment enclosures for closed rational planar boundaries.
use super::planar_area::cross;
use crate::{Result, check, curve::Curve, curve_jets, distance_bounds::Interval, trim_domain};
use std::{cmp::Ordering, collections::BinaryHeap};

#[derive(Clone, Debug)]
pub struct PlanarMassBounds {
    /// Signed area, x, y, x², xy, y² integrals. Coordinates are relative to origin.
    pub moments: [[f64; 2]; 6],
    pub converged: bool,
    /// Total cell evaluations, including discarded parents.
    pub cells: usize,
}

struct Cell {
    curve: usize,
    span: usize,
    domain: [f64; 2],
    values: [Interval; 6],
    priority: f64,
}

impl Ord for Cell {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority.total_cmp(&other.priority)
    }
}
impl PartialOrd for Cell {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialEq for Cell {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for Cell {}

const MONOMIALS: [(Option<usize>, Option<usize>); 6] = [
    (None, None),
    (Some(0), None),
    (Some(1), None),
    (Some(0), Some(0)),
    (Some(0), Some(1)),
    (Some(1), Some(1)),
];

/// Enclose signed area, first moments and second moments for rational
/// XY contours relative to a chosen origin. Tolerances are interval
/// widths for area, first moments and second moments respectively.
/// Exact closure is required. Winding supplies the material signs.
pub fn bound_planar_boundary_moments(
    loops: &[Vec<Curve>],
    origin: [f64; 2],
    tolerances: [f64; 3],
    max_cells: usize,
) -> Result<PlanarMassBounds> {
    check(
        (1..=256).contains(&loops.len())
            && (1..=100_000).contains(&max_cells)
            && origin.iter().all(|v| v.is_finite())
            && tolerances.iter().all(|v| v.is_finite() && *v > 0.),
        "Planar moments need bounded contours, finite origin and positive tolerances",
    )?;
    let mut curves = Vec::new();
    let mut spans = 0;
    for contour in loops {
        check(
            trim_domain::exact_loop_joins(contour)? == Some(true),
            "Planar moments need exact closed joins",
        )?;
        for curve in contour {
            spans += (curve.degree..curve.control_points.len())
                .filter(|&k| curve.knots[k] < curve.knots[k + 1])
                .count();
            curves.push(curve);
        }
        check(
            spans <= max_cells,
            "Moment budget cannot cover all original spans",
        )?;
    }
    let targets = [
        tolerances[0],
        tolerances[1],
        tolerances[1],
        tolerances[2],
        tolerances[2],
        tolerances[2],
    ];
    let mut total = [Interval::point(0.); 6];
    let mut heap = BinaryHeap::new();
    let mut cells = 0;
    let make_cell = |curve: usize, span: usize, domain: [f64; 2]| -> Result<Cell> {
        let values = enclose_cell(curves[curve], span, domain, origin)?;
        let priority = values
            .iter()
            .zip(targets)
            .map(|(b, t)| (b.hi - b.lo) / t)
            .fold(0., f64::max);
        Ok(Cell {
            curve,
            span,
            domain,
            values,
            priority,
        })
    };
    for (i, curve) in curves.iter().enumerate() {
        for k in curve.degree..curve.control_points.len() {
            let domain = [curve.knots[k], curve.knots[k + 1]];
            if domain[0] == domain[1] {
                continue;
            }
            let cell = make_cell(i, k, domain)?;
            accumulate(&mut total, &cell)?;
            heap.push(cell);
            cells += 1;
        }
    }
    let meets = |values: &[Interval; 6]| {
        values
            .iter()
            .zip(targets)
            .all(|(b, t)| (b.hi - b.lo).next_up() <= t)
    };
    while !meets(&total) && cells + 2 <= max_cells {
        let Some(parent) = heap.pop() else { break };
        let [a, b] = parent.domain;
        let mid = a.midpoint(b);
        if mid <= a || mid >= b {
            continue;
        }
        for (sum, old) in total.iter_mut().zip(parent.values) {
            *sum = Interval::new((sum.lo - old.lo).next_down(), (sum.hi - old.hi).next_up())?;
        }
        for domain in [[a, mid], [mid, b]] {
            let child = make_cell(parent.curve, parent.span, domain)?;
            accumulate(&mut total, &child)?;
            heap.push(child);
            cells += 1;
        }
    }
    Ok(PlanarMassBounds {
        moments: total.map(|b| [b.lo, b.hi]),
        converged: meets(&total),
        cells,
    })
}

fn accumulate(total: &mut [Interval; 6], cell: &Cell) -> Result<()> {
    for (sum, value) in total.iter_mut().zip(cell.values) {
        *sum = sum.add(value)?;
    }
    Ok(())
}

fn enclose_cell(
    curve: &Curve,
    span: usize,
    domain: [f64; 2],
    origin: [f64; 2],
) -> Result<[Interval; 6]> {
    let shift = |mut jet: Vec<Vec<Interval>>| -> Result<Vec<Vec<Interval>>> {
        for (value, base) in jet[0].iter_mut().zip(origin) {
            *value = value.sub(Interval::point(base))?;
        }
        Ok(jet)
    };
    let a = shift(curve_jets::endpoint(curve, span, domain, false)?)?;
    let b = shift(curve_jets::endpoint(curve, span, domain, true)?)?;
    let h = shift(curve_jets::enclose(curve, span, domain)?)?;
    let j0 = cross(&h[0], &h[1])?;
    let j1 = cross(&h[0], &h[2])?;
    let j2 = cross(&h[1], &h[2])?.add(cross(&h[0], &h[3])?)?;
    let ja = cross(&a[0], &a[1])?;
    let jb = cross(&b[0], &b[1])?;
    let mut out = [Interval::point(0.); 6];
    for (i, (x, y)) in MONOMIALS.into_iter().enumerate() {
        let degree = usize::from(x.is_some()) + usize::from(y.is_some());
        let [m, dm, ddm] = monomial(&h, x, y)?;
        let fa = monomial(&a, x, y)?[0].mul(ja)?;
        let fb = monomial(&b, x, y)?[0].mul(jb)?;
        let trapezoid = fa.add(fb)?.mul(Interval::point(0.5))?;
        let second = ddm
            .mul(j0)?
            .add(dm.mul(j1)?.mul(Interval::point(2.))?)?
            .add(m.mul(j2)?)?;
        // Parameter derivatives are local to this unit cell. Trapezoid error
        // is bounded by the supremum of the second derivative divided by 12.
        let error = (second.lo.abs().max(second.hi.abs()) / 12.).next_up();
        out[i] = trapezoid
            .add(Interval::new(-error, error)?)?
            .div(Interval::point((degree + 2) as f64))?;
    }
    Ok(out)
}

fn monomial(h: &[Vec<Interval>], x: Option<usize>, y: Option<usize>) -> Result<[Interval; 3]> {
    let one = Interval::point(1.);
    let zero = Interval::point(0.);
    let Some(x) = x else {
        return Ok([one, zero, zero]);
    };
    let Some(y) = y else {
        return Ok([h[0][x], h[1][x], h[2][x]]);
    };
    Ok([
        h[0][x].mul(h[0][y])?,
        h[1][x].mul(h[0][y])?.add(h[0][x].mul(h[1][y])?)?,
        h[2][x]
            .mul(h[0][y])?
            .add(h[1][x].mul(h[1][y])?.mul(Interval::point(2.))?)?
            .add(h[0][x].mul(h[2][y])?)?,
    ])
}
