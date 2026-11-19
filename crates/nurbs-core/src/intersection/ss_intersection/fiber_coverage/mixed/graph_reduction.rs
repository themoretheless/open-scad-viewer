//! Exact diagonal elimination for two polynomial graph carriers. Complete
//! TWO-dimensional coverage implies complete coverage of the original product:
//! the identical injective affine XY frame forces equal normalized parameters.
use super::*;
use num_rational::BigRational as Q;
use num_traits::{Signed, Zero};
type Net = Vec<Vec<Q>>;
type Cell = [[Q; 2]; 2];
mod cells;
use cells::{cut, diagnostic, full_cell};
mod multispan;
pub(in crate::intersection::ss_intersection) use multispan::equivalent_multispan_affine_surface;
mod replay;
mod rational;
#[derive(Clone, Debug, PartialEq)]
enum Node {
    Chart(usize),
    Image { index: usize, axis: usize },
    ImageExcluded { axis: usize },
    Excluded,
    Split { axis: usize, at: Q, children: [usize; 2] },
}
#[derive(Clone, Debug)]
pub(super) struct Proof {
    nodes: Vec<Node>,
    patch_roots: usize,
}
fn q(x: f64) -> Result<Q> {
    Q::from_float(x).ok_or_else(|| crate::input("Nonfinite graph coefficient"))
}
fn admit(s: &Surface) -> Result<()> {
    s.validate()?;
    check(
        !s.periodic_u
            && !s.periodic_v
            && (1..=8).contains(&s.degree_u)
            && (1..=8).contains(&s.degree_v)
            && s.control_points.len() == s.degree_u + 1
            && s.control_points[0].len() == s.degree_v + 1
            && s.control_points.iter().flatten().all(|p| p.len() == 3),
        "Need one spatial degree1..8 polynomial graph span",
    )?;
    for (d, k) in [(s.degree_u, &s.knots_u), (s.degree_v, &s.knots_v)] {
        check(
            k.len() == 2 * (d + 1)
                && k[..=d].iter().all(|v| *v == k[d])
                && k[d + 1..].iter().all(|v| *v == k[d + 1]),
            "Need full clamped graph domains",
        )?;
    }
    Ok(())
}

fn constant_weights(s: &Surface) -> bool {
    let w = s.weights[0][0];
    s.weights.iter().flatten().all(|&value| value == w)
}

fn frame(s: &Surface) -> Result<[Q; 4]> {
    admit(s)?;
    check(constant_weights(s), "Polynomial graph reduction requires constant weights")?;
    let x = q(s.control_points[0][0][0])?;
    let y = q(s.control_points[0][0][1])?;
    let dx = q(s.control_points[s.degree_u][0][0])? - &x;
    let dy = q(s.control_points[0][s.degree_v][1])? - &y;
    check(
        !dx.is_zero() && !dy.is_zero(),
        "Graph projection must be globally injective",
    )?;
    for i in 0..=s.degree_u {
        for j in 0..=s.degree_v {
            check(
                q(s.control_points[i][j][0])? == &x + &dx * Q::new(i.into(), s.degree_u.into())
                    && q(s.control_points[i][j][1])?
                        == &y + &dy * Q::new(j.into(), s.degree_v.into()),
                "Original graph XY affine coefficient identity failed",
            )?;
        }
    }
    Ok([x, y, dx, dy])
}


