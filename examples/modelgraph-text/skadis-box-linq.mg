// @modelgraph-text/1
// SKADIS: крюки вставляются снизу в асимметричный паз.
// Ширина автоматически увеличивается, если крюки не помещаются.
// part: 0 — сборка, 1 — коробка, 2 — один крюк для печати.
segments 40

param part = 0 range 0..2
param requested_width = 120 range 60..300
param depth = 75 range 30..200
param height = 80 range 50..250
param wall = 3 range 2..6
param floor_thickness = 4 range 2..8
param back_thickness = 7 range 7..12
param corner_radius = 5 range 3..12
param hook_count = 2 range 2..5
param hook_spacing_steps = 2 range 1..6
param mount_top_offset = 14 range 14..30
param fit = 0.25 range 0.15..0.4
param friction_ridge_enabled = 1 range 0..1

hook_width = 4.4
hook_spacing = 40 * hook_spacing_steps
hook_span = (hook_count - 1) * hook_spacing
mount_shift = height - mount_top_offset - 66
rib_height = height - mount_top_offset + 12
minimum_width = hook_span + 32
width = requested_width < minimum_width ? minimum_width : requested_width

hook_positions = (0..<hook_count)
  .select(i => (width-hook_span)/2 + i*hook_spacing)

validate height.atLeast(mount_top_offset + 36)
validate depth.atLeast(back_thickness + wall + 12)
validate corner_radius.atLeast(wall)
validate (2 * corner_radius).lessThan(width)
validate (2 * corner_radius).lessThan(depth)
validate (hook_count % 1).equalTo(0)
validate (hook_spacing_steps % 1).equalTo(0)
validate (part % 1).equalTo(0)
validate (friction_ridge_enabled % 1).equalTo(0)

// Прямоугольник со скруглёнными углами.
fn roundedRectangle width: f64, depth: f64, radius: f64 -> Geometry
  ret hull(
    circle(radius).move([radius, radius, 0]),
    circle(radius).move([width-radius, radius, 0]),
    circle(radius).move([radius, depth-radius, 0]),
    circle(radius).move([width-radius, depth-radius, 0]))

male_section = polygon([[-2.2,0],[2.2,0],[6.2,4],[-2.2,4]])
female_section = male_section.offset(delta: fit)

under_roof = polygon([[-3,-2],[8,-2],[8,66+mount_shift],[-3,77+mount_shift]])
  .extrude(40)
  .rotate([90,0,90])
  .move([-20,0,0])

channel = union(
  intersection(female_section.extrude(height+1).move([0,0,-1]), under_roof),
  box([4.4+2*fit,3.1,67.3+mount_shift]).move([-2.2-fit,-3,-1]))

outer = roundedRectangle(width, depth, corner_radius).extrude(height)
inner = roundedRectangle(width-2*wall, depth-back_thickness-wall, corner_radius-wall+1)
  .extrude(height)
  .move([wall,back_thickness,floor_thickness])

rib = roundedRectangle(16, 10, 2).extrude(rib_height)
ribs = hook_positions.select(x => rib.move([x-6,0,0]))
channels = hook_positions.select(x => channel.move([x,0,0]))

body = union(outer.subtract(inner), union(ribs)).subtract(union(channels))

blade_profile = union(
  rect([12.6,4]).move([-9.6,62,0]),
  rect([4,12]).move([-9.6,54,0]),
  rect([3,24]).move([0,42,0]))
  .offset(0.7)
  .offset(delta: -0.7)

friction_ridge = hull(
  box([0.01,0.6,0.1]).move([4.2,2,46]),
  box([0.65,0.6,4]).move([4.2,2,48]),
  box([0.01,0.6,0.1]).move([4.2,2,54]))

ridge = (0..<friction_ridge_enabled)
  .select(i => friction_ridge.move([0,0,mount_shift]))

neck = intersection(
  male_section.extrude(36).move([0,0,42+mount_shift]),
  under_roof.move([0,0,-fit]))
blade = blade_profile.extrude(hook_width)
  .rotate([90,0,90])
  .move([-2.2,0,mount_shift])
hook = union(ridge, neck, blade)

print_hook = hook.move([0,0,-42-mount_shift])
  .rotate([0,-90,0])
  .move([10,0,2.2])

assembly_hooks = foreach x in hook_positions
  yield hook.move([x,0,0])

show match part
  2 => print_hook
  1 => body
  _ => [body, assembly_hooks]
