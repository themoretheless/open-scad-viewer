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

// Full public workflow, including Auto's CPU branch and cached arena growth.
fn bipyramid(ring: usize) -> Field {
    let mut positions = vec![0., 0., 0.8, 0., 0., -0.8];
    let mut indices = Vec::new();
    for i in 0..ring {
        let angle = std::f64::consts::TAU * i as f64 / ring as f64;
        positions.extend([0.8 * angle.cos(), 0.8 * angle.sin(), 0.]);
        let a = 2 + i;
        let b = 2 + (i + 1) % ring;
        indices.extend([0, a, b, 1, b, a]);
    }
    Field::from_triangles(geometry_ops::Triangles { positions, indices }, true).unwrap()
}

#[test]
#[ignore = "large/Auto full workflow qualification; explicit CSV required"]
fn paired_large_polygonize() {
    #[cfg(target_os = "macos")]
    unsafe {
        unsafe extern "C" {
            fn pthread_set_qos_class_self_np(q: u32, p: i32) -> i32;
        }
        assert_eq!(pthread_set_qos_class_self_np(0x19, 0), 0);
    }
    let name = std::env::var("METAL_SMALL_SDF_CASE").unwrap();
    let (fields, cells, mode, gpu, reset) = match name.as_str() {
        "sphere-gpu-64" => (vec![sphere()], 64, Acceleration::Gpu, true, false),
        "sphere-auto-64" => (vec![sphere()], 64, Acceleration::Auto, false, false),
        "tetra-auto-48" => (vec![tetra()], 48, Acceleration::Auto, false, false),
        "tetra-auto-49" => (vec![tetra()], 49, Acceleration::Auto, true, false),
        "tetra-auto-64" => (vec![tetra()], 64, Acceleration::Auto, true, false),
        "mesh84-auto-32" => (vec![bipyramid(42)], 32, Acceleration::Auto, true, false),
        "mesh86-auto-32" => (vec![bipyramid(43)], 32, Acceleration::Auto, true, false),
        "cross-auto-32" => (
            vec![bipyramid(42), bipyramid(43), bipyramid(42)],
            32,
            Acceleration::Auto,
            true,
            true,
        ),
        _ => panic!("unknown case"),
    };
    let grid = Grid {
        min: [-2.; 3],
        max: [2.; 3],
        cells: [cells; 3],
    };
    let c = GpuContext::new().expect("Metal required");
    assert_eq!(c.backend, wgpu::Backend::Metal);
    let a = base(&c);
    let aa = base(&c);
    let b = Rc::new(GpuSdf::new(&c));
    for field in &fields {
        if mode == Acceleration::Auto {
            assert_eq!(
                crate::recommended_for_polygonize(field, &grid).is_gpu(),
                gpu
            );
        }
        use_sampler(&a);
        let reference = polygonize_accelerated(field, &grid, mode).unwrap();
        assert!(!reference.indices.is_empty());
        for sampler in [&aa, &b] {
            use_sampler(sampler);
            assert_eq!(
                mesh_bits(&reference),
                mesh_bits(&polygonize_accelerated(field, &grid, mode).unwrap())
            );
            assert_eq!(sampler.buffers.borrow().is_some(), gpu);
        }
        if gpu {
            let arena = b.buffers.borrow();
            let arena = arena.as_ref().unwrap();
            let expected =
                name != "mesh86-auto-32" && !(reset && arena.triangle_capacity >= 86 * 9);
            assert_eq!(arena.small_input.is_some(), expected);
            if let Some((buffer, _)) = &arena.small_input {
                if name.starts_with("mesh84") || reset {
                    assert_eq!(buffer.size(), 4048);
                }
            }
        } else {
            assert_eq!(
                mesh_bits(&reference),
                mesh_bits(&polygonize_accelerated(field, &grid, Acceleration::Cpu).unwrap())
            );
        }
    }
    // Mesh-distance work exposes device/scheduler variability at short samples.
    // Fix a longer averaging window for these cases before the AA-first series.
    let repetitions = if name.starts_with("mesh") || name.starts_with("cross") {
        32
    } else {
        8
    };
    let controls = std::env::var("METAL_SMALL_SDF_PHASE").unwrap() == "controls";
    let mut csv = String::from("case,field,cells,round,a_ns,b_ns,start_unix_ns,end_unix_ns\n");
    for control in if controls {
        &[true][..]
    } else {
        &[false, true][..]
    } {
        let other = if *control { &aa } else { &b };
        let sample = |second: bool| {
            let sampler = if second { other } else { &a };
            use_sampler(sampler);
            // Time one complete workflow; average adjacent ABBA cycles below.
            // Cache reset stays outside the timer, allocations/growth stay inside.
            if reset {
                *sampler.buffers.borrow_mut() = None;
            }
            let start = Instant::now();
            for field in &fields {
                black_box(polygonize_accelerated(black_box(field), black_box(&grid), mode).unwrap());
            }
            start.elapsed().as_nanos() as f64
        };
        let warm = Instant::now();
        while warm.elapsed() < Duration::from_millis(3000) {
            sample(false);
            sample(true);
        }
        for round in 0..48 {
            let order = if round % 2 == 0 {
                [false, true, true, false]
            } else {
                [true, false, false, true]
            };
            let wall_start = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let mut values = [0.; 4];
            for _ in 0..repetitions {
                for (index, second) in order.iter().enumerate() {
                    values[index] += sample(*second) / repetitions as f64;
                }
            }
            let wall_end = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let (left, right) = if round % 2 == 0 {
                ((values[0] + values[3]) / 2., (values[1] + values[2]) / 2.)
            } else {
                ((values[1] + values[2]) / 2., (values[0] + values[3]) / 2.)
            };
            csv.push_str(&format!(
                "{},{name},{cells},{round},{left},{right},{wall_start},{wall_end}\n",
                if *control { "aa" } else { "batch" }
            ));
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
