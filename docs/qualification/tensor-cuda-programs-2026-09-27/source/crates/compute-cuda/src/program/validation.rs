use crate::CudaError;
use tensor_core::{Layout, Shape};

pub(super) struct Binding<'a> {
    pub layout: &'a Layout,
    pub storage_len: usize,
    pub allocation: usize,
    pub unique: bool,
}
// Runtime ownership is checked by CudaRuntime::check before these layout and
// alias checks. This pure boundary also supports real pre-enqueue unit tests.
pub(super) fn inputs(expected: &[Layout], actual: &[Binding<'_>]) -> Result<(), CudaError> {
    if actual.len() != expected.len() {
        return Err(CudaError::InvalidInput(
            "prepared CUDA input count mismatch",
        ));
    }
    for (expected, actual) in expected.iter().zip(actual) {
        actual.layout.validate_storage_len(actual.storage_len)?;
        if actual.layout != expected {
            return Err(CudaError::InvalidInput(
                "prepared CUDA input layout mismatch",
            ));
        }
    }
    Ok(())
}
pub(super) fn outputs(
    expected: &[Shape],
    inputs: &[Binding<'_>],
    actual: &[Binding<'_>],
) -> Result<(), CudaError> {
    if actual.len() != expected.len() {
        return Err(CudaError::InvalidInput(
            "prepared CUDA output count mismatch",
        ));
    }
    for (i, (expected, binding)) in expected.iter().zip(actual).enumerate() {
        binding.layout.validate_storage_len(binding.storage_len)?;
        if binding.layout.shape() != expected {
            return Err(CudaError::InvalidInput(
                "prepared CUDA output shape mismatch",
            ));
        }
        if !binding.unique || !binding.layout.is_contiguous() || binding.layout.offset() != 0 {
            return Err(CudaError::SharedOutput);
        }
        if inputs
            .iter()
            .any(|input| input.allocation == binding.allocation)
            || actual[..i]
                .iter()
                .any(|previous| previous.allocation == binding.allocation)
        {
            return Err(CudaError::InvalidInput(
                "prepared CUDA outputs alias input or output storage",
            ));
        }
    }
    Ok(())
}

pub(super) fn precisions(
    requests: &[tensor_core::MatmulPrecision],
    mut validate: impl FnMut(tensor_core::MatmulPrecision) -> Result<(), CudaError>,
) -> Result<(), CudaError> {
    for &precision in requests {
        validate(precision)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn shape(dims: &[usize]) -> Shape {
        Shape::new(dims.to_vec()).unwrap()
    }
    fn dense(dims: &[usize]) -> Layout {
        Layout::contiguous(shape(dims)).unwrap()
    }
    fn binding(layout: &Layout, allocation: usize) -> Binding<'_> {
        Binding {
            layout,
            storage_len: layout.required_storage_len().unwrap().max(1),
            allocation,
            unique: true,
        }
    }
    #[test]
    fn fixed_inputs_accept_new_storage_and_readonly_aliases_but_reject_changed_views() {
        let layout = Layout::new(shape(&[2, 3]), vec![1, 2], 7).unwrap();
        assert!(
            inputs(
                &[layout.clone(), layout.clone()],
                &[binding(&layout, 10), binding(&layout, 10)]
            )
            .is_ok()
        );
        assert!(inputs(std::slice::from_ref(&layout), &[binding(&layout, 99)]).is_ok());
        assert!(inputs(std::slice::from_ref(&layout), &[]).is_err());
        let contiguous = dense(&[2, 3]);
        assert!(inputs(std::slice::from_ref(&layout), &[binding(&contiguous, 10)]).is_err());
        let mut short = binding(&layout, 10);
        short.storage_len = 12;
        assert!(inputs(std::slice::from_ref(&layout), &[short]).is_err());
    }
    #[test]
    fn all_outputs_are_validated_before_late_alias_or_layout_failure() {
        let layout = dense(&[2, 3]);
        let expected = [shape(&[2, 3]), shape(&[2, 3])];
        let input = [binding(&layout, 1)];
        assert!(
            outputs(
                &expected,
                &input,
                &[binding(&layout, 2), binding(&layout, 3)]
            )
            .is_ok()
        );
        for identity in [1, 2] {
            assert!(
                outputs(
                    &expected,
                    &input,
                    &[binding(&layout, 2), binding(&layout, identity)]
                )
                .is_err()
            );
        }
        let offset = Layout::new(shape(&[2, 3]), vec![3, 1], 1).unwrap();
        let strided = Layout::new(shape(&[2, 3]), vec![1, 2], 0).unwrap();
        for invalid in [&offset, &strided] {
            assert!(
                outputs(
                    &expected,
                    &input,
                    &[binding(&layout, 2), binding(invalid, 3)]
                )
                .is_err()
            );
        }
        let mut shared = binding(&layout, 3);
        shared.unique = false;
        assert!(matches!(
            outputs(&expected, &input, &[binding(&layout, 2), shared]),
            Err(CudaError::SharedOutput)
        ));
    }
    #[test]
    fn scalar_and_empty_dense_outputs_still_need_unique_storage_and_matching_shape() {
        for dims in [vec![], vec![2, 0, 3]] {
            let layout = Layout::contiguous(shape(&dims)).unwrap();
            assert!(outputs(&[shape(&dims)], &[], &[binding(&layout, 1)]).is_ok());
            let mut shared = binding(&layout, 1);
            shared.unique = false;
            assert!(outputs(&[shape(&dims)], &[], &[shared]).is_err());
        }
        let wrong = dense(&[1]);
        assert!(outputs(&[shape(&[])], &[], &[binding(&wrong, 1)]).is_err());
    }
}
