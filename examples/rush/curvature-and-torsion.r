// @rush/1
parabola = nurbs_curve(degree:2,knots:[0,0,0,1,1,1],control_points:[[0mm,0mm,0mm],[0.5mm,0mm,0mm],[1mm,1mm,0mm]],weights:[1,1,1])
measured = parabola.curve_curvature_torsion(parameter:0.5,query_side:"automatic")
wall = measured.surface_extrude(vector:[0mm,0mm,2mm])
result = wall.surface_principal_curvatures(uv:[0.5,0.5])
show result.tessellate(16,4)
