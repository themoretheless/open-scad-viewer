# Mechanical generators

The application toolbar exposes **Генераторы / Generators**. Select a gear, planetary gearset or thread, edit dimensions, generate and open in the editor. The dialog explicitly offers saving the current source before replacement. RushGraph JSON can be downloaded as the parameterized source of truth; editor SCAD is a resolved execution artifact.

Both stdio and HTTP MCP expose `rush_generate` and resource `openscad://language/rush-mechanical`. Minimal calls are `{kind:"gear"}`, `{kind:"planetary_gears"}` and `{kind:"thread"}`. They return an editable document, revision hash, design report, actual geometry analysis and three bounded PNG previews. Common compile/check/report/parameter-edit/export tools support the generated nodes. Planetary `mechanical_parts` contains independently exportable local gear documents and assembly poses.

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

## Organizer boxes

The **Box / organizer** button is available through **Generators** on the top bar. It creates editable SCAD and opens the source beside its own 3D preview. External dimensions, separate wall/floor thickness, dividers, floor drainage and integral rail hooks are adjustable.

The default **Organic skeleton** uses a shared-vertex cellular web with pointed pore roofs for upright FDM. Disabling the print optimization restores the rounded, irregular hexagonal pattern. These are perturbed cellular patterns, not a stochastic 3D foam or a Voronoi solver. `cell_size` sets the approximate vertical cell size in print mode and nominal diameter in free-form mode (40 mm by default), `rib` sets the in-plane width between cells (2.4 mm), `wall` sets the depth of the ribs (3 mm), and `pattern_seed` chooses a reproducible pattern. A solid border and pads under the hooks remain. The floor stays solid unless drainage is enabled. Disable Organic skeleton to use circular perforations with a solid back, or disable Open walls for solid walls.

Dimensions, compartment width, hook placement and minimum spacing are checked before generation. There is no hole-count cap. Existing geometry-engine work budgets still apply to dense patterns. Profiles are cut in 2D and extruded before assembly. This generator produces SCAD directly and is not part of the `rush_generate` contract. Generic rail hooks require physical fit and load qualification.

Validation: `node node_modules/vitest/vitest.mjs run tests/boxGenerator.test.ts` builds the default organic box through the admitted production provider, checks volume reduction and closed/nonmanifold topology, checks another seed and rib width, compares a plain shell with its analytical volume, and builds the circular/drain/divider variant. Type checking passes. The default organic model was also built and visually inspected in the browser; physical printing is untested.

### Corner radius and front tilt

`corner_radius` rounds all four footprint corners (0 keeps sharp corners). `front_angle` is the front wall angle from vertical: positive opens outward and negative narrows toward the top. Width/depth describe the external footprint at the floor; height is vertical. The back stays vertical. The front begins tilting above the floor, so top depth is `depth + (height-floor)*tan(front_angle)`. Nominal front wall thickness is measured horizontally; its normal thickness decreases with tilt. Curves are faceted for mesh output.

The rounded body is built as a single lofted shell before cutting pores. Corner posts stay solid. Hook pads and hooks move along the remaining straight rear span; dividers follow the tilted front. Radii that do not fit the footprint/top opening and angles that close the cavity are rejected. Mesh tests cover positive/negative tilt, analytical sharp-corner shell volume, rounded/organic walls with/without hooks, and the combined divider/drain variant (24 mm drain pitch). A 12 mm radius and 15° organic example was built and visually inspected in the browser.

### Whole-box arc bend

`bend_radius` curves the entire assembled box in the XY plane, including its floor, pores, dividers and hooks. Zero keeps it straight. The radius is measured at the rear wall; `width` becomes that wall's arc length and `depth` stays radial at the floor. The total angle is `width/bend_radius` radians. `bend_reverse` reverses the direction: the front then lies toward the arc center and the radius must exceed its greatest depth. `bend_segments` controls maximum source edge span along the arc (default `width/24`). Corner rounds and front tilt are applied before the global bend.

The editable source uses the repository-specific `viewer_bend` mesh module; it is not a standard OpenSCAD module. Export STL/3MF for other software. The host calls the already packaged Rust shared-edge knife and cylindrical deformation APIs, then imports the checked result through the regular solid boundary. There is no new WASM artifact or relaxation of runtime admission. Curvature is sampled: this is not an exact circular BRep or a certified surface-deviation bound. Radius/domain checks reject a full wrap and crossing the bend axis.

Validation in `tests/boxBend.test.ts` covers both directions against the inner/outer annular radii and analytical warped-shell volume (bounded mesh approximation), topology of the organic/rounded/tilted/hook combination, invalid domains and malformed module inputs. Browser verification built radius100 mm, corner radius12 mm and front tilt15°. Current mesh/triangle budgets still apply at very high refinement or dense pore layouts.

### Upright FDM cell optimization and structural grading

`print_optimized` is enabled by default. With the floor on the bed and +Z as build direction, wall pores use 6-vertex organic honeycomb/Voronoi cells with trivalent 3-beam nodes, pointed roofs and mitered corners, while `floor_mesh` uses a shared-vertex organic Voronoi/honeycomb lattice instead of a rectangular grid. The shared-vertex cellular pattern clips only at side and bottom borders; every roof apex stays below the top frame, preventing a clipped horizontal ceiling. Roof slope accounts conservatively for front-wall tilt and the maximum circumferential stretch of the whole-box bend. `overhang_angle` is measured **from vertical**, default45°. A front-wall tilt at or above that angle is rejected in this mode. The old rounded/jittered cells remain available by disabling this setting.

To reduce material while retaining stiffness, wall ribs are graded by height (`rib * (1.12 - 0.28 * row / max(1, rows - 1))`): thicker near the floor cantilever root where bending moment is largest, and lighter toward the upper rim. Floor ribs are similarly graded radially (`floor_rib * (0.82 + 0.32 * perimeter_ratio)`), reinforcing the clamped perimeter where plate shear and moment peak while lightening the center span. Internal dividers also receive the organic cellular web when `porous` is enabled and `corner_radius == 0`.

Rear hook pads are localized to the upper anchor zone under each hook rather than full-height solid columns, keeping the lower rear wall porous where bending moment from the rail is minimal while preventing new ceilings inside upper pores. Downturned integral hooks still have unsupported lips and bridge undersides; the UI explicitly identifies this remaining support requirement. This setting does not claim the entire hook-equipped model prints without supports.

`tests/boxPrintability.test.ts` independently inspects downward-facing mesh triangles, excluding build-plate contacts. Tests cover pattern variants, a35° limit, rounded/sloped walls, and bends in both directions, plus failing controls for the old cell ceilings and the hooks. Curved-mesh slivers are screened with a0.4mm one-extrusion-width allowance; this is a finite mesh-face screening rule, not a layerwise support simulation or a universal printer guarantee. Rounded curves and knife refinement can produce such narrow facets even when the source roof is sloped. Physical printing and slicer-based support validation are still required for manufacturing qualification.
