// @rush/1
a = bezier_curve(points:[[0,0,0],[1mm,0,0]])
b = bezier_curve(points:[[1mm,0,0],[3mm,0,0]])
result = a.inspect_motion_chain(axis_vector:[1,0,0],inputs:[b],domains:[[0,1],[0,1]],durations:[1,1],jump_limits:[0.5,1,1],closed:true,position_limit:0.1,max_joins:2)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
