// @rush/1
profile = polyline_curve(points:[[0mm,0mm,2mm],[4mm,0mm,2mm],[4mm,3mm,2mm]],closed:false)
corner = profile.bevel_offset_wire(distance_mm:1mm,tolerance_mm:0.001mm,max_cells:4096,piece:0)
show corner.surface_extrude([0mm,0mm,1mm]).tessellate(12,2)
