//! Bounded approximate curvature search for NEW interior proposals.
//! Every score is sampled and approximate: only independent original interval
//! fields may grant curvature admission. Boundary two-jets stay literal.
use super::*;
type Score = (f64, f64);
const COARSE: [f64; 5] = [0.125, 0.25, 0.5, 0.75, 0.875];
const GRID: usize = 32;
pub(super) const TILES: usize = 4;

fn curvature_at(s: &Surface, uv: [f64; 2], work: &mut Work) -> Result<f64> {
    work.charge(1)?;
    let a = s.knots_u[s.degree_u];
    let b = s.knots_u[s.degree_u + 1];
    let c = s.knots_v[s.degree_v];
    let d = s.knots_v[s.degree_v + 1];
    let evaluation = s.evaluate(a + (b - a) * uv[0], c + (d - c) * uv[1])?;
    let Some((gaussian, mean)) = evaluation.curvatures() else {
        return Ok(f64::INFINITY);
    };
    let curvature = mean.abs() + (mean * mean - gaussian).max(0.).sqrt();
    Ok(if curvature.is_finite() {
        curvature
    } else {
        f64::INFINITY
    })
}

fn add(score: &mut Score, curvature: f64) {
    score.0 = score.0.max(curvature);
    score.1 += curvature * curvature;
}

pub(super) fn coarse_points() -> impl Iterator<Item = [f64; 2]> {
    COARSE.into_iter().flat_map(|u| COARSE.map(|v| [u, v]))
}

pub(super) fn score_at(
    s: &Surface,
    points: impl IntoIterator<Item = [f64; 2]>,
    work: &mut Work,
) -> Result<Score> {
    let mut score = (0., 0.);
    for uv in points {
        let curvature = curvature_at(s, uv, work)?;
        add(&mut score, curvature);
        if !curvature.is_finite() {
            break;
        }
    }
    Ok(score)
}

pub(super) fn score(s: &Surface, work: &mut Work) -> Result<Score> {
    score_at(s, coarse_points(), work)
}

fn better(a: (f64, f64), b: (f64, f64)) -> bool {
    a.0 < b.0 || (a.0 == b.0 && a.1 < b.1)
}
pub(super) fn propose(
    source: &[Surface],
    sweeps: usize,
    stock: [[f64; 2]; 3],
    work: &mut Work,
) -> Result<Vec<Surface>> {
    require(
        (1..=8).contains(&sweeps)
            && stock
                .iter()
                .all(|b| b[0].is_finite() && b[1].is_finite() && b[0] <= b[1]),
        "Bound interior curvature proposal search",
    )?;
    let mut out = Vec::new();
    for s in source {
        bivariate_fairing::validate(s)?;
        let mut candidate = s.clone();
        let mut best = score(&candidate, work)?;
        let spread = (0..3)
            .map(|k| {
                let low = s
                    .control_points
                    .iter()
                    .flatten()
                    .map(|p| p[k])
                    .fold(f64::INFINITY, f64::min);
                let high = s
                    .control_points
                    .iter()
                    .flatten()
                    .map(|p| p[k])
                    .fold(f64::NEG_INFINITY, f64::max);
                high - low
            })
            .fold(0., f64::max);
        let mut step = (spread / 8.).min(0.1);
        require(step.is_finite(), "Curvature proposal step overflow")?;
        for _ in 0..sweeps {
            best = sweep(&mut candidate, step, stock, best, |s| score(s, work))?;
            step *= 0.5;
        }
        require(
            best.0.is_finite(),
            "Curvature search has no regular sampled candidate",
        )?;
        candidate.validate()?;
        out.push(candidate);
    }
    Ok(out)
}
fn sweep(
    candidate: &mut Surface,
    step: f64,
    stock: [[f64; 2]; 3],
    mut best: Score,
    mut evaluate: impl FnMut(&Surface) -> Result<Score>,
) -> Result<Score> {
    // Only poles beyond the first and last three rows in BOTH directions
    // can move. The original boundary curves and their two-jets stay literal.
    for i in 3..=candidate.degree_u - 3 {
        for j in 3..=candidate.degree_v - 3 {
            for (axis, [lo, hi]) in stock.into_iter().enumerate() {
                let old = candidate.control_points[i][j][axis];
                let mut chosen = old;
                for direction in [-1., 1.] {
                    let value = (old + direction * step).clamp(lo, hi);
                    if value == old {
                        continue;
                    }
                    candidate.control_points[i][j][axis] = value;
                    let trial = evaluate(candidate)?;
                    if better(trial, best) {
                        chosen = value;
                        best = trial;
                    }
                }
                candidate.control_points[i][j][axis] = chosen;
            }
        }
    }
    Ok(best)
}

