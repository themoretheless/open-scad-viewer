//! Shared dyadic quintic station jets for authored rational polygon sections.
//! This bounds displacement from the input ruled walls, not an ideal sweep.
//! Shell regularity, contacts and orientation require a fresh model audit.
use super::*;
use nurbs_core::interval_eval::Interval;

pub struct PolygonStationWalls {
    pub sections: Option<Vec<Vec<Vec<Curve>>>>,
    pub sides: Option<Vec<Vec<Surface>>>,
    pub wall_displacement_upper: Option<f64>,
    pub station_orders: Vec<u8>,
    pub work: u64,
    pub reason: &'static str,
}
fn charge(report: &mut PolygonStationWalls, max: u64) -> bool {
    if report.work == max {
        return false;
    }
    report.work += 1;
    true
}

pub fn smooth_polygon_station_walls(
    source: &[Vec<Vec<Curve>>],
    sharp: &[usize],
    closed: bool,
    quantum: f64,
    tolerance: f64,
    max_work: u64,
) -> Result<PolygonStationWalls> {
    let bits = quantum.to_bits();
    if !(2..=129).contains(&source.len())
        || source[0].is_empty()
        || source[0].len() > 16
        || !quantum.is_finite()
        || quantum <= 0.
        || bits & ((1u64 << 52) - 1) != 0
        || (bits >> 52) & 2047 == 0
        || !tolerance.is_finite()
        || tolerance < 0.
        || max_work > 1_000_000
    {
        return Err(err("Invalid polygon station request or dyadic quantum"));
    }
    let n = source.len();
    if sharp.iter().any(|i| *i >= n)
        || sharp
            .iter()
            .enumerate()
            .any(|(i, x)| sharp[..i].contains(x))
    {
        return Err(err("Invalid sharp station indices"));
    }
    let mut report = PolygonStationWalls {
        sections: None,
        sides: None,
        wall_displacement_upper: None,
        station_orders: Vec::new(),
        work: 0,
        reason: "work-limit",
    };
    if closed && (n < 4 || source.first() != source.last()) {
        report.reason = "periodic-source-mismatch";
        return Ok(report);
    }
    let faces = source[0]
        .iter()
        .map(Vec::len)
        .sum::<usize>()
        .checked_mul(n - 1);
    if faces.is_none_or(|x| x + if closed { 0 } else { 2 } > MAX_FACES) {
        report.reason = "face-limit";
        return Ok(report);
    }
    let mut prepared = source.to_vec();
    for (station, row) in source.iter().enumerate() {
        if row.len() != source[0].len() {
            report.reason = "incompatible-section-basis";
            return Ok(report);
        }
        for (l, wire) in row.iter().enumerate() {
            if wire.len() < 3 || wire.len() != source[0][l].len() {
                report.reason = "unsupported-profile";
                return Ok(report);
            }
            for (edge, c) in wire.iter().enumerate() {
                c.validate()?;
                let first = &source[0][l][edge];
                if c.degree != 1
                    || c.control_points.len() != 2
                    || c.control_points.iter().any(|p| p.len() != 3)
                    || c.periodic
                    || c.knots != vec![0., 0., 1., 1.]
                    || c.weights != first.weights
                {
                    report.reason = "incompatible-section-basis";
                    return Ok(report);
                }
                if c.control_points[1] != wire[(edge + 1) % wire.len()].control_points[0] {
                    report.reason = "authored-profile-gap";
                    return Ok(report);
                }
                for i in 0..2 {
                    for k in 0..3 {
                        if !charge(&mut report, max_work) {
                            return Ok(report);
                        }
                        let integer = (c.control_points[i][k] / quantum).round();
                        let point = integer * quantum;
                        if !point.is_finite()
                            || integer.abs() >= 2_f64.powi(47)
                            || point / quantum != integer
                        {
                            report.reason = "source-lattice-range";
                            return Ok(report);
                        }
                        prepared[station][l][edge].control_points[i][k] = point;
                    }
                }
                if prepared[station][l][edge].control_points[0]
                    == prepared[station][l][edge].control_points[1]
                {
                    report.reason = "quantized-profile-collapse";
                    return Ok(report);
                }
            }
        }
    }
    let is_sharp = |i: usize| {
        sharp.contains(&i)
            || (closed
                && (i == 0 || i == n - 1)
                && (sharp.contains(&0) || sharp.contains(&(n - 1))))
    };
    let station_orders = (0..n)
        .map(|i| {
            if is_sharp(i) || (!closed && (i == 0 || i == n - 1)) {
                0
            } else {
                2
            }
        })
        .collect();
    let mut sides = vec![Vec::new(); source[0].len()];
    let mut upper = 0_f64;
    for layer in 0..n - 1 {
        for l in 0..source[0].len() {
            for edge in 0..source[0][l].len() {
                let a = &prepared[layer][l][edge];
                let b = &prepared[layer + 1][l][edge];
                let prev = if layer == 0 {
                    if closed { n - 2 } else { 0 }
                } else {
                    layer - 1
                };
                let next = if layer + 1 == n - 1 {
                    if closed { 1 } else { n - 1 }
                } else {
                    layer + 2
                };
                let mut poles = vec![vec![vec![0.; 3]; 6]; 2];
                for i in 0..2 {
                    let mut squared = [Interval::point(0.); 6];
                    for k in 0..3 {
                        let start = a.control_points[i][k] / quantum;
                        let end = b.control_points[i][k] / quantum;
                        let ta = if (!closed && layer == 0) || is_sharp(layer) {
                            ((end - start) / 5.).round()
                        } else {
                            ((b.control_points[i][k]
                                - prepared[prev][l][edge].control_points[i][k])
                                / quantum
                                / 10.)
                                .round()
                        };
                        let tb = if (!closed && layer + 1 == n - 1) || is_sharp(layer + 1) {
                            ((end - start) / 5.).round()
                        } else {
                            ((prepared[next][l][edge].control_points[i][k]
                                - a.control_points[i][k])
                                / quantum
                                / 10.)
                                .round()
                        };
                        for (j, integer) in [
                            start,
                            start + ta,
                            start + 2. * ta,
                            end - 2. * tb,
                            end - tb,
                            end,
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            if !charge(&mut report, max_work) {
                                return Ok(report);
                            }
                            let point = integer * quantum;
                            if !point.is_finite()
                                || integer.abs() >= 2_f64.powi(50)
                                || point / quantum != integer
                            {
                                report.reason = "control-lattice-range";
                                return Ok(report);
                            }
                            poles[i][j][k] = point;
                            let t = Interval::point(j as f64).div(Interval::point(5.))?;
                            let original =
                                Interval::point(source[layer][l][edge].control_points[i][k])
                                    .mul(Interval::point(1.).sub(t)?)?
                                    .add(
                                        Interval::point(
                                            source[layer + 1][l][edge].control_points[i][k],
                                        )
                                        .mul(t)?,
                                    )?;
                            let delta = Interval::point(point).sub(original)?;
                            let radius = delta.lo.abs().max(delta.hi.abs());
                            squared[j] = squared[j]
                                .add(Interval::point(radius).mul(Interval::point(radius))?)?;
                        }
                    }
                    upper = upper.max(
                        squared
                            .into_iter()
                            .map(|x| x.hi.max(0.).sqrt().next_up())
                            .fold(0., f64::max),
                    );
                }
                sides[l].push(Surface {
                    degree_u: 1,
                    degree_v: 5,
                    knots_u: vec![0., 0., 1., 1.],
                    knots_v: vec![0., 0., 0., 0., 0., 0., 1., 1., 1., 1., 1., 1.],
                    control_points: poles,
                    weights: a.weights.iter().map(|w| vec![*w; 6]).collect(),
                    periodic_u: false,
                    periodic_v: false,
                });
            }
        }
    }
    report.wall_displacement_upper = Some(upper);
    if upper > tolerance {
        report.reason = "displacement-budget";
        return Ok(report);
    }
    report.sections = Some(prepared);
    report.sides = Some(sides);
    report.station_orders = station_orders;
    report.reason = "bounded-polygon-quintic-station-jets";
    Ok(report)
}

#[cfg(test)]
#[path = "polygon_station_tests.rs"]
mod tests;
