//! Native reuse of original-curve length enclosures across station counts.
//! Every returned report retains its original cell charge and exact work policy.
use super::{Cell, Curve, LengthReport, Result, measure_partition};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, OnceLock},
};

type Partition = Arc<(LengthReport, Vec<Cell>)>;
type Entries = VecDeque<(Vec<u64>, Partition)>;
const MAX_ENTRIES: usize = 8;
const MAX_CELLS: usize = 16_384;
static CACHE: OnceLock<Mutex<Entries>> = OnceLock::new();

fn key(curve: &Curve, tolerance: f64, max_cells: usize) -> Vec<u64> {
    // Binary source identity, including lengths and signed zero; no sampled or
    // rounded key and no caller-supplied certificate can enter this cache.
    let mut key = vec![
        curve.degree as u64,
        curve.periodic as u64,
        curve.knots.len() as u64,
        curve.control_points.len() as u64,
        curve.weights.len() as u64,
        tolerance.to_bits(),
        max_cells as u64,
    ];
    key.extend(curve.knots.iter().map(|v| v.to_bits()));
    for point in &curve.control_points {
        key.push(point.len() as u64);
        key.extend(point.iter().map(|v| v.to_bits()));
    }
    key.extend(curve.weights.iter().map(|v| v.to_bits()));
    key
}

pub(super) fn get(curve: &Curve, tolerance: f64, max_cells: usize) -> Result<Partition> {
    let key = key(curve, tolerance, max_cells);
    let cache = CACHE.get_or_init(Default::default);
    if let Ok(entries) = cache.lock() {
        if let Some((_, partition)) = entries.iter().find(|(source, _)| *source == key) {
            return Ok(partition.clone());
        }
    }
    // Compute outside the lock. Budget failures and partial reports remain
    // unchanged; even cached successful work is charged in full by the caller.
    let result = Arc::new(measure_partition(
        curve,
        curve.domain(),
        tolerance,
        max_cells,
    )?);
    if result.1.len() <= MAX_CELLS {
        if let Ok(mut entries) = cache.lock() {
            if let Some((_, partition)) = entries.iter().find(|(source, _)| *source == key) {
                return Ok(partition.clone());
            }
            while entries.len() >= MAX_ENTRIES
                || entries.iter().map(|(_, p)| p.1.len()).sum::<usize>() + result.1.len()
                    > MAX_CELLS
            {
                entries.pop_front();
            }
            entries.push_back((key, result.clone()));
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> Curve {
        Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![
                vec![0., 0., 0.],
                vec![1., 2., 0.],
                vec![3., -1., 1.],
                vec![4., 1., 0.],
            ],
            weights: vec![1., 0.5, 2., 1.],
            periodic: false,
        }
    }

    fn assert_original(curve: &Curve, partition: &Partition, tolerance: f64, budget: usize) {
        let fresh = measure_partition(curve, curve.domain(), tolerance, budget).unwrap();
        assert_eq!(format!("{:?}", partition.0), format!("{:?}", fresh.0));
        assert_eq!(partition.1.len(), fresh.1.len());
        for (a, b) in partition.1.iter().zip(&fresh.1) {
            assert_eq!((a.span, a.serial), (b.span, b.serial));
            assert_eq!(a.domain.map(f64::to_bits), b.domain.map(f64::to_bits));
            assert_eq!(
                [a.length.lo, a.length.hi].map(f64::to_bits),
                [b.length.lo, b.length.hi].map(f64::to_bits)
            );
        }
    }

    #[test]
    fn repeated_source_preserves_every_original_enclosure_and_cell_charge() {
        let curve = source();
        let first = get(&curve, 0.0001, 10_000).unwrap();
        let repeated = get(&curve.clone(), 0.0001, 10_000).unwrap();
        assert!(Arc::ptr_eq(&first, &repeated));
        assert_original(&curve, &repeated, 0.0001, 10_000);
        assert!(repeated.0.cells > 0 && repeated.0.cells <= 10_000);
        let entries = CACHE.get().unwrap().lock().unwrap();
        assert!(entries.len() <= MAX_ENTRIES);
        assert!(entries.iter().map(|(_, p)| p.1.len()).sum::<usize>() <= MAX_CELLS);
    }

    #[test]
    fn source_weight_and_work_policy_changes_cannot_reuse_a_positive_partition() {
        let curve = source();
        let first = get(&curve, 0.0002, 10_000).unwrap();
        let mut changed = curve.clone();
        changed.weights[1] = f64::from_bits(changed.weights[1].to_bits() + 1);
        let mutation = get(&changed, 0.0002, 10_000).unwrap();
        assert!(!Arc::ptr_eq(&first, &mutation));
        assert_original(&changed, &mutation, 0.0002, 10_000);
        let tighter = get(&curve, 0.0001, 10_000).unwrap();
        assert!(!Arc::ptr_eq(&first, &tighter));
        assert_original(&curve, &tighter, 0.0001, 10_000);
        let short = get(&curve, 0.0002, 1).unwrap();
        assert!(!Arc::ptr_eq(&first, &short));
        assert!(!short.0.within_tolerance);
        assert_original(&curve, &short, 0.0002, 1);
        assert!(super::super::divide_by_length(&curve, 16, 0.0002, 0).is_err());
        let division = super::super::divide_by_length(&curve, 16, 0.0002, 1).unwrap();
        assert!(!division.within_tolerance && division.cells <= 1);
    }
}
