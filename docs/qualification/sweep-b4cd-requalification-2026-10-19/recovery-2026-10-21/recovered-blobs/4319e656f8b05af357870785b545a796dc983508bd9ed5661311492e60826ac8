//! Shared skeleton of the subdivision distance searches: a queue of cells keyed
//! by a proven lower bound, a best-witness upper bound, and a bisection loop
//! with tolerance, work-limit, and precision stops. Cell enclosures, split
//! geometry, and witness sampling stay in the per-dimension modules.
use crate::foundation::guards::Budget;
use crate::{DistanceStopReason, Result};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// Queue policy for subdivision cells. Every queue keeps cells keyed by a
/// proven lower distance bound and drops cells that can no longer improve the
/// best upper bound.
pub(crate) trait Queue<C> {
    fn push(&mut self, cell: C, lower: f64);
    /// Smallest lower bound currently queued.
    fn min_lower(&self) -> Option<f64>;
    /// Drop every cell whose lower bound exceeds `upper`.
    fn prune(&mut self, upper: f64);
    /// Remove the minimum-lower cell together with its bound; ties follow the
    /// queue's own rule.
    fn pop_min(&mut self) -> Option<(C, f64)>;
    fn is_empty(&self) -> bool;
}

/// Priority queue: smallest lower bound first, earliest insertion among ties.
pub(crate) struct HeapQueue<C> {
    heap: BinaryHeap<Entry<C>>,
    next: usize,
}
struct Entry<C> {
    lower: f64,
    order: usize,
    cell: C,
}
impl<C> PartialEq for Entry<C> {
    fn eq(&self, b: &Self) -> bool {
        self.lower == b.lower && self.order == b.order
    }
}
impl<C> Eq for Entry<C> {}
impl<C> PartialOrd for Entry<C> {
    fn partial_cmp(&self, b: &Self) -> Option<Ordering> {
        Some(self.cmp(b))
    }
}
impl<C> Ord for Entry<C> {
    fn cmp(&self, b: &Self) -> Ordering {
        b.lower
            .total_cmp(&self.lower)
            .then_with(|| b.order.cmp(&self.order))
    }
}
impl<C> Default for HeapQueue<C> {
    fn default() -> Self {
        Self {
            heap: BinaryHeap::new(),
            next: 0,
        }
    }
}
impl<C> Queue<C> for HeapQueue<C> {
    fn push(&mut self, cell: C, lower: f64) {
        self.heap.push(Entry {
            lower,
            order: self.next,
            cell,
        });
        self.next += 1;
    }
    fn min_lower(&self) -> Option<f64> {
        self.heap.peek().map(|e| e.lower)
    }
    fn prune(&mut self, upper: f64) {
        while self.heap.peek().is_some_and(|e| e.lower > upper) {
            self.heap.pop();
        }
    }
    fn pop_min(&mut self) -> Option<(C, f64)> {
        self.heap.pop().map(|e| (e.cell, e.lower))
    }
    fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }
}

/// Insertion-order queue with a linear minimum scan. Among equal minima the
/// last entry is removed first, and removal swaps with the last entry.
pub(crate) struct ScanQueue<C> {
    cells: Vec<(f64, C)>,
}
impl<C> Default for ScanQueue<C> {
    fn default() -> Self {
        Self { cells: Vec::new() }
    }
}
impl<C> ScanQueue<C> {
    pub(crate) fn len(&self) -> usize {
        self.cells.len()
    }
}
impl<C> Queue<C> for ScanQueue<C> {
    fn push(&mut self, cell: C, lower: f64) {
        self.cells.push((lower, cell));
    }
    fn min_lower(&self) -> Option<f64> {
        self.cells
            .iter()
            .map(|(lower, _)| *lower)
            .reduce(f64::min)
    }
    fn prune(&mut self, upper: f64) {
        self.cells.retain(|(lower, _)| *lower <= upper);
    }
    fn pop_min(&mut self) -> Option<(C, f64)> {
        let index = self
            .cells
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.0.total_cmp(&b.1.0))
            .map(|(index, _)| index)?;
        let (lower, cell) = self.cells.swap_remove(index);
        Some((cell, lower))
    }
    fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

/// Result of splitting one queued cell.
pub(crate) enum Split<C> {
    /// The cell cannot be subdivided further; the search stops at the
    /// precision limit and the cell is requeued so its lower bound still
    /// bounds the answer.
    Precision(C),
    /// Two halves in split order. `Some((cell, lower))` requeues a half;
    /// `None` drops it. Every half counts against the work budget, queued or
    /// not.
    Children([Option<(C, f64)>; 2]),
}

