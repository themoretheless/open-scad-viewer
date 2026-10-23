//! Bounded geometric seed proposals; no scene mutation or image decoding.
use crate::{
    Image, Result,
    camera::{self, Camera},
    diagnostics::ReconstructionReport,
    features::{Feature, Match},
    matching::MatchGraph,
};

pub(crate) struct Seed {
    pub a: usize,
    pub b: usize,
    pub pose: Camera,
    pub inliers: Vec<usize>,
    pub matches: Vec<Match>,
    pub score: f64,
}
/// Opt-in seed selection extensions; the default reproduces the original ranking.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SeedOptions {
    /// Also verify the best runner-up pairs that missed the descriptor-score
    /// selection, then rank all verified seeds by geometric inlier support.
    /// Costs at most one extra geometric verification per selected pair.
    pub verify_runners_up: bool,
}
fn coverage(
    matches: &[Match],
    features: &[Vec<Feature>],
    images: &[Image],
    a: usize,
    b: usize,
) -> f64 {
    let area = |image: usize, side: bool| {
        let mut lo = [f64::INFINITY; 2];
        let mut hi = [f64::NEG_INFINITY; 2];
        for m in matches {
            let f = &features[image][if side { m.a } else { m.b }];
            lo[0] = lo[0].min(f.x);
            lo[1] = lo[1].min(f.y);
            hi[0] = hi[0].max(f.x);
            hi[1] = hi[1].max(f.y);
        }
        ((hi[0] - lo[0]) * (hi[1] - lo[1]) / (images[image].width * images[image].height) as f64)
            .clamp(0., 1.)
    };
    area(a, true).min(area(b, false))
}
#[cfg(test)]
pub(crate) fn propose(
    images: &[Image],
    features: &[Vec<Feature>],
    cache: &mut MatchGraph,
    limit: usize,
    geometry_options: &camera::GeometryOptions,
    report: &mut ReconstructionReport,
    progress: &mut impl FnMut(&str, usize, usize) -> bool,
) -> Result<Vec<Seed>> {
    propose_with_options(
        images,
        features,
        cache,
        limit,
        geometry_options,
        &SeedOptions::default(),
        report,
        progress,
    )
}
/// Image-index pairs considered for seeding: every pair up to 24 photos (the
/// browser project scale), an acquisition-order window of 9 successors beyond
/// that. The browser host-GPU matching path pre-matches exactly this list.
pub(crate) fn pair_candidates(image_count: usize) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    for a in 0..image_count {
        let end = if image_count <= 24 {
            image_count
        } else {
            (a + 9).min(image_count)
        };
        for b in a + 1..end {
            pairs.push((a, b));
        }
    }
    pairs
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn propose_with_options(
    images: &[Image],
    features: &[Vec<Feature>],
    cache: &mut MatchGraph,
    limit: usize,
    geometry_options: &camera::GeometryOptions,
    seed_options: &SeedOptions,
    report: &mut ReconstructionReport,
    progress: &mut impl FnMut(&str, usize, usize) -> bool,
) -> Result<Vec<Seed>> {
    let mut candidates = Vec::new();
    let mut matches_by_pair = std::collections::BTreeMap::new();
    // All pairs for browser projects, acquisition-order window for larger native sets.
    for (a, b) in pair_candidates(images.len()) {
        if !progress(
            "matching",
            cache.computed_pairs,
            images.len() * (images.len() - 1) / 2,
        ) {
            return Err(crate::error("Cancelled"));
        }
        let matches = cache.between(features, a, b).collect::<Vec<_>>();
        if matches.len() >= 12 {
            let score = matches.len() as f64 * coverage(&matches, features, images, a, b).sqrt();
            candidates.push((score, a, b));
            matches_by_pair.insert((a, b), matches);
        }
    }
    // Reserve a small part of the geometric budget for early acquisition pairs;
    // pure descriptor count otherwise favors repeated/planar patches of one surface.
    let mut selected = candidates
        .iter()
        .take((limit / 3).min(4))
        .copied()
        .collect::<Vec<_>>();
    candidates.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    for &candidate in &candidates {
        if selected.len() >= limit {
            break;
        }
        if !selected
            .iter()
            .any(|c| (c.1, c.2) == (candidate.1, candidate.2))
        {
            selected.push(candidate);
        }
    }
    // Opt-in: the descriptor-score selection can miss pairs with fewer but
    // geometrically stronger correspondences; verify the best runners-up too
    // and let the inlier-scored sort below rank every verified seed.
    if seed_options.verify_runners_up {
        let mut extras = 0;
        for &candidate in &candidates {
            if extras >= limit {
                break;
            }
            if !selected
                .iter()
                .any(|c| (c.1, c.2) == (candidate.1, candidate.2))
            {
                selected.push(candidate);
                extras += 1;
            }
        }
    }
    let mut seeds = Vec::new();
    for (_, a, b) in selected {
        if !progress("initial_pair", report.seed_pairs_tested, limit) {
            return Err(crate::error("Cancelled"));
        }
        report.seed_pairs_tested += 1;
        let matches = match matches_by_pair.remove(&(a, b)) {
            Some(matches) => {
                // The pair is already cached; keep the observable request count.
                cache.requests += 1;
                matches
            }
            None => cache.between(features, a, b).collect::<Vec<_>>(),
        };
        let pairs = matches
            .iter()
            .map(|m| {
                let x = &features[a][m.a];
                let y = &features[b][m.b];
                ([x.x, x.y], [y.x, y.y])
            })
            .collect::<Vec<_>>();
        let mut geometry_report = camera::RobustReport::default();
        if let Some((pose, inliers)) = camera::relative_with_options(
            &images[a].camera(),
            &images[b].camera(),
            &pairs,
            geometry_options,
            &mut geometry_report,
        ) {
            let accepted = inliers.iter().map(|&i| matches[i]).collect::<Vec<_>>();
            let score = inliers.len() as f64 * coverage(&accepted, features, images, a, b).sqrt();
            seeds.push(Seed {
                a,
                b,
                pose,
                inliers,
                matches,
                score,
            });
        } else if geometry_report.insufficient_support {
            report.warnings.push(format!("Seed {a}-{b} declined: only {}/{} correspondences support the pose (required {:.0}%)", geometry_report.inliers, pairs.len(), geometry_options.minimum_seed_inlier_ratio * 100.));
        } else if geometry_report.rejected_degenerate {
            report.warnings.push(format!("Seed {a}-{b} declined: correspondences are explained by one homography; unconstrained 3D initialization is ambiguous"));
        }
    }
    seeds.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then(a.a.cmp(&b.a))
            .then(a.b.cmp(&b.b))
    });
    if seed_options.verify_runners_up {
        // Runner-up verification is a widening of the candidate set, not of
        // the seed budget; keep at most `limit` seeds as in the default path.
        seeds.truncate(limit);
    }
    report.matching_requests = cache.requests;
    report.computed_pairs = cache.computed_pairs;
    Ok(seeds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::GeometryOptions;

    fn rot_y(angle: f64) -> [[f64; 3]; 3] {
        let (c, s) = (angle.cos(), angle.sin());
        [[c, 0., s], [0., 1., 0.], [-s, 0., c]]
    }
    fn blank() -> Image {
        Image {
            width: 64,
            height: 64,
            rgb: vec![0; 64 * 64 * 3],
            focal: 100.,
        }
    }
    /// Three synthetic views of one point cloud. View 1 is a pure rotation
    /// from view 0, so their pair is homography-degenerate; view 2 has a real
    /// baseline. Views 0/1 share 40 tracks, view 2 shares 30, so descriptor
    /// count ranks the degenerate pair first.
    fn scene() -> (Vec<Image>, Vec<Vec<Feature>>) {
        let cameras = [
            Camera::identity(100., 32., 32.),
            Camera {
                rotation: rot_y(0.1),
                translation: [0.; 3],
                focal: 100.,
                cx: 32.,
                cy: 32.,
            },
            Camera {
                rotation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                translation: [0.35, 0.05, -0.02],
                focal: 100.,
                cx: 32.,
                cy: 32.,
            },
        ];
        let mut state = 0x9e3779b97f4a7c15u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state % 10000) as f64 / 10000.
        };
        let points = (0..40)
            .map(|_| [next() * 1.6 - 0.8, next() * 1.6 - 0.8, 3. + next() * 3.])
            .collect::<Vec<_>>();
        let features = cameras
            .iter()
            .enumerate()
            .map(|(view, camera)| {
                let visible = if view == 2 { 30 } else { 40 };
                points[..visible]
                    .iter()
                    .enumerate()
                    .map(|(track, point)| {
                        let [x, y] = camera.project(*point).unwrap();
                        let mut descriptor = [0.; 128];
                        descriptor[track] = 1.;
                        Feature { x, y, descriptor }
                    })
                    .collect()
            })
            .collect();
        (vec![blank(); 3], features)
    }
    fn verify(
        features: &[Vec<Feature>],
        limit: usize,
        seed_options: &SeedOptions,
    ) -> Vec<(usize, usize, u64, usize)> {
        let images = vec![blank(); features.len()];
        let mut cache = MatchGraph::default();
        let mut report = ReconstructionReport::default();
        propose_with_options(
            &images,
            features,
            &mut cache,
            limit,
            &GeometryOptions::default(),
            seed_options,
            &mut report,
            &mut |_, _, _| true,
        )
        .unwrap()
        .iter()
        .map(|s| (s.a, s.b, s.score.to_bits(), s.inliers.len()))
        .collect()
    }
    #[test]
    fn default_options_reproduce_original_propose_deterministically() {
        let (_, features) = scene();
        let first = verify(&features, 2, &SeedOptions::default());
        assert_eq!(first, verify(&features, 2, &SeedOptions::default()));
        // The budget covers the degenerate top pair plus one baseline pair.
        assert_eq!(first.len(), 1);
        assert!(first[0].0 != first[0].1 && first[0].1 == 2, "{first:?}");
        // The pub(crate) entry point delegates with default options.
        let images = vec![blank(); 3];
        let mut cache = MatchGraph::default();
        let mut report = ReconstructionReport::default();
        let seeds = propose(
            &images,
            &features,
            &mut cache,
            2,
            &GeometryOptions::default(),
            &mut report,
            &mut |_, _, _| true,
        )
        .unwrap();
        let delegated = seeds
            .iter()
            .map(|s| (s.a, s.b, s.score.to_bits(), s.inliers.len()))
            .collect::<Vec<_>>();
        assert_eq!(first, delegated);
    }
    #[test]
    fn runner_up_verification_finds_baseline_seed_behind_degenerate_top_pair() {
        let (_, features) = scene();
        // The whole budget goes to the degenerate pure-rotation pair.
        assert!(verify(&features, 1, &SeedOptions::default()).is_empty());
        let seeds = verify(
            &features,
            1,
            &SeedOptions {
                verify_runners_up: true,
            },
        );
        assert_eq!(seeds.len(), 1);
        assert_eq!(seeds[0].1, 2);
        assert!(seeds[0].3 >= 15, "{seeds:?}");
    }
}
