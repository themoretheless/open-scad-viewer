// @rush/1
a = bezier_curve(points:[[0,0,0],[1mm,0,0]])
result = a.minimum_profiled_ball_workspace(planes:[[1,0,0,0.75]],radius_profile:[[0,0.05],[0.5,0.2],[1,0.1]],gap_tolerance_mm:0.005,max_cells:1000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
