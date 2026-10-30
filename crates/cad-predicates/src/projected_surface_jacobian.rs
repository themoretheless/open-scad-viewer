//! Exact tensor Bernstein signs of a rational Bezier XY Jacobian numerator.
//! Consistent coefficients imply strict orientation in the open chart; this
//! alone proves neither global injectivity nor boundary ownership.
use crate::{
    Algebra, AuthoredScalar, ContextIdentity, Expansion, InputError, LeafRef, PredicateContext,
    Reason, Sign, exact_inputs,
};
type Poly = Vec<Vec<Expansion>>;
pub struct Decision {
    pub signs: Option<Vec<Vec<Sign>>>,
    /// Outward enclosures in the same positive homogeneous coefficient scale.
    /// Requested explicitly; sign-only callers retain their original work budget.
    pub coefficient_bounds: Option<Vec<Vec<[f64; 2]>>>,
    /// Derivative/transverse queries: positive factor multiplying coefficients
    /// in authored Cartesian/weight units and normalized U/V. Normal Jacobian
    /// queries currently leave this absent. Divide bounds only when present.
    pub coefficient_scale: Option<[f64; 2]>,
    pub reason: Option<Reason>,
    pub work_used: u64,
    pub context: ContextIdentity,
}
fn choose(n: usize, k: usize) -> u64 {
    (0..k).fold(1, |v, i| v * (n - i) as u64 / (i + 1) as u64)
}
fn zeros(u: usize, v: usize) -> Poly {
    vec![vec![Expansion::scalar(0.); v]; u]
}
// Products retain Bernstein form, scaled by the same positive integer for
// every coefficient. Equal bidegrees therefore share the scale in a minor.
fn factors(degree: usize, ctx: &mut PredicateContext<'_>) -> Result<Vec<Expansion>, Reason> {
    let gcd = |mut a: u64, mut b: u64| {
        while b != 0 {
            let r = a % b;
            a = b;
            b = r;
        }
        a
    };
    let mut common = 1u64;
    for k in 0..=degree {
        let c = choose(degree, k);
        common = (common / gcd(common, c))
            .checked_mul(c)
            .ok_or(Reason::MissingProof)?;
    }
    (0..=degree)
        .map(|k| Expansion::integer(common / choose(degree, k), ctx))
        .collect()
}
fn derivative(p: &Poly, axis: usize, ctx: &mut PredicateContext<'_>) -> Result<Poly, Reason> {
    let degree = if axis == 0 {
        p.len() - 1
    } else {
        p[0].len() - 1
    };
    let multiplier = Expansion::integer(degree as u64, ctx)?;
    let mut out = zeros(
        p.len() - usize::from(axis == 0),
        p[0].len() - usize::from(axis == 1),
    );
    for (u, row) in out.iter_mut().enumerate() {
        for (v, x) in row.iter_mut().enumerate() {
            *x = p[u + usize::from(axis == 0)][v + usize::from(axis == 1)]
                .sub(&p[u][v], ctx)?
                .mul(&multiplier, ctx)?;
        }
    }
    Ok(out)
}
pub(crate) fn mul(
    a: &Poly,
    b: &Poly,
    selected_cell: [Option<usize>; 2],
    ctx: &mut PredicateContext<'_>,
) -> Result<Poly, Reason> {
    let degrees = [a.len() + b.len() - 2, a[0].len() + b[0].len() - 2];
    let window =
        std::array::from_fn(|axis| selected_cell[axis].map_or([0, degrees[axis]], |i| [i, i]));
    mul_window(a, b, window, ctx)
}
fn mul_window(
    a: &Poly,
    b: &Poly,
    window: [[usize; 2]; 2],
    ctx: &mut PredicateContext<'_>,
) -> Result<Poly, Reason> {
    let (p, q, r, s) = (a.len() - 1, a[0].len() - 1, b.len() - 1, b[0].len() - 1);
    let mut out = zeros(p + r + 1, q + s + 1);
    let fu = factors(p + r, ctx)?;
    let fv = factors(q + s, ctx)?;
    for (i, row) in a.iter().enumerate() {
        for (j, x) in row.iter().enumerate() {
            if x.sign() == Sign::Zero {
                continue;
            }
            for (k, row) in b.iter().enumerate() {
                for (l, y) in row.iter().enumerate() {
                    if i + k < window[0][0]
                        || i + k > window[0][1]
                        || j + l < window[1][0]
                        || j + l > window[1][1]
                        || y.sign() == Sign::Zero
                    {
                        continue;
                    }
                    let scale = choose(p, i) * choose(q, j) * choose(r, k) * choose(s, l);
                    let term = x.mul(y, ctx)?.mul(&Expansion::integer(scale, ctx)?, ctx)?;
                    out[i + k][j + l] = out[i + k][j + l].add(&term, ctx)?;
                }
            }
        }
    }
    for (i, row) in out.iter_mut().enumerate() {
        for (j, x) in row.iter_mut().enumerate() {
            *x = x.mul(&fu[i], ctx)?.mul(&fv[j], ctx)?;
        }
    }
    Ok(out)
}
fn difference(a: Poly, b: Poly, ctx: &mut PredicateContext<'_>) -> Result<Poly, Reason> {
    a.into_iter()
        .zip(b)
        .map(|(a, b)| a.into_iter().zip(b).map(|(a, b)| a.sub(&b, ctx)).collect())
        .collect()
}
/// XYZ+weight leaves; caller separately validates positive chart spans,
/// clamped Bezier layout, positive weights and original domain containment.
fn compute(
    ctx: &mut PredicateContext<'_>,
    surface: &[Vec<[LeafRef; 4]>],
    axes: [usize; 2],
    row: Option<usize>,
    column: Option<usize>,
    with_bounds: bool,
) -> Result<Decision, InputError> {
    compute_weights(ctx, surface, axes, row, column, with_bounds, None)
}
fn compute_weights(
    ctx: &mut PredicateContext<'_>,
    surface: &[Vec<[LeafRef; 4]>],
    axes: [usize; 2],
    row: Option<usize>,
    column: Option<usize>,
    with_bounds: bool,
    column_factors: Option<&[LeafRef]>,
) -> Result<Decision, InputError> {
    if !(2..=9).contains(&surface.len())
        || !(2..=9).contains(&surface[0].len())
        || surface.iter().any(|r| r.len() != surface[0].len())
        || axes[0] > 2
        || axes[1] > 2
        || axes[0] == axes[1]
        || row.is_some_and(|r| r >= 3 * (surface.len() - 1))
        || column.is_some_and(|c| c >= 3 * (surface[0].len() - 1))
    {
        return Err(InputError::InvalidInput("Invalid projected surface chart"));
    }
    let mut values = surface
        .iter()
        .flatten()
        .flat_map(|p| p.iter())
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    if let Some(factors) = column_factors {
        if factors.len() != surface[0].len() {
            return Err(InputError::InvalidInput(
                "Invalid factorized weight columns",
            ));
        }
        values.extend(
            factors
                .iter()
                .map(|&r| ctx.resolve(r).cloned())
                .collect::<Result<Vec<_>, _>>()?,
        );
    }
    values.push(AuthoredScalar::Binary64Bits(1f64.to_bits()));
    let result = (|| -> Result<(Vec<Vec<Sign>>, Option<Vec<Vec<[f64; 2]>>>), Reason> {
        ctx.charge(values.len() as u64)?;
        let control_len = surface.len() * surface[0].len() * 4;
        let x = if column_factors.is_some() {
            // Normalize Cartesian leaves+unit separately from the factor leaves.
            // Both scales are common positive constants; every homogeneous
            // coordinate receives their same product. Unit weight factors need
            // not inherit the Cartesian exponent normalization.
            let mut coordinates: Vec<_> = values[..control_len]
                .chunks_exact(4)
                .flat_map(|p| p[..3].iter().cloned())
                .collect();
            coordinates.push(values.last().unwrap().clone());
            let coordinates = exact_inputs(&coordinates, ctx)?;
            let factors: Vec<_> = values[..control_len]
                .chunks_exact(4)
                .map(|p| p[3].clone())
                .chain(values[control_len..values.len() - 1].iter().cloned())
                .collect();
            let factors = exact_inputs(&factors, ctx)?;
            let controls = control_len / 4;
            let mut exact = Vec::with_capacity(values.len());
            for i in 0..controls {
                exact.extend_from_slice(&coordinates[i * 3..i * 3 + 3]);
                exact.push(factors[i].clone());
            }
            exact.extend_from_slice(&factors[controls..]);
            exact.push(coordinates.last().unwrap().clone());
            exact
        } else {
            exact_inputs(&values, ctx)?
        };
        let unit = x.last().unwrap();
        if x[..control_len]
            .chunks_exact(4)
            .any(|p| p[3].sign() != Sign::Positive)
        {
            return Err(Reason::MissingProof);
        }
        if column_factors.is_some()
            && x[control_len..x.len() - 1]
                .iter()
                .any(|f| f.sign() != Sign::Positive)
        {
            return Err(Reason::MissingProof);
        }
        let mut h = (0..3)
            .map(|_| zeros(surface.len(), surface[0].len()))
            .collect::<Vec<_>>();
        for (u, row) in surface.iter().enumerate() {
            for (v, _) in row.iter().enumerate() {
                let at = (u * row.len() + v) * 4;
                let weight = if column_factors.is_some() {
                    x[at + 3].mul(&x[control_len + v], ctx)?
                } else {
                    x[at + 3].clone()
                };
                let w = &weight;
                h[0][u][v] = x[at + axes[0]].sub(&x[axes[0]], ctx)?.mul(w, ctx)?;
                h[1][u][v] = x[at + axes[1]].sub(&x[axes[1]], ctx)?.mul(w, ctx)?;
                h[2][u][v] = unit.mul(w, ctx)?;
            }
        }
        let u = h
            .iter()
            .map(|p| derivative(p, 0, ctx))
            .collect::<Result<Vec<_>, _>>()?;
        let v = h
            .iter()
            .map(|p| derivative(p, 1, ctx))
            .collect::<Result<Vec<_>, _>>()?;
        let mut determinant = zeros(3 * (surface.len() - 1), 3 * (surface[0].len() - 1));
        for axis in 0..3 {
            let a = (axis + 1) % 3;
            let b = (axis + 2) % 3;
            // Only minor coefficients whose indices can contribute through H
            // to the requested determinant coefficient/row are needed.
            let degrees = [surface.len() - 1, surface[0].len() - 1];
            let selected = [row, column];
            let window = std::array::from_fn(|axis| {
                selected[axis].map_or([0, 2 * degrees[axis] - 1], |i| {
                    [
                        i.saturating_sub(degrees[axis]),
                        i.min(2 * degrees[axis] - 1),
                    ]
                })
            });
            let minors = difference(
                mul_window(&u[a], &v[b], window, ctx)?,
                mul_window(&u[b], &v[a], window, ctx)?,
                ctx,
            )?;
            let term = mul(&h[axis], &minors, [row, column], ctx)?;
            for (i, row) in term.iter().enumerate() {
                for (j, value) in row.iter().enumerate() {
                    determinant[i][j] = determinant[i][j].add(value, ctx)?;
                }
            }
        }
        let selected = row.map_or(&determinant[..], |r| &determinant[r..r + 1]);
        let signs = selected
            .iter()
            .map(|row| {
                column
                    .map_or(&row[..], |c| &row[c..c + 1])
                    .iter()
                    .map(Expansion::sign)
                    .collect()
            })
            .collect();
        let bounds = if with_bounds {
            Some(
                selected
                    .iter()
                    .map(|row| {
                        column
                            .map_or(&row[..], |c| &row[c..c + 1])
                            .iter()
                            .map(|x| {
                                let b = x.enclosure(ctx)?;
                                Ok([b.lo, b.hi])
                            })
                            .collect::<Result<Vec<_>, Reason>>()
                    })
                    .collect::<Result<Vec<_>, Reason>>()?,
            )
        } else {
            None
        };
        ctx.charge(0)?;
        Ok((signs, bounds))
    })();
    let (signs, coefficient_bounds, reason) = match result {
        Ok((s, b)) => (Some(s), b, None),
        Err(r) => (None, None, Some(r)),
    };
    Ok(Decision {
        signs,
        coefficient_bounds,
        coefficient_scale: None,
        reason,
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}

/// Exact projected Jacobian for separable weights row[u]*column[v]/pivot.
/// A common positive pivot cancels from the rational map. Products remain
/// expansions; no rounded target weight is introduced. The caller validates
/// the positive pivot and clamped Bezier layout. A selected coefficient is
/// only local evidence; whole-chart orientation requires every coefficient.
pub fn factorized_surface_projected_jacobian(
    ctx: &mut PredicateContext<'_>,
    points: &[Vec<[LeafRef; 3]>],
    rows: &[LeafRef],
    columns: &[LeafRef],
    axes: [usize; 2],
    index: Option<[usize; 2]>,
) -> Result<Decision, InputError> {
    factorized_compute(
        ctx,
        points,
        rows,
        columns,
        axes,
        index.map(|i| i[0]),
        index.map(|i| i[1]),
    )
}
/// One exact factorized coefficient row; callers must cover every row.
pub fn factorized_surface_projected_jacobian_row(
    ctx: &mut PredicateContext<'_>,
    points: &[Vec<[LeafRef; 3]>],
    rows: &[LeafRef],
    columns: &[LeafRef],
    axes: [usize; 2],
    row: usize,
) -> Result<Decision, InputError> {
    factorized_compute(ctx, points, rows, columns, axes, Some(row), None)
}
fn factorized_compute(
    ctx: &mut PredicateContext<'_>,
    points: &[Vec<[LeafRef; 3]>],
    rows: &[LeafRef],
    columns: &[LeafRef],
    axes: [usize; 2],
    row: Option<usize>,
    column: Option<usize>,
) -> Result<Decision, InputError> {
    if !(2..=9).contains(&points.len())
        || rows.len() != points.len()
        || !(2..=9).contains(&columns.len())
        || points.iter().any(|r| r.len() != columns.len())
    {
        return Err(InputError::InvalidInput(
            "Invalid factorized projected chart",
        ));
    }
    let surface: Vec<Vec<[LeafRef; 4]>> = points
        .iter()
        .enumerate()
        .map(|(u, row)| row.iter().map(|p| [p[0], p[1], p[2], rows[u]]).collect())
        .collect();
    compute_weights(ctx, &surface, axes, row, column, false, Some(columns))
}

/// All coefficient rows under one bounded predicate context.
pub fn rational_surface_projected_jacobian(
    ctx: &mut PredicateContext<'_>,
    surface: &[Vec<[LeafRef; 4]>],
    axes: [usize; 2],
) -> Result<Decision, InputError> {
    compute(ctx, surface, axes, None, None, false)
}
/// All exact coefficient signs and outward magnitude enclosures under one
/// bounded context. Missing work returns no partial coefficient grid.
pub fn rational_surface_projected_jacobian_bounds(
    ctx: &mut PredicateContext<'_>,
    surface: &[Vec<[LeafRef; 4]>],
    axes: [usize; 2],
) -> Result<Decision, InputError> {
    compute(ctx, surface, axes, None, None, true)
}
/// One original Bernstein coefficient row. The caller must check every row
/// of the same immutable chart before claiming interior orientation.
pub fn rational_surface_projected_jacobian_row(
    ctx: &mut PredicateContext<'_>,
    surface: &[Vec<[LeafRef; 4]>],
    axes: [usize; 2],
    row: usize,
) -> Result<Decision, InputError> {
    compute(ctx, surface, axes, Some(row), None, false)
}
/// One complete coefficient row with outward magnitude enclosures. Callers
/// must cover all rows before treating the result as a whole-chart net.
pub fn rational_surface_projected_jacobian_row_bounds(
    ctx: &mut PredicateContext<'_>,
    surface: &[Vec<[LeafRef; 4]>],
    axes: [usize; 2],
    row: usize,
) -> Result<Decision, InputError> {
    compute(ctx, surface, axes, Some(row), None, true)
}

/// One exact coefficient, for bounded refinement of an exhausted row query.
pub fn rational_surface_projected_jacobian_coefficient(
    ctx: &mut PredicateContext<'_>,
    surface: &[Vec<[LeafRef; 4]>],
    axes: [usize; 2],
    index: [usize; 2],
) -> Result<Decision, InputError> {
    compute(ctx, surface, axes, Some(index[0]), Some(index[1]), false)
}
/// Exact coefficient sign with an outward magnitude enclosure. Identical
/// source inputs and chart degrees share the positive scaling across axes.
pub fn rational_surface_projected_jacobian_coefficient_bounds(
    ctx: &mut PredicateContext<'_>,
    surface: &[Vec<[LeafRef; 4]>],
    axes: [usize; 2],
    index: [usize; 2],
) -> Result<Decision, InputError> {
    compute(ctx, surface, axes, Some(index[0]), Some(index[1]), true)
}

/// Exact Bernstein coefficients of H_axis W - H W_axis for one original
/// coordinate, in a shared positive scale across XYZ. Axis is normalized U/V.
/// Whole requested rows are returned only after all their work is complete.
pub fn rational_surface_derivative_numerator_row_bounds(
    ctx: &mut PredicateContext<'_>,
    surface: &[Vec<[LeafRef; 4]>],
    coordinate: usize,
    axis: usize,
    row: usize,
) -> Result<Decision, InputError> {
    derivative_numerator(ctx, surface, coordinate, axis, row, None, None)
}
pub fn rational_surface_derivative_numerator_coefficient_bounds(
    ctx: &mut PredicateContext<'_>,
    surface: &[Vec<[LeafRef; 4]>],
    coordinate: usize,
    axis: usize,
    index: [usize; 2],
) -> Result<Decision, InputError> {
    derivative_numerator(
        ctx,
        surface,
        coordinate,
        axis,
        index[0],
        Some(index[1]),
        None,
    )
}
/// Exact transverse component Q_k d_a - Q_a d_k, where d is the
/// authored pole direction P_(1,0)-P_(0,0). Zero leading coefficients prove
/// alignment with this fixed source direction; they do not prove wall safety.
pub fn rational_surface_derivative_transverse_row_bounds(
    ctx: &mut PredicateContext<'_>,
    surface: &[Vec<[LeafRef; 4]>],
    coordinate: usize,
    anchor: usize,
    axis: usize,
    row: usize,
) -> Result<Decision, InputError> {
    derivative_numerator(ctx, surface, coordinate, axis, row, None, Some(anchor))
}
pub fn rational_surface_derivative_transverse_coefficient_bounds(
    ctx: &mut PredicateContext<'_>,
    surface: &[Vec<[LeafRef; 4]>],
    coordinate: usize,
    anchor: usize,
    axis: usize,
    index: [usize; 2],
) -> Result<Decision, InputError> {
    derivative_numerator(
        ctx,
        surface,
        coordinate,
        axis,
        index[0],
        Some(index[1]),
        Some(anchor),
    )
}
fn derivative_numerator(
    ctx: &mut PredicateContext<'_>,
    surface: &[Vec<[LeafRef; 4]>],
    coordinate: usize,
    axis: usize,
    row: usize,
    column: Option<usize>,
    projection: Option<usize>,
) -> Result<Decision, InputError> {
    if !(2..=9).contains(&surface.len())
        || !(2..=9).contains(&surface[0].len())
        || surface.iter().any(|r| r.len() != surface[0].len())
        || projection.is_some_and(|a| a > 2 || a == coordinate)
        || coordinate > 2
        || axis > 1
        || row >= 2 * (surface.len() - 1) + usize::from(axis == 1)
        || column.is_some_and(|v| v >= 2 * (surface[0].len() - 1) + usize::from(axis == 0))
    {
        return Err(InputError::InvalidInput(
            "Invalid rational derivative chart",
        ));
    }
    let mut values = surface
        .iter()
        .flatten()
        .flat_map(|p| p.iter())
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    values.push(AuthoredScalar::Binary64Bits(1f64.to_bits()));
    let result = (|| -> Result<(Vec<Vec<Sign>>, Vec<Vec<[f64; 2]>>, [f64; 2]), Reason> {
        ctx.charge(values.len() as u64)?;
        let x = exact_inputs(&values, ctx)?;
        let unit = x.last().unwrap();
        if x[..x.len() - 1]
            .chunks_exact(4)
            .any(|p| p[3].sign() != Sign::Positive)
        {
            return Err(Reason::MissingProof);
        }
        let mut h = zeros(surface.len(), surface[0].len());
        let mut w = h.clone();
        for u in 0..surface.len() {
            for v in 0..surface[0].len() {
                let at = (u * surface[0].len() + v) * 4;
                h[u][v] = x[at + coordinate]
                    .sub(&x[coordinate], ctx)?
                    .mul(&x[at + 3], ctx)?;
                w[u][v] = unit.mul(&x[at + 3], ctx)?;
            }
        }
        let dh = derivative(&h, axis, ctx)?;
        let dw = derivative(&w, axis, ctx)?;
        let mut net = difference(
            mul(&dh, &w, [Some(row), column], ctx)?,
            mul(&h, &dw, [Some(row), column], ctx)?,
            ctx,
        )?;
        if let Some(anchor) = projection {
            let mut ha = h.clone();
            for u in 0..surface.len() {
                for v in 0..surface[0].len() {
                    let at = (u * surface[0].len() + v) * 4;
                    ha[u][v] = x[at + anchor].sub(&x[anchor], ctx)?.mul(&x[at + 3], ctx)?;
                }
            }
            let da = derivative(&ha, axis, ctx)?;
            let qa = difference(
                mul(&da, &w, [Some(row), column], ctx)?,
                mul(&ha, &dw, [Some(row), column], ctx)?,
                ctx,
            )?;
            let at = surface[0].len() * 4;
            let direction_anchor = x[at + anchor].sub(&x[anchor], ctx)?;
            let direction_coordinate = x[at + coordinate].sub(&x[coordinate], ctx)?;
            if direction_anchor.sign() == Sign::Zero {
                return Err(Reason::MissingProof);
            }
            for v in 0..net[row].len() {
                if column.is_some_and(|c| c != v) {
                    continue;
                }
                net[row][v] = net[row][v]
                    .mul(&direction_anchor, ctx)?
                    .sub(&qa[row][v].mul(&direction_coordinate, ctx)?, ctx)?;
            }
        }
        let selected = column.map_or(&net[row][..], |v| &net[row][v..v + 1]);
        let signs = vec![selected.iter().map(Expansion::sign).collect()];
        let bounds = vec![
            selected
                .iter()
                .map(|x| {
                    let b = x.enclosure(ctx)?;
                    Ok([b.lo, b.hi])
                })
                .collect::<Result<Vec<_>, Reason>>()?,
        ];
        let mut scale = Expansion::scalar(1.);
        for _ in 0..if projection.is_some() { 5 } else { 4 } {
            scale = scale.mul(unit, ctx)?;
        }
        let degree_u = 2 * (surface.len() - 1) - usize::from(axis == 0);
        let degree_v = 2 * (surface[0].len() - 1) - usize::from(axis == 1);
        scale = scale
            .mul(&factors(degree_u, ctx)?[0], ctx)?
            .mul(&factors(degree_v, ctx)?[0], ctx)?;
        let scale = scale.enclosure(ctx)?;
        ctx.charge(0)?;
        Ok((signs, bounds, [scale.lo, scale.hi]))
    })();
    let (signs, coefficient_bounds, coefficient_scale, reason) = match result {
        Ok((s, b, scale)) => (Some(s), Some(b), Some(scale), None),
        Err(r) => (None, None, None, Some(r)),
    };
    Ok(Decision {
        signs,
        coefficient_bounds,
        coefficient_scale,
        reason,
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}

#[cfg(test)]
#[path = "projected_surface_jacobian/tests/mod.rs"]
mod tests;

#[path = "projected_surface_jacobian_filter.rs"]
mod interval_filter;
pub use interval_filter::factorized_surface_projected_jacobian_filter;
