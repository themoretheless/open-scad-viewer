//! Inverse-depth planes q: 1/z = q·ray. A plane carries its inclination when
//! propagated, unlike copying the neighbor's depth at a different image ray.
//! This is deterministic bounded local refinement, not the complete ACMH method.
use super::estimation::{GrayImage, Patch, Source};
use super::{cancelled, DenseDiagnostics, DenseOptions};
use crate::{camera::Camera, math::*, Result};

#[derive(Clone, Copy)]
struct Hypothesis {
    plane: V3,
    score: f64,
}
impl Default for Hypothesis {
    fn default() -> Self {
        Self {
            plane: [0.; 3],
            score: -1.,
        }
    }
}

fn patch(gray: &GrayImage, x: usize, y: usize, step: f64, radius: usize) -> Option<Patch> {
    let side = radius * 2 + 1;
    let mut values = [0.; 25];
    for (i, value) in values[..side * side].iter_mut().enumerate() {
        *value = gray
            .sample(
                (x as f64 + (i % side) as f64 - radius as f64) * step,
                (y as f64 + (i / side) as f64 - radius as f64) * step,
            )
            .unwrap();
    }
    Patch::from_values(values, side * side)
}

/// Only geometrically usable, front-facing observations vote. Among them use
/// the locally agreeing views. A source more than 0.15 NCC below the best is
/// excluded; good wide-baseline observations remain in the average instead of
/// always choosing the easiest two views and weakening depth uniqueness.
/// This is photometric visibility selection; no hidden surface is inferred.
struct PixelCost<'a, 'image> {
    ray: V3,
    /// unit(ray), fixed per pixel; hoisted out of the per-hypothesis score.
    unit_ray: V3,
    ray_step: f64,
    patch: &'a Patch,
    sources: &'a [Source<'image>],
    reference_focal: f64,
    options: &'a DenseOptions,
}
impl PixelCost<'_, '_> {
    fn score(&self, q: V3, diagnostics: &mut DenseDiagnostics) -> f64 {
        let Self {
            ray,
            unit_ray,
            ray_step,
            patch,
            sources,
            reference_focal,
            options,
        } = *self;
        diagnostics.evaluated_hypotheses += 1;
        let inv = dot(q, ray);
        if !inv.is_finite() || inv <= 0. {
            return -1.;
        }
        let z = 1. / inv;
        let n = unit(q);
        let ref_cos = dot(n, unit_ray);
        if ref_cos < 0.12 {
            return -1.;
        }
        let mut depths = [0.; 25];
        let side = options.patch_radius * 2 + 1;
        // Each column/row term repeats across the patch. Preserve the original
        // multiplication and addition order so scores remain bit-identical.
        let mut column_offsets = [0.; 5];
        let mut row_offsets = [0.; 5];
        for i in 0..side {
            let offset = (i as f64 - options.patch_radius as f64) * ray_step;
            column_offsets[i] = offset * q[0];
            row_offsets[i] = offset * q[1];
        }
        for (i, depth) in depths[..patch.len].iter_mut().enumerate() {
            let offset = column_offsets[i % side] + row_offsets[i / side];
            let d = inv + offset;
            if d <= 0. {
                return -1.;
            }
            *depth = 1. / d;
        }
        let mut votes = [(-1., 0.); 6];
        let mut usable = 0;
        let mut count = 0;
        for source in sources {
            let center = add(scale(source.rays[patch.len / 2], z), source.translation);
            if center[2] <= 0. {
                continue;
            }
            let src_cos = dot(mv(source.rotation, n), unit(center));
            if src_cos < 0.12 {
                continue;
            }
            let area_ratio = (source.camera.focal * z / (reference_focal * center[2])).powi(2)
                * src_cos
                / ref_cos;
            if !(0.0625..=16.).contains(&area_ratio) {
                continue;
            }
            diagnostics.evaluated_source_patches += 1;
            let mut correlation = patch.accumulator();
            let mut complete = true;
            for i in 0..patch.len {
                let p = add(scale(source.rays[i], depths[i]), source.translation);
                if p[2] <= 0. {
                    complete = false;
                    break;
                }
                if let Some(v) = source.image.sample(
                    source.camera.focal * p[0] / p[2] + source.camera.cx,
                    source.camera.focal * p[1] / p[2] + source.camera.cy,
                ) {
                    correlation.sample(i, v);
                    diagnostics.sampled_source_pixels += 1;
                } else {
                    complete = false;
                    break;
                }
            }
            if !complete {
                continue;
            }
            usable += 1;
            if let Some(ncc) = correlation.finish() {
                if ncc > 0.4 {
                    votes[count] = (ncc, area_ratio.min(1. / area_ratio).sqrt());
                    count += 1;
                }
            }
        }
        let needed = usable.min(2).max(options.min_support_views);
        if count < needed {
            return -1.;
        }
        votes[..count].sort_by(|a, b| b.0.total_cmp(&a.0));
        let agreeing = votes[..count]
            .iter()
            .take_while(|&&(ncc, _)| ncc >= votes[0].0 - 0.15)
            .count();
        if agreeing < needed {
            return -1.;
        }
        let (weighted, weights) = votes[..agreeing]
            .iter()
            .fold((0., 0.), |(sum, w), &(ncc, weight)| {
                (sum + ncc * weight, w + weight)
            });
        weighted / weights.max(1e-12)
    }
}

