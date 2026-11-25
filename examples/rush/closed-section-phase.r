// @rush/1
a = nurbs_curve(degree:1,knots:[0,0,1,2,3,4,4],control_points:[[0mm,0mm,0mm],[2mm,0mm,0mm],[2mm,2mm,0mm],[0mm,2mm,0mm],[0mm,0mm,0mm]],weights:[1,1,1,1,1])
b = nurbs_curve(degree:1,knots:[0,0,1,2,3,4,4],control_points:[[0mm,0mm,2mm],[2mm,0mm,2mm],[2mm,2mm,2mm],[0mm,2mm,2mm],[0mm,0mm,2mm]],weights:[1,1,1,1,1])
aligned = a.phase_aligned_section(inputs:[b],piece:1,landmarks:[[2mm,1mm,-1mm],[2mm,1mm,3mm]],search_tolerance_mm:0.01mm,shape_tolerance_mm:0.1mm,max_cells:10000)
show aligned.surface_extrude(vector:[0mm,0mm,1mm]).tessellate(16,4)
