use super::*;
#[path = "features_fixture.rs"]
mod fixture;
fn coordinate_error(a: &[Feature], b: &[Feature], m: &Match, angle: f64) -> f64 {
    let (u, v) = (a[m.a].x - 160., a[m.a].y - 128.);
    let x = 160.37 + angle.cos() * u - angle.sin() * v;
    let y = 127.72 + angle.sin() * u + angle.cos() * v;
    (b[m.b].x - x).powi(2) + (b[m.b].y - y).powi(2)
}
fn errors(seed: u64, angle: f64, options: FeatureOptions) -> (usize, usize, f64) {
    let a = extract_with_options(&fixture::texture(seed, 0., 1., 0., 0.), 550, &options).unwrap();
    let b = extract_with_options(
        &fixture::texture(seed, angle, 1., 0.37, -0.28),
        550,
        &options,
    )
    .unwrap();
    let pairs = matches(&a, &b);
    let mut correct = 0;
    let mut squared = 0.;
    for m in &pairs {
        let (u, v) = (a[m.a].x - 160., a[m.a].y - 128.);
        let x = 160.37 + angle.cos() * u - angle.sin() * v;
        let y = 127.72 + angle.sin() * u + angle.cos() * v;
        let e = (b[m.b].x - x).powi(2) + (b[m.b].y - y).powi(2);
        if e < 4. {
            correct += 1;
            squared += e;
        }
        assert!(m.distance_squared.is_finite() && m.ratio < 0.8);
    }
    (
        pairs.len(),
        correct,
        (squared / correct.max(1) as f64).sqrt(),
    )
}
#[test]
fn subpixel_reduces_fractional_translation_error_on_unfitted_texture() {
    for seed in [2803, 7129] {
        let baseline = errors(seed, 0., FeatureOptions::BASELINE);
        let subpixel = errors(
            seed,
            0.,
            FeatureOptions {
                subpixel: true,
                ..FeatureOptions::BASELINE
            },
        );
        assert!(subpixel.1 > 100);
        assert!(
            subpixel.2 < baseline.2 * 0.75,
            "baseline={baseline:?} subpixel={subpixel:?}"
        );
        assert!(subpixel.1 * 100 >= subpixel.0 * 98);
    }
}
#[test]
fn interpolated_and_root_histograms_preserve_precision_under_rotation() {
    for seed in [2803, 7129] {
        let baseline = errors(seed, 17f64.to_radians(), FeatureOptions::BASELINE);
        for options in [FeatureOptions::REFINED, FeatureOptions::ROOT] {
            let result = errors(seed, 17f64.to_radians(), options);
            assert!(result.1 > baseline.1);
            assert!(result.1 * 100 >= result.0 * 98, "{result:?}");
            assert!(result.2 < 0.45, "{result:?}");
        }
    }
}
#[test]
fn fractional_coordinates_descriptors_and_quality_are_deterministic() {
    let image = fixture::texture(2803, 0., 1., 0., 0.);
    let first = extract(&image, 300).unwrap();
    let second = extract(&image, 300).unwrap();
    assert_eq!(first.len(), second.len());
    assert!(first.iter().any(|f| f.x.fract() != 0. || f.y.fract() != 0.));
    for (a, b) in first.iter().zip(second) {
        assert_eq!((a.x, a.y, a.descriptor), (b.x, b.y, b.descriptor));
        assert!(a.descriptor.iter().all(|v| v.is_finite() && *v >= 0.));
        assert!((a.descriptor.iter().map(|v| v * v).sum::<f32>() - 1.).abs() < 1e-5);
    }
}

