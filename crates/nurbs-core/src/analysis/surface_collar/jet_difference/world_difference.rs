use super::jet_difference::{Coupled, norm, shape};
use super::*;
/// Coupled original world jets in one common parameter basis. The caller must
/// enclose base and actual-base on the SAME original rectangle; these arrays
/// alone supply neither positional correspondence nor trim ownership.
pub(crate) fn world_difference_bounds(
    base: &crate::surface_measure::jets::Jets,
    delta: &crate::surface_measure::jets::Jets,
    max_work: usize,
) -> Result<(Option<[f64; 2]>, usize)> {
    let mut work = Work {
        used: 0,
        max: max_work,
    };
    let mut jets = [[[Coupled::zero(); 3]; 3]; 3];
    for (u, v) in [(0, 0), (1, 0), (0, 1), (2, 0), (1, 1), (0, 2)] {
        for k in 0..3 {
            jets[u][v][k] = Coupled {
                base: base[u][v][k],
                delta: delta[u][v][k],
            };
        }
    }
    let bound = if let Some((n, b)) = shape(&jets, &mut work)? {
        Some([
            norm(&n.iter().map(|x| x.delta).collect::<Vec<_>>(), &mut work)?.hi,
            norm(&b.iter().map(|x| x.delta).collect::<Vec<_>>(), &mut work)?.hi,
        ])
    } else {
        None
    };
    Ok((bound, work.used))
}
