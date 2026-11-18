//! Import -> edit -> export -> re-import corpus for STEP and IGES.
//!
//! Inputs are the seeded generator bodies (with their analytic oracles) and
//! every self-authored STEP fixture under `tests/fixtures`. Each case is
//! imported, edited deterministically (a rigid motion for every case, plus a
//! corner-notch Boolean for boxes), exported with the AP242 v9 writer and the
//! IGES /2 writer, and read back through two STEP import routes (v9 and the
//! certificate-free v3 route) and the IGES /2 reader.
//!
//! Oracles after the round trip: volume, surface area and centroid (rigidly
//! moved), conservative bounding box, topology counts and the closedness /
//! Euler-Poincare checks of the generator. Every refusal is recorded per case
//! and per stage; the test fails only on oracle mismatches and on refusals of
//! stages the kernel claims to support (generated bodies through STEP).
//!
//! Set `STEP_CORPUS_OUTPUT=<dir>` to write each exported STEP file with a
//! `<case>.expect.json` sidecar; `scripts/verify-step-roundtrip-corpus.py`
//! then checks them with OpenCascade (CI job `independent-step-occt.yml`).
mod common;
use brep_core::Model;
use common::brep_generator::*;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Rotation by 90 degrees about z followed by a translation: exact on the
/// rotation part, so axis-aligned bounds map to axis-aligned bounds.
const SHIFT: [f64; 3] = [1.5, -2.25, 0.75];

fn rigid(model: &Model) -> Result<Model> {
    brep_core::transform::affine(
        model,
        [[0., -1., 0., SHIFT[0]], [1., 0., 0., SHIFT[1]], [0., 0., 1., SHIFT[2]], [0., 0., 0., 1.]],
    )
}

fn move_point(p: [f64; 3]) -> [f64; 3] {
    [-p[1] + SHIFT[0], p[0] + SHIFT[1], p[2] + SHIFT[2]]
}

fn move_bounds(b: [[f64; 3]; 2]) -> [[f64; 3]; 2] {
    let (a, c) = (move_point(b[0]), move_point(b[1]));
    [std::array::from_fn(|i| a[i].min(c[i])), std::array::from_fn(|i| a[i].max(c[i]))]
}

#[derive(Clone, Copy, Debug)]
struct Measure {
    volume: f64,
    area: f64,
    centroid: [f64; 3],
    bounds: [[f64; 3]; 2],
    counts: [usize; 6],
}

fn measure(model: &Model) -> std::result::Result<Measure, String> {
    let props = brep_core::analysis::mass_properties(model, 1e-8, 400_000)
        .map_err(|error| format!("mass properties refused: {error:?}"))?;
    Ok(Measure {
        volume: props.signed_volume_mm3,
        area: props.surface_area_mm2,
        centroid: props.centroid,
        bounds: props.conservative_bounds,
        counts: [
            model.vertices.len(),
            model.edges.len(),
            model.loops.len(),
            model.faces.len(),
            model.shells.len(),
            model.bodies.len(),
        ],
    })
}

/// Compares a round-tripped body with the expected measure. Volume, area and
/// centroid use the generator's 1e-5 relative tolerance; conservative bounds
/// depend on the curve/surface representation, so they get a looser 1e-3 of
/// the diagonal.
fn compare(expected: &Measure, actual: &Measure) -> Vec<String> {
    let mut issues = Vec::new();
    let close = |a: f64, e: f64| (a - e).abs() <= 1e-5 * e.abs().max(1.);
    if !close(actual.volume, expected.volume) {
        issues.push(format!("volume {} != {}", actual.volume, expected.volume));
    }
    if !close(actual.area, expected.area) {
        issues.push(format!("area {} != {}", actual.area, expected.area));
    }
    if (0..3).any(|i| (actual.centroid[i] - expected.centroid[i]).abs() > 1e-5 * (1. + expected.centroid[i].abs())) {
        issues.push(format!("centroid {:?} != {:?}", actual.centroid, expected.centroid));
    }
    let diagonal = (0..3)
        .map(|i| (expected.bounds[1][i] - expected.bounds[0][i]).powi(2))
        .sum::<f64>()
        .sqrt();
    if (0..2).any(|k| (0..3).any(|i| (actual.bounds[k][i] - expected.bounds[k][i]).abs() > 1e-3 * diagonal.max(1.))) {
        issues.push(format!("bounds {:?} != {:?}", actual.bounds, expected.bounds));
    }
    if actual.counts != expected.counts {
        issues.push(format!("topology V/E/L/F/S/B {:?} != {:?}", actual.counts, expected.counts));
    }
    issues
}

