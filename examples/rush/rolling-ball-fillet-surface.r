// @rush/1
a = line_curve(start:[2mm,3mm,4mm],end:[12mm,3mm,4mm]).surface_extrude([0mm,10mm,0mm])
b = line_curve(start:[2mm,3mm,4mm],end:[2mm,13mm,4mm]).surface_extrude([0mm,0mm,10mm])
fillet = a.rolling_ball_fillet_surface(target_geometry:b,radius:1.25mm,side_a:1,side_b:1,tolerance_mm:0.0000001mm,max_march_steps:256,max_sections:9)
show fillet.tessellate(16,16)
