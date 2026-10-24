//! Exact Bernstein signs of a rational chart against a bilinear XYZ graph.
//! Graph corners are authored leaves in order 00,10,01,11. The caller checks
//! their rectangular XY layout; no rounded fitted coefficient is a premise.
use crate::projected_surface_jacobian::{Decision, mul};
use crate::{
    Algebra, AuthoredScalar, Expansion, InputError, LeafRef, PredicateContext, Reason, Sign,
    exact_inputs,
};
/// Signs of W² times the graph equation, up to a common positive integer.
/// Positive weights, Bezier layout and the ownership of any zero row are
/// separate caller obligations. Exhaustion returns no signs.
pub fn rational_bilinear_graph_side(
    ctx: &mut PredicateContext<'_>,
    surface: &[Vec<[LeafRef; 4]>],
    corners: [[LeafRef; 3]; 4],
    axes: [usize; 3],
) -> Result<Decision, InputError> {
    if !(2..=5).contains(&surface.len())
        || !(2..=5).contains(&surface[0].len())
        || surface.iter().any(|r| r.len() != surface[0].len())
        || axes.iter().any(|&k| k > 2)
        || axes[0] == axes[1]
        || axes[0] == axes[2]
        || axes[1] == axes[2]
    {
        return Err(InputError::InvalidInput("Invalid bilinear graph request"));
    }
    let values = surface
        .iter()
        .flatten()
        .flatten()
        .chain(corners.iter().flatten())
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    let result = (|| -> Result<Vec<Vec<Sign>>, Reason> {
        ctx.charge(values.len() as u64)?;
        let v = exact_inputs(&values, ctx)?;
        let count = surface.len() * surface[0].len() * 4;
        let c = |i: usize, k: usize| &v[count + i * 3 + axes[k]];
        if c(0, 0).sub(c(2, 0), ctx)?.sign() != Sign::Zero
            || c(1, 0).sub(c(3, 0), ctx)?.sign() != Sign::Zero
            || c(0, 1).sub(c(1, 1), ctx)?.sign() != Sign::Zero
            || c(2, 1).sub(c(3, 1), ctx)?.sign() != Sign::Zero
        {
            return Err(Reason::MissingProof);
        }
        let dx = c(1, 0).sub(c(0, 0), ctx)?;
        let dy = c(2, 1).sub(c(0, 1), ctx)?;
        if dx.sign() == Sign::Zero || dy.sign() == Sign::Zero {
            return Err(Reason::MissingProof);
        }
        let z0 = c(0, 2);
        let zx = c(1, 2).sub(z0, ctx)?;
        let zy = c(2, 2).sub(z0, ctx)?;
        let mixed = c(3, 2).sub(c(1, 2), ctx)?.sub(c(2, 2), ctx)?.add(z0, ctx)?;
        let mut h = vec![vec![vec![Expansion::scalar(0.); surface[0].len()]; surface.len()]; 4];
        for i in 0..surface.len() {
            for j in 0..surface[0].len() {
                let p = &v[(i * surface[0].len() + j) * 4..];
                let w = &p[3];
                if w.sign() != Sign::Positive {
                    return Err(Reason::MissingProof);
                }
                for k in 0..3 {
                    h[k][i][j] = p[axes[k]].sub(c(0, k), ctx)?.mul(w, ctx)?;
                }
                h[3][i][j] = w.clone();
            }
        }
        let zw = mul(&h[2], &h[3], [None, None], ctx)?;
        let xw = mul(&h[0], &h[3], [None, None], ctx)?;
        let yw = mul(&h[1], &h[3], [None, None], ctx)?;
        let xy = mul(&h[0], &h[1], [None, None], ctx)?;
        let scale = dx.mul(&dy, ctx)?;
        let sx = zx.mul(&dy, ctx)?;
        let sy = zy.mul(&dx, ctx)?;
        let mut out = vec![vec![Sign::Zero; zw[0].len()]; zw.len()];
        for i in 0..out.len() {
            for j in 0..out[0].len() {
                out[i][j] = zw[i][j]
                    .mul(&scale, ctx)?
                    .sub(&xw[i][j].mul(&sx, ctx)?, ctx)?
                    .sub(&yw[i][j].mul(&sy, ctx)?, ctx)?
                    .sub(&xy[i][j].mul(&mixed, ctx)?, ctx)?
                    .sign();
            }
        }
        ctx.charge(0)?;
        Ok(out)
    })();
    let (signs, reason) = match result {
        Ok(s) => (Some(s), None),
        Err(r) => (None, Some(r)),
    };
    Ok(Decision {
        signs,
        coefficient_bounds: None,
        coefficient_scale: None,
        reason,
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Limits, SourceArena, ToleranceContext};
    fn check(p: Vec<Vec<[f64; 4]>>, work: u64) -> Decision {
        let corners = [[0., 0., 0.], [2., 0., 0.], [0., 3., 0.], [2., 3., 6.]];
        let count = p.len() * p[0].len() * 4;
        let values = p
            .iter()
            .flatten()
            .flatten()
            .chain(corners.iter().flatten())
            .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect();
        let arena = SourceArena::authored("independent-graph-side-fixture", 1, values).unwrap();
        let refs = p
            .iter()
            .enumerate()
            .map(|(i, r)| {
                r.iter()
                    .enumerate()
                    .map(|(j, _)| {
                        std::array::from_fn(|k| arena.leaf(4 * (i * r.len() + j) + k).unwrap())
                    })
                    .collect()
            })
            .collect::<Vec<_>>();
        let cr = std::array::from_fn(|i| {
            std::array::from_fn(|k| arena.leaf(count + 3 * i + k).unwrap())
        });
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
        rational_bilinear_graph_side(&mut ctx, &refs, cr, [0, 1, 2]).unwrap()
    }
    #[test]
    fn exact_weighted_graph_and_strict_wall_are_distinct_from_fit_and_samples() {
        let graph = vec![
            vec![[0., 0., 0., 1.], [0., 3., 0., 2.]],
            vec![[2., 0., 0., 2.], [2., 3., 6., 4.]],
        ];
        assert!(
            check(graph.clone(), 100000)
                .signs
                .unwrap()
                .iter()
                .flatten()
                .all(|s| *s == Sign::Zero)
        );
        let wall = vec![
            vec![[1., 0., 0., 1.], [1., 0., 5., 1.]],
            vec![[1., 3., 3., 2.], [1., 3., 8., 2.]],
        ];
        let signs = check(wall, 100000).signs.unwrap();
        assert!(
            signs
                .iter()
                .all(|r| r[0] == Sign::Zero && r[2] == Sign::Positive)
        );
        let mut forged = graph.clone();
        forged[1][1][3] = 5.;
        assert!(
            check(forged, 100000)
                .signs
                .unwrap()
                .iter()
                .flatten()
                .any(|s| *s != Sign::Zero)
        );
        assert!(check(graph, 1).signs.is_none());
    }
    #[test]
    fn crossing_wall_has_mixed_signs_instead_of_boundary_permission() {
        let wall = vec![
            vec![[1., 0., 0., 1.], [1., 0., -5., 1.]],
            vec![[1., 3., 3., 2.], [1., 3., 8., 2.]],
        ];
        let signs = check(wall, 100000).signs.unwrap();
        assert!(signs.iter().flatten().any(|s| *s == Sign::Positive));
        assert!(signs.iter().flatten().any(|s| *s == Sign::Negative));
    }
}
