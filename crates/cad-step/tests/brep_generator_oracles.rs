//! Property-style test: a deterministic seeded generator of valid B-rep
//! solids (boxes, cylinders, revolves and Booleans of them) built only through
//! public brep-core constructors, checked against oracles that do not reuse
//! the kernel's own bookkeeping:
//!
//! - analytic volume, surface area and centroid from the generating
//!   parameters (compared with quadrature mass properties),
//! - the Euler-Poincare relation V - E + 2F - L = 2(S - G) with the genus G
//!   known from the construction,
//! - closedness: every shell closed and every edge used by exactly two
//!   coedges with opposite orientation.
//!
//! Seeds come from a fixed LCG; set `BREP_ORACLE_SEEDS` to run more. A failure
//! prints the seed and the generated case so it can be replayed.
mod common;
use common::brep_generator::*;
use std::collections::BTreeMap;

#[test]
fn generated_solids_satisfy_independent_oracles() {
    let seeds: u64 = std::env::var("BREP_ORACLE_SEEDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(40);
    let mut failures = Vec::new();
    let mut refused = Vec::new();
    let mut checked = BTreeMap::<&str, usize>::new();
    for seed in 0..seeds {
        let mut rng = Lcg(0xB0E5_0000 ^ seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let case = generate(&mut rng);
        let kind = match case {
            Case::Box { .. } => "box",
            Case::Cylinder { .. } => "cylinder",
            Case::Revolve { .. } => "revolve",
            Case::BoxUnion { .. } => "box-union",
            Case::BoxPocket { .. } => "box-pocket",
            Case::DrilledBox { .. } => "drilled-box",
        };
        let (model, expect) = match build(&case) {
            Ok(built) => built,
            Err(error) => {
                refused.push(format!("seed {seed} {case:?}: {error:?}"));
                continue;
            }
        };
        *checked.entry(kind).or_default() += 1;
        let mut issues = topology(&model, &expect);
        issues.extend(metric(&model, &expect));
        if !issues.is_empty() {
            failures.push(format!("seed {seed} {case:?}:\n  {}", issues.join("\n  ")));
        }
    }
    eprintln!("checked per kind: {checked:?}; refused: {}", refused.len());
    for r in &refused {
        eprintln!("refused {r}");
    }
    // Valid constructions of these simple solids must not be refused.
    assert!(refused.is_empty(), "{} refused constructions:\n{}", refused.len(), refused.join("\n"));
    assert!(
        failures.is_empty(),
        "{} oracle failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// STEP round trip keeps the oracle quantities: export a generated solid with
/// the AP214 writer, import it back and compare volume and topology counts.
#[test]
fn generated_solids_survive_step_round_trip() {
    let mut failures = Vec::new();
    for seed in 0..12u64 {
        let mut rng = Lcg(0x57E9_0000 ^ seed);
        let case = generate(&mut rng);
        let Ok((model, expect)) = build(&case) else { continue };
        let text = match cad_step::export_step_v3(&model) {
            Ok((text, ..)) => text,
            Err(error) => {
                failures.push(format!("seed {seed} {case:?}: export refused: {error:?}"));
                continue;
            }
        };
        match cad_step::import_step_v3(&text) {
            Ok((back, ..)) => {
                let mut issues = topology(&back, &expect);
                issues.extend(metric(&back, &expect));
                if !issues.is_empty() {
                    failures.push(format!("seed {seed} {case:?}:\n  {}", issues.join("\n  ")));
                }
            }
            Err(error) => failures.push(format!("seed {seed} {case:?}: import refused: {error:?}")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