fn elevate(n: &Net, axis: usize) -> Net {
    let (du, dv) = (n.len() - 1, n[0].len() - 1);
    let mut out =
        vec![vec![Q::zero(); dv + 1 + usize::from(axis == 1)]; du + 1 + usize::from(axis == 0)];
    for i in 0..out.len() {
        for j in 0..out[0].len() {
            let k = if axis == 0 { i } else { j };
            let d = if axis == 0 { du } else { dv };
            if k > 0 {
                let a = if axis == 0 {
                    &n[i - 1][j]
                } else {
                    &n[i][j - 1]
                };
                out[i][j] += a * Q::new(k.into(), (d + 1).into());
            }
            if k <= d {
                let a = &n[i][j];
                out[i][j] += a * Q::new((d + 1 - k).into(), (d + 1).into());
            }
        }
    }
    out
}
fn derive(source: &FiberSources) -> Result<Net> {
    let (a, b) = (source.first(), source.second());
    check(
        frame(a)? == frame(b)?,
        "Different original affine graph frames",
    )?;
    let mut nets = vec![];
    for s in [a, b] {
        let mut n = s
            .control_points
            .iter()
            .map(|row| row.iter().map(|p| q(p[2])).collect::<Result<Vec<_>>>())
            .collect::<Result<Vec<_>>>()?;
        while n.len() <= a.degree_u.max(b.degree_u) {
            n = elevate(&n, 0);
        }
        while n[0].len() <= a.degree_v.max(b.degree_v) {
            n = elevate(&n, 1);
        }
        nets.push(n);
    }
    Ok(nets[1]
        .iter()
        .zip(&nets[0])
        .map(|(b, a)| b.iter().zip(a).map(|(b, a)| b - a).collect())
        .collect())
}
/// Exclude a closed original UV box for polynomial carriers whose affine
/// XY projections have a common physical rectangle.
pub fn exclude_shared_frame_graph_contact(source: &FiberSources, domain: [[f64; 2]; 4]) -> Result<bool> {
    match restrict_contact(source, domain) {
        Ok(result) => Ok(result.is_none_or(|(net, _)| excluded(&net))),
        Err(_) => exclude_affine_graph_contact(source, domain),
    }
}

/// Exact exclusion on the common physical XY rectangle of two affine graphs.
/// The native parameter windows can differ in scale, orientation, and origin.
fn exclude_affine_graph_contact(source: &FiberSources, domain: Box4) -> Result<bool> {
    AffineGraphExclusion::new(source)?.excludes(domain)
}

pub(in crate::intersection::ss_intersection) struct AffineGraphExclusion {
    frames: [[Q; 4]; 2],
    support: Box4,
    native_to_xy: [[Q; 2]; 4],
    nets: [Net; 2],
    weights: Option<[Net; 2]>,
}

impl AffineGraphExclusion {
    pub(in crate::intersection::ss_intersection) fn new(source: &FiberSources) -> Result<Self> {
        // A verified affine view can span several original knot intervals
        // while preserving every native parameter used for box exclusion.
        let surfaces = source.evaluation_surfaces();
        let (frames, height_axis) = [[0, 1], [0, 2], [1, 0], [1, 2], [2, 0], [2, 1]]
            .into_iter()
            .find_map(|axes| {
                let [a, b] = surfaces.map(|surface| {
                    if axes == [0, 1] && constant_weights(surface) {
                        frame(surface)
                    } else {
                        crate::surface_injectivity::exact_affine_xy::frame_on(surface, axes)
                    }
                });
                Some(([a.ok()?, b.ok()?], 3 - axes[0] - axes[1]))
            })
            .ok_or_else(|| crate::input("Original surfaces have no common axis-aligned affine projection"))?;
        let geometries = surfaces.map(|surface| -> Result<_> {
            let height = surface.control_points[0][0][height_axis];
            if constant_weights(surface)
                || surface.control_points.iter().flatten().all(|point| point[height_axis] == height)
            {
                // A constant height cancels its denominator exactly, independently
                // of the weights used to prove the original projection.
                Ok((rational::coordinate_net(surface, height_axis, None)?, None))
            } else {
                let weights = rational::weight_net(surface)?;
                let height = rational::coordinate_net(surface, height_axis, Some(&weights))?;
                Ok((height, Some(weights)))
            }
        });
        let [a, b] = geometries;
        let [(ah, aw), (bh, bw)] = [a?, b?];
        let weights = if aw.is_none() && bw.is_none() {
            None
        } else {
            let ones = |net: &Net| vec![vec![Q::from_integer(1.into()); net[0].len()]; net.len()];
            Some([aw.unwrap_or_else(|| ones(&ah)), bw.unwrap_or_else(|| ones(&bh))])
        };
        let support = natural(source);
        let native_axis = |k: usize| -> Result<[Q; 2]> {
            let frame = &frames[k / 2];
            let axis = k % 2;
            let lo = q(support[k][0])?;
            let scale = &frame[axis + 2] / (q(support[k][1])? - &lo);
            let offset = &frame[axis] - &scale * lo;
            Ok([scale, offset])
        };
        let native_to_xy = [native_axis(0)?, native_axis(1)?, native_axis(2)?, native_axis(3)?];
        Ok(Self {
            frames,
            support,
            native_to_xy,
            nets: [ah, bh],
            weights,
        })
    }

