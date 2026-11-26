// @rush/1
edgeA = bezier_curve(points:[[0,0,0],[2mm,0,0]])
a = edgeA.surface_extrude(vector:[0,1mm,0])
edgeB = bezier_curve(points:[[3mm,0,0],[4mm,0,0]])
b = edgeB.surface_extrude(vector:[0,1mm,0])
result = a.combine_surface_densities(inputs:[b],tolerance_mm2:0.005,tolerance_uv:0.0000000001,max_surface_cells:20000,max_trim_cells:1000000,densities:[[1,1],[2,2]])
show result.tessellate(8,4)
