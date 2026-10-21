//! Whole original rational-curve secant bounds from second jets, not samples.
use crate::{
    Result, check,
    curve::Curve,
    distance_bounds::{Interval as I, box_distance},
    numerics::curve_jets,
};
#[derive(Clone, Debug)]
pub struct Report {
    pub endpoints: Option<[[[f64; 2]; 3]; 2]>,
    pub secant_error_upper: Option<f64>,
    pub cells: usize,
    pub reason: &'static str,
}
#[derive(Clone, Debug)]
pub struct Range {
    pub bounds: Option<[[f64; 2]; 3]>,
    pub cells: usize,
}
pub(crate) fn map(d: [f64; 2], q: f64, reverse: bool) -> Result<I> {
    if q == 0. || q == 1. {
        return Ok(I::point(d[usize::from((q == 1.) != reverse)]));
    }
    use crate::exact_curve_segments::{exact_add, exact_mul};
    let fraction = if reverse { exact_add(1., -q) } else { Some(q) };
    if let Some(value) = exact_add(d[1], -d[0])
        .and_then(|h| fraction.and_then(|t| exact_mul(h, t)))
        .and_then(|x| exact_add(d[0], x))
    {
        return Ok(I::point(value));
    }
    let t = if reverse {
        I::point(1.).sub(I::point(q))?
    } else {
        I::point(q)
    };
    I::point(d[0])
        .add(I::point(d[1]).sub(I::point(d[0]))?.mul(t)?)?
        .intersect(d[0], d[1])
}
pub(crate) fn curve_box(c: &Curve, t: I, cells: &mut usize, max: usize) -> Result<Option<Vec<I>>> {
    let dimension = c.control_points[0].len();
    let mut bounds = vec![[f64::INFINITY, f64::NEG_INFINITY]; dimension];
    let mut found = false;
    for span in c.degree..c.control_points.len() {
        let lo = t.lo.max(c.knots[span]);
        let hi = t.hi.min(c.knots[span + 1]);
        if c.knots[span] == c.knots[span + 1] || lo > hi {
            continue;
        }
        if *cells == max {
            return Ok(None);
        }
        *cells += 1;
        let b = crate::curve_distance::enclosure(c, span, I::new(lo, hi)?)?;
        for k in 0..dimension {
            bounds[k][0] = bounds[k][0].min(b[k].lo);
            bounds[k][1] = bounds[k][1].max(b[k].hi);
        }
        found = true;
    }
    if !found {
        return Ok(None);
    }
    Ok(Some(
        bounds
            .into_iter()
            .map(|x| I::new(x[0], x[1]))
            .collect::<Result<Vec<_>>>()?,
    ))
}
fn inputs(c: &Curve, q: [f64; 2], max: usize) -> Result<()> {
    c.validate()?;
    check(
        max <= 100000
            && q.iter().all(|x| x.is_finite())
            && 0. <= q[0]
            && q[0] <= q[1]
            && q[1] <= 1.
            && c.control_points[0].len() == 3,
        "Curve trace needs original 3D data, a unit interval and bounded work",
    )
}
pub fn range_bounds(c: &Curve, q: [f64; 2], reverse: bool, max: usize) -> Result<Range> {
    inputs(c, q, max)?;
    let mut out = Range {
        bounds: None,
        cells: 0,
    };
    if c.periodic || c.control_points.len() > 4096 {
        return Ok(out);
    }
    let a = map(c.domain(), q[0], reverse)?;
    let b = map(c.domain(), q[1], reverse)?;
    let t = I::new(a.lo.min(b.lo), a.hi.max(b.hi))?;
    out.bounds =
        curve_box(c, t, &mut out.cells, max)?.map(|b| std::array::from_fn(|k| [b[k].lo, b[k].hi]));
    Ok(out)
}
pub fn inspect(c: &Curve, q: [f64; 2], reverse: bool, max: usize) -> Result<Report> {
    inputs(c, q, max)?;
    check(q[0] < q[1], "Secant interval must have positive width")?;
    let mut out = Report {
        endpoints: None,
        secant_error_upper: None,
        cells: 0,
        reason: "work-limit",
    };
    if c.periodic || c.control_points.len() > 4096 {
        out.reason = "unsupported-original-chart";
        return Ok(out);
    }
    let d = c.domain();
    let ends = [map(d, q[0], reverse)?, map(d, q[1], reverse)?];
    let t = I::new(ends[0].lo.min(ends[1].lo), ends[0].hi.max(ends[1].hi))?;
    for &node in &c.knots {
        if d[0] < node
            && node < d[1]
            && t.lo <= node
            && node <= t.hi
            && c.knots.iter().filter(|&&x| x == node).count() > c.degree
        {
            out.reason = "trace-position-continuity-unproven";
            return Ok(out);
        }
        if t.lo < node
            && node < t.hi
            && d[0] < node
            && node < d[1]
            && c.knots.iter().filter(|&&x| x == node).count() > c.degree.saturating_sub(1)
        {
            out.reason = "trace-C1-unproven";
            return Ok(out);
        }
    }
    let mut points = Vec::new();
    for end in ends {
        let Some(b) = curve_box(c, end, &mut out.cells, max)? else {
            return Ok(out);
        };
        points.push(std::array::from_fn(|k| [b[k].lo, b[k].hi]));
    }
    let width = I::point(d[1])
        .sub(I::point(d[0]))?
        .mul(I::point(q[1]).sub(I::point(q[0]))?)?;
    let mut second = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    let mut found = false;
    for span in c.degree..c.control_points.len() {
        let lo = t.lo.max(c.knots[span]);
        let hi = t.hi.min(c.knots[span + 1]);
        if lo >= hi {
            continue;
        }
        if out.cells == max {
            return Ok(out);
        }
        out.cells += 1;
        let jet = curve_jets::enclose(c, span, [lo, hi])?;
        let scale = width.div(I::point(hi).sub(I::point(lo))?)?;
        let scale = scale.mul(scale)?;
        for k in 0..3 {
            let b = jet[2][k].mul(scale)?;
            second[k][0] = second[k][0].min(b.lo);
            second[k][1] = second[k][1].max(b.hi);
        }
        found = true;
    }
    if !found {
        out.reason = "incomplete-span-coverage";
        return Ok(out);
    }
    let bounds = second
        .map(|x| I::new(x[0], x[1]))
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    let (_, norm) = box_distance(&bounds, &[I::point(0.); 3])?;
    out.endpoints = Some(points.try_into().unwrap());
    out.secant_error_upper = Some(I::point(norm).div(I::point(8.))?.hi);
    out.reason = "complete-original-curve-secant-bound";
    Ok(out)
}
