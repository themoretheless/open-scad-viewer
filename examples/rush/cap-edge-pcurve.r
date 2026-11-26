// @rush/1
axis = line_curve(start:[0mm,0mm,0mm],end:[2mm,0mm,0mm])
sheet = axis.surface_extrude(vector:[0mm,2mm,0mm])
edge = line_curve(start:[0.5mm,0.5mm,0mm],end:[1.5mm,1mm,0mm])
result = sheet.cap_edge_pcurve(target_geometry:edge,tolerance_mm:0.0000001mm)
show result.tessellate(8,4)
