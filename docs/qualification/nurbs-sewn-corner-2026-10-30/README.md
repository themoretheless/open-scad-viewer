# Bounded native sewn-corner qualification

The Rust trihedral-corner fixture qualifies a closed 13-face B-rep with exact
shared boundaries, no self-intersections, original-stock containment and
positive enclosed volume. The final 14 G2 reports and three prescribed
contact-center radius reports pass on the actual saved geometry.

- Volume: `[999.9580224963673, 999.9581224941044]` mm3.
- Removed volume: `[0.04187750589096594, 0.04197750363732667]` mm3.
- Native libraries: 4269 passed, zero failed, six existing ignores.
- No-default-features library check: passed.
- Public map/catalog: 239 modules, 200 entries; ten infrastructure tests pass.
- Global architecture: 69 pre-existing violations outside this change; no
  changed source file violates its size limit.

This is finite native rectangular-stock corner evidence. General Boolean,
offset, STEP, WASM and editor publication gates remain separate.

[Detailed qualification](../../development/nurbs-sewn-corner-qualification.md)
describes the construction and individual proof gates.
[Lossless JSON record](evidence.json.gz) contains commands, input hashes,
artifact hashes and numeric results. Generated raw JSON remains local under
the repository's qualification-evidence policy.

Logs: [four libraries](native.log.gz), [actual corner](corner.log.gz),
[minimal features](minimal.log.gz), [architecture](architecture.log.gz),
[infrastructure](infrastructure.log.gz), [catalog](catalog.log.gz).
