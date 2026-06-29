export const EXAMPLES: Record<string, string> = {
  basic: `// Basic OpenSCAD primitives
cube([20, 15, 10]);

translate([30, 0, 0])
  sphere(r = 8, $fn = 32);

translate([0, 25, 0])
  cylinder(h = 15, r = 6, $fn = 24);

translate([30, 25, 0])
  cylinder(h = 15, r1 = 8, r2 = 3, $fn = 6);
`,

  csg: `// CSG Boolean Operations
// difference: subtracted parts in red

difference() {
    cube([30, 30, 30], center = true);
    sphere(r = 19, $fn = 32);
    translate([0, 0, 15])
        cylinder(h = 10, r = 10, $fn = 32);
}

translate([50, 0, 0])
color([0.2, 0.8, 0.5])
difference() {
    cylinder(h = 25, r = 12, center = true, $fn = 32);
    translate([0, 0, 3])
        cylinder(h = 22, r = 9, center = true, $fn = 32);
}
`,

  house: `// A simple house
// Walls
color([0.85, 0.75, 0.55])
difference() {
    cube([40, 25, 30]);
    // door
    translate([15, -1, 0])
        cube([10, 10, 18]);
    // windows
    translate([3, -1, 10])
        cube([8, 10, 8]);
    translate([29, -1, 10])
        cube([8, 10, 8]);
}

// Roof
color([0.7, 0.2, 0.15])
translate([20, 12.5, 25])
rotate([90, 0, 0])
    cylinder(h = 27, r1 = 0, r2 = 25, $fn = 4, center = true);

// Chimney
color([0.5, 0.3, 0.2])
translate([32, 5, 25])
    cube([5, 5, 12]);

// Door step
color([0.6, 0.6, 0.6])
translate([13, -3, 0])
    cube([14, 3, 2]);
`,

  tower: `// Decorative tower
color([0.4, 0.4, 0.5]) {
    // base
    cylinder(h = 5, r = 20, $fn = 8);

    // tiers
    translate([0, 0, 5])
        cylinder(h = 15, r1 = 18, r2 = 14, $fn = 8);

    translate([0, 0, 20])
        cylinder(h = 15, r1 = 14, r2 = 10, $fn = 8);

    translate([0, 0, 35])
        cylinder(h = 12, r1 = 10, r2 = 7, $fn = 8);
}

// dome
color([0.8, 0.7, 0.2])
translate([0, 0, 47])
    sphere(r = 8, $fn = 24);

// spire
color([0.8, 0.7, 0.2])
translate([0, 0, 53])
    cylinder(h = 15, r1 = 2, r2 = 0.3, $fn = 12);

// balconies
color([0.6, 0.55, 0.65])
translate([0, 0, 20])
    cylinder(h = 1.5, r = 16, $fn = 8);

color([0.6, 0.55, 0.65])
translate([0, 0, 35])
    cylinder(h = 1.5, r = 12, $fn = 8);
`,
}
