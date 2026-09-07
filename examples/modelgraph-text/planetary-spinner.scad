// @modelgraph-text/1
// Шевронный спиннер: солнце 32, венец 40, 18 сателлитов по 4 зуба.
param inner_radius = 30.845mm range 10mm..80mm
param outer_radius = 32mm range 11mm..100mm
param hole = 44mm range 0mm..100mm
param gap = 0.05mm range 0mm..0.6mm
param height = 10mm range 2mm..30mm
param helix = 35deg range 0deg..45deg

spinner = planetary_spinner(
  inner_radius: inner_radius,
  outer_radius: outer_radius,
  bore: hole,
  gap: gap,
  height: height,
  helix_angle: helix
)
show spinner