    pub(in crate::intersection::ss_intersection) fn excludes(&self, domain: Box4) -> Result<bool> {
        Ok(self.aligned_gap(domain)?.is_none_or(|(gap, _)| excluded(&gap)))
    }

    pub(in crate::intersection::ss_intersection) fn matching_parameter(
        &self,
        axis: usize,
        value: f64,
    ) -> Result<[f64; 2]> {
        let other = (axis + 2) % 4;
        let [scale, offset] = &self.native_to_xy[other];
        let physical = self.project_axis(axis, [value; 2])?;
        let mapped = (&physical[0] - offset) / scale;
        Ok([cells::outward(&mapped, false)?, cells::outward(&mapped, true)?])
    }

    pub(in crate::intersection::ss_intersection) fn excludes_subdivided(
        &self,
        domain: Box4,
        max_cells: usize,
    ) -> Result<(bool, usize)> {
        if max_cells == 0 {
            return Ok((false, 0));
        }
        let Some((gap, cell)) = self.aligned_gap(domain)? else {
            return Ok((true, 1));
        };
        let active = cell.map(|[lo, hi]| lo < hi);
        let half = Q::new(1.into(), 2.into());
        let mut pending = vec![(gap, 0)];
        let mut used = 0;
        while let Some((net, depth)) = pending.pop() {
            if used == max_cells {
                return Ok((false, used));
            }
            used += 1;
            if excluded(&net) {
                continue;
            }
            if depth == 12 || !active.iter().any(|&axis| axis) {
                return Ok((false, used));
            }
            let preferred = depth % 2;
            let axis = if active[preferred] { preferred } else { 1 - preferred };
            let [left, right] = split_at(&net, axis, &half);
            pending.extend([(right, depth + 1), (left, depth + 1)]);
        }
        Ok((true, used))
    }

    fn project_axis(&self, axis: usize, bounds: [f64; 2]) -> Result<[Q; 2]> {
        let [scale, offset] = &self.native_to_xy[axis];
        let a = scale * q(bounds[0])? + offset;
        let b = scale * q(bounds[1])? + offset;
        Ok(if a <= b { [a, b] } else { [b, a] })
    }

    fn project_chart(&self, domain: Box4) -> Result<Cell> {
        let intersect = |axis: usize| -> Result<[Q; 2]> {
            let [a, b] = self.project_axis(axis, domain[axis])?;
            let [c, d] = self.project_axis(axis + 2, domain[axis + 2])?;
            Ok([a.max(c), b.min(d)])
        };
        Ok([intersect(0)?, intersect(1)?])
    }

    fn original_bounds(&self, cell: &Cell) -> Result<Box4> {
        let mut result = [[0.; 2]; 4];
        for (axis, bounds) in result.iter_mut().enumerate() {
            let [scale, offset] = &self.native_to_xy[axis];
            let [a, b] = &cell[axis % 2];
            let a = (a - offset) / scale;
            let b = (b - offset) / scale;
            let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
            *bounds = [cells::outward(&lo, false)?, cells::outward(&hi, true)?];
        }
        Ok(result)
    }

    // Both height nets are reparameterized to the same physical rectangle.
    // Keep that exact rectangle for mapping coverage cells back to original UV.
    fn aligned_gap(&self, domain: Box4) -> Result<Option<(Net, Cell)>> {
        let support = self.support;
        check(
            (0..4).all(|k| {
                let [lo, hi] = domain[k];
                support[k][0] <= lo && lo <= hi && hi <= support[k][1]
            }),
            "Affine graph exclusion window leaves natural support",
        )?;
        let frames = &self.frames;
        let mut nets = self.nets.clone();
        let mut weights = self.weights.clone();
        let mut physical = full_cell();
        for (axis, bounds) in physical.iter_mut().enumerate() {
            let [a, b] = self.project_axis(axis, domain[axis])?;
            let [c, d] = self.project_axis(axis + 2, domain[axis + 2])?;
            let (lo, hi) = (a.max(c), b.min(d));
            if lo > hi {
                return Ok(None);
            }
            for side in 0..2 {
                let frame = &frames[side];
                let t = (&lo - &frame[axis]) / &frame[axis + 2];
                let u = (&hi - &frame[axis]) / &frame[axis + 2];
                restrict_affine_net(&mut nets[side], axis, &t, &u);
                if let Some(weights) = &mut weights {
                    restrict_affine_net(&mut weights[side], axis, &t, &u);
                }
            }
            *bounds = [lo, hi];
        }
        if let Some(weights) = weights {
            // Positive denominators leave the sign and zero set invariant.
            let a = rational::product(&nets[0], &weights[1]);
            let b = rational::product(&nets[1], &weights[0]);
            return Ok(Some((b.iter().zip(&a)
                .map(|(b, a)| b.iter().zip(a).map(|(b, a)| b - a).collect())
                .collect(), physical)));
        }
        let rows = nets[0].len().max(nets[1].len());
        let columns = nets[0][0].len().max(nets[1][0].len());
        for net in &mut nets {
            while net.len() < rows {
                *net = elevate(net, 0);
            }
            while net[0].len() < columns {
                *net = elevate(net, 1);
            }
        }
        let gap = nets[1]
            .iter()
            .zip(&nets[0])
            .map(|(b, a)| b.iter().zip(a).map(|(b, a)| b - a).collect())
            .collect();
        Ok(Some((gap, physical)))
    }
}

