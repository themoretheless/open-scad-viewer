// @rush/1
arc = nurbs_curve(degree:2,knots:[0,0,0,1,1,1],control_points:[[0mm,0mm,0mm],[1mm,1mm,0mm],[2mm,0mm,0mm]],weights:[1,1,1])
chord = arc.bounded_curve_chord(tolerance_mm:0.25mm,max_cells:1000,piece:0)
sheet = chord.surface_extrude(vector:[0mm,0mm,1mm])
show sheet.bounded_surface_mesh(tolerance_mm:0.4mm,max_cells:1000)
