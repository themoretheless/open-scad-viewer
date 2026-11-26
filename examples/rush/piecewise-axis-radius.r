// @rush/1
axis = line_curve(start:[0mm,0mm,0mm],end:[2mm,0mm,0mm])
sheet = axis.surface_extrude(vector:[0mm,2mm,0mm])
result = sheet.piecewise_axis_radius(axis_index:2,origin:[0mm,0mm,0mm],max_patches:64)
show result.tessellate(8,4)