fn restrict_affine_net(net: &mut Net, axis: usize, t: &Q, u: &Q) {
    let reversed = t > u;
    let (start, end) = if reversed { (u, t) } else { (t, u) };
    let [prefix, _] = split_at(net, axis, end);
    *net = if end.is_zero() {
        prefix
    } else {
        let [_, suffix] = split_at(&prefix, axis, &(start / end));
        suffix
    };
    if reversed {
        if axis == 0 {
            net.reverse();
        } else {
            net.iter_mut().for_each(|row| row.reverse());
        }
    }
}

/// If the restricted height difference has one sign and is nonzero on
/// each open edge, return every possible root as an exact UV4 corner.
pub fn shared_frame_graph_contact_corners(
    source: &FiberSources,
    domain: [[f64; 2]; 4],
) -> Result<Option<Vec<[Q; 4]>>> {
    let Some((net, ranges)) = restrict_contact(source, domain)? else {
        return Ok(Some(Vec::new()));
    };
    let u = net.len() - 1;
    let v = net[0].len() - 1;
    let [u_open, v_open] = ranges.each_ref().map(|range| range[0] < range[1]);
    if (net.iter().flatten().any(Signed::is_positive)
        && net.iter().flatten().any(Signed::is_negative))
        || (v_open && [0, u].into_iter().any(|i| net[i].iter().all(Zero::is_zero)))
        || (u_open && [0, v].into_iter().any(|j| net.iter().all(|row| row[j].is_zero())))
    {
        return Ok(None);
    }
    let mut corners = Vec::new();
    for (i, row) in [0, u].into_iter().enumerate() {
        for (j, col) in [0, v].into_iter().enumerate() {
            if net[row][col].is_zero() {
                corners.push(std::array::from_fn(|axis| {
                    let t = &ranges[axis % 2][[i, j][axis % 2]];
                    let surface = if axis < 2 { source.first() } else { source.second() };
                    let knots = if axis % 2 == 0 { &surface.knots_u } else { &surface.knots_v };
                    let lo = Q::from_float(knots[0]).unwrap();
                    let hi = Q::from_float(*knots.last().unwrap()).unwrap();
                    &lo + (hi - &lo) * t
                }));
            }
        }
    }
    corners.sort_unstable();
    corners.dedup();
    Ok(Some(corners))
}

fn restrict_contact(
    source: &FiberSources,
    domain: [[f64; 2]; 4],
) -> Result<Option<(Net, Cell)>> {
    let mut net = derive(source)?;
    let mut ranges = full_cell();
    for (axis, range) in ranges.iter_mut().enumerate() {
        let normalize = |surface: &Surface, index: usize| -> Result<[Q; 2]> {
            let knots = if axis == 0 { &surface.knots_u } else { &surface.knots_v };
            let lo = q(knots[0])?;
            let width = q(*knots.last().unwrap())? - &lo;
            let [a, b] = domain[index];
            check(knots[0] <= a && a <= b && b <= *knots.last().unwrap(),
                "Graph exclusion window leaves natural support")?;
            Ok([(q(a)? - &lo) / &width, (q(b)? - lo) / width])
        };
        let [a, b] = normalize(source.first(), axis)?;
        let [c, d] = normalize(source.second(), axis + 2)?;
        let lo = a.max(c);
        let hi = b.min(d);
        if lo > hi {
            return Ok(None);
        }
        *range = [lo.clone(), hi.clone()];
        let [prefix, _] = split_at(&net, axis, &hi);
        net = if hi.is_zero() {
            prefix
        } else {
            let [_, suffix] = split_at(&prefix, axis, &(lo / hi));
            suffix
        };
    }
    Ok(Some((net, ranges)))
}

