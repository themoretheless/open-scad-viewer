// @rush/1
axis = line_curve(start:[0mm,0mm,0mm],end:[2mm,0mm,0mm])
sheet = axis.surface_extrude(vector:[0mm,2mm,0mm])
box = brep_box([0,0,0],[2,2,2])
result = box.transfer_cap_loop(target_geometry:sheet,source_height_mm:0mm,reverse:true,tolerance_mm:0.0000001mm)
show result.brep_tessellate(3)
