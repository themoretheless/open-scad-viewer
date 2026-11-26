// @rush/1
sun = brep_gear(module:1.5mm,teeth:32,height:10mm,helix_angle:-35,herringbone:true,clearance:0.1mm,backlash:0.05mm,bore:42mm)
result = sun.bore_wall_faces(cap_indices:[0,1],hole_indices:[0,0],shell_index:0)
show result.brep_tessellate(3)
