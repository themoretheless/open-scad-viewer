// @modelgraph-text/1
// Шевронный спиннер: солнце 32, венец 40, 18 сателлитов по 4 зуба.
// Каждая шестерня - точное эвольвентное NURBS-тело (brep_gear).
param inner_radius = 30.845mm range 10mm..80mm
param outer_radius = 32mm range 11mm..100mm
param hole = 44mm range 0mm..100mm
param gap = 0.05mm range 0mm..0.6mm
param height = 10mm range 2mm..30mm
param helix = 35deg range 0deg..45deg

sun_teeth = 32
planet_teeth = 4
ring_teeth = 40
planet_count = 18
gear_scale = inner_radius / 30.845mm
gear_module = 1.5mm * gear_scale
clearance = 0.1mm * gear_scale
orbit_radius = gear_module * (sun_teeth + planet_teeth) / 2
ring_rim = outer_radius - (gear_module * ring_teeth / 2 + gear_module + clearance)

validate gap.atMost(0.6mm * gear_scale).message("Зазор не подходит к размеру зубьев")
validate ring_rim.atLeast(gear_module / 4).message("Недостаточная толщина кольца: увеличьте outer_radius или уменьшите inner_radius")
validate outer_radius.greaterThan(inner_radius).message("outer_radius должен быть больше inner_radius")
validate hole.atMost(2 * (gear_module * sun_teeth / 2 - gear_module - clearance) - gear_module / 2).message("Отверстие выходит за корень центральной шестерни")

// Солнце и сателлиты - противоположного наклона, сателлиты и венец - одного.
sun = gear(teeth: sun_teeth, module: gear_module, thickness: height, helix_angle: -helix, herringbone: true,
  bore: hole, clearance: clearance, backlash: gap)
ring = gear(teeth: ring_teeth, module: gear_module, thickness: height, helix_angle: helix, herringbone: true,
  internal: true, rim_width: ring_rim, clearance: clearance, backlash: gap)
planet = gear(teeth: planet_teeth, module: gear_module, thickness: height, helix_angle: helix, herringbone: true,
  clearance: clearance, backlash: gap)

// Фаза зацепления несмещённого эвольвентного ряда при солнце в нуле.
planets = [for i in 0..<planet_count
  let orbit = i * 360deg / planet_count
  let phase = (1 + sun_teeth / planet_teeth) * orbit + 180deg - 180deg / planet_teeth
  => planet.rotate(z: phase - orbit).translate(x: orbit_radius).rotate(z: orbit)]

show [sun, ring, planets]
