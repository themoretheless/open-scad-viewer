/** Bounded inputs for the browser's own-Rust OpenSCAD subset. */
export const cpuFixtures = [
  {
    id: 'small-bracket',
    description: 'Small parametric plate with four screw holes; interactive edit baseline.',
    source: `$fn=32;
difference() {
  cube([40, 30, 4], center=true);
  ${[-14, 14].flatMap(x => [-9, 9].map(y => `translate([${x},${y},0]) cylinder(h=8, r=2, center=true);`)).join('\n  ')}
}`,
  },
  {
    id: 'medium-csg',
    description: '64 cylindrical cuts in a panel; boolean evaluation and mesh analysis.',
    source: `$fn=32;
difference() {
  cube([86,86,8], center=true);
  ${Array.from({ length: 64 }, (_, i) => `translate([${-35 + (i % 8) * 10},${-35 + Math.floor(i / 8) * 10},0]) cylinder(h=12, r=3, center=true);`).join('\n  ')}
}`,
  },
  {
    id: 'many-bodies',
    description: '256 independent cubes; per-entity overhead and publication validation.',
    source: Array.from({ length: 256 }, (_, i) =>
      `translate([${(i % 16) * 3},${Math.floor(i / 16) * 3},0]) cube([2,2,2]);`).join('\n'),
  },
  {
    id: 'dense-sphere',
    description: 'One mesh containing three separated high-resolution spheres; dense analysis, BVH and export.',
    source: '$fn=128; union() { sphere(r=30); translate([90,0,0]) sphere(r=30); translate([180,0,0]) sphere(r=30); }',
  },
]
