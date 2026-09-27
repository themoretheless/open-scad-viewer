//! Temporarily move primary/auxiliary destinations out of the pool so all
//! source borrows remain disjoint; restore both before propagating an error.
use crate::CudaError;

pub(super) fn with_outputs<T, R>(
    slots: &mut [Option<T>],
    primary: usize,
    auxiliary: Option<usize>,
    launch: impl FnOnce(&[Option<T>], &mut T, Option<&mut T>) -> Result<R, CudaError>,
) -> Result<R, CudaError> {
    if slots.get(primary).is_none_or(Option::is_none)
        || auxiliary.is_some_and(|i| i == primary || slots.get(i).is_none_or(Option::is_none))
    {
        return Err(CudaError::InvalidInput("invalid prepared scratch destinations"));
    }
    let mut main = slots[primary].take().unwrap();
    let mut extra = auxiliary.map(|i| slots[i].take().unwrap());
    let result = launch(slots, &mut main, extra.as_mut());
    slots[primary] = Some(main);
    if let Some(i) = auxiliary { slots[i] = extra; }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn two_destinations_are_disjoint_and_restored_after_enqueue_failure() {
        let mut slots = vec![Some(3), Some(4), Some(5)];
        let result: Result<(), _> = with_outputs(&mut slots, 2, Some(0), |sources, main, extra| {
            assert!(sources[0].is_none() && sources[2].is_none());
            assert_eq!(sources[1], Some(4));
            *main = 7;
            *extra.unwrap() = 8;
            Err(CudaError::InvalidInput("mock enqueue failure"))
        });
        assert!(result.is_err());
        assert_eq!(slots, [Some(8), Some(4), Some(7)]);
    }
    #[test]
    fn bad_slots_do_not_take_storage_or_call_launch() {
        let mut slots = vec![Some(3), None];
        for (primary, extra) in [(0, Some(0)), (0, Some(1)), (0, Some(2)), (1, None), (2, None)] {
            let result: Result<(), _> = with_outputs(&mut slots, primary, extra, |_, _, _| panic!("must not enqueue"));
            assert!(result.is_err());
            assert_eq!(slots, [Some(3), None]);
        }
    }
}
