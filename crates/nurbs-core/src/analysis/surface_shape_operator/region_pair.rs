//! Original-rectangle comparison with one shared shape-span allowance.
use super::*;
#[derive(Clone, Copy, Debug)]
pub struct ShapeDifference {
    /// Euclidean distance of oriented unit normals.
    pub normal: [f64; 2],
    /// Frobenius distance of signed world-coordinate curvature tensors.
    pub tensor: [f64; 2],
}
pub struct RegionPairReport {
    /// Same and opposite orientation. Each sign applies to normal AND tensor.
    pub bounds: Option<[ShapeDifference; 2]>,
    pub cells: usize,
}
/// Bounds all pairs in both original rectangles, including their closures.
/// Does not establish positional correspondence between the two surfaces.
pub fn compare_rectangles(
    surfaces: [&Surface; 2],
    query: [[[f64; 2]; 2]; 2],
    max_cells: usize,
) -> Result<RegionPairReport> {
    crate::check(
        max_cells <= 100000,
        "Shape comparison needs bounded shared work",
    )?;
    let a = rectangle_bounds(surfaces[0], query[0], max_cells)?;
    let b = rectangle_bounds(surfaces[1], query[1], max_cells - a.cells)?;
    let mut out = RegionPairReport {
        bounds: None,
        cells: a.cells + b.cells,
    };
    let (Some(an), Some(at), Some(bn), Some(bt)) = (
        a.shape.normal,
        a.shape.tensor,
        b.shape.normal,
        b.shape.tensor,
    ) else {
        return Ok(out);
    };
    let values = |n: [[f64; 2]; 3], t: [[[f64; 2]; 3]; 3]| -> Result<[Vec<I>; 2]> {
        Ok([
            n.into_iter()
                .map(|x| I::new(x[0], x[1]))
                .collect::<Result<_>>()?,
            t.into_iter()
                .flatten()
                .map(|x| I::new(x[0], x[1]))
                .collect::<Result<_>>()?,
        ])
    };
    let a = values(an, at)?;
    let b = values(bn, bt)?;
    let mut bounds = [ShapeDifference {
        normal: [0.; 2],
        tensor: [0.; 2],
    }; 2];
    for sign in 0..2 {
        let mut differences = [[0.; 2]; 2];
        for component in 0..2 {
            let other = b[component]
                .iter()
                .map(|x| {
                    if sign == 0 {
                        Ok(*x)
                    } else {
                        I::new(-x.hi, -x.lo)
                    }
                })
                .collect::<Result<Vec<_>>>()?;
            let (lo, hi) = box_distance(&a[component], &other)?;
            differences[component] = [lo, if component == 0 { hi.min(2.) } else { hi }];
        }
        bounds[sign] = ShapeDifference {
            normal: differences[0],
            tensor: differences[1],
        };
    }
    out.bounds = Some(bounds);
    Ok(out)
}
