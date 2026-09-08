// SKADIS: bottom-insert, asymmetric dovetail; sloped closed top bears load.
// part 0 assembly, 1 box upright, 2 hook on its flat side. Millimetres.
/* [Output] */
part = 0; // [0:Assembly,1:Box,2:Hook for printing]
/* [Box dimensions in mm] */
width = 120; // [60:1:300]
depth = 75; // [30:1:200]
height = 80; // [50:1:250]
wall = 3; // [2:0.2:6]
floor_thickness = 4; // [2:0.2:8]
back_thickness = 7; // [7:0.5:12]
corner_radius = 5; // [3:0.5:12]
/* [Mount placement] */
hook_count = 2; // [2:1:5]
// Distance between adjacent hooks = this value times the 40 mm board grid.
hook_spacing_steps = 2; // [1:1:6]
// Distance from box rim to hook neck top.
mount_top_offset = 14; // [14:1:30]
/* [Fit] */
fit = 0.25; // [0.15:0.05:0.4]
// Small friction ridge, not a positive latch.
friction_ridge_enabled = true;
/* [Hidden] */
hook_width=4.4;
board_pitch=40;
hook_spacing=board_pitch*hook_spacing_steps;
hook_span=(hook_count-1)*hook_spacing;
mount_shift=height-mount_top_offset-66;
rib_height=height-mount_top_offset+12;
$fn=40;
assert(width >= hook_span+32, "Box too narrow: reduce hook spacing/count or increase width.");
assert(height >= mount_top_offset+36, "Box too short for hook and upper stop.");
assert(wall >= 2 && floor_thickness >= 2 && back_thickness >= 7, "Walls/floor below minimum.");
assert(depth >= back_thickness+wall+12, "Box too shallow.");
assert(corner_radius >= wall && 2*corner_radius < min(width,depth), "Invalid corner radius.");
assert(fit >= 0.15 && fit <= 0.4, "Fit must be between 0.15 and 0.4 mm.");
assert(hook_count >= 2 && hook_count == floor(hook_count), "Use at least two hooks.");
assert(hook_spacing_steps >= 1 && hook_spacing_steps == floor(hook_spacing_steps), "Spacing must follow board grid.");
module rr(w,d,r) { hull() for(x=[r,w-r]) for(y=[r,d-r]) translate([x,y]) circle(r=r); }
// Cross section has a flat left side, permitting hook printing on that side.
module male_section() { polygon([[-2.2,0],[2.2,0],[6.2,4],[-2.2,4]]); }
module female_section() { offset(delta=fit) male_section(); }
// Wedge under z=74-y. Roof closes from inner wall outward at 45 degrees.
module under_roof() {
 translate([-20,0,0]) rotate([90,0,90]) linear_extrude(40)
  polygon([[-3,-2],[8,-2],[8,66+mount_shift],[-3,77+mount_shift]]);
}
module channel() {
 intersection() {
  translate([0,0,-1]) linear_extrude(height+1) female_section();
  under_roof();
 }
 // Open throat for the neck, also open at bottom of the rail.
 translate([-2.2-fit,-3,-1]) cube([4.4+2*fit,3.1,67.3+mount_shift]);
}
module body() {
 difference() {
  union() {
   difference() {
    linear_extrude(height) rr(width,depth,corner_radius);
    translate([wall,back_thickness,floor_thickness]) linear_extrude(height) rr(width-2*wall,depth-back_thickness-wall,max(1,corner_radius-wall+1));
   }
   // Internal ribs start at floor: no floating socket undersides.
   for(i=[0:hook_count-1]) translate([((width-hook_span)/2+i*hook_spacing)-6,0,0]) linear_extrude(rib_height) rr(16,10,2);
  }
  for(i=[0:hook_count-1]) translate([((width-hook_span)/2+i*hook_spacing),0,0]) channel();
 }
}
module blade_profile() {
 offset(delta=-0.7) offset(r=0.7) union() {
  translate([-9.6,62]) square([12.6,4]);
  translate([-9.6,54]) square([4,12]);
  translate([0,42]) square([3,24]);
 }
}
module friction_ridge() {
 hull() {
 translate([4.2,2,46]) cube([0.01,0.6,0.1]);
 translate([4.2,2,48]) cube([0.65,0.6,4]);
 translate([4.2,2,54]) cube([0.01,0.6,0.1]);
 }
}
module hook() {
 union() {
  if(friction_ridge_enabled) translate([0,0,mount_shift]) friction_ridge();
  intersection() {
   translate([0,0,42+mount_shift]) linear_extrude(36) male_section();
   translate([0,0,-fit]) under_roof();
  }
  translate([-2.2,0,mount_shift]) multmatrix([[0,0,1,0],[1,0,0,0],[0,1,0,0],[0,0,0,1]])
   linear_extrude(hook_width) blade_profile();
 }
}
if(part==1) body();
else if(part==2) translate([10,0,2.2]) rotate([0,-90,0]) translate([0,0,-42-mount_shift]) hook();
else {
 color([0.18,0.55,0.64]) body();
 for(i=[0:hook_count-1]) color([0.95,0.65,0.22]) translate([((width-hook_span)/2+i*hook_spacing),0,0]) hook();
}
