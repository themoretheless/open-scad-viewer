//! Stable child index selection, preserving operand order and duplicate slots.
#[derive(Debug, PartialEq)]
pub enum RangeIssue {
    Invalid,
    Limit,
}
pub fn expand_range(start: f64, step: f64, end: f64, maximum: u64) -> Result<Vec<f64>, RangeIssue> {
    if step == 0. || ![start, step, end].iter().all(|v| v.is_finite()) {
        return Err(RangeIssue::Invalid);
    }
    let epsilon = 1_f64.max(start.abs()).max(end.abs()) * 1e-12;
    let mut output = Vec::new();
    let mut item = start;
    while if step > 0. {
        item <= end + epsilon
    } else {
        item >= end - epsilon
    } {
        if output.len() as u64 >= maximum {
            return Err(RangeIssue::Limit);
        }
        output.push(item);
        item += step;
    }
    Ok(output)
}
#[derive(Debug, PartialEq)]
pub enum Issue {
    Invalid { candidate: usize },
    OutOfBounds { index: f64 },
}
#[derive(Debug, PartialEq)]
pub struct Selection {
    pub indices: Vec<u64>,
    pub issues: Vec<Issue>,
}
pub fn select(candidates: &[Option<f64>], child_count: u64) -> Selection {
    let mut result = Selection {
        indices: Vec::new(),
        issues: Vec::new(),
    };
    for (candidate, value) in candidates.iter().enumerate() {
        let Some(value) = value.filter(|v| v.is_finite()) else {
            result.issues.push(Issue::Invalid { candidate });
            continue;
        };
        let truncated = value.trunc();
        let index = if truncated == 0. { 0. } else { truncated };
        if index < 0. || index >= child_count as f64 {
            result.issues.push(Issue::OutOfBounds { index });
        } else {
            result.indices.push(index as u64);
        }
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ranges_preserve_direction_epsilon_and_work_limit() {
        assert_eq!(expand_range(2., -1., 0., 3), Ok(vec![2., 1., 0.]));
        assert_eq!(expand_range(0., 1., 2., 2), Err(RangeIssue::Limit));
        assert_eq!(expand_range(0., 0., 2., 3), Err(RangeIssue::Invalid));
        assert_eq!(expand_range(f64::NAN, 1., 2., 3), Err(RangeIssue::Invalid));
        assert!(expand_range(2., 1., 0., 3).unwrap().is_empty());
        let fractional = expand_range(0., 0.1, 0.3, 4).unwrap();
        assert_eq!(fractional.len(), 4);
        assert_eq!(fractional[3], 0.30000000000000004);
        assert_eq!(expand_range(1e20, 1., 1e20, 4), Err(RangeIssue::Limit));
    }
    #[test]
    fn truncation_duplicates_invalid_values_and_empty_children() {
        let result = select(
            &[
                Some(-0.5),
                Some(2.9),
                Some(2.),
                None,
                Some(f64::NAN),
                Some(-1.),
                Some(3.),
            ],
            3,
        );
        assert_eq!(result.indices, vec![0, 2, 2]);
        assert_eq!(
            result.issues,
            vec![
                Issue::Invalid { candidate: 3 },
                Issue::Invalid { candidate: 4 },
                Issue::OutOfBounds { index: -1. },
                Issue::OutOfBounds { index: 3. }
            ]
        );
        assert_eq!(
            select(&[Some(0.)], 0).issues,
            vec![Issue::OutOfBounds { index: 0. }]
        );
    }
}
