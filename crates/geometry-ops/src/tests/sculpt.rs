use super::*;
fn near(a: Point, b: Point) {
    for k in 0..3 {
        assert!((a[k] - b[k]).abs() < 1e-9, "{a:?} != {b:?}");
    }
}
/// 3x3 grid in the z=0 plane, normals +z, 4-neighborhood adjacency.
fn grid() -> (Vec<Point>, Vec<Point>, Vec<Vec<usize>>) {
    let mut positions = Vec::new();
    for j in 0..3 {
        for i in 0..3 {
            positions.push([i as f64 - 1., j as f64 - 1., 0.]);
        }
    }
    let normals = vec![[0., 0., 1.]; 9];
    let mut adjacency = vec![Vec::new(); 9];
    for j in 0..3i32 {
        for i in 0..3i32 {
            let v = (j * 3 + i) as usize;
            for (di, dj) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (ni, nj) = (i + di, j + dj);
                if (0..3).contains(&ni) && (0..3).contains(&nj) {
                    adjacency[v].push((nj * 3 + ni) as usize);
                }
            }
        }
    }
    (positions, normals, adjacency)
}
fn target(g: &(Vec<Point>, Vec<Point>, Vec<Vec<usize>>)) -> SculptTarget {
    SculptTarget {
        positions: g.0.clone(),
        normals: g.1.clone(),
        adjacency: g.2.clone(),
    }
}
fn sculpt(target: &SculptTarget, brush: &SculptBrush) -> Result<Vec<Point>> {
    target.sculpt(brush)
}
#[test]
fn falloff_profiles_are_bounded_monotone_and_named() {
    for f in Falloff::ALL {
        assert_eq!(Falloff::parse(f.name()), Some(f));
        assert_eq!(f.weight(0.), 1.);
        assert_eq!(f.weight(1.), 0.);
        assert_eq!(f.weight(2.), 0.);
        assert_eq!(f.weight(f64::NAN), 0.);
        let mut last = 1.;
        for i in 1..=100 {
            let w = f.weight(i as f64 / 100.);
            assert!((0. ..=1.).contains(&w));
            assert!(w <= last + 1e-12, "{f:?} not monotone");
            last = w;
        }
    }
    assert_eq!(Falloff::Smooth.weight(0.5), 0.5);
    assert_eq!(Falloff::Linear.weight(0.25), 0.75);
    assert_eq!(Falloff::Constant.weight(0.999), 1.);
    assert!((Falloff::Sphere.weight(0.6) - 0.8).abs() < 1e-12);
    assert!(Falloff::parse("gaussian").is_none());
}
#[test]
fn grab_matches_legacy_brush_and_is_local() {
    let g = grid();
    let brush = SculptBrush::new(
        SculptKind::Grab {
            displacement: [0., 0., 2.],
        },
        [0.; 3],
        1.5,
    );
    let out = sculpt(&target(&g), &brush).unwrap();
    near(out[4], [0., 0., 2.]);
    let legacy = crate::Brush {
        center: [0.; 3],
        radius: 1.5,
        displacement: [0., 0., 2.],
    };
    for (p, q) in g.0.iter().zip(&out) {
        near(legacy.apply(*p).unwrap(), *q);
    }
    assert!(out[4][2] > out[1][2] && out[1][2] > out[0][2] && out[0][2] > 0.);
}
#[test]
fn draw_and_inflate_follow_normals() {
    let mut g = grid();
    let brush = SculptBrush::new(SculptKind::Draw { strength: 1. }, [0.; 3], 1.5);
    let out = sculpt(&target(&g), &brush).unwrap();
    near(out[4], [0., 0., 1.]);
    assert!(out[0][2] > 0. && out[0][2] < out[1][2]);
    // Negative strength carves.
    let carve = SculptBrush::new(SculptKind::Draw { strength: -1. }, [0.; 3], 1.5);
    assert_eq!(sculpt(&target(&g), &carve).unwrap()[4][2], -1.);
    // Tilt one normal: draw uses the area normal, inflate uses each vertex's own.
    g.1[4] = [1., 0., 0.];
    let drawn = sculpt(&target(&g), &brush).unwrap();
    assert!(
        drawn[4][2] > 0.5 && drawn[4][0] > 0.,
        "area normal tilts slightly"
    );
    let inflate = SculptBrush::new(SculptKind::Inflate { strength: 1. }, [0.; 3], 1.5);
    let inflated = sculpt(&target(&g), &inflate).unwrap();
    near(inflated[4], [1., 0., 0.]);
    near(inflated[1], [0., -1., Falloff::Smooth.weight(1. / 1.5)]);
}
#[test]
fn smooth_relaxes_toward_ring_mean() {
    let mut g = grid();
    g.0[4] = [0., 0., 1.];
    let brush = SculptBrush::new(SculptKind::Smooth { strength: 1. }, [0., 0., 1.], 0.5);
    let out = sculpt(&target(&g), &brush).unwrap();
    near(out[4], [0., 0., 0.]);
    for i in (0..9).filter(|&i| i != 4) {
        near(out[i], g.0[i]);
    }
    let half = SculptBrush::new(SculptKind::Smooth { strength: 0.5 }, [0., 0., 1.], 0.5);
    near(sculpt(&target(&g), &half).unwrap()[4], [0., 0., 0.5]);
    assert!(
        sculpt(
            &target(&g),
            &SculptBrush::new(SculptKind::Smooth { strength: 1.5 }, [0.; 3], 1.)
        )
        .is_err()
    );
}
#[test]
fn flatten_projects_onto_area_plane() {
    let mut g = grid();
    g.0[4] = [0., 0., 1.];
    g.0[0] = [-1., -1., -1.];
    let brush = SculptBrush::new(SculptKind::Flatten { strength: 1. }, [0.; 3], 10.);
    brush.validate().unwrap();
    let flat = SculptBrush {
        falloff: Falloff::Constant,
        ..brush
    };
    let out = sculpt(&target(&g), &flat).unwrap();
    let z = out[0][2];
    for p in &out {
        assert!(
            (p[2] - z).abs() < 1e-9,
            "all vertices share the plane height"
        );
        assert!((p[0] - g.0[out.iter().position(|q| q == p).unwrap()][0]).abs() < 1e-9);
    }
    assert!(
        (z - 0.).abs() < 1e-9,
        "weighted mean height of +1 and -1 spikes is zero"
    );
}
#[test]
fn pinch_pulls_tangentially_toward_center() {
    let g = grid();
    let brush = SculptBrush {
        falloff: Falloff::Constant,
        ..SculptBrush::new(SculptKind::Pinch { strength: 0.5 }, [0.; 3], 10.)
    };
    let out = sculpt(&target(&g), &brush).unwrap();
    near(out[0], [-0.5, -0.5, 0.]);
    near(out[4], [0., 0., 0.]);
    near(out[5], [0.5, 0., 0.]);
    let full = SculptBrush {
        kind: SculptKind::Pinch { strength: 1. },
        ..brush
    };
    for p in sculpt(&target(&g), &full).unwrap() {
        near(p, [0.; 3]);
    }
}
#[test]
fn symmetry_mirrors_center_and_displacement() {
    let g = grid();
    let mut brush = SculptBrush::new(
        SculptKind::Grab {
            displacement: [1., 0., 1.],
        },
        [1., 0., 0.],
        0.5,
    );
    brush.symmetry = Symmetry {
        axes: [true, false, false],
        origin: [0.; 3],
    };
    let out = sculpt(&target(&g), &brush).unwrap();
    near(out[5], [2., 0., 1.]);
    near(out[3], [-2., 0., 1.]);
    near(out[4], [0., 0., 0.]);
    // Center on the mirror plane: the pass is not doubled.
    brush.center = [0.; 3];
    let once = sculpt(&target(&g), &brush).unwrap();
    near(once[4], [1., 0., 1.]);
    // Three axes: eight passes touch every corner of a cube of vertices.
    let cube: Vec<Point> = (0..8)
        .map(|b| std::array::from_fn(|k| if b >> k & 1 == 0 { -1. } else { 1. }))
        .collect();
    let normals = cube.iter().map(|p| unit(*p)).collect::<Vec<_>>();
    let t = SculptTarget {
        positions: cube.clone(),
        normals,
        adjacency: vec![],
    };
    let mut inflate = SculptBrush::new(SculptKind::Inflate { strength: 1. }, [1., 1., 1.], 0.5);
    inflate.symmetry.axes = [true; 3];
    let out = sculpt(&t, &inflate).unwrap();
    for (p, q) in cube.iter().zip(&out) {
        near(*q, add(*p, unit(*p)));
    }
}
#[test]
fn validation_rejects_bad_brushes_and_targets() {
    let g = grid();
    let ok = SculptBrush::new(SculptKind::Draw { strength: 1. }, [0.; 3], 1.);
    for radius in [0., -1., f64::NAN, 1e6 + 1.] {
        assert!(
            SculptBrush {
                radius,
                ..ok.clone()
            }
            .validate()
            .is_err()
        );
    }
    assert!(
        SculptBrush {
            center: [f64::INFINITY, 0., 0.],
            ..ok.clone()
        }
        .validate()
        .is_err()
    );
    assert!(
        SculptBrush {
            symmetry: Symmetry {
                axes: [true; 3],
                origin: [f64::NAN; 3]
            },
            ..ok.clone()
        }
        .validate()
        .is_err()
    );
    assert!(SculptKind::Draw { strength: f64::NAN }.validate().is_err());
    assert!(SculptKind::Inflate { strength: 1e7 }.validate().is_err());
    assert!(SculptKind::Flatten { strength: -0.1 }.validate().is_err());
    assert!(SculptKind::Pinch { strength: 1.1 }.validate().is_err());
    assert!(
        SculptKind::Grab {
            displacement: [1e7, 0., 0.]
        }
        .validate()
        .is_err()
    );
    // Missing normals for a normal-based brush; missing adjacency for smooth.
    let no_normals = SculptTarget {
        normals: vec![],
        ..target(&g)
    };
    assert!(sculpt(&no_normals, &ok).is_err());
    assert!(
        sculpt(
            &no_normals,
            &SculptBrush::new(
                SculptKind::Grab {
                    displacement: [0., 0., 1.]
                },
                [0.; 3],
                1.
            )
        )
        .is_ok()
    );
    let no_ring = SculptTarget {
        adjacency: vec![],
        ..target(&g)
    };
    assert!(
        sculpt(
            &no_ring,
            &SculptBrush::new(SculptKind::Smooth { strength: 1. }, [0.; 3], 1.)
        )
        .is_err()
    );
    let empty = SculptTarget::default();
    assert!(sculpt(&empty, &ok).is_err());
    let t = SculptTarget {
        adjacency: vec![vec![99usize]; 9],
        ..target(&g)
    };
    assert!(
        sculpt(
            &t,
            &SculptBrush::new(SculptKind::Smooth { strength: 1. }, [0.; 3], 1.)
        )
        .is_err()
    );
}
#[test]
fn from_faces_builds_outward_unit_normals_and_rings_for_any_arity() {
    // Unit cube as six quads (CCW seen from outside) and as twelve triangles.
    let cube: Vec<Point> = (0..8)
        .map(|b| std::array::from_fn(|k| if b >> k & 1 == 0 { -1. } else { 1. }))
        .collect();
    let quads: Vec<Vec<usize>> = vec![
        vec![0, 2, 3, 1],
        vec![4, 5, 7, 6],
        vec![0, 1, 5, 4],
        vec![2, 6, 7, 3],
        vec![0, 4, 6, 2],
        vec![1, 3, 7, 5],
    ];
    let tris: Vec<Vec<usize>> = quads
        .iter()
        .flat_map(|q| [vec![q[0], q[1], q[2]], vec![q[0], q[2], q[3]]])
        .collect();
    let from_quads =
        SculptTarget::from_faces(cube.clone(), quads.iter().map(Vec::as_slice)).unwrap();
    let from_tris =
        SculptTarget::from_faces(cube.clone(), tris.iter().map(Vec::as_slice)).unwrap();
    for (i, p) in cube.iter().enumerate() {
        near(from_quads.normals[i], unit(*p));
        // Fan triangulation weights the corner unevenly; direction stays outward and unit.
        assert!((norm(from_tris.normals[i]) - 1.).abs() < 1e-12);
        assert!(dot(from_tris.normals[i], unit(*p)) > 0.9);
        assert_eq!(from_quads.adjacency[i].len(), 3);
        assert!(
            from_quads.adjacency[i]
                .iter()
                .all(|j| from_tris.adjacency[i].contains(j))
        );
    }
    assert!(SculptTarget::from_faces(cube, [[0usize, 1, 9].as_slice()]).is_err());
    assert_eq!(SculptTarget::positions(vec![[0.; 3]]).normals.len(), 0);
}
#[cfg(feature = "codec")]
#[test]
fn brush_round_trips_through_codec_with_defaults() {
    let brush = SculptBrush {
        kind: SculptKind::Flatten { strength: 0.3 },
        center: [1., 2., 3.],
        radius: 4.,
        falloff: Falloff::Root,
        symmetry: Symmetry {
            axes: [true, false, true],
            origin: [0., 1., 0.],
        },
    };
    let back: SculptBrush =
        value_codec::from_value(value_codec::to_value(brush.clone()).unwrap()).unwrap();
    assert_eq!(back, brush);
    let mut minimal = Map::new();
    minimal.insert("kind".into(), Value::String("grab".into()));
    minimal.insert("displacement".into(), [0., 0., 1.].to_value());
    minimal.insert("center".into(), [0.; 3].to_value());
    minimal.insert("radius".into(), 2f64.to_value());
    let parsed: SculptBrush = value_codec::from_value(Value::Object(minimal.clone())).unwrap();
    assert_eq!(parsed.falloff, Falloff::Smooth);
    assert_eq!(parsed.symmetry, Symmetry::default());
    minimal.insert("kind".into(), Value::String("twirl".into()));
    assert!(value_codec::from_value::<SculptBrush>(Value::Object(minimal.clone())).is_err());
    minimal.insert("kind".into(), Value::String("draw".into()));
    assert!(
        value_codec::from_value::<SculptBrush>(Value::Object(minimal)).is_err(),
        "draw needs strength"
    );
}