#[derive(Default)]
struct Outcome {
    passed: Vec<String>,
    refused: Vec<String>,
    failures: Vec<String>,
}

impl Outcome {
    fn report(&self, title: &str) {
        eprintln!(
            "{title}: {} stage passes, {} refusals, {} oracle failures",
            self.passed.len(),
            self.refused.len(),
            self.failures.len()
        );
        for refusal in &self.refused {
            eprintln!("  refused {refusal}");
        }
    }
}

fn output_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("STEP_CORPUS_OUTPUT")?);
    std::fs::create_dir_all(&dir).expect("create STEP_CORPUS_OUTPUT");
    Some(dir)
}

fn write_case(dir: &Path, name: &str, step: &str, expected: &Measure) {
    std::fs::write(dir.join(format!("{name}.step")), step).unwrap();
    let json = format!(
        "{{\"volume\":{:.17e},\"area\":{:.17e},\"centroid\":[{:.17e},{:.17e},{:.17e}],\"faces\":{},\"solids\":{}}}\n",
        expected.volume,
        expected.area,
        expected.centroid[0],
        expected.centroid[1],
        expected.centroid[2],
        expected.counts[3],
        expected.counts[5],
    );
    std::fs::write(dir.join(format!("{name}.expect.json")), json).unwrap();
}

/// Runs the export/re-import stages on an edited body and checks each
/// re-imported body against `expected`.
fn exchange(name: &str, edited: &Model, expected: &Measure, out: &mut Outcome, required_step: bool) {
    let refuse = |out: &mut Outcome, stage: &str, detail: String| {
        let line = format!("{name} {stage}: {detail}");
        if required_step && stage.starts_with("step") {
            out.failures.push(line);
        } else {
            out.refused.push(line);
        }
    };
    match cad_step::export_step_v9(edited) {
        Ok((step, ..)) => {
            if let Some(dir) = output_dir() {
                write_case(&dir, name, &step, expected);
            }
            type Reader = fn(&str) -> std::result::Result<Model, nurbs_core::Error>;
            let readers: [(&str, Reader); 2] = [
                ("step-v9-reimport", |text| cad_step::import_step_v9(text).map(|r| r.0)),
                ("step-v3-reimport", |text| cad_step::import_step_v3(text).map(|r| r.0)),
            ];
            for (stage, read) in readers {
                match read(&step).map_err(|error| format!("{error:?}")).and_then(|back| measure(&back)) {
                    Ok(actual) => {
                        let issues = compare(expected, &actual);
                        if issues.is_empty() {
                            out.passed.push(format!("{name} {stage}"));
                        } else {
                            out.failures.push(format!("{name} {stage}:\n    {}", issues.join("\n    ")));
                        }
                    }
                    // The v3 route predates periodic/pole charts; its refusals
                    // are recorded, never required.
                    Err(detail) if stage == "step-v3-reimport" => {
                        out.refused.push(format!("{name} {stage}: {detail}"))
                    }
                    Err(detail) => refuse(out, stage, detail),
                }
            }
        }
        Err(error) => refuse(out, "step-v9-export", format!("{error:?}")),
    }
    match cad_step::export_iges_v2(edited)
        .map_err(|error| format!("export: {error:?}"))
        .and_then(|(iges, ..)| cad_step::import_iges_v2(&iges).map_err(|error| format!("import: {error:?}")))
        .and_then(|(back, ..)| measure(&back))
    {
        Ok(actual) => {
            let issues = compare(expected, &actual);
            if issues.is_empty() {
                out.passed.push(format!("{name} iges-v2"));
            } else {
                // IGES /2 may legitimately re-split or merge faces; topology
                // differences are recorded as refusals of exact identity,
                // metric differences stay failures.
                let (topology, metric): (Vec<_>, Vec<_>) =
                    issues.into_iter().partition(|issue| issue.starts_with("topology"));
                if metric.is_empty() {
                    out.refused.push(format!("{name} iges-v2 topology identity: {}", topology.join("; ")));
                } else {
                    out.failures.push(format!("{name} iges-v2:\n    {}", metric.join("\n    ")));
                }
            }
        }
        Err(detail) => out.refused.push(format!("{name} iges-v2: {detail}")),
    }
}

