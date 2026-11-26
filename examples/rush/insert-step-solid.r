// @rush/1
assembly = planet_occurrences_step(module:1.5mm,teeth:4,sun_teeth:32,height:10mm,pressure_angle:20,helix_angle:0,herringbone:false,clearance:0.1mm,backlash:0.05mm,orbit_angles:[0,180],index:0)
part = brep_box([0,0,0],[2,3,4])
edited = assembly.insert_step_solid(target_geometry:part,placement:[[1,0,0,7],[0,1,0,0],[0,0,1,0],[0,0,0,1]],preview_index:0)
show edited.brep_tessellate(3)
