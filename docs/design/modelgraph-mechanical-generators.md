# Mechanical generators

The application toolbar exposes **Генераторы / Generators**. Select a gear, planetary gearset or thread, edit dimensions, generate and open in the editor. The dialog explicitly offers saving the current source before replacement. ModelGraph JSON can be downloaded as the parameterized source of truth; editor SCAD is a resolved execution artifact.

Both stdio and HTTP MCP expose `modelgraph_generate` and resource `openscad://language/modelgraph-mechanical`. Minimal calls are `{kind:"gear"}`, `{kind:"planetary_gears"}` and `{kind:"thread"}`. They return an editable document, revision hash, design report, actual geometry analysis and three bounded PNG previews. Common compile/check/report/parameter-edit/export tools support the generated nodes. Planetary `mechanical_parts` contains independently exportable local gear documents and assembly poses.

## Spur gears

Own sampled involute construction, without a third-party gear generator. Module, tooth count and pressure angle define pitch and base radii. External tooth thickness at the pitch circle is `pi*module/2-backlash`; matching members each reduce their material tooth thickness, making nominal pair circumferential backlash twice the setting. `clearance` is the additional root depth in millimetres. `bore` is a diameter; internal rings require bore zero and use their toothed opening. Teeth point along +X at zero rotation.

Limits: module 0.1–100 mm, 8–128 teeth, pressure angle 14.5–30°, thickness 0.1–1000 mm, 3–12 flank subdivisions. Unsupported unshifted undercut cases are rejected using `ceil(2/sin(pressure_angle)^2)` minimum external tooth count (18 at20°). Internal tips must remain above the base circle. Profile circles and involutes are discretized; root transitions below the base circle are radial, without cutter-generated trochoidal fillets or profile shifts. This does not certify mating interference for arbitrary independent gear pairs.

Reference: [KHK gear dimension technical reference](https://khkgears.net/gear-knowledge/gear-technical-reference/calculation-gear-dimensions/).

## Planetary gearsets

Fixed ring, sun input and carrier output. `ring_teeth=sun_teeth+2*planet_teeth`; ratio `1+ring_teeth/sun_teeth`. Equally spaced planets require `(sun_teeth+ring_teeth)/planet_count` integer. The generator checks adjacent planet tip-circle separation and a conservative internal involute contact condition. Tooth phases account for odd/even planet tooth counts. `carrier_angle` drives sun/planet positions using the Willis relation; the ring remains fixed. There are 2–6 planets, subject to spacing and geometry budgets.

Defaults: 24-tooth sun, three 24-tooth planets, 72-tooth ring, module2, ratio4:1. Parts remain independent top-level solids. There is no carrier plate, shaft, bearing, housing, motion solver or torque/strength calculation. A gearset export preserves current positions; use individual part documents to arrange a print bed.

Reference: [KHK internal gears, planetary tooth conditions](https://khkgears.net/pdf/2025/internal-gears.pdf).

## Helical threads

Own closed polyhedron sampled along helical profile boundaries, including cap intersections. The basic axial profile has 60° flanks, flat crest width `pitch/8`, flat root width `pitch/4` and depth `5*sqrt(3)*pitch/16`. External output is a threaded rod; internal output is a cylindrical threaded sleeve with radial `wall` thickness. Right/left hand and 1–4 starts are supported. `lead=starts*pitch`.

`clearance` represents nominal radial gap for a matching pair with equal settings: the external radius decreases by half the setting and the internal cavity radius increases by half. Both partners need matching pitch, starts, handedness and phase. Wall thickness is measured from the cavity major radius. There are no lead-in chamfers, root rounding, runout, tolerance classes or certification for metal fasteners.

Limits: diameter at least twice pitch; length from0.25 to64 pitches; 16–96 angular segments, at least eight per start; at most3500 triangles and200000 generated source characters. Long/fine threads can exceed the mesh budget even when individual parameter ranges are valid; the generator reports this and asks for shorter length, larger pitch or lower resolution. Resolution does not silently change.

Reference: [ISO 68-1:2023 basic and design thread profiles](https://www.iso.org/standard/85107.html). The generator uses a faceted basic profile, not an ISO tolerance-class implementation.

## Validation

Regression tests build all defaults through the production geometry worker and check positive volume and closed/nonmanifold topology. Gear profile area is checked against extrusion volume. Planetary test poses include even and odd planet tooth counts and verify sun/planet and ring/planet overlap volumes. Thread tests cover left/right handedness, multiple starts, internal/external closure and paired overlap with positive clearance. MCP tests exercise generator discovery, document/part reports, PNG images and 3MF export. UI verification builds all three through the dialog.

Mesh validation is not physical print testing. Generated source is additionally capped at220000 characters across expanded mechanical nodes; existing language and worker limits still apply to complex downstream compositions.
