//! Restriction query: preserves the original arithmetic and proof budgets.
use super::*;

/// Enclose the original rational image of a rectangle inside a knot cell.
pub(super) fn restricted_controls(
    s: &Surface,
    span: [usize; 2],
    domain: [[f64; 2]; 2],
) -> Result<Vec<Vec<Interval>>> {
    let [iu, iv] = span;
    let origin = &s.control_points[iu - s.degree_u][iv - s.degree_v];
    let mut scale: f64 = 0.;
    for i in iu - s.degree_u..=iu {
        for j in iv - s.degree_v..=iv {
            scale = scale.max(s.weights[i][j]);
        }
    }
    let mut weights = [f64::INFINITY, 0_f64];
    let mut controls = Vec::with_capacity(s.degree_u + 1);
    let mut hull = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for i in iu - s.degree_u..=iu {
        let mut row = Vec::with_capacity(s.degree_v + 1);
        for j in iv - s.degree_v..=iv {
            let ratio = s.weights[i][j] / scale;
            let w = if s.weights[i][j] == scale {
                Interval::point(1.)
            } else {
                Interval::new(ratio.next_down().max(0.), ratio.next_up())?
            };
            weights[0] = weights[0].min(w.lo);
            weights[1] = weights[1].max(w.hi);
            let mut p = [Interval::point(0.); 4];
            for axis in 0..3 {
                let x = s.control_points[i][j][axis];
                hull[axis][0] = hull[axis][0].min(x);
                hull[axis][1] = hull[axis][1].max(x);
                p[axis] = Interval::point(x)
                    .sub(Interval::point(origin[axis]))?
                    .mul(w)?;
            }
            p[3] = w;
            row.push(p);
        }
        controls.push(row);
    }
    // A complete clamped original Bernstein span can be restricted with two
    // convex subdivisions instead of separately evaluating every blossom.
    // This changes interval computation only; original surface controls stay intact.
    if bezier_restriction::is_complete_span(s.degree_u, &s.knots_u, iu)
        && bezier_restriction::is_complete_span(s.degree_v, &s.knots_v, iv) {
        let rows = controls.iter().map(|row| bezier_restriction::restrict(row,
            [s.knots_v[iv], s.knots_v[iv+1]], domain[1], weights))
            .collect::<Result<Vec<_>>>()?;
        let mut restricted = Vec::with_capacity((s.degree_u + 1) * rows[0].len());
        let mut column = Vec::with_capacity(rows.len());
        for j in 0..rows[0].len() {
            column.clear();
            column.extend(rows.iter().map(|row| row[j]));
            for h in bezier_restriction::restrict(&column,
                [s.knots_u[iu], s.knots_u[iu+1]], domain[0], weights)? {
                let mut control = Vec::with_capacity(3);
                for k in 0..3 {
                    control.push(h[k].div(h[3])?.add(Interval::point(origin[k]))?
                        .intersect(hull[k][0], hull[k][1])?);
                }
                restricted.push(control);
            }
        }
        return Ok(restricted);
    }
    // Blossom values at lo^(p-k), hi^k are the restricted Bernstein
    // coefficients of this original span. Keep their homogeneous arithmetic
    // interval-valued; convex hulls of the positive rational coefficients then
    // enclose the entire rectangle, including edges and knot endpoints.
    let parameters = |degree: usize, d: [f64; 2]| -> Vec<Vec<f64>> {
        if d[0] == d[1] {
            return vec![vec![d[0]; degree]];
        }
        (0..=degree)
            .map(|k| {
                (0..degree)
                    .map(|j| if j < degree - k { d[0] } else { d[1] })
                    .collect()
            })
            .collect()
    };
    let us = parameters(s.degree_u, domain[0]);
    let vs = parameters(s.degree_v, domain[1]);
    let u_plans = us.iter().map(|u| interpolation_plan(s.degree_u, &s.knots_u, iu, u))
        .collect::<Result<Vec<_>>>()?;
    let mut restricted = Vec::with_capacity(us.len() * vs.len());
    let mut rows = Vec::with_capacity(controls.len());
    for v in vs {
        let v_plan = interpolation_plan(s.degree_v, &s.knots_v, iv, &v)?;
        rows.clear();
        for row in &controls {
            rows.push(interpolate_planned(row, s.degree_v, &v_plan, weights)?);
        }
        for u_plan in &u_plans {
            let h = interpolate_planned(&rows, s.degree_u, u_plan, weights)?;
            let mut control = Vec::with_capacity(3);
            for k in 0..3 {
                let value = h[k]
                    .div(h[3])?
                    .add(Interval::point(origin[k]))?
                    .intersect(hull[k][0], hull[k][1])?;
                control.push(value);
            }
            restricted.push(control);
        }
    }
    Ok(restricted)
}
