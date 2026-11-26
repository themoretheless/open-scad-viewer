// @rush/1
panel = line_curve(start:[0mm,0mm,0mm],end:[2mm,0mm,0mm]).surface_extrude([0mm,1mm,0mm])
offset = panel.offset_nurbs_surface(distance:2mm,tolerance_mm:0.001mm)
show offset.tessellate(8,8)
