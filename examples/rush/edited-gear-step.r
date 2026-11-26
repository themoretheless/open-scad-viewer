// @rush/1
sun = brep_gear(module:1.5mm,teeth:32,height:5mm,clearance:0.1mm,bore:5mm,helix_angle:-35,herringbone:true)
result = sun.edited_gear_step(sector_index:31,flank_index:0,pole_index:1,offset_x_mm:0.00000001mm,offset_y_mm:0mm,tolerance_mm:0.0000001mm)
show result.brep_tessellate(3)
