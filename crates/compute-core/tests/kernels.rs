//! compute-core kernel tests: naga validation of the shipped WGSL, plus GPU
//! correctness against CPU references. GPU tests skip without an adapter.

use compute_core::gpu_compute::GpuContext;
use compute_core::shaders::{ALL, BLOCK_SUM_WGSL, SCALE_ADD_WGSL, ZIP_MUL_WGSL};
use compute_core::{Binding, Kernel, read_f32, storage_f32, storage_f32_zeroed, uniform_f32};

fn validate(name: &str, source: &str) {
    let module = naga::front::wgsl::parse_str(source)
        .unwrap_or_else(|error| panic!("{name}: WGSL parse failed: {error}"));
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    validator
        .validate(&module)
        .unwrap_or_else(|error| panic!("{name}: WGSL validation failed: {error}"));
}

#[test]
fn shipped_kernels_validate_with_naga() {
    for (name, source) in ALL {
        validate(name, source);
    }
}

#[test]
fn tuned_workgroup_substitution_stays_valid() {
    // Every power-of-two size a backend tuner may pick must still validate.
    let Some(context) = GpuContext::new() else { return };
    for size in [64u32, 128, 256, 512] {
        let source = SCALE_ADD_WGSL.replacen(
            "const WG: u32 = 256;",
            &format!("const WG: u32 = {size};"),
            1,
        );
        validate(&format!("scale_add@WG{size}"), &source);
        let kernel = Kernel::with_workgroup_size(
            &context.device,
            "scale_add",
            SCALE_ADD_WGSL,
            "main",
            &[Binding::Uniform, Binding::StorageRead, Binding::StorageReadWrite],
            size,
        );
        assert!(kernel.is_ok(), "WG{size} kernel failed: {:?}", kernel.err());
    }
}

#[test]
fn non_power_of_two_workgroup_is_rejected() {
    let Some(context) = GpuContext::new() else { return };
    let kernel = Kernel::with_workgroup_size(
        &context.device,
        "scale_add",
        SCALE_ADD_WGSL,
        "main",
        &[Binding::Uniform, Binding::StorageRead, Binding::StorageReadWrite],
        192,
    );
    assert!(kernel.is_err(), "non-power-of-two workgroup must be rejected");
}

#[test]
fn broken_wgsl_is_an_error_not_a_panic() {
    let Some(context) = GpuContext::new() else { return };
    let kernel = Kernel::new(
        &context.device,
        "broken",
        "fn oops( {",
        "main",
        &[],
    );
    assert!(kernel.is_err());
}

/// Params layout of scale_add: count(u32), scale(f32), offset(f32), pad.
fn scale_add_params(count: u32, scale: f32, offset: f32) -> Vec<f32> {
    let mut floats = vec![0.0f32; 4];
    floats[0] = f32::from_le_bytes(count.to_le_bytes());
    floats[1] = scale;
    floats[2] = offset;
    floats
}

#[test]
fn scale_add_matches_cpu_reference() {
    let Some(context) = GpuContext::new() else {
        eprintln!("compute-core: no GPU adapter available, skipping");
        return;
    };
    eprintln!("compute-core: computing on {}", context.backend_label());
    let device = &context.device;
    let queue = &context.queue;
    let kernel = Kernel::tuned(
        &context,
        "scale_add",
        SCALE_ADD_WGSL,
        "main",
        &[Binding::Uniform, Binding::StorageRead, Binding::StorageReadWrite],
        128,
        256,
    )
    .expect("scale_add builds");

    let input: Vec<f32> = (0..1000).map(|i| (i as f32 - 500.0) * 0.25).collect();
    let params = uniform_f32(device, queue, &scale_add_params(input.len() as u32, 2.5, -1.25));
    let input_buf = storage_f32(device, queue, &input);
    let output_buf = storage_f32_zeroed(device, queue, input.len());
    kernel.dispatch(device, queue, &[&params, &input_buf, &output_buf], input.len() as u32);

    let output = read_f32(device, queue, &output_buf, input.len());
    for (i, (&got, &x)) in output.iter().zip(&input).enumerate() {
        let want = x * 2.5 - 1.25;
        assert!(
            (got - want).abs() < 1e-5,
            "scale_add mismatch at {i}: got {got}, want {want}"
        );
    }
}

#[test]
fn zip_mul_then_block_sum_computes_the_dot_product() {
    let Some(context) = GpuContext::new() else { return };
    let device = &context.device;
    let queue = &context.queue;

    let n = 10_000u32;
    let a: Vec<f32> = (0..n).map(|i| (i % 97) as f32 * 0.1).collect();
    let b: Vec<f32> = (0..n).map(|i| (i % 53) as f32 * -0.2).collect();

    let mul = Kernel::new(
        device,
        "zip_mul",
        ZIP_MUL_WGSL,
        "main",
        &[Binding::Uniform, Binding::StorageRead, Binding::StorageRead, Binding::StorageReadWrite],
    )
    .expect("zip_mul builds");
    let sum = Kernel::new(
        device,
        "block_sum",
        BLOCK_SUM_WGSL,
        "main",
        &[Binding::Uniform, Binding::StorageRead, Binding::StorageReadWrite],
    )
    .expect("block_sum builds");

    // products = a * b
    let mut params = vec![0.0f32; 4];
    params[0] = f32::from_le_bytes(n.to_le_bytes());
    let params_buf = uniform_f32(device, queue, &params);
    let a_buf = storage_f32(device, queue, &a);
    let b_buf = storage_f32(device, queue, &b);
    let products = storage_f32_zeroed(device, queue, n as usize);
    mul.dispatch(device, queue, &[&params_buf, &a_buf, &b_buf, &products], n);

    // partial reduction on the GPU, final fold on the CPU.
    let groups = sum.workgroup_count(n);
    let partials = storage_f32_zeroed(device, queue, groups as usize);
    sum.dispatch(device, queue, &[&params_buf, &products, &partials], n);
    let partials = read_f32(device, queue, &partials, groups as usize);

    let want: f32 = a.iter().zip(&b).map(|(&x, &y)| x * y).sum();
    let got: f32 = partials.iter().sum();
    assert!((got - want).abs() < want.abs().max(1.0) * 1e-4,
        "dot product mismatch: got {got}, want {want}");
}

#[test]
fn reduction_handles_non_multiple_lengths() {
    let Some(context) = GpuContext::new() else { return };
    let device = &context.device;
    let queue = &context.queue;
    let sum = Kernel::with_workgroup_size(
        device,
        "block_sum",
        BLOCK_SUM_WGSL,
        "main",
        &[Binding::Uniform, Binding::StorageRead, Binding::StorageReadWrite],
        64,
    )
    .expect("block_sum builds");

    // 257 elements over 64-wide workgroups: 5 groups, last one partial.
    let n = 257u32;
    let data: Vec<f32> = (1..=n).map(|i| i as f32).collect();
    let mut params = vec![0.0f32; 4];
    params[0] = f32::from_le_bytes(n.to_le_bytes());
    let params_buf = uniform_f32(device, queue, &params);
    let input = storage_f32(device, queue, &data);
    let groups = sum.workgroup_count(n);
    assert_eq!(groups, 5);
    let partials = storage_f32_zeroed(device, queue, groups as usize);
    sum.dispatch(device, queue, &[&params_buf, &input, &partials], n);
    let partials = read_f32(device, queue, &partials, groups as usize);
    let want: f32 = data.iter().sum();
    let got: f32 = partials.iter().sum();
    assert_eq!(got, want, "partial sums must fold to the exact total");
}