/// Per-dimension hooks of the subdivision distance search.
pub(crate) trait Search {
    type Cell;
    /// Proven upper distance bound from the best witness, if one exists.
    fn upper(&self) -> Option<f64>;
    /// Extra stop reasons, checked after the tolerance stop and before the
    /// work-limit stop. `queue_empty` reflects the queue after pruning.
    fn early_stop(&self, queue_empty: bool) -> Option<DistanceStopReason> {
        let _ = queue_empty;
        None
    }
    /// Split a cell into two halves, sampling witnesses (and thereby raising
    /// the upper bound) while constructing them.
    fn split(&mut self, cell: Self::Cell) -> Result<Split<Self::Cell>>;
}

/// How the loop stopped, how many cells were created, and the final global
/// lower bound.
pub(crate) struct Outcome {
    pub reason: DistanceStopReason,
    pub cells: usize,
    pub lower: f64,
}

/// Run the subdivision loop over the queued initial cells. `cells` is the
/// number of cells already created during initialization; every split half
/// adds one more, and the loop stops before creation would exceed `max_cells`.
pub(crate) fn run<S: Search>(
    search: &mut S,
    queue: &mut impl Queue<S::Cell>,
    mut cells: usize,
    max_cells: usize,
    tolerance: f64,
) -> Result<Outcome> {
    let reason;
    // Unified guard backing the max_cells work limit (item 1065): each loop
    // iteration creates at most two cells and the work-limit stop fires first,
    // so this guard is pure runaway insurance.
    let mut guard = Budget::with_iterations(max_cells.max(1))?.guard("distance_subdivision");
    loop {
        guard.tick()?;
        guard.check()?;
        let upper = search.upper();
        queue.prune(upper.unwrap_or(f64::INFINITY));
        let lower = queue
            .min_lower()
            .unwrap_or_else(|| upper.unwrap_or(0.))
            .min(upper.unwrap_or(f64::INFINITY));
        if upper.is_some_and(|u| u - lower <= tolerance) {
            reason = DistanceStopReason::Tolerance;
            break;
        }
        if let Some(stop) = search.early_stop(queue.is_empty()) {
            reason = stop;
            break;
        }
        if cells + 2 > max_cells {
            reason = DistanceStopReason::WorkLimit;
            break;
        }
        let (cell, lower) = queue.pop_min().unwrap();
        match search.split(cell)? {
            Split::Precision(cell) => {
                queue.push(cell, lower);
                reason = DistanceStopReason::PrecisionLimit;
                break;
            }
            Split::Children(children) => {
                for child in children {
                    cells += 1;
                    if let Some((cell, lower)) = child {
                        queue.push(cell, lower);
                    }
                }
            }
        }
    }
    let upper = search.upper();
    let lower = queue
        .min_lower()
        .unwrap_or_else(|| upper.unwrap_or(0.))
        .min(upper.unwrap_or(f64::INFINITY));
    Ok(Outcome {
        reason,
        cells,
        lower,
    })
}

#[cfg(test)]
mod guard_tests {
    use super::*;

    /// Trivial search that never improves: one fixed cell, always split.
    struct Nop;
    impl Search for Nop {
        type Cell = ();
        fn upper(&self) -> Option<f64> {
            Some(1.)
        }
        fn split(&mut self, cell: ()) -> Result<Split<()>> {
            Ok(Split::Children([Some(((), 0.)), Some(((), 0.))]))
        }
    }

    #[test]
    fn work_limit_stop_is_reached_before_the_guard_fires() {
        let mut search = Nop;
        let mut queue = HeapQueue::default();
        queue.push((), 0.);
        let outcome = run(&mut search, &mut queue, 1, 5, 1e-12).unwrap();
        assert_eq!(outcome.reason, DistanceStopReason::WorkLimit);
        assert!(outcome.cells <= 5);
    }

    #[test]
    fn guard_sized_at_work_limit_errors_one_tick_beyond() {
        let mut guard = Budget::with_iterations(5).unwrap().guard("distance_subdivision");
        for _ in 0..5 {
            guard.tick().unwrap();
        }
        let err = guard.tick().unwrap_err();
        assert_eq!(err.code, crate::RESOURCE_LIMIT, "{err:?}");
        assert!(err.contains("distance_subdivision"));
    }
}