// A slope update reconstructs q.z to preserve the center depth; evaluating
// q·ray can cross an endpoint by roundoff. Scale the guard by its arithmetic
// terms, not by scene units or a user-selected geometric tolerance.
fn in_depth_range(inv: f64, q: V3, ray: V3, low: f64, high: f64) -> bool {
    if !inv.is_finite() || inv <= 0. {
        return false;
    }
    if (low..=high).contains(&inv) {
        return true;
    }
    let magnitude = (q[0] * ray[0]).abs()
        + (q[1] * ray[1]).abs()
        + (q[2] * ray[2]).abs()
        + low.abs().max(high.abs());
    let roundoff = 16. * f64::EPSILON * magnitude;
    roundoff.is_finite() && inv >= low - roundoff && inv <= high + roundoff
}

#[cfg(test)]
#[test]
fn depth_range_accepts_roundoff_but_rejects_real_excursions() {
    let low = 0.215727407728201120_f64;
    let below = f64::from_bits(low.to_bits() - 1);
    assert!(in_depth_range(
        below,
        [0., 0., below],
        [0., 0., 1.],
        low,
        0.32
    ));
    assert!(in_depth_range(
        f64::from_bits(0.32_f64.to_bits() + 1),
        [0., 0., 0.32],
        [0., 0., 1.],
        low,
        0.32
    ));
    assert!(!in_depth_range(
        low - 1e-8,
        [0., 0., low - 1e-8],
        [0., 0., 1.],
        low,
        0.32
    ));
    assert!(!in_depth_range(
        0.32 + 1e-8,
        [0., 0., 0.32 + 1e-8],
        [0., 0., 1.],
        low,
        0.32
    ));
    for inv in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0., -1.] {
        assert!(!in_depth_range(inv, [0., 0., inv], [0., 0., 1.], low, 0.32));
    }
}

fn at_depth_with_slope(q: V3, ray: V3, inverse_depth: f64) -> V3 {
    [q[0], q[1], inverse_depth - q[0] * ray[0] - q[1] * ray[1]]
}

