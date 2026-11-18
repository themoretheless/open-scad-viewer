//! step v3 export algorithm: periodic_patch_grid_compatible.
use super::*;
pub fn periodic_patch_grid_compatible(
    model: &Model,
    u_count: usize,
    v_count: usize,
) -> bool {
    let compatible = |a: &Surface, ai: usize, aj: usize, b: &Surface, bi: usize, bj: usize| {
        a.control_points[ai][aj]
            .iter()
            .zip(&b.control_points[bi][bj])
            .map(|(x, y)| (x - y) * (x - y))
            .sum::<f64>()
            .sqrt()
            <= 1e-9
            && (a.weights[ai][aj] - b.weights[bi][bj]).abs() <= 1e-12
    };
    for v in 0..v_count {
        for u in 0..u_count {
            let a = &model.faces[v * u_count + u].surface;
            let b = &model.faces[v * u_count + (u + 1) % u_count].surface;
            if a.control_points.len() != 3
                || b.control_points.len() != 3
                || a.control_points[0].len() != b.control_points[0].len()
            {
                return false;
            }
            for j in 0..a.control_points[0].len() {
                if !compatible(a, 2, j, b, 0, j) {
                    return false;
                }
            }
            if v_count > 1 {
                let b = &model.faces[((v + 1) % v_count) * u_count + u].surface;
                if a.control_points[0].len() != 3 || b.control_points[0].len() != 3 {
                    return false;
                }
                for i in 0..3 {
                    if !compatible(a, i, 2, b, i, 0) {
                        return false;
                    }
                }
            }
        }
    }
    true
}
