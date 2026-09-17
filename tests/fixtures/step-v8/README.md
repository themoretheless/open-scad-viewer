Self-authored AP242 Edition 4 `/8` fixtures generated from exact kernel B-reps by:

`cargo run --locked --manifest-path crates/Cargo.toml -p brep-core --example generate_step_v5_fixture -- v8-<kind> <output>`

The cylinder, cone, sphere, and torus fixtures carry whole-domain rational
regularity evidence. The pole cone and sphere use collapsed parameter
boundaries represented by degenerate topology edges. No external model data
or STEPcode-generated code is linked into the kernel.
