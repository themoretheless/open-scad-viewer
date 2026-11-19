// @rush/1
line = bezier_curve(points:[[0,0,0],[8mm,0,0]])
elevated = line.curve_exact_elevate(degree:2,max_work:1000000)
half = elevated.curve_exact_split(parameter:0.5,part:1,max_work:1000000)
trimmed = half.curve_exact_restrict(domain:[0.625,1],max_work:1000000)
piece = trimmed.curve_exact_partition(cuts:[0.75,0.875],part:1,max_work:1000000)
show piece.surface_extrude(vector:[0,0,4mm]).tessellate(8,8)