pub(super) fn scan(s: &Surface, work: &mut Work) -> Result<(Score, [[f64; 2]; TILES * TILES])> {
    let mut score = (0., 0.);
    let mut peaks = [(-1., [0.; 2]); TILES * TILES];
    for i in 1..GRID {
        for j in 1..GRID {
            let uv = [i as f64 / GRID as f64, j as f64 / GRID as f64];
            let curvature = curvature_at(s, uv, work)?;
            add(&mut score, curvature);
            let tile = &mut peaks[(i * TILES / GRID) * TILES + j * TILES / GRID];
            if curvature > tile.0 {
                *tile = (curvature, uv);
            }
        }
    }
    Ok((score, peaks.map(|peak| peak.1)))
}

fn descent_direction(
    source: &Surface,
    points: &[[f64; 2]],
    scale: f64,
    stock: [[f64; 2]; 3],
    work: &mut Work,
) -> Result<Vec<(usize, usize, usize, f64)>> {
    let mut probe = source.clone();
    let mut direction = Vec::new();
    let energy = |s: &Surface, work: &mut Work| -> Result<f64> {
        let mut sum = 0.;
        for &uv in points {
            sum += (curvature_at(s, uv, work)? / scale).powi(4);
        }
        Ok(sum)
    };
    for i in 3..=source.degree_u - 3 {
        for j in 3..=source.degree_v - 3 {
            for (axis, [lo, hi]) in stock.into_iter().enumerate() {
                let original = source.control_points[i][j][axis];
                let lower = (original - 1e-5).max(lo);
                let upper = (original + 1e-5).min(hi);
                if lower == upper {
                    continue;
                }
                probe.control_points[i][j][axis] = lower;
                let below = energy(&probe, work)?;
                probe.control_points[i][j][axis] = upper;
                let above = energy(&probe, work)?;
                probe.control_points[i][j][axis] = original;
                let slope = (below - above) / (upper - lower);
                if slope.is_finite() {
                    direction.push((i, j, axis, slope));
                }
            }
        }
    }
    let largest = direction
        .iter()
        .map(|entry| entry.3.abs())
        .fold(0., f64::max);
    if largest > 0. {
        for entry in &mut direction {
            entry.3 /= largest;
        }
    }
    Ok(direction)
}

pub(super) fn refine_peaks(
    source: &[Surface],
    sweeps: usize,
    stock: [[f64; 2]; 3],
    work: &mut Work,
) -> Result<Vec<Surface>> {
    refine(source, sweeps, stock, false, work)
}

pub(super) fn refine_coordinated(
    source: &[Surface],
    sweeps: usize,
    stock: [[f64; 2]; 3],
    work: &mut Work,
) -> Result<Vec<Surface>> {
    require(
        (1..=64).contains(&sweeps),
        "Bound coordinated refinement sweeps",
    )?;
    let mut coordinate = source.to_vec();
    let mut remaining = sweeps;
    while remaining > 0 {
        let batch = remaining.min(16);
        coordinate = refine(&coordinate, batch, stock, false, work)?;
        remaining -= batch;
    }
    let mut combined = refine(source, sweeps, stock, true, work)?;
    for (alternative, patch) in coordinate.into_iter().zip(&mut combined) {
        if better(scan(&alternative, work)?.0, scan(patch, work)?.0) {
            *patch = alternative;
        }
    }
    Ok(combined)
}

