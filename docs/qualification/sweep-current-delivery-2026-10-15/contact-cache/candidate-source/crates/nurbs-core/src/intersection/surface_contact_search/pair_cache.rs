//! Same-audit reuse of exact interval routine results. Full coefficient bits
//! bind each entry; no proof outcome, domain decision or work count is cached.
use crate::{surface::Surface, surface_contact::SurfaceEvaluationCache, surface_distance::SurfaceControlCache};
use std::collections::VecDeque;

const SURFACES: usize = 8;
const SOURCE_CONTROLS: usize = 4096;

pub(super) struct CachedSurface {
    source: Surface,
    pub(super) evaluation: SurfaceEvaluationCache,
    pub(super) control: SurfaceControlCache,
}

/// Bounded scratch storage for one immutable sweep face-pair audit. Existing
/// per-surface cache limits still apply. Larger charts use the uncached path.
#[derive(Default)]
pub struct PreparedSweepContactSearch {
    entries: VecDeque<CachedSurface>,
}

fn same_bits(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
}
fn same_source(a: &Surface, b: &Surface) -> bool {
    a.degree_u == b.degree_u && a.degree_v == b.degree_v
        && a.periodic_u == b.periodic_u && a.periodic_v == b.periodic_v
        && same_bits(&a.knots_u, &b.knots_u) && same_bits(&a.knots_v, &b.knots_v)
        && a.weights.len() == b.weights.len()
        && a.weights.iter().zip(&b.weights).all(|(x, y)| same_bits(x, y))
        && a.control_points.len() == b.control_points.len()
        && a.control_points.iter().zip(&b.control_points).all(|(x, y)| {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same_bits(p, q))
        })
}
impl PreparedSweepContactSearch {
    pub(super) fn eligible(surface: &Surface) -> bool {
        surface.control_points.iter().try_fold(0usize, |n, row| n.checked_add(row.len()))
            .is_some_and(|n| n <= SOURCE_CONTROLS)
    }
    pub(super) fn take(&mut self, surface: &Surface) -> CachedSurface {
        if let Some(index) = self.entries.iter().position(|entry| same_source(&entry.source, surface)) {
            return self.entries.remove(index).unwrap();
        }
        CachedSurface { source: surface.clone(), evaluation: Default::default(), control: Default::default() }
    }
    pub(super) fn put(&mut self, entry: CachedSurface) {
        if self.entries.len() == SURFACES { self.entries.pop_front(); }
        self.entries.push_back(entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identity_checks_every_coefficient_bit_including_signed_zero() {
        let source = Surface {degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
            weights:vec![vec![1.;2];2],control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]],vec![vec![1.,0.,0.],vec![1.,1.,0.]]],
            periodic_u:false,periodic_v:false};
        assert!(same_source(&source,&source.clone()));
        let mut changed=source.clone();changed.control_points[0][0][0]=-0.;
        assert!(!same_source(&source,&changed));
        let mut changed=source.clone();changed.weights[1][1]=1f64.next_up();
        assert!(!same_source(&source,&changed));
        let mut changed=source.clone();changed.knots_u[0]=-0.;
        assert!(!same_source(&source,&changed));
    }
}
