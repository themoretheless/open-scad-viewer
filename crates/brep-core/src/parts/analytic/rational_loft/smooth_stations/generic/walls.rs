//! Stages of the general adaptive-degree station-wall construction.
//! Every stage records its refusal in the shared report and returns `None`.
use super::*;

/// Wall V degrees tried in order; higher degrees use smaller jet offsets.
const DEGREE_LADDER: [usize; 3] = [5, 13, 25];
/// Largest retained Bezier piece accepted as a wall U row.
const MAX_PIECE_CONTROLS: usize = 33;
/// Exclusive bound (2^48) on lattice units of a rounded station tangent.
const TANGENT_UNIT_LIMIT: f64 = 281_474_976_710_656.;

type Prepared = Vec<Vec<Vec<Curve>>>;
type Poles = Vec<Vec<Vec<f64>>>;

fn refuse<T>(out: &mut SmoothStationWalls, reason: &'static str) -> Result<Option<T>> {
    out.reason = reason;
    Ok(None)
}

pub(super) fn build(
    sections: &[Vec<Vec<Curve>>],
    sharp: &[usize],
    closed: bool,
    quantum: f64,
    tolerance: f64,
    max_work: u64,
) -> Result<SmoothStationWalls> {
    let mut out = SmoothStationWalls {
        sides: None,
        wall_displacement_upper: None,
        work: 0,
        reason: "work-limit",
    };
    let Some(prepared) = prepare(sections, closed, &mut out, max_work)? else {
        return Ok(out);
    };
    let mut walls = Walls {
        prepared: &prepared,
        sharp,
        closed,
        n: prepared.len(),
        loops: prepared[0].len(),
        quantum,
        tolerance,
        max_work,
        out,
    };
    for degree in DEGREE_LADDER {
        if walls.attempt(degree)? != Attempt::Retry {
            break;
        }
    }
    Ok(walls.out)
}

/// Splits every station into retained Bezier pieces sharing one basis.
fn prepare(
    sections: &[Vec<Vec<Curve>>],
    closed: bool,
    out: &mut SmoothStationWalls,
    max_work: u64,
) -> Result<Option<Prepared>> {
    let n = sections.len();
    let Some(first_station) = sections.first().filter(|_| n >= 2) else {
        return refuse(out, "unsupported-profile");
    };
    let loops = first_station.len();
    let span_limit = (MAX_FACES - if closed { 0 } else { 2 }) / (n - 1);
    let mut prepared = Vec::with_capacity(n);
    for station in sections {
        if station.len() != loops {
            return refuse(out, "incompatible-section-basis");
        }
        let mut row = Vec::with_capacity(loops);
        let mut station_spans = 0;
        for (wire, first) in station.iter().zip(first_station) {
            let remaining = span_limit - station_spans;
            let Some(spans) = prepare_wire(wire, first, remaining, out, max_work)? else {
                return Ok(None);
            };
            station_spans += spans.len();
            row.push(spans);
        }
        prepared.push(row);
    }
    let count = prepared[0].iter().map(Vec::len).sum::<usize>();
    if count * (n - 1) + if closed { 0 } else { 2 } > MAX_FACES {
        return refuse(out, "face-limit");
    }
    if !same_piece_basis(&prepared) {
        return refuse(out, "incompatible-section-basis");
    }
    Ok(Some(prepared))
}

fn prepare_wire(
    wire: &[Curve],
    first: &[Curve],
    remaining: usize,
    out: &mut SmoothStationWalls,
    max_work: u64,
) -> Result<Option<Vec<Curve>>> {
    if wire.len() > remaining {
        return refuse(out, "face-limit");
    }
    if wire.len() != first.len() {
        return refuse(out, "incompatible-section-basis");
    }
    let mut spans = Vec::new();
    for (c, first) in wire.iter().zip(first) {
        c.validate()?;
        if !charge(&mut out.work, max_work) {
            return Ok(None);
        }
        if c.degree != first.degree
            || c.knots != first.knots
            || c.weights != first.weights
            || c.periodic != first.periodic
            || c.control_points.iter().any(|p| p.len() != 3)
        {
            return refuse(out, "incompatible-section-basis");
        }
        spans.extend(retained_bezier_pieces(c)?);
        if spans.len() > remaining {
            return refuse(out, "face-limit");
        }
    }
    if spans.is_empty()
        || spans.len() > MAX_FACES
        || spans.iter().any(|p| {
            p.control_points.len() != p.degree + 1 || p.control_points.len() > MAX_PIECE_CONTROLS
        })
    {
        return refuse(out, "unsupported-profile");
    }
    Ok(Some(spans))
}

fn same_piece_basis(prepared: &Prepared) -> bool {
    prepared[1..].iter().all(|station| {
        prepared[0].iter().zip(station).all(|(a, b)| {
            a.len() == b.len()
                && a.iter().zip(b).all(|(a, b)| {
                    a.degree == b.degree && a.knots == b.knots && a.weights == b.weights
                })
        })
    })
}

