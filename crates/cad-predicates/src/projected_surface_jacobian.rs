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
                    if selected_cell[0].is_some_and(|row| i + k != row)
                        || selected_cell[1].is_some_and(|column| j + l != column)
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
        let x = exact_inputs(&values, ctx)?;
        let unit = x.last().unwrap();
        let control_len = surface.len() * surface[0].len() * 4;
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
            let minors = difference(
                mul(&u[a], &v[b], [None, None], ctx)?,
                mul(&u[b], &v[a], [None, None], ctx)?,
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
mod tests {
    use super::*;
    use crate::{Limits, SourceArena, ToleranceContext};
    #[test]
    fn transverse_derivative_retains_small_leading_tangent_and_detects_real_transverse_term() {
        for perturbation in [0., 1e-12] {
            let points: Vec<Vec<[f64; 4]>> = (0..3)
                .map(|u| {
                    (0..2)
                        .map(|v| {
                            [
                                u as f64 / 2.,
                                if u == 2 {
                                    v as f64
                                } else if u == 1 && v == 1 {
                                    perturbation
                                } else {
                                    0.
                                },
                                0.,
                                if u == 1 && v == 1 { 3. } else { (v + 1) as f64 },
                            ]
                        })
                        .collect()
                })
                .collect();
            let arena = SourceArena::authored(
                "original-transverse-derivative",
                1,
                points
                    .iter()
                    .flatten()
                    .flatten()
                    .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
                    .collect(),
            )
            .unwrap();
            let leaves = (0..3)
                .map(|u| {
                    (0..2)
                        .map(|v| std::array::from_fn(|k| arena.leaf((u * 2 + v) * 4 + k).unwrap()))
                        .collect()
                })
                .collect::<Vec<_>>();
            let tolerance = ToleranceContext::default_valid();
            let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
            let tangent =
                rational_surface_derivative_numerator_row_bounds(&mut ctx, &leaves, 0, 1, 1)
                    .unwrap();
            assert!(tangent.signs.unwrap()[0].iter().any(|s| *s != Sign::Zero));
            let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
            let d =
                rational_surface_derivative_transverse_row_bounds(&mut ctx, &leaves, 1, 0, 1, 1)
                    .unwrap();
            let signs = d.signs.unwrap();
            let bounds = d.coefficient_bounds.unwrap();
            let scale = d.coefficient_scale.unwrap();
            assert!(scale[0] > 0.);
            assert_eq!(
                signs[0].iter().all(|s| *s == Sign::Zero),
                perturbation == 0.
            );
            let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
            let du =
                rational_surface_derivative_transverse_row_bounds(&mut ctx, &leaves, 1, 0, 0, 0)
                    .unwrap();
            assert_eq!(
                du.signs.unwrap()[0].iter().all(|s| *s == Sign::Zero),
                perturbation == 0.
            );
            let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
            let endpoint =
                rational_surface_derivative_transverse_row_bounds(&mut ctx, &leaves, 1, 0, 1, 4)
                    .unwrap();
            let scale = endpoint.coefficient_scale.unwrap();
            assert!(
                endpoint.coefficient_bounds.unwrap()[0]
                    .iter()
                    .all(|b| b[0] <= scale[1] && b[1] >= scale[0]),
                "source transverse endpoint coefficient is exactly one"
            );
            for column in 0..2 {
                let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
                let cell = rational_surface_derivative_transverse_coefficient_bounds(
                    &mut ctx,
                    &leaves,
                    1,
                    0,
                    1,
                    [1, column],
                )
                .unwrap();
                assert_eq!(cell.signs.unwrap()[0][0], signs[0][column]);
                assert_eq!(cell.coefficient_bounds.unwrap()[0][0], bounds[0][column]);
            }
            let mut ctx = PredicateContext::new(
                &arena,
                &tolerance,
                Limits {
                    max_work: 1,
                    ..Limits::default()
                },
                None,
            );
            let d =
                rational_surface_derivative_transverse_row_bounds(&mut ctx, &leaves, 1, 0, 1, 1)
                    .unwrap();
            assert!(d.signs.is_none() && d.coefficient_bounds.is_none());
            let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
            let d =
                rational_surface_derivative_transverse_row_bounds(&mut ctx, &leaves, 0, 2, 1, 1)
                    .unwrap();
            assert!(d.signs.is_none() && d.coefficient_bounds.is_none());
        }
    }
    #[test]
    fn rational_derivative_rows_preserve_quadratic_factor_and_refuse_partial_work() {
        // S=(u,2u²v/(1+v),0): V weights vary, but Q_v=(0,2u²,0).
        let points: Vec<Vec<[f64; 4]>> = (0..3)
            .map(|u| {
                (0..2)
                    .map(|v| {
                        [
                            u as f64 / 2.,
                            if u == 2 { v as f64 } else { 0. },
                            0.,
                            (v + 1) as f64,
                        ]
                    })
                    .collect()
            })
            .collect();
        let arena = SourceArena::authored(
            "rational-derivative-factor",
            1,
            points
                .iter()
                .flatten()
                .flatten()
                .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
                .collect(),
        )
        .unwrap();
        let leaves = (0..3)
            .map(|u| {
                (0..2)
                    .map(|v| std::array::from_fn(|k| arena.leaf((u * 2 + v) * 4 + k).unwrap()))
                    .collect()
            })
            .collect::<Vec<_>>();
        let tolerance = ToleranceContext::default_valid();
        for coordinate in 0..3 {
            for row in 0..5 {
                let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
                let d = rational_surface_derivative_numerator_row_bounds(
                    &mut ctx, &leaves, coordinate, 1, row,
                )
                .unwrap();
                let signs = d.signs.unwrap();
                let bounds = d.coefficient_bounds.unwrap();
                let scale = d.coefficient_scale.unwrap();
                assert!(scale[0] > 0.);
                if coordinate == 1 && row == 4 {
                    assert!(
                        bounds[0]
                            .iter()
                            .all(|b| b[0] <= 2. * scale[1] && b[1] >= 2. * scale[0])
                    );
                }
                assert_eq!(bounds.len(), 1);
                assert_eq!(bounds[0].len(), 2);
                assert!(signs[0].iter().all(|s| *s
                    == if coordinate == 1 && row >= 2 {
                        Sign::Positive
                    } else {
                        Sign::Zero
                    }));
                for column in 0..2 {
                    let mut ctx =
                        PredicateContext::new(&arena, &tolerance, Limits::default(), None);
                    let cell = rational_surface_derivative_numerator_coefficient_bounds(
                        &mut ctx,
                        &leaves,
                        coordinate,
                        1,
                        [row, column],
                    )
                    .unwrap();
                    assert_eq!(cell.signs.unwrap()[0][0], signs[0][column]);
                    assert_eq!(cell.coefficient_bounds.unwrap()[0][0], bounds[0][column]);
                }
            }
        }
        for row in 0..4 {
            let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
            let d = rational_surface_derivative_numerator_row_bounds(&mut ctx, &leaves, 0, 0, row)
                .unwrap();
            assert_eq!(d.coefficient_bounds.unwrap()[0].len(), 3);
            assert!(d.signs.unwrap()[0].iter().all(|s| *s == Sign::Positive));
        }
        let mut ctx = PredicateContext::new(
            &arena,
            &tolerance,
            Limits {
                max_work: 1,
                ..Limits::default()
            },
            None,
        );
        let d =
            rational_surface_derivative_numerator_row_bounds(&mut ctx, &leaves, 1, 1, 2).unwrap();
        assert!(d.signs.is_none() && d.coefficient_bounds.is_none() && d.work_used <= 1);
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        assert!(
            rational_surface_derivative_numerator_row_bounds(&mut ctx, &leaves, 1, 1, 5).is_err()
        );
    }

    #[test]
    fn exact_normal_component_bounds_preserve_common_scale_and_refuse_exhaustion() {
        let points = [
            [[0., 0., 0., 1.], [0., 1., 3., 1.]],
            [[1., 0., 2., 1.], [1., 1., 5., 1.]],
        ];
        let arena = SourceArena::authored(
            "normal-bounds-plane",
            1,
            points
                .iter()
                .flatten()
                .flatten()
                .map(|x: &f64| AuthoredScalar::Binary64Bits(x.to_bits()))
                .collect(),
        )
        .unwrap();
        let leaves = (0..2)
            .map(|u| {
                (0..2)
                    .map(|v| std::array::from_fn(|k| arena.leaf((u * 2 + v) * 4 + k).unwrap()))
                    .collect()
            })
            .collect::<Vec<_>>();
        let tolerance = ToleranceContext::default_valid();
        let mut bounds = Vec::new();
        for axes in [[1, 2], [2, 0], [0, 1]] {
            let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
            let d = rational_surface_projected_jacobian_coefficient_bounds(
                &mut ctx,
                &leaves,
                axes,
                [0, 0],
            )
            .unwrap();
            let selected = d.coefficient_bounds.unwrap()[0][0];
            let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
            let whole =
                rational_surface_projected_jacobian_bounds(&mut ctx, &leaves, axes).unwrap();
            let net = whole.coefficient_bounds.unwrap();
            assert_eq!(net.len(), 3);
            assert!(net.iter().all(|row| row.len() == 3));
            assert_eq!(net[0][0], selected);
            let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
            let row =
                rational_surface_projected_jacobian_row_bounds(&mut ctx, &leaves, axes, 0).unwrap();
            assert_eq!(row.coefficient_bounds.unwrap(), vec![net[0].clone()]);
            assert!(whole.signs.unwrap().iter().flatten().all(|s| *s
                == if axes == [0, 1] {
                    Sign::Positive
                } else {
                    Sign::Negative
                }));
            bounds.push(selected);
        }
        assert!(bounds[2][0] > 0.);
        for (axis, ratio) in [(0, 2.), (1, 3.)] {
            assert!(bounds[axis][0] + ratio * bounds[2][0] <= 0.);
            assert!(bounds[axis][1] + ratio * bounds[2][1] >= 0.);
        }
        let mut ctx = PredicateContext::new(
            &arena,
            &tolerance,
            Limits {
                max_work: 1,
                ..Limits::default()
            },
            None,
        );
        let d = rational_surface_projected_jacobian_coefficient_bounds(
            &mut ctx,
            &leaves,
            [0, 1],
            [0, 0],
        )
        .unwrap();
        assert!(d.signs.is_none() && d.coefficient_bounds.is_none());
        let mut ctx = PredicateContext::new(
            &arena,
            &tolerance,
            Limits {
                max_work: 1,
                ..Limits::default()
            },
            None,
        );
        let d = rational_surface_projected_jacobian_bounds(&mut ctx, &leaves, [0, 1]).unwrap();
        assert!(d.signs.is_none() && d.coefficient_bounds.is_none());
    }

    fn check(points: Vec<Vec<[f64; 4]>>, axes: [usize; 2], work: u64) -> Decision {
        check_row(points, axes, work, None)
    }
    fn check_row(
        points: Vec<Vec<[f64; 4]>>,
        axes: [usize; 2],
        work: u64,
        row: Option<usize>,
    ) -> Decision {
        check_cell(points, axes, work, row, None)
    }
    fn check_cell(
        points: Vec<Vec<[f64; 4]>>,
        axes: [usize; 2],
        work: u64,
        row: Option<usize>,
        column: Option<usize>,
    ) -> Decision {
        let arena = SourceArena::authored(
            "projected-jacobian-test",
            1,
            points
                .iter()
                .flatten()
                .flatten()
                .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
                .collect(),
        )
        .unwrap();
        let leaves = points
            .iter()
            .enumerate()
            .map(|(i, row)| {
                row.iter()
                    .enumerate()
                    .map(|(j, _)| {
                        std::array::from_fn(|a| arena.leaf((i * row.len() + j) * 4 + a).unwrap())
                    })
                    .collect()
            })
            .collect::<Vec<_>>();
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(
            &arena,
            &tolerance,
            Limits {
                max_work: work,
                ..Limits::default()
            },
            None,
        );
        match (row, column) {
            (Some(row), Some(column)) => rational_surface_projected_jacobian_coefficient(
                &mut ctx,
                &leaves,
                axes,
                [row, column],
            )
            .unwrap(),
            (Some(row), None) => {
                rational_surface_projected_jacobian_row(&mut ctx, &leaves, axes, row).unwrap()
            }
            (None, None) => rational_surface_projected_jacobian(&mut ctx, &leaves, axes).unwrap(),
            _ => unreachable!(),
        }
    }
    fn has_orientation(d: &Decision, wanted: Sign) -> bool {
        d.signs.as_ref().is_some_and(|rows| {
            rows.iter()
                .flatten()
                .all(|s| *s == Sign::Zero || *s == wanted)
                && rows.iter().flatten().any(|s| *s == wanted)
        })
    }
    fn factorized_check(
        points: Vec<Vec<[f64; 3]>>,
        rows: [f64; 2],
        columns: [f64; 2],
        axes: [usize; 2],
        work: u64,
        index: Option<[usize; 2]>,
    ) -> Decision {
        let values: Vec<_> = points
            .iter()
            .flatten()
            .flatten()
            .copied()
            .chain(rows)
            .chain(columns)
            .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect();
        let arena = SourceArena::authored("factorized-jacobian", 1, values).unwrap();
        let points: Vec<Vec<_>> = points
            .iter()
            .enumerate()
            .map(|(u, row)| {
                row.iter()
                    .enumerate()
                    .map(|(v, _)| std::array::from_fn(|a| arena.leaf((u * 2 + v) * 3 + a).unwrap()))
                    .collect()
            })
            .collect();
        let rows = [arena.leaf(12).unwrap(), arena.leaf(13).unwrap()];
        let columns = [arena.leaf(14).unwrap(), arena.leaf(15).unwrap()];
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(
            &arena,
            &tolerance,
            Limits {
                max_work: work,
                ..Limits::default()
            },
            None,
        );
        factorized_surface_projected_jacobian(&mut ctx, &points, &rows, &columns, axes, index)
            .unwrap()
    }
    #[test]
    fn factorized_weights_preserve_exact_orientation_and_refuse_partial_work() {
        let points = vec![
            vec![[0., 0., 0.], [0., 1., 0.]],
            vec![[1., 0., 0.], [1., 1., 0.]],
        ];
        // 1.1*1.1 is not an exact binary64 weight; this path never flattens it.
        let full = factorized_check(
            points.clone(),
            [1., 1.1],
            [1., 1.1],
            [0, 1],
            1_000_000,
            None,
        );
        assert!(has_orientation(&full, Sign::Positive));
        let signs = full.signs.unwrap();
        for (u, row) in signs.iter().enumerate() {
            for (v, sign) in row.iter().enumerate() {
                let cell = factorized_check(
                    points.clone(),
                    [1., 1.1],
                    [1., 1.1],
                    [0, 1],
                    1_000_000,
                    Some([u, v]),
                );
                assert_eq!(cell.signs.unwrap(), vec![vec![*sign]]);
            }
        }
        assert!(has_orientation(
            &factorized_check(
                points.clone(),
                [1., 1.1],
                [1., 1.1],
                [1, 0],
                1_000_000,
                None
            ),
            Sign::Negative
        ));
        for work in [0, 1, 16] {
            let denied = factorized_check(points.clone(), [1., 1.1], [1., 1.1], [0, 1], work, None);
            assert!(denied.signs.is_none());
            assert!(denied.reason.is_some());
        }
        let denied = factorized_check(points.clone(), [1., -1.], [1., 1.], [0, 1], 1_000_000, None);
        assert!(denied.signs.is_none());
        let folded = vec![
            vec![[0., 0., 0.], [1., 1., 0.]],
            vec![[1., 0., 0.], [0., 1., 0.]],
        ];
        let d = factorized_check(folded, [1., 1.1], [1., 1.1], [0, 1], 1_000_000, None);
        let signs = d.signs.unwrap();
        assert!(signs.iter().flatten().any(|s| *s == Sign::Positive));
        assert!(signs.iter().flatten().any(|s| *s == Sign::Negative));
    }
    #[test]
    fn independently_bounded_rows_match_whole_original_chart() {
        let p = vec![
            vec![[0., 0., 0., 1.], [0., 1., 0., 2.], [0., 2., 0., 1.]],
            vec![[0.5, -1., 0., 2.], [0.5, 2., 0., 1.], [0.5, 1., 0., 2.]],
            vec![[1., 0., 0., 1.], [1., 1., 0., 2.], [1., 2., 0., 1.]],
        ];
        let whole = check(p.clone(), [0, 1], 1_000_000).signs.unwrap();
        for (row, expected) in whole.into_iter().enumerate() {
            assert_eq!(
                check_row(p.clone(), [0, 1], 1_000_000, Some(row))
                    .signs
                    .unwrap(),
                vec![expected.clone()]
            );
            for (column, sign) in expected.into_iter().enumerate() {
                assert_eq!(
                    check_cell(p.clone(), [0, 1], 1_000_000, Some(row), Some(column))
                        .signs
                        .unwrap(),
                    vec![vec![sign]]
                );
            }
        }
    }
    #[test]
    fn plane_orientation_and_axis_reversal() {
        let p = vec![
            vec![[0., 0., 0., 1.], [0., 1., 0., 1.]],
            vec![[1., 0., 0., 1.], [1., 1., 0., 1.]],
        ];
        assert!(has_orientation(
            &check(p.clone(), [0, 1], 1_000_000),
            Sign::Positive
        ));
        assert!(has_orientation(
            &check(p, [1, 0], 1_000_000),
            Sign::Negative
        ));
    }
    #[test]
    fn collapsed_boundary_and_flat_endpoint_preserve_interior_sign() {
        // (u, u*(2v-v²)): Jacobian 2u*(1-v), zero only on chart boundaries.
        let p = vec![
            vec![[0., 0., 0., 1.]; 3],
            vec![[1., 0., 0., 1.], [1., 1., 0., 1.], [1., 1., 0., 1.]],
        ];
        let d = check(p, [0, 1], 1_000_000);
        assert!(has_orientation(&d, Sign::Positive));
        assert!(d.signs.unwrap().iter().flatten().any(|s| *s == Sign::Zero));
    }
    #[test]
    fn interior_fold_has_both_signs() {
        // (u, u*(v-v²)): orientation reverses at v=1/2.
        let p = vec![
            vec![[0., 0., 0., 1.]; 3],
            vec![[1., 0., 0., 1.], [1., 0.5, 0., 1.], [1., 0., 0., 1.]],
        ];
        let d = check(p, [0, 1], 1_000_000);
        let signs = d.signs.unwrap();
        assert!(signs.iter().flatten().any(|s| *s == Sign::Positive));
        assert!(signs.iter().flatten().any(|s| *s == Sign::Negative));
    }
    #[test]
    fn rational_weights_and_work_refusal() {
        let p = vec![
            vec![[0., 0., 0., 1.], [0., 1., 0., 2.]],
            vec![[1., 0., 0., 1.], [1., 1., 0., 2.]],
        ];
        assert!(has_orientation(
            &check(p.clone(), [0, 1], 1_000_000),
            Sign::Positive
        ));
        let d = check(p, [0, 1], 1);
        assert!(d.signs.is_none());
        assert!(d.reason.is_some());
    }
    #[test]
    fn rank_deficient_projection_has_no_strict_sign() {
        let p = vec![
            vec![[0., 0., 0., 1.], [0., 0., 1., 1.]],
            vec![[1., 0., 0., 1.], [1., 0., 1., 1.]],
        ];
        let d = check(p, [0, 1], 1_000_000);
        assert!(d.signs.unwrap().iter().flatten().all(|s| *s == Sign::Zero));
    }
}
