    use super::*;

    #[test]
    fn polygonize_auto_recommendation_tracks_work_size() {
        let sphere = Field::Sphere {
            center: [0.; 3],
            radius: 1.,
        };
        let tiny = Grid {
            min: [-1.5; 3],
            max: [1.5; 3],
            cells: [8, 8, 8],
        };
        assert_eq!(
            recommended_for_polygonize(&sphere, &tiny),
            Acceleration::Cpu
        );
        let large = Grid {
            cells: [64, 64, 64],
            ..tiny.clone()
        };
        let smooth = Field::SmoothUnion {
            a: Box::new(sphere.clone()),
            b: Box::new(Field::Box {
                center: [0.5; 3],
                half_size: [0.75; 3],
            }),
            radius: 0.25,
        };
        assert!(recommended_for_polygonize(&smooth, &large).is_gpu());
        let mut mesh = Triangles {
            positions: vec![
                0., 0., 0., //
                1., 0., 0., 0., 1., 0.,
            ],
            indices: Vec::new(),
        };
        for _ in 0..1024 {
            mesh.indices.extend([0, 1, 2]);
        }
        let mesh_field = Field::from_triangles(mesh, false).unwrap();
        assert!(recommended_for_polygonize(&mesh_field, &tiny).is_gpu());
    }

    #[test]
    fn polygonize_auto_matches_cpu_on_small_grid() {
        let field = Field::Sphere {
            center: [0.; 3],
            radius: 1.,
        };
        let grid = Grid {
            min: [-1.5; 3],
            max: [1.5; 3],
            cells: [8, 8, 8],
        };
        let cpu = polygonize(&field, &grid).unwrap();
        let automatic = polygonize_accelerated(&field, &grid, Acceleration::Auto).unwrap();
        assert_eq!(cpu.indices, automatic.indices);
        assert_eq!(cpu.positions, automatic.positions);
    }

    #[cfg(feature = "gpu")]
    #[test]
    fn gpu_sampling_matches_cpu_field_within_tolerance() {
        let field = Field::Translate {
            input: Box::new(Field::SmoothUnion {
                a: Box::new(Field::Sphere {
                    center: [0., 0., 0.],
                    radius: 10.,
                }),
                b: Box::new(Field::Box {
                    center: [8., 0., 0.],
                    half_size: [6., 6., 6.],
                }),
                radius: 3.,
            }),
            vector: [1., -2., 0.5],
        };
        let Some(flat) = field.to_flat() else {
            panic!("CSG tree should flatten")
        };
        let grid = Grid {
            min: [-12., -12., -12.],
            max: [16., 12., 12.],
            cells: [24, 16, 16],
        };
        let Some(values) = gpu::sample_grid_gpu(&flat, &grid) else {
            eprintln!("no GPU adapter; skipping");
            return;
        };
        let [nx, ny, nz] = grid.cells;
        let mut max_diff = 0f64;
        for z in 0..=nz {
            for y in 0..=ny {
                for x in 0..=nx {
                    let p = std::array::from_fn(|i| {
                        grid.min[i]
                            + (grid.max[i] - grid.min[i]) * [x, y, z][i] as f64
                                / grid.cells[i] as f64
                    });
                    let gpu = values[(z * (ny + 1) + y) * (nx + 1) + x] as f64;
                    max_diff = max_diff.max((gpu - field.sample(p)).abs());
                }
            }
        }
        assert!(max_diff < 0.01, "GPU field diverges: {max_diff}");
        // Full extraction: same topology class and volume within f32 tolerance.
        let cpu_mesh = polygonize(&field, &grid).unwrap();
        let gpu_mesh = polygonize_accelerated(&field, &grid, Acceleration::Gpu).unwrap();
        let cpu_tris = cpu_mesh.indices.len() / 3;
        let gpu_tris = gpu_mesh.indices.len() / 3;
        assert!(
            (cpu_tris as f64 - gpu_tris as f64).abs() <= 0.02 * cpu_tris as f64,
            "triangle counts diverge: {cpu_tris} vs {gpu_tris}"
        );
    }

    #[cfg(feature = "gpu")]
    #[test]
    fn gpu_mesh_distance_matches_cpu_signed_and_unsigned() {
        // Closed outward tetrahedron around the origin-ish region.
        let mesh = Triangles {
            positions: vec![
                10., 10., 10., //
                30., 10., 10., 10., 30., 10., 10., 10., 30.,
            ],
            indices: vec![0, 2, 1, 0, 1, 3, 1, 2, 3, 2, 0, 3],
        };
        let field = Field::from_triangles(mesh, true).unwrap();
        let Some(flat) = field.to_flat() else {
            panic!("mesh field should flatten")
        };
        assert_eq!(flat.triangles.len(), 4 * 9);
        let grid = Grid {
            min: [0., 0., 0.],
            max: [40., 40., 40.],
            cells: [8, 8, 8],
        };
        let Some(values) = gpu::sample_grid_gpu(&flat, &grid) else {
            eprintln!("no GPU adapter; skipping");
            return;
        };
        let [nx, ny, nz] = grid.cells;
        let mut max_diff = 0f64;
        let mut inside_cpu = 0;
        let mut inside_gpu = 0;
        for z in 0..=nz {
            for y in 0..=ny {
                for x in 0..=nx {
                    let p = std::array::from_fn(|i| {
                        grid.min[i]
                            + (grid.max[i] - grid.min[i]) * [x, y, z][i] as f64
                                / grid.cells[i] as f64
                    });
                    let cpu = field.sample(p);
                    let gpu = values[(z * (ny + 1) + y) * (nx + 1) + x] as f64;
                    max_diff = max_diff.max((gpu - cpu).abs());
                    if cpu < 0. {
                        inside_cpu += 1;
                    }
                    if gpu < 0. {
                        inside_gpu += 1;
                    }
                }
            }
        }
        assert!(inside_cpu > 0, "tetrahedron should contain grid points");
        assert_eq!(
            inside_cpu, inside_gpu,
            "inside/outside classification diverges"
        );
        assert!(max_diff < 0.01, "signed distance diverges: {max_diff}");
    }

    /// CSG tree plus a signed mesh-distance leaf: every node kind the flat
    /// interpreter implements, so the CUDA port is exercised end to end.
    #[cfg(feature = "cuda")]
    fn cuda_fixture() -> (Field, Grid) {
        let tetra = Triangles {
            positions: vec![
                -6., -6., -6., //
                6., -6., -6., -6., 6., -6., -6., -6., 6.,
            ],
            indices: vec![0, 2, 1, 0, 1, 3, 1, 2, 3, 2, 0, 3],
        };
        let field = Field::Translate {
            input: Box::new(Field::Difference {
                a: Box::new(Field::SmoothUnion {
                    a: Box::new(Field::Sphere {
                        center: [0., 0., 0.],
                        radius: 8.,
                    }),
                    b: Box::new(Field::Offset {
                        input: Box::new(Field::Torus {
                            center: [4., 0., 0.],
                            major_radius: 4.,
                            minor_radius: 1.5,
                        }),
                        distance: 0.5,
                    }),
                    radius: 3.,
                }),
                b: Box::new(Field::Intersection {
                    a: Box::new(Field::Box {
                        center: [0., 0., 0.],
                        half_size: [5., 5., 5.],
                    }),
                    b: Box::new(Field::from_triangles(tetra, true).unwrap()),
                }),
            }),
            vector: [1., -2., 0.5],
        };
        let grid = Grid {
            min: [-12., -12., -12.],
            max: [16., 12., 12.],
            cells: [28, 24, 24],
        };
        (field, grid)
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn cuda_sampling_matches_cpu_field_within_tolerance() {
        let (field, grid) = cuda_fixture();
        let flat = field.to_flat().expect("fixture should flatten");
        let Some(values) = cuda::sample_grid_cuda(&flat, &grid) else {
            eprintln!("no CUDA device; skipping");
            return;
        };
        let [nx, ny, nz] = grid.cells;
        assert_eq!(values.len(), (nx + 1) * (ny + 1) * (nz + 1));
        let mut max_diff = 0f64;
        let (mut inside_cpu, mut inside_cuda) = (0, 0);
        for z in 0..=nz {
            for y in 0..=ny {
                for x in 0..=nx {
                    let p = std::array::from_fn(|i| {
                        grid.min[i]
                            + (grid.max[i] - grid.min[i]) * [x, y, z][i] as f64
                                / grid.cells[i] as f64
                    });
                    let cpu = field.sample(p);
                    let cuda = values[(z * (ny + 1) + y) * (nx + 1) + x] as f64;
                    max_diff = max_diff.max((cuda - cpu).abs());
                    inside_cpu += (cpu < 0.) as usize;
                    inside_cuda += (cuda < 0.) as usize;
                }
            }
        }
        assert!(inside_cpu > 0);
        assert_eq!(
            inside_cpu, inside_cuda,
            "inside/outside classification diverges"
        );
        assert!(max_diff < 0.01, "CUDA field diverges: {max_diff}");
        let cpu_mesh = polygonize(&field, &grid).unwrap();
        let cuda_mesh = polygonize_accelerated(&field, &grid, Acceleration::Cuda).unwrap();
        let cpu_tris = cpu_mesh.indices.len() / 3;
        let cuda_tris = cuda_mesh.indices.len() / 3;
        assert!(
            (cpu_tris as f64 - cuda_tris as f64).abs() <= 0.02 * cpu_tris as f64,
            "triangle counts diverge: {cpu_tris} vs {cuda_tris}"
        );
    }

    /// The PTX and WGSL kernels are ports of the same text; on the same device
    /// class they should agree to f32 rounding, far tighter than the CPU bound.
    #[cfg(feature = "cuda")]
    #[test]
    fn cuda_and_wgpu_samplers_agree() {
        let (field, grid) = cuda_fixture();
        let flat = field.to_flat().expect("fixture should flatten");
        let (Some(cuda), Some(wgpu)) = (
            cuda::sample_grid_cuda(&flat, &grid),
            gpu::sample_grid_gpu(&flat, &grid),
        ) else {
            eprintln!("CUDA or wgpu unavailable; skipping");
            return;
        };
        assert_eq!(cuda.len(), wgpu.len());
        let max_diff = cuda
            .iter()
            .zip(&wgpu)
            .map(|(a, b)| (a - b).abs())
            .fold(0f32, f32::max);
        assert!(
            max_diff < 1e-3,
            "CUDA and wgpu samplers diverge: {max_diff}"
        );
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn cuda_request_without_device_falls_back() {
        // Whatever the host has, `Cuda` must never fail where the CPU succeeds.
        let (field, grid) = cuda_fixture();
        let cpu = polygonize(&field, &grid).unwrap();
        let cuda = polygonize_accelerated(&field, &grid, Acceleration::Cuda).unwrap();
        assert!(!cuda.indices.is_empty());
        let flat = field.to_flat().unwrap();
        if !cuda::available() && gpu::sample_grid_gpu(&flat, &grid).is_none() {
            assert_eq!(
                cpu.indices, cuda.indices,
                "CPU fallback must be bit-identical"
            );
        }
    }

    fn sphere() -> Field {
        Field::Sphere {
            center: [0.; 3],
            radius: 1.,
        }
    }
    fn grid(n: usize) -> Grid {
        Grid {
            min: [-1.5; 3],
            max: [1.5; 3],
            cells: [n; 3],
        }
    }
    #[test]
    fn primitives_and_csg() {
        assert_eq!(sphere().evaluate([0.; 3]).unwrap(), -1.);
        let f = Field::Difference {
            a: Box::new(sphere()),
            b: Box::new(Field::Sphere {
                center: [0.; 3],
                radius: 0.5,
            }),
        };
        assert_eq!(f.evaluate([0.; 3]).unwrap(), 0.5);
        assert!(f.evaluate([0.75, 0., 0.]).unwrap() < 0.);
    }
    fn signed_volume(mesh: &Triangles) -> f64 {
        mesh.indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| {
                let a = point(mesh, t[0]);
                let b = point(mesh, t[1]);
                let c = point(mesh, t[2]);
                dot(a, cross(b, c)) / 6.
            })
            .sum()
    }
    #[test]
    fn sculpt_sphere_adds_and_removes_material() {
        let base = sphere();
        let added = base.sculpt_sphere([1.5, 0., 0.], 0.5, false).unwrap();
        assert!(matches!(added, Field::Union { .. }));
        assert_eq!(base.evaluate([1.5, 0., 0.]).unwrap(), 0.5);
        assert_eq!(added.evaluate([1.5, 0., 0.]).unwrap(), -0.5);
        assert_eq!(added.evaluate([0.; 3]).unwrap(), -1.);
        let removed = base.sculpt_sphere([0.; 3], 0.5, true).unwrap();
        assert!(matches!(removed, Field::Difference { .. }));
        assert_eq!(removed.evaluate([0.; 3]).unwrap(), 0.5);
        assert!(removed.evaluate([0.75, 0., 0.]).unwrap() < 0.);
        // Chained strokes compose and the result still polygonizes to a closed shell.
        let strokes = removed
            .sculpt_sphere([0., 1., 0.], 0.3, false)
            .unwrap()
            .sculpt_sphere([0., -1., 0.], 0.3, true)
            .unwrap();
        assert!(strokes.evaluate([0., 1., 0.]).unwrap() < 0.);
        assert!(strokes.evaluate([0., -1., 0.]).unwrap() > 0.);
        let mesh = polygonize(&strokes, &grid(16)).unwrap();
        assert!(triangles_closed(&mesh));
        assert!(triangles_ok(&mesh));
    }
    #[test]
    fn capsule_and_smooth_difference_sample_correctly() {
        let capsule = Field::Capsule {
            a: [0.; 3],
            b: [2., 0., 0.],
            radius: 0.5,
        };
        assert_eq!(capsule.evaluate([1., 0., 0.]).unwrap(), -0.5);
        assert_eq!(capsule.evaluate([3., 0., 0.]).unwrap(), 0.5);
        assert_eq!(capsule.evaluate([1., 1., 0.]).unwrap(), 0.5);
        let degenerate = Field::Capsule {
            a: [0.; 3],
            b: [0.; 3],
            radius: 1.,
        };
        assert_eq!(
            degenerate.evaluate([0., 2., 0.]).unwrap(),
            1.,
            "zero-length capsule is a sphere"
        );
        let hard = Field::Difference {
            a: Box::new(sphere()),
            b: Box::new(Field::Sphere {
                center: [1., 0., 0.],
                radius: 0.5,
            }),
        };
        let smooth = Field::SmoothDifference {
            a: Box::new(sphere()),
            b: Box::new(Field::Sphere {
                center: [1., 0., 0.],
                radius: 0.5,
            }),
            radius: 0.3,
        };
        // Far from the seam the smooth result equals the hard result; near it, material is removed more.
        assert!(
            (smooth.evaluate([-0.9, 0., 0.]).unwrap() - hard.evaluate([-0.9, 0., 0.]).unwrap())
                .abs()
                < 1e-12
        );
        assert!(
            (smooth.evaluate([1., 0., 0.]).unwrap() - hard.evaluate([1., 0., 0.]).unwrap()).abs()
                < 1e-12
        );
        // On the seam circle both distances vanish; the blend lifts the value by k/4.
        let seam = [0.875, (1. - 0.875f64 * 0.875).sqrt(), 0.];
        assert!(hard.evaluate(seam).unwrap().abs() < 1e-9);
        assert!((smooth.evaluate(seam).unwrap() - 0.3 / 4.).abs() < 1e-9);
        assert!(triangles_closed(&polygonize(&smooth, &grid(16)).unwrap()));
    }
    #[test]
    fn sculpt_strokes_compose_tools_and_blend_modes() {
        let base = sphere();
        let add = base
            .sculpt(&SdfStroke {
                tool: SdfTool::Box {
                    center: [1.2, 0., 0.],
                    half_size: [0.3; 3],
                },
                remove: false,
                blend: None,
            })
            .unwrap();
        assert!(matches!(add, Field::Union { .. }));
        assert!(add.evaluate([1.2, 0., 0.]).unwrap() < 0.);
        let smooth_add = base
            .sculpt(&SdfStroke {
                tool: SdfTool::Capsule {
                    a: [0., 0., 0.8],
                    b: [0., 0., 1.6],
                    radius: 0.2,
                },
                remove: false,
                blend: Some(0.2),
            })
            .unwrap();
        assert!(matches!(smooth_add, Field::SmoothUnion { .. }));
        assert!(smooth_add.evaluate([0., 0., 1.6]).unwrap() < 0.);
        let hard_remove = base
            .sculpt(&SdfStroke {
                tool: SdfTool::Sphere {
                    center: [0.; 3],
                    radius: 0.5,
                },
                remove: true,
                blend: None,
            })
            .unwrap();
        assert_eq!(hard_remove.evaluate([0.; 3]).unwrap(), 0.5);
        assert!(matches!(hard_remove, Field::Difference { .. }));
        let smooth_remove = base
            .sculpt(&SdfStroke {
                tool: SdfTool::Sphere {
                    center: [1., 0., 0.],
                    radius: 0.5,
                },
                remove: true,
                blend: Some(0.25),
            })
            .unwrap();
        assert!(matches!(smooth_remove, Field::SmoothDifference { .. }));
        assert!(smooth_remove.evaluate([0.6, 0., 0.]).unwrap() > 0.);
        let mesh = polygonize(&smooth_remove, &grid(16)).unwrap();
        assert!(triangles_closed(&mesh) && triangles_ok(&mesh));
        #[cfg(feature = "codec")]
        {
            let round: SdfStroke = value_codec::from_value(
                value_codec::to_value(SdfStroke {
                    tool: SdfTool::Capsule {
                        a: [0.; 3],
                        b: [1., 0., 0.],
                        radius: 0.2,
                    },
                    remove: true,
                    blend: Some(0.1),
                })
                .unwrap(),
            )
            .unwrap();
            assert_eq!(round.blend, Some(0.1));
            assert!(round.remove);
        }
        for bad in [
            SdfStroke {
                tool: SdfTool::Sphere {
                    center: [0.; 3],
                    radius: 0.,
                },
                remove: false,
                blend: None,
            },
            SdfStroke {
                tool: SdfTool::Box {
                    center: [0.; 3],
                    half_size: [1., -1., 1.],
                },
                remove: false,
                blend: None,
            },
            SdfStroke {
                tool: SdfTool::Capsule {
                    a: [f64::NAN; 3],
                    b: [0.; 3],
                    radius: 1.,
                },
                remove: false,
                blend: None,
            },
            SdfStroke {
                tool: SdfTool::Sphere {
                    center: [0.; 3],
                    radius: 1.,
                },
                remove: false,
                blend: Some(0.),
            },
        ] {
            assert!(base.sculpt(&bad).is_err());
        }
    }
    #[test]
    fn sculpt_sphere_rejects_invalid_strokes() {
        let base = sphere();
        for radius in [0., -1., f64::NAN, f64::INFINITY] {
            assert!(
                base.sculpt_sphere([0.; 3], radius, false).is_err(),
                "{radius}"
            );
        }
        assert!(base.sculpt_sphere([f64::NAN, 0., 0.], 1., true).is_err());
        let invalid = Field::Sphere {
            center: [0.; 3],
            radius: -1.,
        };
        assert!(invalid.sculpt_sphere([0.; 3], 1., false).is_err());
    }
    #[test]
    fn closed_outward_sphere_and_convergence() {
        let coarse = polygonize(&sphere(), &grid(8)).unwrap();
        let fine = polygonize(&sphere(), &grid(16)).unwrap();
        assert!(triangles_closed(&fine));
        assert!(triangles_ok(&fine));
        let exact = 4. * std::f64::consts::PI / 3.;
        assert!((signed_volume(&fine) - exact).abs() < (signed_volume(&coarse) - exact).abs());
        assert!((signed_volume(&fine) - exact).abs() < 0.1);
    }
    #[test]
    fn exact_grid_vertices_and_empty() {
        let mut g = grid(12);
        g.min = [-2.; 3];
        g.max = [2.; 3];
        assert!(triangles_closed(&polygonize(&sphere(), &g).unwrap()));
        assert!(polygonize_with(|_| 1., &g).unwrap().indices.is_empty());
        assert!(polygonize_with(|_| f64::NAN, &g).is_err());
        assert!(polygonize_with(|_| -1., &g).is_err());
    }
    #[test]
    fn tpms_chamfer_onion_elongate_and_surface_nets() {
        let neovius = Field::Neovius {
            origin: [0.; 3],
            period: 4.0,
            thickness: 0.5,
        };
        let lidinoid = Field::Lidinoid {
            origin: [0.; 3],
            period: 4.0,
            thickness: 0.5,
        };
        let iwp = Field::SchoenIWP {
            origin: [0.; 3],
            period: 4.0,
            thickness: 0.5,
        };
        for tpms in [&neovius, &lidinoid, &iwp] {
            assert!(tpms.validate().is_ok());
            assert!(tpms.evaluate([0.5, 0.7, 1.1]).unwrap().is_finite());
        }

        let onion_sphere = Field::Onion {
            input: Box::new(sphere()),
            thickness: 0.2,
        };
        // On the original sphere surface (r=1), onion field is -0.1 (inside shell); at origin it's +0.9 (outside shell).
        assert!((onion_sphere.evaluate([1., 0., 0.]).unwrap() + 0.1).abs() < 1e-12);
        assert!((onion_sphere.evaluate([0., 0., 0.]).unwrap() - 0.9).abs() < 1e-12);

        let elongated = Field::Elongate {
            input: Box::new(sphere()),
            half_extents: [1.0, 0.0, 0.0],
        };
        // Elongating a unit sphere by 1.0 along +-X produces a capsule of total length 4 along X.
        assert!((elongated.evaluate([2.0, 0., 0.]).unwrap()).abs() < 1e-12);
        assert!(elongated.evaluate([1.5, 0., 0.]).unwrap() < 0.);

        let chamfer = Field::ChamferUnion {
            a: Box::new(sphere()),
            b: Box::new(Field::Box {
                center: [0.8, 0., 0.],
                half_size: [0.6; 3],
            }),
            radius: 0.2,
        };
        assert!(chamfer.validate().is_ok());

        let sn_mesh = polygonize_surface_nets(&chamfer, &grid(16)).unwrap();
        assert!(triangles_closed(&sn_mesh));
        assert!(triangles_ok(&sn_mesh));
    }

    #[test]
    fn octahedron_prisms_mirror_taper_raycast_and_volume() {
        let oct = Field::Octahedron {
            center: [0.; 3],
            size: 1.0,
        };
        assert!(oct.validate().is_ok());
        assert!(oct.evaluate([0.; 3]).unwrap() < 0.0);
        assert!((oct.evaluate([1.0, 0.0, 0.0]).unwrap()).abs() < 1e-12);

        let hex = Field::HexPrism {
            center: [0.; 3],
            radius: 1.0,
            half_height: 0.8,
        };
        assert!(hex.validate().is_ok());
        assert!(hex.evaluate([0.; 3]).unwrap() < 0.0);

        let tri_prism = Field::TriangularPrism {
            center: [0.; 3],
            radius: 1.0,
            half_height: 0.8,
        };
        assert!(tri_prism.validate().is_ok());
        assert!(tri_prism.evaluate([0.; 3]).unwrap() < 0.0);

        let s_inter = Field::SmoothIntersection {
            a: Box::new(sphere()),
            b: Box::new(Field::Box {
                center: [0.; 3],
                half_size: [0.8; 3],
            }),
            radius: 0.2,
        };
        assert!(s_inter.validate().is_ok());
        assert!(s_inter.evaluate([0.; 3]).unwrap() < 0.0);

        let mirrored = Field::Mirror {
            input: Box::new(Field::Sphere {
                center: [1.5, 0.0, 0.0],
                radius: 0.5,
            }),
            axis: [true, false, false],
        };
        assert!((mirrored.evaluate([-1.5, 0.0, 0.0]).unwrap() + 0.5).abs() < 1e-12);

        let tapered = Field::Taper {
            input: Box::new(Field::Cylinder {
                center: [0.; 3],
                radius: 1.0,
                half_height: 1.0,
            }),
            height: 2.0,
            factor: 0.5,
        };
        assert!(tapered.validate().is_ok());
        assert!(tapered.evaluate([0.0, 0.0, 0.8]).unwrap() < 0.0);
        assert!(tapered.evaluate([0.8, 0.0, 0.8]).unwrap() > 0.0);

        // Raycast from (-3, 0, 0) along +X against unit sphere at origin
        let (hit_p, hit_n, dist) = sphere()
            .raycast([-3.0, 0.0, 0.0], [1.0, 0.0, 0.0], 10.0, 1e-5)
            .unwrap();
        assert!((dist - 2.0).abs() < 1e-4);
        assert!((hit_p[0] + 1.0).abs() < 1e-4);
        assert!((hit_n[0] + 1.0).abs() < 1e-3);

        // Volume estimation of unit sphere (~4/3 * pi = 4.18879)
        let vol = sphere().estimate_volume(32);
        let exact = 4.0 * std::f64::consts::PI / 3.0;
        assert!((vol - exact).abs() < 0.15, "estimated {vol} vs exact {exact}");
    }
