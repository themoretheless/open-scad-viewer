// @rush/1
edge = line_curve(start:[0mm,0mm,0mm],end:[10mm,0mm,0mm])
panel = edge.surface_extrude([0mm,10mm,0mm])
path = panel.trace_geodesic(start_uv:[0.2,0.3],direction_uv:[1,0],length:3mm,max_steps:128,return_field:true)
show path.surface_extrude([0mm,0mm,1mm]).tessellate(8,2)