fn split_cell(cell: &Cell, net: &Net, axis: usize, at: &Q) -> Result<([Cell; 2], [Net; 2])> {
    let cells = cut(cell, axis, at)?;
    let parameter = (at - &cell[axis][0]) / (&cell[axis][1] - &cell[axis][0]);
    Ok((cells, split_at(net, axis, &parameter)))
}

fn split_parameter(bounds: &[Q; 2], chart_bounds: impl Iterator<Item = Q>) -> Q {
    let width = &bounds[1] - &bounds[0];
    let mid = &bounds[0] + &width / Q::from_integer(2.into());
    let quarter = width / Q::from_integer(4.into());
    chart_bounds
        .map(|at| ((&at - &mid).abs(), at))
        .filter(|(distance, _)| *distance <= quarter)
        .min()
        .map_or(mid, |(_, at)| at)
}
fn split_at(n: &Net, axis: usize, parameter: &Q) -> [Net; 2] {
    let complement = Q::from_integer(1.into()) - parameter;
    let (nu, nv) = (n.len(), n[0].len());
    let mut out = [vec![vec![Q::zero(); nv]; nu], vec![vec![Q::zero(); nv]; nu]];
    for line in 0..if axis == 0 { nv } else { nu } {
        let count = if axis == 0 { nu } else { nv };
        let mut p = (0..count)
            .map(|k| {
                if axis == 0 {
                    n[k][line].clone()
                } else {
                    n[line][k].clone()
                }
            })
            .collect::<Vec<_>>();
        for step in 0..count {
            if axis == 0 {
                out[0][step][line] = p[0].clone();
                out[1][count - step - 1][line] = p[count - step - 1].clone();
            } else {
                out[0][line][step] = p[0].clone();
                out[1][line][count - step - 1] = p[count - step - 1].clone();
            }
            for k in 0..count - step - 1 {
                p[k] = &p[k] * &complement + &p[k + 1] * parameter;
            }
        }
    }
    out
}
fn excluded(n: &Net) -> bool {
    n.iter().flatten().all(|v| v.is_positive()) || n.iter().flatten().all(|v| v.is_negative())
}
// The affine XY map forces equal normalized parameters on the two carriers.
// Pull each fiber back to that diagonal once before subdividing.
fn diagonal_chart(domain: Box4, actual: Box4) -> Result<Cell> {
    let normalized = |k: usize| -> Result<[Q; 2]> {
        let origin = q(domain[k][0])?;
        let width = q(domain[k][1])? - &origin;
        Ok([
            (q(actual[k][0])? - &origin) / &width,
            (q(actual[k][1])? - &origin) / &width,
        ])
    };
    let intersect = |axis| -> Result<[Q; 2]> {
        let [a_lo, a_hi] = normalized(axis)?;
        let [b_lo, b_hi] = normalized(axis + 2)?;
        Ok([a_lo.max(b_lo), a_hi.min(b_hi)])
    };
    Ok([intersect(0)?, intersect(1)?])
}

