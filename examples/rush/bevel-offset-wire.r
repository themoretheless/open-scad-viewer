// @rush/1
profile = polyline_curve(points:[[0mm,0mm,0mm],[4mm,0mm,0mm],[4mm,3mm,0mm]],closed:false)
corner = profile.bevel_offset_wire(distance_mm:1mm,tolerance_mm:0.001mm,max_cells:100000,piece:1)
show corner.surface_extrude([0mm,0mm,1mm]).tessellate(4,2)
