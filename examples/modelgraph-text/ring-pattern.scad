// @modelgraph-text/1
param radius = 24mm range 15mm..40mm
param hole = 12mm range 4mm..20mm
param height = 8mm range 2mm..16mm
param count = 12 range 3..24

angle = i => i * 360deg / count
blank = circle(radius).extrude(height)
center = circle(hole).extrude(height)
holes = repeat(count, i =>
  cylinder(1.5mm, height)
.translate([radius - 4mm, 0, 0])
.rotate([0, 0, angle(i)])
)
body = blank.subtract(center, holes)
show body