fn contained(cell: &Cell, chart: &Cell) -> bool {
    cell.iter()
        .zip(chart)
        .all(|(range, bounds)| bounds[0] <= range[0] && range[0] <= range[1] && range[1] <= bounds[1])
}
fn empty_image(image: &Cell) -> bool {
    image.iter().any(|range| range[0] > range[1])
}
fn overlaps(cell: &Cell, chart: &Cell) -> bool {
    cell.iter().zip(chart).all(|(range, bounds)| {
        bounds[0] <= bounds[1] && bounds[0] <= range[1] && range[0] <= bounds[1]
    })
}
fn root_image(cell: &Cell, n: &Net, axis: usize) -> Option<[[Q; 2]; 2]> {
    if axis >= 2 {
        return None;
    }
    let count = if axis == 0 { n.len() } else { n[0].len() };
    let lines = if axis == 0 { n[0].len() } else { n.len() };
    let degree = count.checked_sub(1)?;
    if degree == 0 {
        return None;
    }
    let coefficient = |line: usize, k: usize| {
        if axis == 0 { &n[k][line] } else { &n[line][k] }
    };
    // Reject a derivative enclosure containing zero before evaluating the
    // Bernstein midpoint. Subdivision cells around extrema often reach here.
    let first = coefficient(0, 1) - coefficient(0, 0);
    let (mut dlo, mut dhi) = (first.clone(), first);
    for line in 0..lines {
        for k in 0..degree {
            let gap = coefficient(line, k + 1) - coefficient(line, k);
            if gap < dlo {
                dlo = gap;
            } else if gap > dhi {
                dhi = gap;
            }
            if !dlo.is_positive() && !dhi.is_negative() {
                return None;
            }
        }
    }
    let derivative_scale = num_bigint::BigInt::from(degree);
    let (dlo, dhi) = (dlo * &derivative_scale, dhi * derivative_scale);

    // Bernstein midpoint weights share the same power-of-two denominator.
    let mut binomial = num_bigint::BigInt::from(1);
    let midpoint_weights: Vec<_> = (0..count)
        .map(|index| {
            let weight = binomial.clone();
            binomial *= degree - index;
            binomial /= index + 1;
            weight
        })
        .collect();
    let midpoint_scale = num_bigint::BigInt::from(1) << degree;
    let center = |line| {
        // Subdivision frequently leaves one denominator divisible by every
        // other denominator in the row. Accumulate its numerator in integers
        // and reduce once. Noncommensurate rows keep the general exact sum.
        let common = (0..count).map(|k| coefficient(line, k).denom()).max().unwrap();
        let numerator = midpoint_weights.iter().enumerate()
            .try_fold(num_bigint::BigInt::from(0), |sum, (k, weight)| {
                let value = coefficient(line, k);
                if !(common % value.denom()).is_zero() {
                    return None;
                }
                Some(sum + value.numer() * (common / value.denom()) * weight)
            });
        if let Some(numerator) = numerator {
            return Q::new(numerator, common * &midpoint_scale);
        }
        midpoint_weights.iter().enumerate()
            .map(|(k, weight)| coefficient(line, k) * weight)
            .sum::<Q>() / &midpoint_scale
    };
    let first = center(0);
    let (mut flo, mut fhi) = (first.clone(), first);
    for line in 1..lines {
        let value = center(line);
        if value < flo {
            flo = value;
        } else if value > fhi {
            fhi = value;
        }
    }
    // Both derivative bounds have the same strict sign. Reflect a negative
    // denominator interval so each extremum needs only one exact division.
    let (flo, fhi, dlo, dhi) = if dhi.is_negative() {
        (-fhi, -flo, -dhi, -dlo)
    } else {
        (flo, fhi, dlo, dhi)
    };
    let quotient_lo = &flo / if flo.is_negative() { &dlo } else { &dhi };
    let quotient_hi = &fhi / if fhi.is_positive() { &dlo } else { &dhi };
    let half = Q::new(1.into(), 2.into());
    let lo = Q::zero().max(&half - quotient_hi);
    let hi = Q::from_integer(1.into()).min(&half - quotient_lo);
    let mut roots = cell.clone();
    let width = &roots[axis][1] - &roots[axis][0];
    roots[axis] = [&roots[axis][0] + &width * lo, &roots[axis][0] + &width * hi];
    Some(roots)
}
fn derive_patches(source: &FiberSources, work: &mut Work) -> Result<Vec<(Cell, Net)>> {
    if let Ok(net) = derive(source) {
        return Ok(vec![(full_cell(), net)]);
    }
    multispan::derive(source, work)
}
// The natural knot cells cover the normalized diagonal. Restrict each
// Bernstein net and its cell together, retaining shared closed boundaries.
fn window_patches(
    patches: Vec<(Cell, Net)>,
    window: &Cell,
    work: &mut Work,
) -> Result<Vec<(Cell, Net)>> {
    let mut clipped = Vec::new();
    for (mut cell, mut net) in patches {
        let overlap: Cell = std::array::from_fn(|axis| {
            [
                cell[axis][0].clone().max(window[axis][0].clone()),
                cell[axis][1].clone().min(window[axis][1].clone()),
            ]
        });
        if overlap.iter().any(|bounds| bounds[0] >= bounds[1]) {
            continue;
        }
        for (axis, bounds) in overlap.into_iter().enumerate() {
            if bounds == cell[axis] {
                continue;
            }
            tick(work)?;
            let width = &cell[axis][1] - &cell[axis][0];
            let lo = (&bounds[0] - &cell[axis][0]) / &width;
            let hi = (&bounds[1] - &cell[axis][0]) / width;
            let [prefix, _] = split_at(&net, axis, &hi);
            let [_, suffix] = split_at(&prefix, axis, &(lo / hi));
            cell[axis] = bounds;
            net = suffix;
        }
        clipped.push((cell, net));
    }
    Ok(clipped)
}

