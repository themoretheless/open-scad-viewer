// @modelgraph-text/1
// SKADIS bottom-insert asymmetric dovetail. Dimensions in mm.
// part: 0 assembly, 1 box upright, 2 hook on its flat side.
segments 40
param part: 0 range 0..2
param width: 120 range 60..300
param depth: 75 range 30..200
param height: 80 range 50..250
param wall: 3 range 2..6
param floor_thickness: 4 range 2..8
param back_thickness: 7 range 7..12
param corner_radius: 5 range 3..12
param hook_count: 2 range 2..5
param hook_spacing_steps: 2 range 1..6
param mount_top_offset: 14 range 14..30
param fit: 0.25 range 0.15..0.4
param friction_ridge_enabled: 1 range 0..1

hook_width = 4.4
hook_spacing = 40 * hook_spacing_steps
hook_span = (hook_count - 1) * hook_spacing
mount_shift = height - mount_top_offset - 66
rib_height = height - mount_top_offset + 12
validate width |> atLeast(hook_span + 32) |> message("Box too narrow for hooks")
validate height |> atLeast(mount_top_offset + 36)
validate depth |> atLeast(back_thickness + wall + 12)
validate corner_radius |> atLeast(wall)
validate 2 * corner_radius |> lessThan(width)
validate 2 * corner_radius |> lessThan(depth)
validate hook_count % 1 |> equalTo(0)
validate hook_spacing_steps % 1 |> equalTo(0)
validate part % 1 |> equalTo(0)
validate friction_ridge_enabled % 1 |> equalTo(0)

// Rounded rectangle with named inputs and result.
fn roundedRectangle width: f64, depth: f64, radius: f64 -> Geometry
    ret hull(
        circle(radius).translate([radius, radius, 0]),
        circle(radius).translate([width-radius, radius, 0]),
        circle(radius).translate([radius, depth-radius, 0]),
        circle(radius).translate([width-radius, depth-radius, 0]),
    )
male_section = polygon([[-2.2,0],[2.2,0],[6.2,4],[-2.2,4]])
female_section = male_section.offset(delta: fit)
under_roof = polygon([[-3,-2],[8,-2],[8,66+mount_shift],[-3,77+mount_shift]])
    |> extrude(40)
    |> rotate([90,0,90])
    |> translate([-20,0,0])
channel = union(
    intersection(female_section.extrude(height+1).translate([0,0,-1]), under_roof),
    box([4.4+2*fit,3.1,67.3+mount_shift]).translate([-2.2-fit,-3,-1])
)
// corner_radius >= wall makes corner_radius-wall+1 >= 1.
outer = roundedRectangle(width, depth, corner_radius).extrude(height)
inner = roundedRectangle(width-2*wall, depth-back_thickness-wall, corner_radius-wall+1)
    |> extrude(height)
    |> translate([wall,back_thickness,floor_thickness])
rib = roundedRectangle(16, 10, 2).extrude(rib_height)
ribs = union([for i in 0..<hook_count => rib.translate([(width-hook_span)/2+i*hook_spacing-6,0,0])])
channels = union([for i in 0..<hook_count => channel.translate([(width-hook_span)/2+i*hook_spacing,0,0])])
body = union(outer.subtract(inner), ribs).subtract(channels)

blade_profile = union(
    rectangle([12.6,4]).translate([-9.6,62,0]),
    rectangle([4,12]).translate([-9.6,54,0]),
    rectangle([3,24]).translate([0,42,0])
).offset(0.7).offset(delta: -0.7)
friction_ridge = hull(
    box([0.01,0.6,0.1]).translate([4.2,2,46]),
    box([0.65,0.6,4]).translate([4.2,2,48]),
    box([0.01,0.6,0.1]).translate([4.2,2,54])
)
ridge = [for i in 0..<1 where friction_ridge_enabled == 1 => friction_ridge.translate([0,0,mount_shift])]
neck = intersection(
    male_section.extrude(36).translate([0,0,42+mount_shift]),
    under_roof.translate([0,0,-fit])
)
blade = blade_profile.extrude(hook_width).rotate([90,0,90]).translate([-2.2,0,mount_shift])
hook = union(ridge, neck, blade)
print_hook = hook.translate([0,0,-42-mount_shift]).rotate([0,-90,0]).translate([10,0,2.2])

box_output = [for i in 0..<1 where part != 2 => body]
hook_output = [for i in 0..<1 where part == 2 => print_hook]
assembly_hooks = [for i in 0..<hook_count where part == 0 => hook.translate([(width-hook_span)/2+i*hook_spacing,0,0])]
show [box_output, hook_output, assembly_hooks]
