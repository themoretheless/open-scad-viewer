# G-code branch compatibility check

## Scope

Compared main `b1dcd920` with `origin/gcode-core-fdm` (`a96efc17`).
This is a replacement compatibility probe, not a three-way merge result.
The branch remains unmerged; its unique lexer/interpreter work needs selective
integration against current APIs.

## Reproduction

In an isolated temporary directory, extract an archive of main, then overlay
only `crates/gcode-core` from the branch. Run:

```sh
cargo check --offline --manifest-path crates/Cargo.toml -p slicer-core
```

The initial `--locked` attempt stopped at a lockfile update requirement.
Allowing the temporary lockfile to update offline exposed nine compiler errors
in the dependent `gcode-optimize` crate. No production lockfile was changed.

Missing exports include `Flavor`, `JOB_DIALECT`, `JobProfile`, `MeshBody`,
`emit_3mf`, `emit_gcode_3mf_job`, `emit_job`, and `parse_job`.
The branch's `GcodeMove` also lacks the `layer_index` consumed by
`gcode-optimize/src/from_gcode.rs`.

On unchanged main, the control command succeeds:

```sh
cargo check --locked --manifest-path crates/Cargo.toml -p slicer-core
```

## Integration Constraints

- Preserve the current print-preview v2 dialect and job/3MF API.
- Keep layer identity available to the optimization pipeline.
- Compare the branch's FDM interpretation with current foreign-dialect parsing
  using shared fixtures before adopting any implementation.
- Preserve current resource bounds unless workload measurements and refusal
  tests justify changing them.

The failed replacement probe does not establish that every branch feature is
obsolete, or that a selectively resolved merge cannot work. It establishes that
accepting the older crate wholesale is not a compatible integration.

## Selective Parser Follow-up

[Borrowed parser and bounded arc expansion](gcode-borrowed-parser-2026-09-20.md)
records the first selective integration, native controls and a reproduced
compact-extrusion defect in the old branch. The v2/job/3MF APIs remain intact;
the branch's volumetric, thermal and firmware-retraction features are still
pending adaptation rather than discarded or claimed as merged.

[Feedrate overrides](gcode-feedrate-override-2026-09-20.md) implement the
previously ignored M220 behavior in the current reader, with explicit modal
state and preview bounds. This follow-up does not close the remaining branch.