fn refine(
    source: &[Surface],
    sweeps: usize,
    stock: [[f64; 2]; 3],
    coordinated: bool,
    work: &mut Work,
) -> Result<Vec<Surface>> {
    require(
        (1..=32).contains(&source.len())
            && (1..=if coordinated { 64 } else { 16 }).contains(&sweeps),
        "Bound peak-guided curvature refinement",
    )?;
    let mut result = Vec::with_capacity(source.len());
    for surface in source {
        bivariate_fairing::validate(surface)?;
        require(
            surface
                .control_points
                .iter()
                .flatten()
                .all(|p| (0..3).all(|axis| stock[axis][0] <= p[axis] && p[axis] <= stock[axis][1])),
            "Curvature refinement input leaves stock",
        )?;
        let mut candidate = surface.clone();
        let mut best = scan(&candidate, work)?;
        let mut step = (0..3)
            .map(|axis| {
                let mut lo = f64::INFINITY;
                let mut hi = f64::NEG_INFINITY;
                for point in surface.control_points.iter().flatten() {
                    lo = lo.min(point[axis]);
                    hi = hi.max(point[axis]);
                }
                hi - lo
            })
            .fold(0., f64::max)
            .min(0.24)
            / 8.;
        for _ in 0..sweeps {
            let mut trial = candidate.clone();
            let anchors = best.1;
            let points = || coarse_points().chain(anchors);
            let initial = score_at(&trial, points(), work)?;
            sweep(&mut trial, step, stock, initial, |s| {
                score_at(s, points(), work)
            })?;
            let mut measured = scan(&trial, work)?;
            if coordinated {
                let points: Vec<_> = points().collect();
                let direction =
                    descent_direction(&candidate, &points, best.0.0.max(1.), stock, work)?;
                for fraction in [1., 0.5, 0.25, 0.125] {
                    let mut together = candidate.clone();
                    for &(i, j, axis, change) in &direction {
                        let [lo, hi] = stock[axis];
                        let position = candidate.control_points[i][j][axis];
                        together.control_points[i][j][axis] =
                            (position + change * step * fraction).clamp(lo, hi);
                    }
                    let proposed = scan(&together, work)?;
                    if better(proposed.0, measured.0) {
                        trial = together;
                        measured = proposed;
                    }
                }
            }
            // Rescan after a sweep so newly created peaks can veto it.
            if better(measured.0, best.0) {
                candidate = trial;
                best = measured;
            } else {
                step *= 0.5;
            }
        }
        require(
            best.0.0.is_finite(),
            "Refined patch still has an irregular sample",
        )?;
        candidate.validate()?;
        result.push(candidate);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Surface {
        Surface {
            degree_u: 6,
            degree_v: 7,
            knots_u: [vec![0.; 7], vec![1.; 7]].concat(),
            knots_v: [vec![0.; 8], vec![1.; 8]].concat(),
            control_points: (0..7)
                .map(|i| {
                    (0..8)
                        .map(|j| {
                            vec![
                                i as f64 / 6.,
                                j as f64 / 7.,
                                if i == 3 && (j == 3 || j == 4) {
                                    0.05
                                } else {
                                    0.
                                },
                            ]
                        })
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 8]; 7],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn actual_sample_curvature_decreases_without_changing_any_boundary_two_jet_pole() {
        let s = fixture();
        let before = s.clone();
        let start = score(
            &s,
            &mut Work {
                used: 0,
                max: 100000,
            },
        )
        .unwrap();
        let p = propose(
            std::slice::from_ref(&s),
            6,
            [[0., 1.]; 3],
            &mut Work {
                used: 0,
                max: 100000,
            },
        )
        .unwrap();
        let p = &p[0];
        let end = score(
            p,
            &mut Work {
                used: 0,
                max: 100000,
            },
        )
        .unwrap();
        assert!(end.0 < start.0 / 10., "{start:?} -> {end:?}");
        for i in 0..7 {
            for j in 0..8 {
                if i < 3 || i + 3 >= 7 || j < 3 || j + 3 >= 8 {
                    assert_eq!(p.control_points[i][j], s.control_points[i][j]);
                }
            }
        }
        assert_eq!(s, before);
        assert_eq!(p.weights, s.weights);
        // An independent original interval point bound, never a whole-field
        // claim from the search score. This fixture becomes essentially flat.
        let r = nurbs_core::surface_shape_operator::rectangle_bounds(p, [[0.5; 2]; 2], 8).unwrap();
        let bounds =
            nurbs_core::surface_shape_operator::absolute_spectral_bounds(r.shape.tensor.unwrap())
                .unwrap()
                .unwrap();
        assert!(bounds[1] < 1e-6, "{bounds:?}");
    }
    #[test]
    fn peak_refinement_moves_interiors_and_preserves_literal_boundary_jets() {
        let input = fixture();
        let mut work = Work {
            used: 0,
            max: 100000,
        };
        let before = scan(&input, &mut work).unwrap().0;
        let output = refine_peaks(std::slice::from_ref(&input), 8, [[0., 1.]; 3], &mut work)
            .unwrap()
            .pop()
            .unwrap();
        let after = scan(&output, &mut work).unwrap().0;
        assert!(after.0 < before.0 / 4., "{before:?} -> {after:?}");
        assert_eq!(output.knots_u, input.knots_u);
        assert_eq!(output.knots_v, input.knots_v);
        assert_eq!(output.weights, input.weights);
        for i in 0..=input.degree_u {
            for j in 0..=input.degree_v {
                if i < 3 || i > input.degree_u - 3 || j < 3 || j > input.degree_v - 3 {
                    assert_eq!(output.control_points[i][j], input.control_points[i][j]);
                }
            }
        }
    }

    #[test]
    fn exhausted_sample_allowance_and_invalid_stock_do_not_publish_partial_searches() {
        let s = fixture();
        assert!(
            propose(
                std::slice::from_ref(&s),
                6,
                [[0., 1.]; 3],
                &mut Work { used: 0, max: 25 }
            )
            .is_err()
        );
        assert!(
            propose(
                std::slice::from_ref(&s),
                9,
                [[0., 1.]; 3],
                &mut Work {
                    used: 0,
                    max: 100000
                }
            )
            .is_err()
        );
        let mut collapsed = s.clone();
        for p in collapsed.control_points.iter_mut().flatten() {
            *p = vec![0.5; 3];
        }
        assert!(
            propose(
                std::slice::from_ref(&collapsed),
                6,
                [[0.5, 0.5]; 3],
                &mut Work {
                    used: 0,
                    max: 100000
                }
            )
            .is_err()
        );
        assert!(
            propose(
                std::slice::from_ref(&s),
                6,
                [[1., 0.]; 3],
                &mut Work {
                    used: 0,
                    max: 100000
                }
            )
            .is_err()
        );
    }
}
