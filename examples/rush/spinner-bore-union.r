// @rush/1
sun = brep_gear(module:1.5mm,teeth:32,height:10mm,helix_angle:-35,herringbone:true,clearance:0.1mm,backlash:0.05mm,bore:42mm)
cutter = brep_cylinder(radius:22mm,height:10mm)
result = brep_union(sun,cutter)
show result.brep_tessellate(3)
