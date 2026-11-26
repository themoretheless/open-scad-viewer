// @rush/1
axis = line_curve(start:[0mm,0mm,0mm],end:[2mm,0mm,0mm])
sheet = axis.surface_extrude(vector:[0mm,2mm,0mm])
axis2 = line_curve(start:[0.9mm,1mm,0.1mm],end:[1.1mm,1mm,0.1mm])
second = axis2.surface_extrude(vector:[0mm,0.1mm,0mm])
result = sheet.patch_pair_clearance(inputs:[second],minimum_mm:0.2,tolerance_mm:0.02,max_cells:10000)
show result.tessellate(8,8)
