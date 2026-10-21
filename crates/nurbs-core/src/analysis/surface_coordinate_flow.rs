//! Original-span derivative bounds for monotone coordinate flow, not injectivity.
use crate::{
    Result, check, distance_bounds::Interval as I, surface::Surface, surface_measure::jets,
};
pub struct Report {
    pub cells: usize,
    pub complete: bool,
    pub derivative_interval: Option<[f64; 2]>,
    pub direction: Option<i8>,
    pub speed_upper: Option<f64>,
    pub inverse_distance_factor: Option<f64>,
}
/// If direction is present, changing parameter `axis` changes world coordinate
/// `coordinate` strictly with one sign throughout the ORIGINAL chart. The factor
/// bounds world displacement divided by coordinate displacement on each fixed
/// other-parameter line. Continuous piecewise knot spans suffice; no sample or
/// rounded restriction provides evidence.
pub fn inspect(s: &Surface, axis: usize, coordinate: usize, max_cells: usize) -> Result<Report> {
    s.validate()?;
    check(
        axis < 2 && coordinate < 3 && max_cells <= 100000,
        "Coordinate flow needs valid axes and bounded work",
    )?;
    let mut out = Report {
        cells: 0,
        complete: false,
        derivative_interval: None,
        direction: None,
        speed_upper: None,
        inverse_distance_factor: None,
    };
    if s.periodic_u || s.periodic_v {
        return Ok(out);
    }
    // A jump at a fully multiple interior knot invalidates the global
    // displacement integral even if every individual span is monotone.
    for (knots, degree, count) in [(&s.knots_u, s.degree_u, s.control_points.len()),
        (&s.knots_v, s.degree_v, s.control_points[0].len())] {
        let mut i = degree + 1;
        while i < count {
            let mut end = i + 1;
            while end < knots.len() && knots[end] == knots[i] { end += 1; }
            if knots[i] > knots[degree] && knots[i] < knots[count] && end - i > degree {
                return Ok(out);
            }
            i = end;
        }
    }
    let mut derivative = [f64::INFINITY, f64::NEG_INFINITY];
    let mut speed = 0_f64;
    for u in s.degree_u..s.control_points.len() {
        for v in s.degree_v..s.control_points[0].len() {
            let domain = [
                [s.knots_u[u], s.knots_u[u + 1]],
                [s.knots_v[v], s.knots_v[v + 1]],
            ];
            if domain.iter().any(|d| d[0] == d[1]) {
                continue;
            }
            if out.cells == max_cells {
                return Ok(out);
            }
            out.cells += 1;
            let j = jets::calculate_first_stable(s, [u, v], domain)?;
            let length = I::point(domain[axis][1]).sub(I::point(domain[axis][0]))?;
            if length.lo <= 0. {
                return Ok(out);
            }
            let d = j[axis + 1].map(|x| x.div(length));
            let d = [d[0].clone()?, d[1].clone()?, d[2].clone()?];
            derivative[0] = derivative[0].min(d[coordinate].lo);
            derivative[1] = derivative[1].max(d[coordinate].hi);
            let mut squared = I::point(0.);
            for x in d {
                let m = I::point(x.lo.abs().max(x.hi.abs()));
                squared = squared.add(m.mul(m)?)?;
            }
            speed = speed.max(squared.hi.max(0.).sqrt().next_up());
        }
    }
    if out.cells == 0 {
        return Ok(out);
    }
    out.complete = true;
    out.derivative_interval = Some(derivative);
    out.speed_upper = Some(speed);
    let lower = if derivative[0] > 0. {
        out.direction = Some(1);
        derivative[0]
    } else if derivative[1] < 0. {
        out.direction = Some(-1);
        -derivative[1]
    } else {
        return Ok(out);
    };
    out.inverse_distance_factor = Some(I::point(speed).div(I::point(lower))?.hi);
    Ok(out)
}
