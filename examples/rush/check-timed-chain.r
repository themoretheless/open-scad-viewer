// @rush/1
a = bezier_curve(points:[[0,0,0],[1mm,0,0]])
b = bezier_curve(points:[[1mm,0,0],[3mm,0,0]])
result = a.check_timed_chain(inputs:[b],domains:[[0,1],[0,1]],durations:[1,1],axis_vector:[1,0,0],jump_limits:[2,1,1],segment_limits:[1.5,1,1],tolerances:[0.001,0.001,0.001],max_cells:1000,closed:false,position_limit:0.01,max_joins:1)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
