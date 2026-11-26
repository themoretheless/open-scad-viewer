// @rush/1
panel = line_curve(start:[0mm,0mm,0mm],end:[2mm,0mm,0mm]).surface_extrude([0mm,3mm,0mm])
body = panel.thicken_nurbs_patch(thickness_mm:4mm,tolerance_mm:0.000001mm)
show body.brep_tessellate(4)
