// @rush/1
profile = bezier_curve(points:[[0,0,0],[0.5mm,0,0],[1mm,0,0]],weights:[1,2,4])
simple = profile.curve_normalize_weights().curve_cancel_factor(max_work:1000000)
show simple.surface_extrude(vector:[0,0,2mm]).tessellate(8,8)
