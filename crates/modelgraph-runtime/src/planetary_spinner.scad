// Herringbone planetary spinner: sun 32, ring 40, 18 planets of 4 teeth,
// every gear an exact involute NURBS body (brep_gear). Module 1.5 at the
// reference inner_radius of 30.845 mm; helix 35 degrees by default. Sun and
// planets take opposite hands, planets and the internal ring the same hand.
gear_scale = inner_radius / 30.845;
gear_module = 1.5 * gear_scale;
sun_teeth = 32;
planet_teeth = 4;
ring_teeth = 40;
planet_count = 18;
orbit_radius = gear_module * (sun_teeth + planet_teeth) / 2;
tooth_clearance = 0.1 * gear_scale;
ring_rim = outer_radius - (gear_module * ring_teeth / 2 + gear_module + tooth_clearance);
assert(gap >= 0 && gap <= 0.6*gear_scale,
"gap должен быть от 0 до 0.6*gear_scale: зазор не подходит к размеру зубьев");
assert(ring_rim >= gear_module / 4,
"Недостаточная толщина кольца: увеличьте outer_radius или уменьшите inner_radius");
assert(inner_radius > 0, "inner_radius должен быть больше нуля");
assert(outer_radius > inner_radius, "outer_radius должен быть больше inner_radius");
assert(spinner_height > 0, "spinner_height должен быть больше нуля");
assert(helix_angle >= 0 && helix_angle < 80, "helix_angle должен быть от 0 до 80 градусов, не включая 80");
assert(center_hole_diameter >= 0 && center_hole_diameter <= 2*(gear_module*sun_teeth/2 - gear_module - tooth_clearance) - gear_module/2,
"Отверстие выходит за корень центральной шестерни: уменьшите center_hole_diameter или увеличьте inner_radius");
brep_gear(module=gear_module, teeth=sun_teeth, height=spinner_height, pressure_angle=20, helix=-helix_angle, herringbone=true, bore=center_hole_diameter, clearance=tooth_clearance, backlash=gap);
brep_gear(module=gear_module, teeth=ring_teeth, height=spinner_height, pressure_angle=20, helix=helix_angle, herringbone=true, internal=true, rim_width=ring_rim, clearance=tooth_clearance, backlash=gap);
module planet(orbit_angle) {
  // Meshing phase of an unshifted involute train with the sun at angle 0.
  planet_angle = (1 + sun_teeth / planet_teeth) * orbit_angle + 180 - 180 / planet_teeth;
  rotate([0, 0, orbit_angle]) translate([orbit_radius, 0, 0]) rotate([0, 0, planet_angle - orbit_angle])
    brep_gear(module=gear_module, teeth=planet_teeth, height=spinner_height, pressure_angle=20, helix=helix_angle, herringbone=true, clearance=tooth_clearance, backlash=gap);
}
planet(0);
planet(20);
planet(40);
planet(60);
planet(80);
planet(100);
planet(120);
planet(140);
planet(160);
planet(180);
planet(200);
planet(220);
planet(240);
planet(260);
planet(280);
planet(300);
planet(320);
planet(340);
