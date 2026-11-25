// @rush/1
source = nurbs_curve(degree:1,knots:[0,0,1,2,2],control_points:[[0,0,0],[1mm,0,0],[2mm,0,0]],weights:[2,2,2])
reduced = source.simplify_linear_curve(max_work:1000000)
longer = reduced.extend_terminal_curve(source_end:"end",distance_mm:1mm,continuity_class:"G2")
sheet = longer.surface_extrude(vector:[0,1mm,0])
extended = sheet.extend_surface_boundary(source_edge:"v_min",distance_mm:0.5mm,continuity_class:"C2")
show extended.tessellate(8,8)
