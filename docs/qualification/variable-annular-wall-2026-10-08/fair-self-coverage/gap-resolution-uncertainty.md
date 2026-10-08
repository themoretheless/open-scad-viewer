# Independent unresolved gap inspection

Re-admitted the unchanged original body and independently reran all 32 pairs classified as `source-face-gap-resolution-limit`, each with 100,000 cells and 100,000 spans. All 32 remain unresolved. Cell usage ranges from 239 to 1557; additional global work alone does not resolve these pairs.

The archived UV intervals show contraction to floating-point scale on both charts. This evidence directs the next investigation toward interval enclosure tightness or retained-domain/normal exclusion, rather than another budget increase. It does not prove a material wall defect or self-intersection. Exact UV values and counters are preserved in `gap-uncertainty.json.gz`.

Reproduction: `cargo run --release --locked --manifest-path crates/Cargo.toml -p geometry-bridge --example source-gap-uncertainty -- REQUEST.json INVENTORY.json OUTPUT.json`. REQUEST is the original annular body request; INVENTORY is the decompressed chart-close native report.