fn tick(work: &mut Work) -> Result<()> {
    check(work.used < work.max, "Exact reduced graph work exhausted")?;
    work.used += 1;
    Ok(())
}

struct GraphProduct {
    patches: Vec<(Cell, Net)>,
    affine: Option<AffineGraphExclusion>,
    support: Box4,
}

impl GraphProduct {
    fn prepare(source: &FiberSources, domain: Box4, work: &mut Work) -> Result<Self> {
        let support = natural(source);
        let patches = match derive_patches(source, work) {
            Ok(patches) => patches,
            Err(error) if error.code == crate::RESOURCE_LIMIT => return Err(error),
            Err(_) => {
                let affine = AffineGraphExclusion::new(source)?;
                tick(work)?;
                let (net, cell) = affine.aligned_gap(domain)?
                    .filter(|(_, cell)| cell.iter().all(|range| range[0] < range[1]))
                    .ok_or_else(|| crate::input("Affine graph product has no positive XY overlap"))?;
                return Ok(Self { patches: vec![(cell, net)], affine: Some(affine), support });
            }
        };
        let patches = if domain == support {
            patches
        } else {
            let window = diagonal_chart(support, domain)?;
            check(
                window.iter().all(|range| range[0] < range[1]),
                "Reduced graph product has no positive diagonal overlap",
            )?;
            window_patches(patches, &window, work)?
        };
        Ok(Self { patches, affine: None, support })
    }

    fn chart(&self, domain: Box4) -> Result<Cell> {
        match &self.affine {
            Some(affine) => affine.project_chart(domain),
            None => diagonal_chart(self.support, domain),
        }
    }

    fn repair_box(&self, cell: &Cell, window: Box4) -> Result<Box4> {
        let enclosure = match &self.affine {
            Some(affine) => affine.original_bounds(cell)?,
            None => diagnostic(cell, self.support)?,
        };
        Ok(std::array::from_fn(|axis| {
            [
                enclosure[axis][0].max(window[axis][0]),
                enclosure[axis][1].min(window[axis][1]),
            ]
        }))
    }
}

