// @language openscad-viewer/brep-1
// Internal diagnostic: node --import tsx scripts/check-brep-semantic.mjs examples/brep/semantic-enclosure.brep.scad
// The permanent browser/MCP brep-1 provider is not yet available.
difference() {
  union() {
    cylinder(r=18, h=8);
    translate([16,0,4]) cylinder(r=8, h=12);
  }
  translate([0,0,2]) cylinder(r=11, h=16);
}
