//! Operation-local replay of the SAME immutable owned complete arrangement.
//! Entries hold strong Arcs, preventing pointer reuse or safe in-place mutation.
//! Every new outer operation starts empty; failed replays are never cached.
use super::*;
use std::{cell::RefCell, marker::PhantomData, rc::Rc, sync::Arc};
const MAX_PINNED: usize = 64;
thread_local! {static ACTIVE:RefCell<Option<Vec<Arc<OwnedPeriodicArrangementCuts>>>>=const{RefCell::new(None)};}
#[cfg(test)]
thread_local! {static DISABLED:std::cell::Cell<usize>=const{std::cell::Cell::new(0)};static COUNTS:std::cell::Cell<(usize,usize)>=const{std::cell::Cell::new((0,0))};}
#[cfg(test)]
fn disabled() -> bool {
    DISABLED.with(|v| v.get() != 0)
}
#[cfg(not(test))]
fn disabled() -> bool {
    false
}
#[must_use = "Keep the replay scope alive for the complete source operation"]
pub(super) struct Scope {
    outer: bool,
    _thread: PhantomData<Rc<()>>,
}
pub(super) fn scope() -> Scope {
    let outer = if disabled() {
        false
    } else {
        ACTIVE.with(|s| {
            let mut state = s.borrow_mut();
            if state.is_none() {
                *state = Some(Vec::new());
                true
            } else {
                false
            }
        })
    };
    Scope {
        outer,
        _thread: PhantomData,
    }
}
impl Drop for Scope {
    fn drop(&mut self) {
        if self.outer {
            let entries = ACTIVE.with(|s| s.borrow_mut().take());
            drop(entries)
        }
    }
}
pub(super) fn verify(owner: &Arc<OwnedPeriodicArrangementCuts>) -> Result<()> {
    if !disabled()
        && ACTIVE.with(|s| {
            s.borrow()
                .as_ref()
                .is_some_and(|v| v.iter().any(|p| Arc::ptr_eq(p, owner)))
        })
    {
        #[cfg(test)]
        COUNTS.with(|c| {
            let (cold, hits) = c.get();
            c.set((cold, hits.saturating_add(1)))
        });
        return Ok(());
    }
    #[cfg(test)]
    COUNTS.with(|c| {
        let (cold, hits) = c.get();
        c.set((cold.saturating_add(1), hits))
    });
    // No RefCell borrow is held during independent source replay.
    owner.reverify_source()?;
    if !disabled() {
        ACTIVE.with(|s| {
            if let Some(entries) = s.borrow_mut().as_mut() {
                if entries.len() < MAX_PINNED && !entries.iter().any(|p| Arc::ptr_eq(p, owner)) {
                    entries.push(owner.clone())
                }
            }
        })
    }
    Ok(())
}
#[cfg(test)]
fn counts() -> (usize, usize) {
    COUNTS.with(|c| c.get())
}
#[cfg(test)]
fn reset_counts() {
    COUNTS.with(|c| c.set((0, 0)))
}
#[cfg(test)]
fn without<T>(f: impl FnOnce() -> T) -> T {
    struct Disabled;
    impl Drop for Disabled {
        fn drop(&mut self) {
            DISABLED.with(|v| v.set(v.get() - 1))
        }
    }
    DISABLED.with(|v| v.set(v.get() + 1));
    let _guard = Disabled;
    f()
}
#[cfg(test)]
mod tests {
    use super::*;
    fn source() -> Arc<OwnedPeriodicArrangementCuts> {
        let curve = Curve::from_polyline(vec![vec![0.25, 0.25], vec![0.75, 0.25]]).unwrap();
        let pieces = Arc::new(
            own_original_periodic_pieces(
                &[curve],
                [[0., 1.], [0., 1.]],
                [false, false],
                1e-12,
                ParameterOrderLimits::default(),
                16,
            )
            .unwrap(),
        );
        Arc::new(
            own_periodic_arrangement_cuts(pieces, 1e-12, ParameterOrderLimits::default(), 100)
                .unwrap(),
        )
    }
    #[test]
    fn one_operation_reuses_only_the_same_pinned_arc_and_nested_scopes() {
        let a = source();
        let b = Arc::new(a.as_ref().clone());
        reset_counts();
        {
            let _scope = scope();
            verify(&a).unwrap();
            verify(&a).unwrap();
            {
                let _nested = scope();
                verify(&a).unwrap();
            }
            verify(&b).unwrap();
            assert_eq!(counts(), (2, 2));
            assert!(Arc::strong_count(&a) >= 2);
        }
        {
            let _next = scope();
            verify(&a).unwrap();
        }
        assert_eq!(counts(), (3, 2));
    }
    #[test]
    fn failed_source_replays_are_never_promoted_or_memoized() {
        let source = source();
        let bad = super::super::periodic_arrangement::corrupt_for_replay(&source);
        reset_counts();
        {
            let _scope = scope();
            assert!(verify(&bad).is_err());
            assert!(verify(&bad).is_err());
            assert_eq!(counts(), (2, 0));
            assert_eq!(Arc::strong_count(&bad), 1);
        }
    }
    #[test]
    fn scope_cleanup_releases_pins_after_return_error_and_unwind() {
        let a = source();
        let weak = Arc::downgrade(&a);
        {
            let _scope = scope();
            verify(&a).unwrap();
            drop(a);
            assert!(weak.upgrade().is_some());
        }
        assert!(weak.upgrade().is_none());
        let a = source();
        let result = (|| -> Result<()> {
            let _scope = scope();
            verify(&a)?;
            Err(refused("deliberate replay scope error"))
        })();
        assert!(result.is_err());
        assert_eq!(Arc::strong_count(&a), 1);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _scope = scope();
            verify(&a).unwrap();
            panic!("deliberate replay scope unwind")
        }));
        assert!(result.is_err());
        assert_eq!(Arc::strong_count(&a), 1);
    }
    #[test]
    fn bounded_capacity_falls_back_to_full_replay_instead_of_skipping_proofs() {
        let a = source();
        let owners = (0..MAX_PINNED + 1)
            .map(|_| Arc::new(a.as_ref().clone()))
            .collect::<Vec<_>>();
        reset_counts();
        {
            let _scope = scope();
            for owner in &owners {
                verify(owner).unwrap()
            }
            verify(owners.last().unwrap()).unwrap();
            verify(&owners[0]).unwrap();
            assert_eq!(counts(), (MAX_PINNED + 2, 1));
        }
    }
    #[test]
    fn independent_threads_have_independent_fresh_source_operations() {
        let a = source();
        let threads = (0..2)
            .map(|_| {
                let a = a.clone();
                std::thread::spawn(move || {
                    reset_counts();
                    let _scope = scope();
                    verify(&a).unwrap();
                    verify(&a).unwrap();
                    assert_eq!(counts(), (1, 1));
                })
            })
            .collect::<Vec<_>>();
        for thread in threads {
            thread.join().unwrap()
        }
        assert_eq!(Arc::strong_count(&a), 1);
    }
    #[test]
    #[ignore = "Explicit alternating paired measurement, run separately from correctness tests"]
    fn paired_actual_owned_chart_constructor_measurement() {
        use std::time::Instant;
        let input = super::super::periodic_chart_cycles::actual_fan_tests::build(
            super::super::periodic_support_regions::tests::discs(true),
            2,
        );
        let intervals = input.owner().owner().clone();
        let expected = (
            input.owner().vertices().to_vec(),
            input.owner().edges().to_vec(),
            input.owner().walks().to_vec(),
        );
        for (run, enabled) in [false, true, true, false, false, true, true, false]
            .into_iter()
            .enumerate()
        {
            reset_counts();
            let started = Instant::now();
            let build = || {
                construct_periodic_chart_winding(
                    intervals.clone(),
                    PeriodicChartCycleLimits::default(),
                    PeriodicChartWindingLimits::default(),
                )
                .unwrap()
            };
            let output = if enabled { build() } else { without(build) };
            let elapsed = started.elapsed();
            let (cold, hits) = counts();
            assert_eq!(
                (
                    output.owner().vertices().to_vec(),
                    output.owner().edges().to_vec(),
                    output.owner().walks().to_vec()
                ),
                expected
            );
            let p = [["1".into(), "4".into()], ["1".into(), "4".into()]];
            assert_eq!(output.material(&p).unwrap(), Some(true));
            eprintln!(
                "REPLAY_PAIRED run={run} enabled={enabled} elapsed_ns={} cold={cold} hits={hits}",
                elapsed.as_nanos()
            );
        }
    }
}
