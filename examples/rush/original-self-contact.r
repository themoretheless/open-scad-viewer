// @rush/1
profile = nurbs_curve(degree:2,knots:[0,0,0,1,1,1],control_points:[[0mm,0mm,0mm],[0.5mm,0mm,1mm],[1mm,0mm,0mm]],weights:[1,1,1])
checked = profile.curve_self_contacts(tolerance_mm:0.001mm,max_cells:1024,max_pairs:128)
skin = checked.surface_extrude(vector:[0mm,2mm,0mm])
inspected = skin.surface_self_contacts(max_spans:256)
result = inspected.surface_span_regularity(max_cells:256)
show result.tessellate(16,4)
