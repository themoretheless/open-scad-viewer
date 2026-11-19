// @rush/1
profile = nurbs_curve(degree:2,knots:[0,1,2,3,4,5,6,7,8,9,10],control_points:[[1mm,0,0],[0.5mm,1mm,0],[-0.5mm,1mm,0],[-1mm,0,0],[-0.5mm,-1mm,0],[0.5mm,-1mm,0],[1mm,0,0],[0.5mm,1mm,0]],weights:[1,2,1,2,1,2,1,2],periodic:true)
weighted = profile.curve_edit_weights(edits:[[0,3]])
moved = weighted.curve_relocate_seam(parameter:3,max_deviation:0.1mm,max_cells:20000)
strip = moved.surface_extrude(vector:[0,0,2mm])
panel = strip.surface_edit_weights(edits:[[2,1,3]])
shifted = panel.surface_relocate_seam(direction:"u",parameter:4,max_deviation:0.5mm,max_cells:20000)
show shifted.tessellate(24,8)
