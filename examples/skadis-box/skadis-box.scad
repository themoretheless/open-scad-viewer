// SKADIS box: mm. Four M3 screws, washers and nuts; rear board access required.
// part: 0 assembled preview, 1 box for printing, 2 one backing rail (print twice).
part = 0; // [0:Assembly,1:Box,2:Backing rail]
width = 120;
depth = 75;
height = 80;
wall = 3;
back = 5;
floor_thickness = 4;
board_thickness = 5;
hole_pitch = 80;
vertical_pitch = 40;
screw_clearance = 3.4;
$fn = 48;
module rounded_rect(w,d,r) {
 hull() for(x=[r,w-r]) for(y=[r,d-r]) translate([x,y]) circle(r=r);
}
module bore_y(x,y,z,length) {
 translate([x,y,z]) rotate([-90,0,0]) cylinder(d=screw_clearance,h=length);
}
module box_body() {
 difference() {
  linear_extrude(height) rounded_rect(width,depth,5);
  translate([wall,back,floor_thickness])
   linear_extrude(height) rounded_rect(width-2*wall,depth-back-wall,3);
  for(x=[(width-hole_pitch)/2,(width+hole_pitch)/2])
   for(z=[20,20+vertical_pitch]) bore_y(x,-1,z,back+2);
 }
}
// Rounded, solid rear rails spread clamp force; nut pockets face the wall.
module rail() {
 difference() {
  linear_extrude(6) rounded_rect(hole_pitch+18,14,4);
  for(x=[9,9+hole_pitch]) {
   translate([x,7,-1]) cylinder(d=screw_clearance,h=8);
   translate([x,7,3.3]) cylinder(d=6.6,h=3.7,$fn=6);
  }
 }
}
if(part==1) box_body();
else if(part==2) rail();
else {
 color([0.18,0.55,0.64]) box_body();
 for(z=[20,20+vertical_pitch])
  color([0.95,0.65,0.22])
   translate([(width-hole_pitch)/2-9,-board_thickness,z-7]) rotate([90,0,0]) rail();
}
