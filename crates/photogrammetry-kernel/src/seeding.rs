//! Bounded geometric seed proposals; no scene mutation or image decoding.
use crate::{
    camera::{self, Camera},
    diagnostics::ReconstructionReport,
    features::{Feature, Match},
    matching::MatchGraph,
    Image, Result,
};

pub(crate) struct Seed {
    pub a: usize,
    pub b: usize,
    pub pose: Camera,
    pub inliers: Vec<usize>,
    pub matches: Vec<Match>,
    pub score: f64,
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
pub(crate) fn propose(
    images: &[Image],
    features: &[Vec<Feature>],
    cache: &mut MatchGraph,
    limit: usize,
    geometry_options: &camera::GeometryOptions,
    report: &mut ReconstructionReport,
    progress: &mut impl FnMut(&str, usize, usize) -> bool,
) -> Result<Vec<Seed>> {
    let mut candidates = Vec::new();
    // All pairs for browser projects, acquisition-order window for larger native sets.
    for a in 0..images.len() {
        let end = if images.len() <= 24 {
            images.len()
        } else {
            (a + 9).min(images.len())
        };
        for b in a + 1..end {
            if !progress(
                "matching",
                cache.computed_pairs,
                images.len() * (images.len() - 1) / 2,
            ) {
                return Err("Cancelled".into());
            }
            let matches = cache.between(features, a, b).collect::<Vec<_>>();
            if matches.len() >= 12 {
                let score =
                    matches.len() as f64 * coverage(&matches, features, images, a, b).sqrt();
                candidates.push((score, a, b));
            }
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
    for candidate in candidates {
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
    let mut seeds = Vec::new();
    for (_, a, b) in selected {
        if !progress("initial_pair", report.seed_pairs_tested, limit) {
            return Err("Cancelled".into());
        }
        report.seed_pairs_tested += 1;
        let matches = cache.between(features, a, b).collect::<Vec<_>>();
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
    report.matching_requests = cache.requests;
    report.computed_pairs = cache.computed_pairs;
    Ok(seeds)
}
