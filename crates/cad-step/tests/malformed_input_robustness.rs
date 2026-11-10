//! Mutation robustness of every STEP and IGES importer: valid exchange files
//! are damaged in many ways, and each importer must answer with `Ok` or
//! `Err`, never a panic. This is a deterministic, dependency-free stand-in
//! for coverage-guided fuzzing; a failure prints the mutation to replay.
use cad_step::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

type Import = fn(&str) -> bool;

fn importers() -> Vec<(&'static str, Import)> {
    vec![
        ("import_step", |t| import_step(t).is_ok()),
        ("import_step_v2", |t| import_step_v2(t).is_ok()),
        ("import_step_v3", |t| import_step_v3(t).is_ok()),
        ("import_step_v4", |t| import_step_v4(t).is_ok()),
        ("import_step_v5", |t| import_step_v5(t).is_ok()),
        ("import_step_v6", |t| import_step_v6(t).is_ok()),
        ("import_step_v7", |t| import_step_v7(t).is_ok()),
        ("import_step_v8", |t| import_step_v8(t).is_ok()),
        ("import_step_v9", |t| import_step_v9(t).is_ok()),
        ("import_step_v10", |t| import_step_v10(t).is_ok()),
        ("import_nurbs_step", |t| import_nurbs_step(t).is_ok()),
        ("import_nurbs_step_solid", |t| import_nurbs_step_solid(t).is_ok()),
        ("import_nurbs_step_solid_v2", |t| import_nurbs_step_solid_v2(t).is_ok()),
        ("import_nurbs_step_trimmed", |t| import_nurbs_step_trimmed(t).is_ok()),
        ("import_iges", |t| import_iges(t).is_ok()),
        ("import_iges_v2", |t| import_iges_v2(t).is_ok()),
    ]
}

/// Valid inputs: the checked-in STEP fixtures plus files written by the
/// crate's own exporters, so every importer sees at least one input it was
/// designed for.
fn seeds() -> Vec<(String, String)> {
    let mut seeds = Vec::new();
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures");
    for dir in ["step-v3", "step-v5"] {
        let Ok(entries) = std::fs::read_dir(format!("{root}/{dir}")) else {
            continue;
        };
        let mut paths: Vec<_> = entries.flatten().map(|e| e.path()).collect();
        paths.sort();
        for path in paths {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if matches!(ext, "step" | "stp")
                && let Ok(text) = std::fs::read_to_string(&path)
            {
                seeds.push((path.display().to_string(), text));
            }
        }
    }
    if let Ok(solid) = freeform_cuboid_solid([0., 0., 0.], [2., 1., 1.]) {
        if let Ok((text, _)) = export_nurbs_step_solid(&solid) {
            seeds.push(("export_nurbs_step_solid".into(), text));
        }
        if let Ok((text, ..)) = export_nurbs_step_solid_v2(&solid) {
            seeds.push(("export_nurbs_step_solid_v2".into(), text));
        }
    }
    let bicubic = nurbs_core::surface::Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: (0..4)
            .map(|i| {
                (0..4)
                    .map(|j| vec![i as f64, j as f64, ((i * j) % 3) as f64 * 0.25])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 4]; 4],
        periodic_u: false,
        periodic_v: false,
    };
    if let Ok(face) = bicubic_open_face(bicubic.clone())
        && let Ok((text, _)) = export_nurbs_step(&face)
    {
        seeds.push(("export_nurbs_step".into(), text));
    }
    let hole = [[0.4, 0.4], [0.6, 0.4], [0.6, 0.6], [0.4, 0.6]];
    if let Ok(face) = bicubic_trimmed_face(bicubic, hole)
        && let Ok((text, _)) = export_nurbs_step_trimmed(&face)
    {
        seeds.push(("export_nurbs_step_trimmed".into(), text));
    }
    if let Ok(cube) = brep_core::cuboid([0., 0., 0.], [1., 2., 3.]) {
        for (name, text) in [
            ("export_step", export_step(&cube).ok().map(|r| r.0)),
            ("export_step_v2", export_step_v2(&cube).ok().map(|r| r.0)),
            ("export_step_v3", export_step_v3(&cube).ok().map(|r| r.0)),
            ("export_iges", export_iges(&cube).ok().map(|r| r.0)),
            ("export_iges_v2", export_iges_v2(&cube).ok().map(|r| r.0)),
        ] {
            if let Some(text) = text {
                seeds.push((name.into(), text));
            }
        }
    }
    seeds
}

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 17
    }
    fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound.max(1) as u64) as usize
    }
}

/// Replacement tokens that stress number parsing, references and limits.
const TOKENS: &[&str] = &[
    "", "0", "-0", "1E308", "-1E308", "1E-320", "NaN", "inf", "#0", "#999999999",
    "#-1", "$", "*", "()", "((((((((", "''", "'unterminated", ".T.", ".X.", "9999999999999999999",
    "1.5D3", ",", ";", "=",
];

