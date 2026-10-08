# mechanical-core

Native sampled gear profiles, planetary gear placement and faceted helical thread
meshes. With `default-features = false` it has no runtime dependencies and emits
ordinary Rust structs and buffers.

```rust
use mechanical_core::{GearOptions, ThreadOptions, gear_profile, thread_mesh};
let gear = gear_profile(&GearOptions { bore: 4., ..Default::default() })?;
assert_eq!(gear.loops.len(), 2);
assert!(gear.report.profile_area_mm2 > 0.);
let thread = thread_mesh(&ThreadOptions::default())?;
assert!(!thread.indices.is_empty());
# Ok::<(), mechanical_core::Error>(())
```

- `gear_profile(&GearOptions)` returns oriented profile loops and a typed report.
  External gears can have a bore; internal gears have a toothed opening and rim.
  Limits: 3..256 teeth, 3..12 flank segments, 6000 profile vertices.
- `planetary_profiles(&PlanetaryOptions)` returns sun, ring and planet profiles
  with typed poses, tooth counts, clearance margins and the sun/carrier ratio.
  It validates equal-spacing assembly and adjacent-planet/involute interference;
  2..6 planets are supported. Carrier plates, bearings and housing are separate.
- `thread_mesh(&ThreadOptions)` returns indexed xyz vertices and triangle corners;
  `thread_radius_at` samples the same periodic profile. Supports internal/external,
  left/right hand and 1..4 starts, with at most 3500 triangles.

Default `codec` preserves existing `gear`, `planetary`, `thread`, `thread_geometry`,
SCAD source and Value report APIs through adapters. Gear and thread numeric
algorithms are shared with the native API; source budgets remain in adapters.
The native gear result is a sampled cross-section. Helical/herringbone source
options in the compatibility API require a subsequent 3D construction.

Profiles have radial root transitions, without generated trochoidal root fillets.
Thread meshes use a faceted metric 60-degree basic profile. Strength, mating fit,
lead-ins, runout and tolerance classes require separate validation.
