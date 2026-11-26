// @rush/1
sun = brep_gear(module:1.5mm,teeth:32,height:10mm,helix_angle:-35,herringbone:true,clearance:0.1mm,backlash:0.05mm,bore:42mm)
result = sun.cut_bore_cylinder(radius_mm:22mm,height_mm:10mm)
show result.brep_tessellate(3)
