// @modelgraph-text/1
// SKADIS bottom-insert asymmetric dovetail. Dimensions in mm.
// part: 0 assembly, 1 box upright, 2 hook on its flat side.
segments 40
param part = 0 range 0..2
param width = 120 range 60..300
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

mount_shift = height - mount_top_offset - 66
hook_span = (hook_count - 1) * 40 * hook_spacing_steps
positions = [for i in 0..<hook_count => (width-hook_span)/2 + i*40*hook_spacing_steps]
assert width.atLeast(hook_span + 32).message("Box too narrow for hooks")
assert height.atLeast(mount_top_offset + 36)
assert depth.atLeast(back_thickness + wall + 12)
assert corner_radius >= wall && 2*corner_radius < width && 2*corner_radius < depth
assert [hook_count,hook_spacing_steps,part,friction_ridge_enabled].all(n => n % 1 == 0)

// Rounded solids share one implementation; w/d/h/r are dimensions and radius.
trait Rounded {
  w: f64, d: f64, h: f64, r: f64
  fn solid self: Self -> Geometry
    {w,d,h,r} = self
    ret hull([for y in [r,d-r] for x in [r,w-r] => circle(r).move(x,y,0)]).extrude(h)
}
fn mounted shape: Geometry, dx: f64 = 0 ret [for x in positions => shape.move(x+dx,0,0)]
size = {w:width, d:depth, h:height, r:corner_radius}
inside = size with {w:width-2*wall, d:depth-back_thickness-wall, r:corner_radius-wall+1}
shell = size.solid().subtract(inside.solid().move(wall,back_thickness,floor_thickness))
rib = {w:16, d:10, h:height-mount_top_offset+12, r:2}.solid()

male = polygon([[-2.2,0],[2.2,0],[6.2,4],[-2.2,4]])
roof = polygon([[-3,-2],[8,-2],[8,66+mount_shift],[-3,77+mount_shift]])
  .extrude(40).rotate([90,0,90]).move(-20,0,0)
channel = union(male.offset(delta:fit).extrude(height+1).move(0,0,-1).intersection(roof),
  box(4.4+2*fit,3.1,67.3+mount_shift).move(-2.2-fit,-3,-1))
body = union(shell,union(mounted(rib,-6))).subtract(union(mounted(channel)))

blade = union([for (w,h,x,y) in [[12.6,4,-9.6,62],[4,12,-9.6,54],[3,24,0,42]] => rect([w,h]).move(x,y,0)])
  .offset(0.7).offset(delta:-0.7).extrude(4.4).rotate([90,0,90]).move(-2.2,0,mount_shift)
ridge = friction_ridge_enabled ? hull([for (w,h,z) in [[0.01,0.1,46],[0.65,4,48],[0.01,0.1,54]] => box(w,0.6,h).move(4.2,2,z)]) : union([])
neck = male.extrude(36).move(0,0,42+mount_shift).intersection(roof.move(0,0,-fit))
hook = union(ridge.move(0,0,mount_shift),neck,blade)
show match part
  1 => body
  2 => hook.move(0,0,-42-mount_shift).rotate([0,-90,0]).move(10,0,2.2)
  _ => [body,mounted(hook)]
