//! Junction detection over native branch samples.
use super::distance;
use std::collections::BTreeSet;

pub(super) fn junction_components(samples: &[(usize, [f64; 3])], floor: f64) -> BTreeSet<usize> {
    let mut junctions = BTreeSet::new();
    for (index, (component, point)) in samples.iter().enumerate() {
        for (other, other_point) in &samples[index + 1..] {
            if component != other && distance(point, other_point) <= floor * 8. {
                junctions.insert(*component);
                junctions.insert(*other);
            }
        }
    }
    junctions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_both_distinct_branches_at_the_inclusive_distance_limit() {
        let samples = [(3, [0.; 3]), (7, [0., 0., 8.]), (9, [20., 0., 0.])];
        assert_eq!(junction_components(&samples, 1.), BTreeSet::from([3, 7]));
    }

    #[test]
    fn repeated_samples_on_one_branch_do_not_create_a_junction() {
        assert!(junction_components(&[(2, [0.; 3]), (2, [0.; 3])], 1.).is_empty());
        assert!(junction_components(&[], 1.).is_empty());
    }
}
