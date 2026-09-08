// @modelgraph-text/1
param radius = 20mm range 5mm..40mm
param hole = 8mm range 1mm..30mm
param height = 6mm range 1mm..20mm

validate radius. greaterThan(hole + 1mm)
.message("Оставьте стенку толще 1 мм")

outer = circle(radius).extrude(height)
inner = circle(hole).extrude(height)
body = outer.subtract(inner)
show body

assert body. hasBodies(1)
.isWatertight()
.hasNoDegenerateTriangles()

assert measure(body).height. approximately(height, tolerance: 0.01mm)
