//! Original cover: preserves the original arithmetic and proof budgets.
use super::*;

pub(super) fn certify_original_cover_with_projection(
    s: &Surface,
    mut cover: Vec<([usize; 2], [[f64; 2]; 2])>,
    mut report: Report,
    max_spans: usize,
    max_depth: usize,
    success_reason: &'static str,
    allow_oblique: bool,
    column_scale: [f64; 2],
) -> Result<(Report, Option<[[f64; 3]; 2]>, [f64; 2])> {
    let mut projection_matrix = None;
    let mut proven_scale = column_scale;
    for depth in 0..=max_depth {
        let mut section_jacobians = Vec::new();
        let mut j = [[I {
            lo: f64::INFINITY,
            hi: f64::NEG_INFINITY,
        }; 2]; 3];
        for &(span, section) in &cover {
            if report.spans == max_spans {
                report.reason = "work-limit";
                return Ok((report, projection_matrix, proven_scale));
            }
            let next = section_jacobian(s, span, section)?;
            if allow_oblique {
                let mut local = next;
                for k in 0..3 {
                    for axis in 0..2 {
                        if column_scale[axis] != 1. {
                            local[k][axis] = local[k][axis].mul(I::point(column_scale[axis]))?;
                        }
                    }
                }
                section_jacobians.push(local);
            }
            report.spans += 1;
            for k in 0..3 {
                for axis in 0..2 {
                    j[k][axis] = I {
                        lo: j[k][axis].lo.min(next[k][axis].lo),
                        hi: j[k][axis].hi.max(next[k][axis].hi),
                    };
                }
            }
        }
        for world in 0..3 {
            for axis in 0..2 {
                if column_scale[axis] != 1. {
                    j[world][axis] = j[world][axis].mul(I::point(column_scale[axis]))?;
                }
            }
        }
        for axes in [[0, 1], [0, 2], [1, 2]] {
            if let Some(q) = contraction([j[axes[0]], j[axes[1]]])? {
                if q < 1. {
                    projection_matrix = Some(std::array::from_fn(|row| {
                        std::array::from_fn(|world| if axes[row] == world { 1. } else { 0. })
                    }));
                    report.proven = true;
                    report.projection = Some(axes);
                    report.contraction_upper = Some(q);
                    report.reason = success_reason;
                    return Ok((report, projection_matrix, proven_scale));
                }
            }
        }
        if allow_oblique {
            for nx in -1i32..=1 {
                for ny in -1i32..=1 {
                    for nz in -1i32..=1 {
                        let n = [nx, ny, nz];
                        let Some(pivot) = n.iter().position(|v| *v != 0) else {
                            continue;
                        };
                        if n[pivot] != 1 || n.iter().filter(|v| **v != 0).count() < 2 {
                            continue;
                        }
                        let other = (0..3).filter(|k| *k != pivot).collect::<Vec<_>>();
                        let matrix: [[f64; 3]; 2] = std::array::from_fn(|row| {
                            std::array::from_fn(|k| {
                                if k == pivot {
                                    f64::from(n[other[row]])
                                } else if k == other[row] {
                                    -1.
                                } else {
                                    0.
                                }
                            })
                        });
                        let mut projected = [[I {
                            lo: f64::INFINITY,
                            hi: f64::NEG_INFINITY,
                        }; 2]; 2];
                        for local in &section_jacobians {
                            for row in 0..2 {
                                for axis in 0..2 {
                                    let mut value = I::point(0.);
                                    for k in 0..3 {
                                        if matrix[row][k] != 0. {
                                            value = value.add(
                                                local[k][axis].mul(I::point(matrix[row][k]))?,
                                            )?;
                                        }
                                    }
                                    projected[row][axis].lo = projected[row][axis].lo.min(value.lo);
                                    projected[row][axis].hi = projected[row][axis].hi.max(value.hi);
                                }
                            }
                        }
                        for extra in [1., 0.5, 2., 0.25, 4., 0.125, 8.] {
                            let effective = column_scale[0] * extra;
                            if !effective.is_finite()
                                || effective <= 0.
                                || crate::exact_products::sum_sign(&[
                                    (column_scale[0], extra),
                                    (-effective, 1.),
                                ]) != std::cmp::Ordering::Equal
                            {
                                continue;
                            }
                            let mut weighted = projected;
                            if extra != 1. {
                                for row in 0..2 {
                                    weighted[row][0] = weighted[row][0].mul(I::point(extra))?;
                                }
                            }
                            if let Some(q) = contraction(weighted)? {
                                if q < 1. {
                                    report.proven = true;
                                    report.projection = None;
                                    report.contraction_upper = Some(q);
                                    report.reason = success_reason;
                                    projection_matrix = Some(matrix);
                                    proven_scale = [effective, column_scale[1]];
                                    return Ok((report, projection_matrix, proven_scale));
                                }
                            }
                        }
                    }
                }
            }
        }
        if depth == max_depth {
            break;
        }
        if cover.len() * 4 > max_spans - report.spans {
            report.reason = "work-limit";
            return Ok((report, projection_matrix, proven_scale));
        }
        let mut refined = Vec::with_capacity(cover.len() * 4);
        for (span, section) in cover {
            let mid = section.map(|r| 0.5 * r[0] + 0.5 * r[1]);
            if (0..2).any(|a| mid[a] <= section[a][0] || mid[a] >= section[a][1]) {
                report.reason = "subdivision-limit";
                return Ok((report, projection_matrix, proven_scale));
            }
            for u in [[section[0][0], mid[0]], [mid[0], section[0][1]]] {
                for v in [[section[1][0], mid[1]], [mid[1], section[1][1]]] {
                    refined.push((span, [u, v]));
                }
            }
        }
        cover = refined;
    }
    Ok((report, projection_matrix, proven_scale))
}
