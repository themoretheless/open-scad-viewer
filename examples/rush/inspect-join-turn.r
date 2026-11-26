// @rush/1
a = bezier_curve(points:[[0,0,0],[1mm,0,0]])
b = bezier_curve(points:[[1mm,0,0],[1mm,2mm,0]])
result = a.inspect_join_turn(inputs:[b],domains:[[0,1],[0,1]],minimum_cosine:0.5)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