// One bounded local refinement step for a single pixel. Returns whether the
// pixel's hypothesis strictly improved, for the optional adaptive budget.
#[allow(clippy::too_many_arguments)]
fn refine_pixel(
    gray: &GrayImage,
    reference: &Camera,
    sources: &mut [Source<'_>],
    planes: &mut [Hypothesis],
    coarse: &[(f64, f64, f64)],
    width: usize,
    x: usize,
    y: usize,
    step: f64,
    ray_step: f64,
    low: f64,
    high: f64,
    interval: f64,
    pass: usize,
    budget: usize,
    reverse: bool,
    adaptive: bool,
    options: &DenseOptions,
    diagnostics: &mut DenseDiagnostics,
) -> bool {
    let Some(patch) = patch(gray, x, y, step, options.patch_radius) else {
        return false;
    };
    let i = y * width + x;
    let ray = reference.ray([x as f64 * step, y as f64 * step]);
    for source in sources.iter_mut() {
        source.prepare_pixel(ray);
    }
    let cost = PixelCost {
        ray,
        unit_ray: unit(ray),
        ray_step,
        patch: &patch,
        sources,
        reference_focal: reference.focal,
        options,
    };
    let mut best = planes[i];
    let mut proposals = [[0.; 3]; 6];
    // Four directions keep vertical as well as horizontal slants.
    proposals[0] = [0., 0., coarse[i].2];
    for (k, j) in [i - 1, i + 1, i - width, i + width].into_iter().enumerate() {
        proposals[k + 2] = planes[j].plane;
    }
    // Finite differences propose inclination; photometry decides
    // whether a gradient is a true plane or a depth discontinuity.
    let ix = if reverse { i + 1 } else { i - 1 };
    let iy = if reverse { i + width } else { i - width };
    let signed_step = if reverse { ray_step } else { -ray_step };
    let inv = dot(best.plane, ray);
    let adjacent_x = add(ray, [signed_step, 0., 0.]);
    let adjacent_y = add(ray, [0., signed_step, 0.]);
    let slope = [
        (dot(planes[ix].plane, adjacent_x) - inv) / signed_step,
        (dot(planes[iy].plane, adjacent_y) - inv) / signed_step,
        0.,
    ];
    proposals[1] = at_depth_with_slope(slope, ray, inv);
    for (proposal_index, &q) in proposals[..budget.min(6)].iter().enumerate() {
        // The pixel, sources and scoring policy are unchanged within this loop.
        // Strict improvement means an identical earlier proposal cannot win again.
        if proposals[..proposal_index].contains(&q) {
            diagnostics.evaluated_hypotheses += 1;
            continue;
        }
        // offset == 0 makes proposals[0] equal the current best
        // plane; an equal candidate cannot pass the strict
        // improvement test, so skip its score entirely.
        if q == best.plane {
            diagnostics.evaluated_hypotheses += 1;
            continue;
        }
        let candidate_inv = dot(q, ray);
        let value = if in_depth_range(candidate_inv, q, ray, low, high) {
            cost.score(q, diagnostics)
        } else {
            diagnostics.evaluated_hypotheses += 1;
            -1.
        };
        if value > best.score {
            best = Hypothesis {
                plane: q,
                score: value,
            };
        }
    }
    // A full axis/sign cycle without improvement ends the local descent early;
    // the order and the cycle length are fixed, so the stop is deterministic.
    let mut cycle_improved = true;
    for k in 6..budget {
        if adaptive && (k - 6) % 6 == 0 {
            if !cycle_improved {
                break;
            }
            cycle_improved = false;
        }
        let current_inv = dot(best.plane, ray);
        let cycle = (k - 6) / 6;
        let shrink = 0.5f64.powi((pass + cycle) as i32);
        let (axis, sign) = ((k - 6) % 6 / 2, if k % 2 == 0 { 1. } else { -1. });
        let mut q = best.plane;
        if axis == 2 {
            q = at_depth_with_slope(q, ray, current_inv + sign * interval * 0.5 * shrink);
        } else {
            q[axis] += sign * current_inv * 0.7 * shrink;
            q = at_depth_with_slope(q, ray, current_inv);
        }
        // Evaluate immediately, allowing coordinate updates within
        // the budget rather than committing to a stale center.
        let candidate_inv = dot(q, ray);
        let value = if in_depth_range(candidate_inv, q, ray, low, high) {
            cost.score(q, diagnostics)
        } else {
            diagnostics.evaluated_hypotheses += 1;
            -1.
        };
        if value > best.score {
            best = Hypothesis {
                plane: q,
                score: value,
            };
            cycle_improved = true;
        }
    }
    let improved = best.score > planes[i].score;
    planes[i] = best;
    improved
}

#[allow(clippy::too_many_arguments)]
pub(super) fn estimate(
    gray: &GrayImage,
    reference: &Camera,
    sources: &mut [Source<'_>],
    width: usize,
    height: usize,
    step: f64,
    near: f64,
    far: f64,
    options: &DenseOptions,
    diagnostics: &mut DenseDiagnostics,
    progress: &mut impl FnMut(&str, usize, usize) -> bool,
) -> Result<(Vec<f64>, Vec<f32>)> {
    let count = width * height;
    let mut planes = vec![Hypothesis::default(); count];
    // Keep a distant global competitor, not the neighboring slope refinement,
    // as ambiguity evidence. Local refinement does not manufacture confidence.
    let mut coarse = vec![(0., -1., 0.); count];
    let mut alternative = vec![-1.; count];
    let initial = options.depth_hypotheses / 2;
    let refine = options.depth_hypotheses - initial;
    let pass_budget = [refine / 2, refine - refine / 2];
    let low = 1. / far;
    let high = 1. / near;
    let interval = (high - low) / (initial - 1) as f64;
    let ray_step = step / reference.focal;
    for y in 3..height - 3 {
        cancelled(progress, "depth", y, height * 3)?;
        for x in 3..width - 3 {
            let Some(patch) = patch(gray, x, y, step, options.patch_radius) else {
                continue;
            };
            let ray = reference.ray([x as f64 * step, y as f64 * step]);
            for source in sources.iter_mut() {
                source.prepare_pixel(ray);
            }
            let cost = PixelCost {
                ray,
                unit_ray: unit(ray),
                ray_step,
                patch: &patch,
                sources,
                reference_focal: reference.focal,
                options,
            };
            let mut scores = [-1.; 64];
            for (d, value) in scores[..initial].iter_mut().enumerate() {
                *value = cost.score([0., 0., low + d as f64 * interval], diagnostics);
            }
            let best = (0..initial)
                .max_by(|&a, &b| scores[a].total_cmp(&scores[b]))
                .unwrap();
            let i = y * width + x;
            planes[i] = Hypothesis {
                plane: [0., 0., low + best as f64 * interval],
                score: scores[best],
            };
            let mut offset = 0.;
            if best > 0 && best + 1 < initial {
                let (a, b, c) = (scores[best - 1], scores[best], scores[best + 1]);
                let denom = a - 2. * b + c;
                if a > 0. && c > 0. && denom.abs() > 1e-8 {
                    offset = (0.5 * (a - c) / denom).clamp(-0.5, 0.5);
                }
            }
            coarse[i] = (
                dot(planes[i].plane, ray),
                scores[best],
                low + (best as f64 + offset) * interval,
            );
            // Same inverse-depth separation as three bins in the full sweep.
            let separation = 3. * (high - low) / (options.depth_hypotheses - 1) as f64;
            alternative[i] = (0..initial)
                .filter(|&d| (d.abs_diff(best) as f64 * interval) > separation)
                .map(|d| scores[d])
                .fold(-1., f64::max);
        }
    }
    let red_black = options.red_black_propagation;
    let adaptive = options.adaptive_refine_budget;
    for (pass, &budget) in pass_budget.iter().enumerate() {
        let mut pass_improved = false;
        if red_black {
            // Checkerboard propagation (ACMH-style): each color updates against
            // only the opposite, stable color, so a half-sweep has no
            // read-after-write dependency. The color selects the finite
            // difference direction, mirroring the forward/backward alternation.
            for color in 0..2usize {
                let reverse = color == 1;
                for row in 3..height - 3 {
                    cancelled(progress, "depth", height * (pass + 1) + row, height * 3)?;
                    let y = row;
                    for column in 3..width - 3 {
                        let x = column;
                        if (x + y) % 2 != color {
                            continue;
                        }
                        pass_improved |= refine_pixel(
                            gray,
                            reference,
                            sources,
                            &mut planes,
                            &coarse,
                            width,
                            x,
                            y,
                            step,
                            ray_step,
                            low,
                            high,
                            interval,
                            pass,
                            budget,
                            reverse,
                            adaptive,
                            options,
                            diagnostics,
                        );
                    }
                }
            }
        } else {
            let reverse = pass % 2 == 1;
            for row in 3..height - 3 {
                cancelled(progress, "depth", height * (pass + 1) + row, height * 3)?;
                let y = if reverse { height - 1 - row } else { row };
                for column in 3..width - 3 {
                    let x = if reverse { width - 1 - column } else { column };
                    pass_improved |= refine_pixel(
                        gray,
                        reference,
                        sources,
                        &mut planes,
                        &coarse,
                        width,
                        x,
                        y,
                        step,
                        ray_step,
                        low,
                        high,
                        interval,
                        pass,
                        budget,
                        reverse,
                        adaptive,
                        options,
                        diagnostics,
                    );
                }
            }
        }
        // A whole pass without a single strict improvement cannot be rescued
        // by repeating the same proposals; remaining passes are skipped.
        if adaptive && !pass_improved {
            break;
        }
    }
    let mut depth = vec![0.; count];
    let mut confidence = vec![0.; count];
    for y in 3..height - 3 {
        cancelled(progress, "depth", height * 3, height * 3)?;
        for x in 3..width - 3 {
            let i = y * width + x;
            let h = planes[i];
            let ray = reference.ray([x as f64 * step, y as f64 * step]);
            let inv = dot(h.plane, ray);
            let separation = 3. * (high - low) / (options.depth_hypotheses - 1) as f64;
            let alt = if (inv - coarse[i].0).abs() > separation {
                alternative[i].max(coarse[i].1)
            } else {
                alternative[i]
            };
            if h.score >= options.min_correlation && h.score - alt >= options.uniqueness_margin {
                depth[i] = 1. / inv;
                confidence[i] = h.score as f32;
            }
        }
    }
    Ok((depth, confidence))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn red_black_and_adaptive_refinement_are_deterministic() {
        use crate::Image;
        let camera = Camera::identity(100., 8., 8.);
        let mut rgb = Vec::new();
        for y in 0..16 {
            for x in 0..16 {
                let value = ((x * 13 + y * 7 + x * y * 3) % 31) as f64;
                rgb.extend([((value + 10.) * 4.) as u8; 3]);
            }
        }
        let image = Image {
            width: 16,
            height: 16,
            rgb,
            focal: 100.,
        };
        let gray = GrayImage::new(&image);
        let run = |options: &DenseOptions| {
            let mut source_camera = camera.clone();
            source_camera.translation = [-0.05, 0., 0.];
            let mut sources = [Source {
                image: &gray,
                camera: &source_camera,
                rotation: ID,
                translation: [-0.05, 0., 0.],
                offsets: std::array::from_fn(|i| {
                    [
                        ((i % 5) as f64 - 2.) / 100.,
                        ((i / 5) as f64 - 2.) / 100.,
                        0.,
                    ]
                }),
                rays: [[0.; 3]; 25],
            }];
            estimate(
                &gray,
                &camera,
                &mut sources,
                16,
                16,
                1.,
                2.,
                8.,
                options,
                &mut DenseDiagnostics::default(),
                &mut |_, _, _| true,
            )
            .unwrap()
        };
        let default_a = run(&DenseOptions::default());
        let default_b = run(&DenseOptions::default());
        assert_eq!(default_a.0, default_b.0);
        assert_eq!(default_a.1, default_b.1);
        for options in [
            DenseOptions {
                red_black_propagation: true,
                ..Default::default()
            },
            DenseOptions {
                adaptive_refine_budget: true,
                ..Default::default()
            },
            DenseOptions {
                red_black_propagation: true,
                adaptive_refine_budget: true,
                ..Default::default()
            },
        ] {
            let a = run(&options);
            let b = run(&options);
            assert_eq!(a.0, b.0);
            assert_eq!(a.1, b.1);
            for &d in &a.0 {
                assert!(d == 0. || (1.99..=8.01).contains(&d));
            }
        }
    }
    #[test]
    fn propagated_plane_changes_depth_but_preserves_surface() {
        let q = [0.2, -0.1, 0.25];
        for ray in [[0., 0., 1.], [0.2, 0.1, 1.], [-0.1, 0.3, 1.]] {
            let p = scale(ray, 1. / dot(q, ray));
            assert!((dot(q, p) - 1.).abs() < 1e-12);
        }
        assert_ne!(1. / dot(q, [0., 0., 1.]), 1. / dot(q, [0.2, 0., 1.]));
    }
    #[test]
    fn local_visibility_requires_consensus_not_one_favorable_source() {
        use crate::Image;
        let camera = Camera::identity(100., 8., 8.);
        let make = |noisy: bool| {
            let mut rgb = Vec::new();
            for y in 0..16 {
                for x in 0..16 {
                    let base = ((x * 13 + y * 7 + x * y * 3) % 31) as f64;
                    let other = ((x * 7 + y * 23 + x * y) % 31) as f64;
                    let value = if noisy {
                        base * 0.65 + other * 0.75
                    } else {
                        base
                    };
                    rgb.extend([((value + 10.) * 4.) as u8; 3]);
                }
            }
            GrayImage::new(&Image {
                width: 16,
                height: 16,
                rgb,
                focal: 100.,
            })
        };
        let clean = make(false);
        let disagreeing = make(true);
        let options = DenseOptions {
            patch_radius: 2,
            ..Default::default()
        };
        let patch = patch(&clean, 8, 8, 1., 2).unwrap();
        let ray = camera.ray([8., 8.]);
        let make_source = |image| {
            let mut source = Source {
                image,
                camera: &camera,
                rotation: ID,
                translation: [0.; 3],
                offsets: std::array::from_fn(|i| {
                    [
                        ((i % 5) as f64 - 2.) / 100.,
                        ((i / 5) as f64 - 2.) / 100.,
                        0.,
                    ]
                }),
                rays: [[0.; 3]; 25],
            };
            source.prepare_pixel(ray);
            source
        };
        let sources = [
            make_source(&clean),
            make_source(&clean),
            make_source(&disagreeing),
        ];
        let cost = PixelCost {
            ray,
            unit_ray: unit(ray),
            ray_step: 0.01,
            patch: &patch,
            sources: &sources,
            reference_focal: 100.,
            options: &options,
        };
        assert!(cost.score([0., 0., 0.25], &mut DenseDiagnostics::default()) > 0.999);
        let sources = [
            make_source(&clean),
            make_source(&disagreeing),
            make_source(&disagreeing),
        ];
        let cost = PixelCost {
            sources: &sources,
            ..cost
        };
        assert_eq!(
            cost.score([0., 0., 0.25], &mut DenseDiagnostics::default()),
            -1.
        );
    }
}
