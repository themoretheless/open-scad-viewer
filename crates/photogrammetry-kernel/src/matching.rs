//! A per-run, direction-independent correspondence cache and unambiguous track associations.
use crate::features::{self, Feature, Match};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(crate) struct MatchGraph {
    pairs: BTreeMap<(usize, usize), Vec<Match>>,
    pub requests: usize,
    pub computed_pairs: usize,
}
impl MatchGraph {
    pub fn between<'a>(
        &'a mut self,
        features: &[Vec<Feature>],
        a: usize,
        b: usize,
    ) -> impl Iterator<Item = Match> + 'a {
        self.requests += 1;
        let (lo, hi) = (a.min(b), a.max(b));
        let pairs = self.pairs.entry((lo, hi)).or_insert_with(|| {
            self.computed_pairs += 1;
            features::matches(&features[lo], &features[hi])
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
pub(crate) struct AssociationVotes(BTreeMap<usize, BTreeMap<usize, usize>>);
impl AssociationVotes {
    pub fn add(&mut self, feature: usize, point: usize) {
        *self.0.entry(feature).or_default().entry(point).or_default() += 1;
    }
    pub fn resolve(self) -> (BTreeMap<usize, usize>, usize) {
        let mut candidates = Vec::new();
        let mut rejected = 0;
        for (feature, points) in self.0 {
            if points.len() != 1 {
                rejected += 1;
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
}
