// @rush/1
profile = bezier_curve(points:[[0,0,0],[0.5mm,0,0],[1mm,0,0]],weights:[1,2,4])
scaled = profile.curve_normalize_weights()
panel = scaled.surface_extrude(vector:[0,0,2mm]).surface_cancel_factor(direction:"u",max_work:1000000)
show panel.tessellate(8,8)
