use crate::CudaError;

/// The same gate is used by native submission and CPU boundary tests. Validation
/// errors never grant a permit; execution errors permanently poison the state.
#[derive(Default)]
pub(super) struct ExecutionState {
    pub poisoned: bool,
}
pub(super) struct Permit<'a>(&'a mut ExecutionState);
impl ExecutionState {
    pub fn check(&self) -> Result<(), CudaError> {
        if self.poisoned {
            Err(CudaError::ProgramPoisoned)
        } else {
            Ok(())
        }
    }
    pub fn begin(
        &mut self,
        validate: impl FnOnce() -> Result<(), CudaError>,
    ) -> Result<Permit<'_>, CudaError> {
        self.check()?;
        validate()?;
        Ok(Permit(self))
    }
}
impl Permit<'_> {
    pub fn execute(self, enqueue: impl FnOnce() -> Result<(), CudaError>) -> Result<(), CudaError> {
        let result = enqueue();
        if result.is_err() {
            self.0.poisoned = true;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::super::validation::{self, Binding};
    use super::*;
    use std::cell::Cell;
    use tensor_core::{Layout, MatmulPrecision, Shape};
    #[test]
    fn late_invalid_output_cannot_enqueue_and_validation_errors_are_reusable() {
        let mut state = ExecutionState::default();
        let launches = Cell::new(0);
        let layout = Layout::contiguous(Shape::new(vec![3]).unwrap()).unwrap();
        let expected = vec![layout.shape().clone(); 2];
        let binding = |id| Binding {
            layout: &layout,
            storage_len: 3,
            allocation: id,
            unique: true,
        };
        let result = state
            .begin(|| validation::outputs(&expected, &[binding(1)], &[binding(2), binding(1)]));
        assert!(result.is_err());
        assert_eq!(launches.get(), 0);
        assert!(!state.poisoned);
        state
            .begin(|| validation::outputs(&expected, &[binding(1)], &[binding(2), binding(3)]))
            .unwrap()
            .execute(|| {
                launches.set(launches.get() + 1);
                Ok(())
            })
            .unwrap();
        assert_eq!(launches.get(), 1);
    }
    #[test]
    fn late_precision_failure_blocks_enqueue_without_poisoning() {
        let mut state = ExecutionState::default();
        let launches = Cell::new(0);
        let precisions = [MatmulPrecision::F32, MatmulPrecision::AllowTf32];
        let result = state.begin(|| {
            validation::precisions(&precisions, |p| {
                if p == MatmulPrecision::AllowTf32 {
                    Err(CudaError::UnsupportedPrecision("TF32 disabled"))
                } else {
                    Ok(())
                }
            })
        });
        assert!(result.is_err());
        assert_eq!(launches.get(), 0);
        assert!(!state.poisoned);
        state
            .begin(|| validation::precisions(&precisions, |_| Ok(())))
            .unwrap()
            .execute(|| {
                launches.set(1);
                Ok(())
            })
            .unwrap();
        assert_eq!(launches.get(), 1);
    }
    #[test]
    fn an_enqueue_error_permanently_blocks_future_validation_and_launches() {
        let mut state = ExecutionState::default();
        let launches = Cell::new(0);
        let validations = Cell::new(0);
        state
            .begin(|| Ok(()))
            .unwrap()
            .execute(|| {
                launches.set(1);
                Err(CudaError::InvalidInput("mock driver enqueue error"))
            })
            .unwrap_err();
        assert!(state.poisoned);
        let result = state.begin(|| {
            validations.set(1);
            Ok(())
        });
        assert!(matches!(result, Err(CudaError::ProgramPoisoned)));
        assert_eq!((validations.get(), launches.get()), (0, 1));
    }
}