#[test]
fn bounded_matching_agrees_bit_for_bit_with_exhaustive_distances() {
    let mut seed = 0x91b62c7u64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed as u32) as f32 / u32::MAX as f32 * 0.15
    };
    let a = (0..64)
        .map(|_| Feature {
            x: 0.,
            y: 0.,
            descriptor: std::array::from_fn(|_| next()),
        })
        .collect::<Vec<_>>();
    let mut b = a
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let mut f = f.clone();
            if i % 3 != 0 {
                for value in &mut f.descriptor {
                    *value += next() * 0.04;
                }
            }
            f
        })
        .collect::<Vec<_>>();
    // Exact duplicate neighbors exercise the strict ratio gate and tie order.
    b.push(a[0].clone());
    b.push(a[17].clone());
    let distances = a
        .iter()
        .map(|x| {
            b.iter()
                .map(|y| {
                    let mut d = 0.;
                    for k in 0..128 {
                        let v = x.descriptor[k] - y.descriptor[k];
                        d += v * v;
                    }
                    d
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut expected = Vec::new();
    for i in 0..a.len() {
        let mut order = (0..b.len()).collect::<Vec<_>>();
        order.sort_by(|&j, &k| distances[i][j].total_cmp(&distances[i][k]));
        let (j, second) = (order[0], distances[i][order[1]]);
        let d = distances[i][j];
        let reverse = (0..a.len())
            .min_by(|&x, &y| distances[x][j].total_cmp(&distances[y][j]))
            .unwrap();
        if reverse == i && d < 0.64 * second && d < 0.9 {
            expected.push((i, j, d.to_bits(), (d / second).sqrt().to_bits()));
        }
    }
    let actual = matches(&a, &b)
        .into_iter()
        .map(|m| (m.a, m.b, m.distance_squared.to_bits(), m.ratio.to_bits()))
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
    assert!(actual.len() > 50);
    assert!(matches(&[], &b).is_empty());
    assert!(matches(&a, &[]).is_empty());
}

#[test]
fn second_chance_appends_relaxed_mutual_matches_without_precision_regression() {
    fn bits(ms: &[Match]) -> Vec<(usize, usize, u32, u32)> {
        ms.iter()
            .map(|m| (m.a, m.b, m.distance_squared.to_bits(), m.ratio.to_bits()))
            .collect()
    }
    for seed in [2803, 7129] {
        let angle = 17f64.to_radians();
        let strict = FeatureOptions::ROOT;
        let relaxed = FeatureOptions {
            second_chance: true,
            ..strict
        };
        let a =
            extract_with_options(&fixture::texture(seed, 0., 1., 0., 0.), 550, &strict).unwrap();
        let b = extract_with_options(
            &fixture::texture(seed, angle, 1., 0.37, -0.28),
            550,
            &strict,
        )
        .unwrap();
        // Opt-out is bit-identical to the plain matcher.
        let strict_matches = matches(&a, &b);
        assert_eq!(
            bits(&matches_with_options(&a, &b, &strict)),
            bits(&strict_matches)
        );
        let extended = matches_with_options(&a, &b, &relaxed);
        // Deterministic across runs.
        assert_eq!(
            bits(&extended),
            bits(&matches_with_options(&a, &b, &relaxed))
        );
        // First-pass order is untouched; extras are appended, sorted by distance.
        assert!(extended.len() > strict_matches.len());
        assert_eq!(
            bits(&extended[..strict_matches.len()]),
            bits(&strict_matches)
        );
        assert!(extended[strict_matches.len()..].windows(2).all(|w| {
            w[0].distance_squared
                .total_cmp(&w[1].distance_squared)
                .then(w[0].a.cmp(&w[1].a))
                .then(w[0].b.cmp(&w[1].b))
                .is_le()
        }));
        let correct = |m: &Match| coordinate_error(&a, &b, m, angle) < 4.;
        let strict_correct = strict_matches.iter().filter(|m| correct(m)).count();
        let extras = &extended[strict_matches.len()..];
        let extras_correct = extras.iter().filter(|m| correct(m)).count();
        // Extras are a recall play verified downstream by geometry; the
        // symmetric ratio gate keeps overall precision at the strict level.
        assert!(strict_correct + extras_correct >= strict_correct);
        assert!(
            (strict_correct + extras_correct) * 100 >= extended.len() * 98,
            "precision {}/{:?}",
            strict_correct + extras_correct,
            extended.len()
        );
    }
}

#[test]
fn feature_pyramid_stops_at_supported_sizes_and_respects_zero_limit() {
    for (width, height) in [(48, 48), (95, 97), (96, 96), (191, 193), (192, 192)] {
        let image = Image {
            width,
            height,
            focal: 100.,
            rgb: (0..width * height * 3)
                .map(|i| ((i * 37 + i / width * 53) % 256) as u8)
                .collect(),
        };
        assert!(extract(&image, 0).unwrap().is_empty());
        for options in [
            FeatureOptions::BASELINE,
            FeatureOptions::REFINED,
            FeatureOptions::ROOT,
        ] {
            let features = extract_with_options(&image, 25, &options).unwrap();
            assert!(features.len() <= 25);
            assert!(features.iter().all(|f| f.x >= 0.
                && f.x < width as f64
                && f.y >= 0.
                && f.y < height as f64
                && f.descriptor.iter().all(|d| d.is_finite())));
        }
    }
}
