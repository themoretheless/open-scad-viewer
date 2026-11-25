// @rush/1
original = line_curve(start:[0mm,0mm,0mm],end:[2mm,0mm,0mm])
edited = line_curve(start:[0mm,0mm,3mm],end:[2mm,0mm,3mm])
comparison = original.curve_edit_deviation(target_geometry:edited,tolerance_mm:0.01mm,max_cells:256)
a = comparison.surface_extrude(vector:[0mm,1mm,0mm])
b = edited.surface_extrude(vector:[0mm,1mm,0mm])
surface_comparison = a.surface_edit_deviation(target_geometry:b)
show surface_comparison.tessellate(8,8)
