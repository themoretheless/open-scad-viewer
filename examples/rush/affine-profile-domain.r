// @rush/1
profile = bezier_curve(points:[[0,0,0],[4mm,8mm,0],[8mm,0,0]],weights:[1,2,1])
mapped = profile.curve_reparameterize(domain:[2,6])
show mapped.surface_extrude(vector:[0,0,4mm]).tessellate(12,8)
