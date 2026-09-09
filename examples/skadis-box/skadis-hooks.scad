// SKADIS slide-in box with two replaceable hooks, printed on their sides.
part = 0; // [0:Assembly,1:Box,2:Hook]
width = 120;
depth = 75;
height = 80;
hook_width = 4.4;
socket_clearance = 0.25;
$fn = 40;
module rr(w,d,r) { hull() for(x=[r,w-r]) for(y=[r,d-r]) translate([x,y]) circle(r=r); }
module body() {
 difference() {
  union() {
   difference() {
    linear_extrude(height) rr(width,depth,5);
    translate([3,5,4]) linear_extrude(height) rr(width-6,depth-8,3);
   }
   for(x=[20,100]) translate([x-9,0,42]) linear_extrude(38) rr(18,14,3);
   for(x=[20,100]) translate([x-7,-2,8]) cube([14,3,10]);
  }
  for(x=[20,100]) translate([x-hook_width/2-socket_clearance,2-socket_clearance,48.8])
   cube([hook_width+2*socket_clearance,8+2*socket_clearance,32]);
  for(x=[20,100]) translate([x-hook_width/2-socket_clearance,-1,71.5]) cube([hook_width+2*socket_clearance,3.3,9]);
 }
}
// Profile is in board-normal / vertical axes. Rounded inside elbow spreads stress.
// Board occupies y=-7..-2; inner hook face y=-7.6 gives 0.6 mm clearance.
module hook_profile() {
 offset(delta=-0.8) offset(r=0.8) union() {
  translate([2,49]) square([8,30]);
  translate([-11.6,76]) square([21.6,4]);
  translate([-11.6,68]) square([4,12]);
  polygon([[-1.5,76],[2,72],[5,76]]);
 }
}
module hook_print() { translate([12,-49,0]) linear_extrude(hook_width) hook_profile(); }
module hook_mounted(x) {
 translate([x-hook_width/2,0,0])
 multmatrix([[0,0,1,0],[1,0,0,0],[0,1,0,0],[0,0,0,1]]) linear_extrude(hook_width) hook_profile();
}
if(part==1) body();
else if(part==2) hook_print();
else {
 color([0.18,0.55,0.64]) body();
 for(x=[20,100]) color([0.95,0.65,0.22]) hook_mounted(x);
}
