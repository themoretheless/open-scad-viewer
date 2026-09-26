//! A per-run, direction-independent correspondence cache and unambiguous track associations.
use crate::features::{self, Feature, Match};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(crate) struct MatchGraph {
    pairs: BTreeMap<(usize, usize), Vec<Match>>,
    pub requests: usize,
    pub computed_pairs: usize,
    options: features::FeatureOptions,
}
impl MatchGraph {
    /// Default construction keeps plain strict matching; this opt-in carries
    /// feature options (currently `second_chance`) into pair matching.
    pub fn with_options(options: features::FeatureOptions) -> Self {
        Self {
            options,
            ..Default::default()
        }
    }
    /// Inserts externally computed matches (host-GPU round trip) so `between`
    /// serves them from the cache without host-side descriptor matching.
    /// Counts as a computed pair for progress and diagnostics.
    pub(crate) fn precompute(&mut self, a: usize, b: usize, matches: Vec<Match>) {
        self.pairs
            .entry((a.min(b), a.max(b)))
            .or_insert(matches);
        self.computed_pairs += 1;
    }
    pub fn between<'a>(
        &'a mut self,
        features: &[Vec<Feature>],
        a: usize,
        b: usize,
    ) -> impl Iterator<Item = Match> + 'a {
        self.requests += 1;
        let (lo, hi) = (a.min(b), a.max(b));
        let options = self.options;
        let pairs = self.pairs.entry((lo, hi)).or_insert_with(|| {
            self.computed_pairs += 1;
            features::matches_with_options(&features[lo], &features[hi], &options)
        });
        pairs.iter().map(move |&m| {
            if a <= b {
                m
            } else {
                Match {
                    a: m.b,
                    b: m.a,
                    ..m
                }
            }
        })
    }
}

/// A feature must refer to one world point, and a point to one feature in a view.
/// Conflicting links are excluded, not resolved by the order cameras happened to register.
#[derive(Default)]
pub(crate) struct AssociationVotes(Vec<Vec<(usize, usize)>>);
impl AssociationVotes {
    pub fn add(&mut self, feature: usize, point: usize) {
        if self.0.len() <= feature {
            self.0.resize_with(feature + 1, Vec::new);
        }
        let points = &mut self.0[feature];
        match points.iter_mut().find(|(p, _)| *p == point) {
            Some((_, votes)) => *votes += 1,
            None => points.push((point, 1)),
        }
    }
    pub fn resolve(self) -> (BTreeMap<usize, usize>, usize) {
        let mut candidates = Vec::new();
        let mut rejected = 0;
        for (feature, points) in self.0.into_iter().enumerate() {
            if points.len() != 1 {
                if !points.is_empty() {
                    rejected += 1;
                }
                continue;
            }
            let (point, votes) = points.into_iter().next().unwrap();
            candidates.push((votes, feature, point));
        }
        candidates.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let mut used = BTreeSet::new();
        let mut result = BTreeMap::new();
        for (_, feature, point) in candidates {
            if used.insert(point) {
                result.insert(feature, point);
            } else {
                rejected += 1;
            }
        }
        (result, rejected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn conflicting_tracks_and_duplicate_view_observations_are_excluded() {
        let mut v = AssociationVotes::default();
        v.add(0, 7);
        v.add(0, 8);
        v.add(1, 9);
        v.add(2, 9);
        v.add(2, 9);
        let (r, rejected) = v.resolve();
        assert_eq!(r, BTreeMap::from([(2, 9)]));
        assert_eq!(rejected, 2);
    }
    #[test]
    fn reverse_pair_uses_same_computation() {
        let mut a = Feature {
            x: 10.,
            y: 10.,
            descriptor: [0.; 128],
        };
        a.descriptor[0] = 1.;
        let mut b = a.clone();
        b.descriptor = [0.; 128];
        b.descriptor[1] = 1.;
        let fs = vec![vec![a.clone(), b.clone()], vec![b, a]];
        let mut g = MatchGraph::default();
        let forward = g.between(&fs, 0, 1).map(|m| (m.a, m.b)).collect::<Vec<_>>();
        let mut reverse = g.between(&fs, 1, 0).map(|m| (m.b, m.a)).collect::<Vec<_>>();
        reverse.sort();
        assert_eq!(forward, reverse);
        assert_eq!(g.computed_pairs, 1);
        assert_eq!(g.requests, 2);
    }
    #[test]
    fn second_chance_relaxes_only_mutual_ratio_rejects() {
        let a = Feature {
            x: 10.,
            y: 10.,
            descriptor: [0.; 128],
        };
        // Mutual nearest neighbor at squared distance 0.7225 with the second
        // nearest at 1.0: ratio 0.85 fails the strict 0.8 gate, passes 0.9.
        let mut near = a.clone();
        near.descriptor[0] = 0.85;
        let mut far = a.clone();
        far.descriptor[0] = 1.0;
        let fs = vec![vec![a], vec![near, far]];
        let mut strict = MatchGraph::default();
        assert_eq!(strict.between(&fs, 0, 1).count(), 0);
        let mut relaxed = MatchGraph::with_options(features::FeatureOptions {
            second_chance: true,
            ..features::FeatureOptions::ROOT
        });
        let got = relaxed.between(&fs, 0, 1).collect::<Vec<_>>();
        assert_eq!(got.len(), 1);
        assert_eq!((got[0].a, got[0].b), (0, 0));
        assert_eq!(got[0].distance_squared, 0.85f32 * 0.85f32);
        // The default graph stays strict even when another graph relaxed.
        let mut strict = MatchGraph::default();
        assert_eq!(strict.between(&fs, 0, 1).count(), 0);
    }
}
