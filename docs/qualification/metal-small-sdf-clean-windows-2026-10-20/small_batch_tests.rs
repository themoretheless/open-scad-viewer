use super::*;
use crate::{Acceleration, Field, polygonize_accelerated};
use std::{
    hint::black_box,
    rc::Rc,
    time::{Duration, Instant},
};
fn base(c: &GpuContext) -> Rc<GpuSdf> {
    let mut s = GpuSdf::new(c);
    s.disable_small_updates = true;
    Rc::new(s)
}
fn sphere() -> Field {
    Field::Sphere {
        center: [0.; 3],
        radius: 1.,
    }
}
fn tetra() -> Field {
    Field::from_triangles(
        geometry_ops::Triangles {
            positions: vec![1., 1., 1., -1., -1., 1., -1., 1., -1., 1., -1., -1.],
            indices: vec![0, 2, 1, 0, 1, 3, 0, 3, 2, 1, 2, 3],
        },
        true,
    )
    .unwrap()
}
fn use_sampler(s: &Rc<GpuSdf>) {
    TEST_SDF.with(|slot| *slot.borrow_mut() = Some(s.clone()));
}
fn mesh_bits(m: &geometry_ops::Triangles) -> (Vec<u64>, Vec<usize>) {
    (
        m.positions.iter().map(|v| v.to_bits()).collect(),
        m.indices.clone(),
    )
}
#[test]
fn small_inputs_preserve_sdf_values_and_reused_capacity() {
    let c = GpuContext::new();
    assert!(c.is_some() || std::env::var_os("COMPUTE_REQUIRE_GPU").is_none());
    let Some(c) = c else { return };
    let a = base(&c);
    let b = Rc::new(GpuSdf::new(&c));
    let grid = Grid {
        min: [-2.; 3],
        max: [2.; 3],
        cells: [4; 3],
    };
    for field in [sphere(), tetra(), sphere()] {
        let flat = field.to_flat().unwrap();
        assert_eq!(
            a.run(&flat, &grid)
                .into_iter()
                .map(f32::to_bits)
                .collect::<Vec<_>>(),
            b.run(&flat, &grid)
                .into_iter()
                .map(f32::to_bits)
                .collect::<Vec<_>>()
        );
        use_sampler(&a);
        let before = polygonize_accelerated(&field, &grid, Acceleration::Gpu).unwrap();
        use_sampler(&b);
        let after = polygonize_accelerated(&field, &grid, Acceleration::Gpu).unwrap();
        assert_eq!(mesh_bits(&before), mesh_bits(&after));
        assert_eq!(
            b.buffers.borrow().as_ref().unwrap().small_input.is_some(),
            gpu_compute::shared_memory_uploads(&c.device)
        );
    }
    let mut large = sphere();
    for _ in 0..63 {
        large = Field::Union {
            a: Box::new(large),
            b: Box::new(sphere()),
        };
    }
    for field in [large, sphere()] {
        let flat = field.to_flat().unwrap();
        assert_eq!(
            a.run(&flat, &grid)
                .into_iter()
                .map(f32::to_bits)
                .collect::<Vec<_>>(),
            b.run(&flat, &grid)
                .into_iter()
                .map(f32::to_bits)
                .collect::<Vec<_>>()
        );
        assert!(b.buffers.borrow().as_ref().unwrap().small_input.is_none());
    }
    TEST_SDF.with(|slot| *slot.borrow_mut() = None);
}
#[test]
#[ignore = "full polygonize qualification, explicit output required"]
fn paired_polygonize() {
    #[cfg(target_os = "macos")]
    unsafe {
        unsafe extern "C" {
            fn pthread_set_qos_class_self_np(q: u32, p: i32) -> i32;
        }
        assert_eq!(pthread_set_qos_class_self_np(0x19, 0), 0);
    }
    let c = GpuContext::new().expect("GPU required");
    assert_eq!(c.backend, wgpu::Backend::Metal);
    let a = base(&c);
    let aa = base(&c);
    let b = Rc::new(GpuSdf::new(&c));
    let phase = std::env::var("METAL_SMALL_SDF_PHASE").unwrap_or_else(|_| "full".into());
    assert!(phase == "controls" || phase == "full");
    let controls: &[bool] = if phase == "controls" {
        &[true]
    } else {
        &[false, true]
    };
    let mut csv = String::from("case,field,cells,round,a_ns,b_ns\n");
    for (name, field) in [("sphere", sphere()), ("tetra", tetra())] {
        for cells in [4, 8] {
            if let Ok(case) = std::env::var("METAL_SMALL_SDF_CASE") {
                if case != format!("{name}-{cells}") {
                    continue;
                }
            }
            let grid = Grid {
                min: [-2.; 3],
                max: [2.; 3],
                cells: [cells; 3],
            };
            use_sampler(&a);
            let reference = polygonize_accelerated(&field, &grid, Acceleration::Gpu).unwrap();
            for s in [&aa, &b] {
                use_sampler(s);
                assert_eq!(
                    mesh_bits(&reference),
                    mesh_bits(&polygonize_accelerated(&field, &grid, Acceleration::Gpu).unwrap())
                );
            }
            assert!(b.buffers.borrow().as_ref().unwrap().small_input.is_some());
            for &control in controls {
                let other = if control { &aa } else { &b };
                let sample = |v: bool| {
                    use_sampler(if v { other } else { &a });
                    let start = Instant::now();
                    for _ in 0..64 {
                        black_box(
                            polygonize_accelerated(
                                black_box(&field),
                                black_box(&grid),
                                Acceleration::Gpu,
                            )
                            .unwrap(),
                        );
                    }
                    start.elapsed().as_nanos() as f64 / 64.
                };
                let warm = Instant::now();
                while warm.elapsed() < Duration::from_millis(500) {
                    sample(false);
                    sample(true);
                }
                for round in 0..48 {
                    let order = if round % 2 == 0 {
                        [false, true, true, false]
                    } else {
                        [true, false, false, true]
                    };
                    let s = order.map(sample);
                    let (a, b) = if round % 2 == 0 {
                        ((s[0] + s[3]) / 2., (s[1] + s[2]) / 2.)
                    } else {
                        ((s[1] + s[2]) / 2., (s[0] + s[3]) / 2.)
                    };
                    csv.push_str(&format!(
                        "{},{name},{cells},{round},{a},{b}\n",
                        if control { "aa" } else { "batch" }
                    ));
                }
            }
        }
    }
    TEST_SDF.with(|slot| *slot.borrow_mut() = None);
    use std::io::Write;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(std::env::var("METAL_SMALL_SDF_CSV").unwrap())
        .unwrap()
        .write_all(csv.as_bytes())
        .unwrap();
}

