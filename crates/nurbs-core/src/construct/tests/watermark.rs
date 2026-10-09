use super::*;

/// Свободная бикубическая поверхность 24×24 с горбом, clamped knots
/// (576 базисных функций — вмещает 16-байтовый payload: 432 кодовых бита).
fn freeform_surface() -> Surface {
    let nu = 24;
    let nv = 24;
    let clamped = |n: usize, degree: usize| {
        let mut k = vec![0.; degree];
        k.extend((0..=(n - degree)).map(|v| v as f64));
        k.extend(vec![(n - degree) as f64; degree]);
        k
    };
    let surface = Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: clamped(nu, 3),
        knots_v: clamped(nv, 3),
        control_points: (0..nu)
            .map(|i| {
                (0..nv)
                    .map(|j| {
                        let x = i as f64 * 4.;
                        let y = j as f64 * 3.3;
                        let z = 8. * (i as f64 / 6.).sin() * (j as f64 / 5.).cos()
                            + 0.02 * (i * j) as f64;
                        vec![x, y, z]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; nv]; nu],
        periodic_u: false,
        periodic_v: false,
    };
    surface.validate().unwrap();
    surface
}

/// Плоская равномерная сетка — идеальный случай безреференсного приёма.
fn planar_surface() -> Surface {
    let mut s = freeform_surface();
    for (i, row) in s.control_points.iter_mut().enumerate() {
        for (j, p) in row.iter_mut().enumerate() {
            *p = vec![i as f64 * 4., j as f64 * 3.3, 0.];
        }
    }
    s
}

fn payload() -> Vec<u8> {
    (0u8..16).map(|i| i.wrapping_mul(37).wrapping_add(11)).collect()
}

#[test]
fn round_trip_recovers_payload_without_errors() {
    let surface = freeform_surface();
    let data = payload();
    let report = embed(&surface, &data, 42, 0.05, WatermarkOptions::default()).unwrap();
    let extracted = extract(&report.surface, 42, data.len(), Some(&surface)).unwrap();
    assert_eq!(extracted.payload, data);
    assert_eq!(extracted.corrected_bits, 0);
    assert_eq!(extracted.parity_failures, 0);
    assert!(extracted.min_confidence > 0.5);
}

#[test]
fn deviation_stays_well_below_tolerance() {
    // Аналог «5 мкм при допуске 50 мкм»: амплитуда = допуск / 10.
    let surface = freeform_surface();
    let report = embed(&surface, &payload(), 7, 0.05, WatermarkOptions::default()).unwrap();
    assert!(report.max_deviation <= 0.005 + 1e-12);
    let check = max_deviation(&surface, &report.surface, DEFAULT_GRID).unwrap();
    assert!(check <= 0.05);
}

#[test]
fn extraction_survives_uniform_scale_and_rotation() {
    let surface = freeform_surface();
    let data = payload();
    let mut marked = embed(&surface, &data, 99, 0.05, WatermarkOptions::default())
        .unwrap()
        .surface;
    // Поворот на 90° вокруг z, масштаб 3.7, перенос.
    for p in marked.control_points.iter_mut().flatten() {
        let (x, y, z) = (p[0], p[1], p[2]);
        *p = vec![-3.7 * y + 100., 3.7 * x - 40., 3.7 * z + 5.];
    }
    let extracted = extract(&marked, 99, data.len(), Some(&surface)).unwrap();
    assert_eq!(extracted.payload, data);
    assert_eq!(extracted.corrected_bits, 0);
}

#[test]
fn ecc_repairs_single_bit_corruption() {
    let surface = freeform_surface();
    let data = payload();
    let report = embed(&surface, &data, 5, 0.05, WatermarkOptions::default()).unwrap();
    let mut damaged = report.surface.clone();
    // Инвертируем один кодовый бит: удвоенный сдвиг против знака.
    let coded_len = data.len() * 9 * REPETITIONS;
    let order = pyramid_basis(24, 24, coded_len);
    let (permutation, _) = key_schedule(5, coded_len);
    let (i, j) = order[permutation[3]];
    let normal = greville_normal(&surface, i, j).unwrap();
    let original = &surface.control_points[i][j];
    let marked = &report.surface.control_points[i][j];
    for d in 0..3 {
        damaged.control_points[i][j][d] =
            original[d] - (marked[d] - original[d]) - 1e-9 * normal[d];
    }
    let extracted = extract(&damaged, 5, data.len(), Some(&surface)).unwrap();
    assert_eq!(extracted.payload, data);
    assert!(extracted.corrected_bits >= 1);
}

#[test]
fn amplitude_above_tolerance_is_rejected() {
    let surface = freeform_surface();
    let options = WatermarkOptions {
        amplitude: Some(0.05),
        ..WatermarkOptions::default()
    };
    assert!(embed(&surface, &payload(), 1, 0.05, options).is_err());
    let options = WatermarkOptions {
        amplitude: Some(0.5),
        ..WatermarkOptions::default()
    };
    assert!(embed(&surface, &payload(), 1, 0.05, options).is_err());
}

#[test]
fn embedding_is_deterministic_per_seed() {
    let surface = freeform_surface();
    let a = embed(&surface, &payload(), 123, 0.05, WatermarkOptions::default()).unwrap();
    let b = embed(&surface, &payload(), 123, 0.05, WatermarkOptions::default()).unwrap();
    assert_eq!(a.surface, b.surface);
    assert_eq!(a.max_deviation, b.max_deviation);
    let c = embed(&surface, &payload(), 124, 0.05, WatermarkOptions::default()).unwrap();
    assert_ne!(a.surface, c.surface);
}

#[test]
fn reference_free_extraction_on_planar_surface() {
    let surface = planar_surface();
    let data = payload();
    let report = embed(&surface, &data, 77, 0.05, WatermarkOptions::default()).unwrap();
    let extracted = extract(&report.surface, 77, data.len(), None).unwrap();
    assert_eq!(extracted.payload, data);
    assert_eq!(extracted.parity_failures, 0);
}

#[test]
fn non_finite_tolerance_and_amplitude_are_typed_boundary_errors() {
    let surface = freeform_surface();
    let data = payload();
    let err = embed(&surface, &data, 1, f64::NAN, WatermarkOptions::default()).unwrap_err();
    assert!(err.contains("watermark_tolerance"), "{err}");
    assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
    let err = embed(&surface, &data, 1, f64::INFINITY, WatermarkOptions::default()).unwrap_err();
    assert!(err.contains("watermark_tolerance"), "{err}");
    let options = WatermarkOptions {
        amplitude: Some(f64::NAN),
        ..WatermarkOptions::default()
    };
    let err = embed(&surface, &data, 1, 0.05, options).unwrap_err();
    assert!(err.contains("watermark_amplitude"), "{err}");
}

#[test]
fn digest_is_deterministic_and_negative_zero_stable() {
    let surface = freeform_surface();
    let a = digest(&surface).unwrap();
    let b = digest(&surface).unwrap();
    assert_eq!(a, b, "digest must be deterministic");
    // −0.0 канонизируется в +0.0: дайджест не должен измениться.
    // z(0,0) = 8·sin(0)·cos(0) + 0.02·0 = 0 в freeform_surface.
    assert_eq!(surface.control_points[0][0][2], 0.);
    let mut flipped = surface.clone();
    flipped.control_points[0][0][2] = -0.0;
    flipped.validate().unwrap();
    assert_eq!(digest(&flipped).unwrap(), a, "−0.0 must hash like +0.0");
    // Любое реальное изменение координаты меняет дайджест.
    let mut moved = surface.clone();
    moved.control_points[3][4][2] += 1e-9;
    moved.validate().unwrap();
    assert_ne!(digest(&moved).unwrap(), a);
}

#[test]
fn embed_preserves_digest_up_to_marked_shifts() {
    // Дайджест размеченной поверхности отличается (сдвиги контрольных
    // точек), но детерминирован: два embed'а с одним ключом совпадают.
    let surface = freeform_surface();
    let data = payload();
    let a = embed(&surface, &data, 42, 0.05, WatermarkOptions::default()).unwrap();
    let b = embed(&surface, &data, 42, 0.05, WatermarkOptions::default()).unwrap();
    assert_eq!(digest(&a.surface).unwrap(), digest(&b.surface).unwrap());
    assert_ne!(digest(&a.surface).unwrap(), digest(&surface).unwrap());
}

#[test]
fn oversized_payload_is_rejected() {
    let surface = freeform_surface();
    let big = vec![0u8; MAX_PAYLOAD_BYTES + 1];
    assert!(embed(&surface, &big, 1, 0.05, WatermarkOptions::default()).is_err());
    // Маленькая сетка 8×9 = 72 базиса: 3 байта → 81 кодовый бит не влезает.
    let mut small = freeform_surface();
    small.control_points.truncate(8);
    small.weights.truncate(8);
    for (row, weights) in small.control_points.iter_mut().zip(&mut small.weights) {
        row.truncate(9);
        weights.truncate(9);
    }
    small.knots_u = vec![0., 0., 0., 0., 1., 2., 3., 4., 5., 5., 5., 5.];
    small.knots_v = {
        let mut k = vec![0.; 3];
        k.extend((0..=6).map(|v| v as f64));
        k.extend(vec![6.; 3]);
        k
    };
    small.validate().unwrap();
    assert!(embed(&small, &[1, 2, 3], 1, 0.05, WatermarkOptions::default()).is_err());
}