fn report(cells: usize, excluded_cells: usize, rectangles: usize, entire: bool) -> FiberCoverageReport {
    FiberCoverageReport {
        status: FiberCoverageStatus::Complete,
        reason: "complete-original-graph-reduction",
        cells,
        excluded_cells,
        rectangles,
        requested_entire_surfaces: entire,
        unresolved_box: None,
    }
}
/// None means this exact reduction family does not apply. A failed admitted
/// reduction reports unresolved coverage; it never becomes an empty root set.
pub(super) fn construct(
    fibers: &[QualifiedFiber],
    window: Option<Box4>,
    limits: CoverageLimits,
) -> Result<Option<MixedFiberCoverageResult>> {
    limits.validate()?;
    if fibers.is_empty() {
        return Ok(None);
    }
    let support = natural(fibers[0].source());
    let domain = window.unwrap_or(support);
    let entire = domain == support;
    validate(fibers, domain)?;
    let mut work = Work {
        used: 0,
        max: limits.max_rectangles,
    };
    let mut product = match GraphProduct::prepare(fibers[0].source(), domain, &mut work) {
        Ok(product) => product,
        Err(error) if error.code == crate::RESOURCE_LIMIT => return Err(error),
        Err(_) => return Ok(None),
    };
    let patches = std::mem::take(&mut product.patches);
    if patches.len() > limits.max_cells {
        return Err(crate::resource(
            "Original graph knot census exceeds cell budget",
        ));
    }
    let patch_roots = patches.len();
    let mut r = MixedFiberCoverageResult {
        coverage: None,
        report: report(1, 0, 0, entire),
    };
    r.report.status = FiberCoverageStatus::Unresolved;
    r.report.reason = "reduced-graph-unproved";
    for f in fibers {
        if replay_fiber(f, &mut work).is_err() {
            r.report.rectangles = work.used;
            return Ok(Some(r));
        }
    }
    let charts = fibers
        .iter()
        .map(|fiber| product.chart(fiber.domain()))
        .collect::<Result<Vec<_>>>()?;
    let mut nodes = vec![Node::Excluded; patch_roots];
    let mut pending: Vec<_> = patches
        .into_iter()
        .enumerate()
        .map(|(i, (cell, net))| {
            let candidates = (0..charts.len()).collect::<Vec<_>>();
            (i, cell, net, 0, candidates)
        })
        .collect();
    let mut excludes = 0;
    let mut covered = 0;
    'cells: while let Some((id, cell, net, depth, mut candidates)) = pending.pop() {
        if tick(&mut work).is_err() {
            r.report.reason = "reduced-graph-work-budget";
            r.report.unresolved_box = Some(product.repair_box(&cell, domain)?);
            break;
        }
        if excluded(&net) {
            nodes[id] = Node::Excluded;
            excludes += 1;
            continue;
        }
        // Descendants and their clipped root images stay inside this cell.
        // Keep touching charts so ownership on shared closed edges is preserved.
        candidates.retain(|&index| overlaps(&cell, &charts[index]));
        let mut owner = None;
        for &index in &candidates {
            if contained(&cell, &charts[index]) {
                owner = Some(index);
                break;
            }
        }
        if let Some(index) = owner {
            nodes[id] = Node::Chart(index);
            covered += 1;
            continue;
        }
        let mut image_owner = None;
        for axis in 0..2 {
            if let Some(roots) = root_image(&cell, &net, axis) {
                if empty_image(&roots) {
                    nodes[id] = Node::ImageExcluded { axis };
                    excludes += 1;
                    continue 'cells;
                }
                for &index in &candidates {
                    if contained(&roots, &charts[index]) {
                        image_owner = Some((index, axis));
                        break;
                    }
                }
            }
            if image_owner.is_some() {
                break;
            }
        }
        if let Some((index, axis)) = image_owner {
            nodes[id] = Node::Image { index, axis };
            covered += 1;
            continue;
        }
        if depth == limits.max_depth || nodes.len() + 2 > limits.max_cells {
            r.report.reason = "reduced-graph-subdivision-budget";
            // Diagnostic cell in ORIGINAL native coordinates, rounded outward.
            r.report.unresolved_box = Some(product.repair_box(&cell, domain)?);
            break;
        }
        let axis = usize::from(&cell[1][1] - &cell[1][0] > &cell[0][1] - &cell[0][0]);
        // A balanced cut on a chart shore avoids repeatedly bisecting cells
        // that straddle the same certified ownership boundary.
        let chart_bounds = candidates.iter()
            .filter(|_| product.affine.is_some())
            .flat_map(|&index| charts[index][axis].iter().cloned());
        let at = split_parameter(&cell[axis], chart_bounds);
        let (cells, nets) = split_cell(&cell, &net, axis, &at)?;
        let children = [nodes.len(), nodes.len() + 1];
        nodes.extend([Node::Excluded, Node::Excluded]);
        nodes[id] = Node::Split { axis, at, children };
        let [left_cell, right_cell] = cells;
        let [left_net, right_net] = nets;
        pending.push((children[1], right_cell, right_net, depth + 1, candidates.clone()));
        pending.push((children[0], left_cell, left_net, depth + 1, candidates));
    }
    r.report.cells = nodes.len();
    r.report.excluded_cells = excludes;
    r.report.rectangles = work.used;
    if pending.is_empty() && r.report.unresolved_box.is_none() && covered > 0 {
        r.report = report(nodes.len(), excludes, work.used, entire);
        r.coverage = Some(QualifiedMixedFiberCoverage {
            fibers: fibers.to_vec(),
            domain,
            nodes: vec![],
            limits,
            report: r.report.clone(),
            reduction: Some(Proof { nodes, patch_roots }),
        });
    }
    Ok(Some(r))
}

#[cfg(test)]
mod tests;
