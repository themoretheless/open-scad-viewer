export const EXAMPLES: Record<string, string> = {
  basic: `// OpenSCAD primitives — rendered with a real geometry kernel
cube([20, 15, 10]);

translate([30, 0, 8])
  sphere(r = 8, $fn = 32);

translate([0, 28, 0])
  cylinder(h = 18, r = 7, $fn = 32);

translate([30, 28, 0])
  cylinder(h = 18, r1 = 8, r2 = 3, $fn = 6);
`,

  csg: `// Real Manifold boolean operations
difference() {
  cube([32, 32, 24], center = true);
  sphere(r = 18, $fn = 48);
  rotate([90, 0, 0])
    cylinder(h = 40, r = 6, center = true, $fn = 32);
}

translate([48, 0, 0])
color("#37c88a")
intersection() {
  cube(24, center = true);
  sphere(r = 16, $fn = 40);
}
`,

  house: `// Variables, modules and real difference()
wall = 2;
width = 40;
depth = 26;
height = 28;

module window(x) {
  translate([x, -1, 10]) cube([8, wall + 2, 8]);
}

color([0.85, 0.75, 0.55])
difference() {
  cube([width, depth, height]);
  translate([14, -1, 0]) cube([12, wall + 2, 18]);
  window(3);
  window(29);
}

color([0.7, 0.2, 0.15])
translate([width / 2, depth / 2, height])
rotate([90, 0, 0])
  cylinder(h = depth + 2, r1 = 0, r2 = 25, $fn = 4, center = true);

color("brown")
translate([31, 6, height]) cube([5, 5, 12]);
`,

  tower: `// Parametric model with a range loop
levels = 4;
level_h = 11;

color([0.42, 0.45, 0.55])
for (i = [0:levels - 1]) {
  radius = 18 - i * 3;
  translate([0, 0, i * level_h])
    cylinder(h = level_h, r1 = radius, r2 = radius - 2, $fn = 10);
}

color([0.88, 0.72, 0.18])
translate([0, 0, levels * level_h])
  sphere(r = 7, $fn = 32);

translate([0, 0, levels * level_h + 5])
  cylinder(h = 14, r1 = 2, r2 = 0, $fn = 16);
`,
}