#[derive(PartialEq, Eq)]
enum Attempt {
    /// The report is final (accepted or refused for good).
    Stop,
    /// The report holds a refusal a higher degree may still clear.
    Retry,
}

struct Walls<'a> {
    prepared: &'a Prepared,
    sharp: &'a [usize],
    closed: bool,
    n: usize,
    loops: usize,
    quantum: f64,
    tolerance: f64,
    max_work: u64,
    out: SmoothStationWalls,
}

impl Walls<'_> {
    fn is_sharp(&self, i: usize) -> bool {
        let (sharp, n) = (self.sharp, self.n);
        sharp.contains(&i)
            || self.closed
                && (i == 0 || i == n - 1)
                && (sharp.contains(&0) || sharp.contains(&(n - 1)))
    }

    fn charge(&mut self) -> bool {
        charge(&mut self.out.work, self.max_work)
    }

    fn attempt(&mut self, degree: usize) -> Result<Attempt> {
        self.out.wall_displacement_upper = None;
        self.out.reason = "work-limit";
        let Some((sides, mut upper)) = self.sides(degree)? else {
            return Ok(Attempt::Stop);
        };
        // This outward norm enclosure has a positive rounding floor at zero.
        // Preserve the original complete refusal without spending extra work.
        if self.tolerance == 0. && upper > 0. {
            self.out.wall_displacement_upper = Some(upper);
            self.out.reason = "displacement-budget";
            return Ok(Attempt::Stop);
        }
        if upper > self.tolerance {
            let tighter =
                error_bound::tighten(&sides, self.tolerance, &mut self.out.work, self.max_work)?;
            let Some(tighter) = tighter else {
                self.out.wall_displacement_upper = None;
                self.out.reason = "work-limit";
                return Ok(Attempt::Stop);
            };
            upper = tighter;
        }
        self.out.wall_displacement_upper = Some(upper);
        if !upper.is_finite() || upper > self.tolerance {
            self.out.reason = "displacement-budget";
            return Ok(Attempt::Retry);
        }
        if !self.jets_shared(&sides) {
            // A rounded low-degree proposal can lose exact jets at an exponent
            // boundary. Higher degrees use smaller offsets; recheck them under
            // the remaining allowance.
            self.out.reason = "shared-station-jets-unproved";
            return Ok(Attempt::Retry);
        }
        self.out.sides = Some(sides);
        self.out.reason = if degree == DEGREE_LADDER[0] {
            "bounded-general-quintic-station-jets"
        } else {
            "bounded-general-higher-degree-station-jets"
        };
        Ok(Attempt::Stop)
    }

    /// All wall patches of one degree with their joint displacement bound.
    fn sides(&mut self, degree: usize) -> Result<Option<(Vec<Vec<Surface>>, f64)>> {
        let mut sides = vec![Vec::new(); self.loops];
        let mut upper = 0_f64;
        for layer in 0..self.n - 1 {
            for (l, side) in sides.iter_mut().enumerate() {
                for q in 0..self.prepared[0][l].len() {
                    let Some((surface, patch_upper)) = self.patch(degree, layer, l, q)? else {
                        return Ok(None);
                    };
                    upper = upper.max(patch_upper);
                    side.push(surface);
                }
            }
        }
        Ok(Some((sides, upper)))
    }

    fn patch(
        &mut self,
        degree: usize,
        layer: usize,
        l: usize,
        q: usize,
    ) -> Result<Option<(Surface, f64)>> {
        let prepared = self.prepared;
        let a = &prepared[layer][l][q];
        let b = &prepared[layer + 1][l][q];
        let Some(poles) = self.poles(degree, layer, l, q)? else {
            return Ok(None);
        };
        let Some(upper) = self.displacement(a, b, &poles, degree)? else {
            return Ok(None);
        };
        let surface = Surface {
            degree_u: a.degree,
            degree_v: degree,
            knots_u: a.knots.clone(),
            knots_v: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
            control_points: poles,
            weights: a.weights.iter().map(|&w| vec![w; degree + 1]).collect(),
            periodic_u: false,
            periodic_v: false,
        };
        if surface.validate().is_err() {
            return refuse(&mut self.out, "control-lattice-range");
        }
        Ok(Some((surface, upper)))
    }

    /// Rounded lattice tangent between two stations of one control coordinate.
    fn tangent(
        &self,
        [before, after]: [usize; 2],
        den: f64,
        [l, q, i, k]: [usize; 4],
    ) -> Option<f64> {
        let prepared = self.prepared;
        let units = ((prepared[after][l][q].control_points[i][k]
            - prepared[before][l][q].control_points[i][k])
            / den
            / self.quantum)
            .round();
        (units.is_finite() && units.abs() < TANGENT_UNIT_LIMIT).then_some(units * self.quantum)
    }

    fn start_tangent(&self, degree: usize, layer: usize, at: [usize; 4]) -> Option<f64> {
        if (!self.closed && layer == 0) || self.is_sharp(layer) {
            self.tangent([layer, layer + 1], degree as f64, at)
        } else {
            let before = if layer == 0 { self.n - 2 } else { layer - 1 };
            self.tangent([before, layer + 1], (2 * degree) as f64, at)
        }
    }

    fn end_tangent(&self, degree: usize, layer: usize, at: [usize; 4]) -> Option<f64> {
        let last = self.n - 1;
        if (!self.closed && layer + 1 == last) || self.is_sharp(layer + 1) {
            self.tangent([layer, layer + 1], degree as f64, at)
        } else {
            let after = if layer + 1 == last { 1 } else { layer + 2 };
            self.tangent([layer, after], (2 * degree) as f64, at)
        }
    }

    fn poles(&mut self, degree: usize, layer: usize, l: usize, q: usize) -> Result<Option<Poles>> {
        let prepared = self.prepared;
        let a = &prepared[layer][l][q];
        let b = &prepared[layer + 1][l][q];
        let mut poles = vec![vec![vec![0.; 3]; degree + 1]; a.control_points.len()];
        for (i, row) in poles.iter_mut().enumerate() {
            for k in 0..3 {
                if !self.charge() {
                    return Ok(None);
                }
                let at = [l, q, i, k];
                let ta = self.start_tangent(degree, layer, at);
                let tb = self.end_tangent(degree, layer, at);
                let (Some(ta), Some(tb)) = (ta, tb) else {
                    return refuse(&mut self.out, "station-tangent-range");
                };
                let p = a.control_points[i][k];
                let r = b.control_points[i][k];
                let paired_end = self.closed || layer + 1 != self.n - 1;
                let (left_near, left_far) = if paired_end && !self.is_sharp(layer + 1) {
                    if !self.charge() {
                        return Ok(None);
                    }
                    // Derive the left jet from the same rounded right controls
                    // used by the next patch. This avoids independent p +/- t
                    // rounding across binary exponent boundaries. Exact
                    // predicates below still decide whether both jets agree.
                    let right_near = r + tb;
                    let right_far = r + 2. * tb;
                    let near = 2. * r - right_near;
                    (near, (2. * near - 2. * right_near) + right_far)
                } else {
                    (r - tb, r - 2. * tb)
                };
                for (j, pole) in row.iter_mut().enumerate() {
                    let value = match j {
                        0 => p,
                        1 => p + ta,
                        2 => p + 2. * ta,
                        j if j == degree - 2 => left_far,
                        j if j == degree - 1 => left_near,
                        j if j == degree => r,
                        _ => p + (r - p) * (j as f64 / degree as f64),
                    };
                    if !value.is_finite() {
                        return refuse(&mut self.out, "control-lattice-range");
                    }
                    pole[k] = value;
                }
            }
        }
        Ok(Some(poles))
    }

    /// Outward bound on the distance of the poles from the ruled wall.
    fn displacement(
        &mut self,
        a: &Curve,
        b: &Curve,
        poles: &Poles,
        degree: usize,
    ) -> Result<Option<f64>> {
        let mut upper = 0_f64;
        for (i, row) in poles.iter().enumerate() {
            for (j, pole) in row.iter().enumerate() {
                let mut squared = Interval::point(0.);
                for k in 0..3 {
                    if !self.charge() {
                        return Ok(None);
                    }
                    let t = Interval::point(j as f64).div(Interval::point(degree as f64))?;
                    let original = Interval::point(a.control_points[i][k])
                        .mul(Interval::point(1.).sub(t)?)?
                        .add(Interval::point(b.control_points[i][k]).mul(t)?)?;
                    let delta = Interval::point(pole[k]).sub(original)?;
                    let radius = delta.lo.abs().max(delta.hi.abs());
                    squared = squared.add(Interval::point(radius).mul(Interval::point(radius))?)?;
                }
                upper = upper.max(squared.hi.max(0.).sqrt().next_up());
            }
        }
        Ok(Some(upper))
    }

    /// Exact shared jets at every smooth interior (and closing) station.
    fn jets_shared(&mut self, sides: &[Vec<Surface>]) -> bool {
        let n = self.n;
        for (l, side) in sides.iter().enumerate() {
            let spans = self.prepared[0][l].len();
            let mut joins: Vec<(usize, usize)> = (1..n - 1)
                .filter(|&station| !self.is_sharp(station))
                .flat_map(|station| {
                    (0..spans).map(move |q| ((station - 1) * spans + q, station * spans + q))
                })
                .collect();
            if self.closed && !self.is_sharp(0) {
                joins.extend((0..spans).map(|q| ((n - 2) * spans + q, q)));
            }
            for (left, right) in joins {
                if !shared_jets(&side[left], &side[right], &mut self.out.work, self.max_work) {
                    return false;
                }
            }
        }
        true
    }
}
