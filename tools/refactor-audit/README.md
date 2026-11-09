# refactor-audit

Read-only measurements behind `docs/refactor/`. One command:

```sh
tools/refactor-audit/run.sh
```

It writes raw data to `tools/refactor-audit/.out/audit/` (ignored by git) and regenerates
`docs/refactor/INVENTORY.md` and `docs/refactor/inventory/*.md`. `AUDIT_LINTS=0` skips the clippy and
dead-code passes; the rest takes about two minutes.

| Step | Tool | Output |
| --- | --- | --- |
| Rust functions, files, types, module tree, references | `audit-rs` (`syn`) | `functions.tsv`, `files.tsv`, `types.tsv`, `refs.tsv`, `moddecls.tsv`, `allows.tsv` |
| Per-crate totals and rankings | `summarize.py` | `summary.json`, `summary.txt` |
| Exact clones | `dupes.py` | `dupes-rs.json`, `dupes-ts-src.json` |
| Functions that are textually identical in two compiled files | `duptests.py`, `cmpfiles.py` | `duptests.json` |
| Module graphs, cycles, unused dependency candidates | `graph.py` | `graph.json`, `graph.txt` |
| Files no Cargo target reaches | `orphans.py` | `orphans.json`, `orphan-decision.json`, `delete-rs.txt` |
| Layer violations inside `nurbs-core` | `nurbs_layers.py` | `nurbs-layers.json` |
| Coupling between candidate contexts of `brep-core` | `brep_families.py` | `brep-families.json` |
| TypeScript and Svet functions, import graph | `audit-ts.mjs` | `ts.json` |
| TypeScript modules unreachable from entry points | `ts_reach.py` | `ts-reach.json` |
| Clippy and forced dead-code warnings | `lints.py` | `clippy-summary.json`, `forcewarn-summary.json` |

## How the numbers are defined

- A function is production code when its file belongs to a `lib` or `bin` target and it is outside
  `#[cfg(test)]`. `tests/`, `examples/` and `benches/` are separate target kinds.
- Cyclomatic complexity: 1 plus `if`, loops, extra `match` arms, guards, `&&`, `||`, `let ... else`.
  Cognitive complexity adds the nesting depth to each branching construct.
- A file is unreachable when it cannot be reached from any Cargo target root through `mod`,
  `#[path]` or `include!`. The module tree is resolved with the compiler's rules, including inline
  modules and `#[path]`.
- A clone is a run of identical lines after trimming whitespace and dropping blank lines, comments,
  attributes, `use` lines and lines of brackets only. Minimum run: 30 lines for Rust, 20 for TypeScript.
- Macro bodies are not parsed, so calls inside `assert!`, `vec!` and similar are not counted.
- `do yeet e` is read as `return e`, because `syn` 2 has no `yeet_expr`.

`audit-rs` is a standalone package with its own lock file. It is not a member of `crates/`.