#[test]
fn byte_parts_validate_before_queue_mutation() {
    let Some(c) = GpuContext::new() else {
        assert!(std::env::var_os("COMPUTE_REQUIRE_GPU").is_none());
        return;
    };
    let buffer = mk(
        &c.device,
        "parts validation",
        4096,
        BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
    );
    let parts = [1u32.to_le_bytes(), 2u32.to_le_bytes()];
    gpu_compute::write_small_parts(&c.queue, &buffer, 0, parts.iter().map(|p| p.as_slice()))
        .unwrap();
    for (offset, bytes) in [(2, 4), (0, 7), (4096, 4), (0, 4100), (u64::MAX, 4)] {
        assert!(
            gpu_compute::write_small_parts(
                &c.queue,
                &buffer,
                offset,
                std::iter::once(&vec![99u8; bytes][..])
            )
            .is_err()
        );
    }
    assert_eq!(
        compute_core::try_read_u32(&c.device, &c.queue, &buffer, 2).unwrap(),
        [1, 2]
    );
    gpu_compute::write_small_parts(&c.queue, &buffer, 4096, std::iter::empty()).unwrap();
    let missing = mk(&c.device, "missing COPY_DST", 4, BufferUsages::STORAGE);
    assert!(
        gpu_compute::write_small_parts(&c.queue, &missing, 0, std::iter::once(&[0; 4][..]))
            .is_err()
    );
    let max = [13; 4096];
    gpu_compute::write_small_parts(
        &c.queue,
        &buffer,
        0,
        [&max[..2048], &max[2048..]].into_iter(),
    )
    .unwrap();
    assert_eq!(
        compute_core::try_read_u32(&c.device, &c.queue, &buffer, 1024).unwrap(),
        vec![0x0d0d0d0d; 1024]
    );
}