#[test]
fn generated_bodies_survive_edit_and_exchange() {
    let seeds: u64 = std::env::var("STEP_CORPUS_SEEDS").ok().and_then(|v| v.parse().ok()).unwrap_or(24);
    let mut out = Outcome::default();
    let mut kinds = BTreeMap::<&str, usize>::new();
    for seed in 0..seeds {
        let mut rng = Lcg(0xC13E_0000 ^ seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let case = generate(&mut rng);
        let name = format!("gen-{seed:02}-{}", case.kind());
        let Ok((model, expect)) = build(&case) else {
            out.refused.push(format!("{name} build"));
            continue;
        };
        *kinds.entry(case.kind()).or_default() += 1;
        // Import first, so the edit runs on an imported body, as in the app.
        let imported = match cad_step::export_step_v9(&model)
            .and_then(|(text, ..)| cad_step::import_step_v9(&text))
        {
            Ok((imported, ..)) => imported,
            Err(error) => {
                out.failures.push(format!("{name} initial import: {error:?}"));
                continue;
            }
        };
        let Ok(before) = measure(&imported) else {
            out.failures.push(format!("{name} imported body has no mass properties"));
            continue;
        };
        let moved = match rigid(&imported) {
            Ok(moved) => moved,
            Err(error) => {
                out.refused.push(format!("{name} rigid edit: {error:?}"));
                continue;
            }
        };
        let moved_expect = Expect {
            volume: expect.volume,
            area: expect.area,
            centroid: expect.centroid.map(move_point),
            shells: expect.shells,
            genus: expect.genus,
        };
        let mut issues = topology(&moved, &moved_expect);
        issues.extend(metric(&moved, &moved_expect));
        if !issues.is_empty() {
            out.failures.push(format!("{name} analytic oracle after edit:\n    {}", issues.join("\n    ")));
        }
        let expected = Measure {
            centroid: move_point(before.centroid),
            bounds: move_bounds(before.bounds),
            ..before
        };
        exchange(&name, &moved, &expected, &mut out, true);

        // Boolean edit on imported boxes: removing the corner octant
        // [max - d/2, max + 1] takes prod(d/2) of volume and keeps the area.
        if let Case::Box { min, max } = case {
            let d: [f64; 3] = std::array::from_fn(|i| max[i] - min[i]);
            let notch_min = std::array::from_fn(|i| max[i] - d[i] / 2.);
            let notch_max = std::array::from_fn(|i| max[i] + 1.);
            let notched = brep_core::cuboid(notch_min, notch_max)
                .and_then(|notch| brep_core::operations::boolean(&imported, &notch, "difference"));
            let notched = match notched {
                Ok(notched) => notched,
                Err(error) => {
                    out.refused.push(format!("{name}-notch boolean edit: {error:?}"));
                    continue;
                }
            };
            let notch_expect = Expect {
                volume: expect.volume - d.iter().map(|v| v / 2.).product::<f64>(),
                area: expect.area,
                centroid: None,
                shells: 1,
                genus: 0,
            };
            let mut issues = topology(&notched, &notch_expect);
            issues.extend(metric(&notched, &notch_expect));
            if !issues.is_empty() {
                out.failures.push(format!("{name}-notch analytic oracle:\n    {}", issues.join("\n    ")));
                continue;
            }
            match measure(&notched) {
                Ok(m) => exchange(&format!("{name}-notch"), &notched, &m, &mut out, true),
                Err(detail) => out.failures.push(format!("{name}-notch: {detail}")),
            }
        }
    }
    eprintln!("generated kinds: {kinds:?}");
    out.report("generated corpus");
    assert!(out.failures.is_empty(), "{} failures:\n{}", out.failures.len(), out.failures.join("\n"));
    assert!(out.passed.len() >= seeds as usize * 2, "too few stage passes: {}", out.passed.len());
}

fn fixture_files() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    let mut files = Vec::new();
    for dir in ["step-v3", "step-v5", "step-v6", "step-v8", "step-v9", "loft"] {
        let Ok(entries) = std::fs::read_dir(root.join(dir)) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if matches!(path.extension().and_then(|e| e.to_str()), Some("step" | "stp")) {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

#[test]
fn step_fixtures_survive_edit_and_exchange() {
    type Importer = fn(&str) -> std::result::Result<Model, nurbs_core::Error>;
    let importers: [(&str, Importer); 5] = [
        ("v9", |t| cad_step::import_step_v9(t).map(|r| r.0)),
        ("v8", |t| cad_step::import_step_v8(t).map(|r| r.0)),
        ("v6", |t| cad_step::import_step_v6(t).map(|r| r.0)),
        ("v5", |t| cad_step::import_step_v5(t).map(|r| r.0)),
        ("v3", |t| cad_step::import_step_v3(t).map(|r| r.0)),
    ];
    let files = fixture_files();
    assert!(files.len() >= 20, "fixture corpus shrank to {}", files.len());
    let mut out = Outcome::default();
    let mut imported_count = 0;
    for path in &files {
        let parent = path.parent().and_then(|p| p.file_name()).unwrap().to_string_lossy();
        let stem = path.file_stem().unwrap().to_string_lossy();
        let name = format!("fixture-{parent}-{stem}");
        let text = std::fs::read_to_string(path).unwrap();
        let mut errors = Vec::new();
        let mut model = None;
        for (route, import) in importers {
            match import(&text) {
                Ok(found) => {
                    model = Some((route, found));
                    break;
                }
                Err(error) => errors.push(format!("{route}: {error:?}")),
            }
        }
        let Some((_route, model)) = model else {
            out.refused.push(format!("{name} import: {}", errors.join(" | ")));
            continue;
        };
        imported_count += 1;
        let before = match measure(&model) {
            Ok(before) => before,
            Err(detail) => {
                // Open shells have no volume; they still exercise exchange.
                out.refused.push(format!("{name} measure: {detail}"));
                continue;
            }
        };
        let moved = match rigid(&model) {
            Ok(moved) => moved,
            Err(error) => {
                out.refused.push(format!("{name} rigid edit: {error:?}"));
                continue;
            }
        };
        let expected = Measure {
            centroid: move_point(before.centroid),
            bounds: move_bounds(before.bounds),
            ..before
        };
        match measure(&moved) {
            Ok(after_edit) => {
                let issues = compare(&expected, &after_edit);
                if !issues.is_empty() {
                    out.failures.push(format!("{name} edit:\n    {}", issues.join("\n    ")));
                    continue;
                }
            }
            Err(detail) => {
                out.failures.push(format!("{name} edit: {detail}"));
                continue;
            }
        }
        exchange(&name, &moved, &expected, &mut out, false);
    }
    out.report("fixture corpus");
    assert!(imported_count >= 15, "only {imported_count} fixtures imported");
    assert!(out.failures.is_empty(), "{} failures:\n{}", out.failures.len(), out.failures.join("\n"));
}