fn mutate(text: &str, rng: &mut Lcg) -> (String, String) {
    let lines: Vec<&str> = text.lines().collect();
    match rng.below(7) {
        0 => {
            let cut = rng.below(text.len());
            let cut = (0..=cut).rev().find(|&i| text.is_char_boundary(i)).unwrap_or(0);
            (format!("truncate at {cut}"), text[..cut].to_string())
        }
        1 => {
            let i = rng.below(lines.len());
            let mut out = lines.clone();
            out.remove(i);
            (format!("delete line {i}"), out.join("\n"))
        }
        2 => {
            let i = rng.below(lines.len());
            let mut out = lines.clone();
            out.insert(i, lines[i]);
            (format!("duplicate line {i}"), out.join("\n"))
        }
        3 => {
            // Replace one numeric run with a hostile token.
            let starts: Vec<usize> = text
                .char_indices()
                .filter(|&(i, c)| {
                    c.is_ascii_digit()
                        && text[..i].chars().last().is_none_or(|p| !p.is_ascii_digit())
                })
                .map(|(i, _)| i)
                .collect();
            if starts.is_empty() {
                return ("unchanged".into(), text.to_string());
            }
            let start = starts[rng.below(starts.len())];
            let end = text[start..]
                .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == 'E' || c == '-'))
                .map_or(text.len(), |n| start + n);
            let token = TOKENS[rng.below(TOKENS.len())];
            (
                format!("number at {start} -> {token:?}"),
                format!("{}{token}{}", &text[..start], &text[end..]),
            )
        }
        4 => {
            let i = rng.below(lines.len());
            let j = rng.below(lines.len());
            let mut out = lines.clone();
            out.swap(i, j);
            (format!("swap lines {i} {j}"), out.join("\n"))
        }
        5 => {
            let mut bytes = text.as_bytes().to_vec();
            let i = rng.below(bytes.len());
            bytes[i] = b"#()',;=$*.0-E\n"[rng.below(14)];
            (
                format!("byte {i} -> {:?}", bytes[i] as char),
                String::from_utf8_lossy(&bytes).into_owned(),
            )
        }
        _ => {
            // Point a random entity reference at a random other id.
            let refs: Vec<usize> = text.match_indices('#').map(|(i, _)| i).collect();
            if refs.is_empty() {
                return ("unchanged".into(), text.to_string());
            }
            let at = refs[rng.below(refs.len())] + 1;
            let end = text[at..]
                .find(|c: char| !c.is_ascii_digit())
                .map_or(text.len(), |n| at + n);
            let target = rng.below(400);
            (
                format!("reference at {at} -> #{target}"),
                format!("{}{target}{}", &text[..at], &text[end..]),
            )
        }
    }
}

#[test]
fn importers_never_panic_on_damaged_exchange_files() {
    let seeds = seeds();
    assert!(seeds.len() >= 8, "only {} seeds", seeds.len());
    let importers = importers();
    let rounds: usize = std::env::var("CAD_STEP_MUTATION_ROUNDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(25);
    // Silence the default hook so expected-free runs stay readable; panics
    // are collected and reported below.
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let mut panics = Vec::new();
    let mut accepted_seeds = vec![false; importers.len()];
    let mut rng = Lcg(0x57E9);
    let mut runs = 0;
    for (name, seed) in &seeds {
        let mut inputs = vec![("original".to_string(), seed.clone())];
        for _ in 0..rounds {
            inputs.push(mutate(seed, &mut rng));
        }
        for (mutation, text) in &inputs {
            for (index, (importer, run)) in importers.iter().enumerate() {
                runs += 1;
                match catch_unwind(AssertUnwindSafe(|| run(text))) {
                    Ok(true) if mutation == "original" => accepted_seeds[index] = true,
                    Ok(_) => {}
                    Err(payload) => {
                        let message = payload
                            .downcast_ref::<String>()
                            .cloned()
                            .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                            .unwrap_or_default();
                        panics.push(format!("{importer} on {name} [{mutation}]: {message}"));
                    }
                }
            }
        }
    }
    std::panic::set_hook(hook);
    eprintln!("{runs} import runs over {} seeds", seeds.len());
    let unseeded: Vec<_> = importers
        .iter()
        .zip(&accepted_seeds)
        .filter(|(_, ok)| !**ok)
        .map(|((name, _), _)| *name)
        .collect();
    eprintln!("importers with no accepted seed (mutations only reach the front end): {unseeded:?}");
    panics.sort();
    panics.dedup();
    assert!(
        panics.is_empty(),
        "{} distinct panics:\n{}",
        panics.len(),
        panics[..panics.len().min(20)].join("\n")
    );
}
