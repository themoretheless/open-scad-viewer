// @rush/1
line = nurbs_curve(degree:1,knots:[0,0,1,1],control_points:[[0,0,0],[8mm,0,0]],weights:[1,1])
m_limits = line.curve_axis_motion_limits(interval:[0,1],directions:[[1,0,0]],limits:[[4,10,10]],tolerances:[0.001,0.001,0.001],max_cells:192)
m_slowdown = m_limits.curve_axis_motion_slowdown(interval:[0,1],directions:[[1,0,0]],limits:[[4,10,10]],tolerances:[0.001,0.001,0.001],max_cells:192)
m_multi_slowdown = m_slowdown.curve_axis_motion_multi_slowdown(interval:[0,1],directions:[[1,0,0],[0,1,0]],limits:[[4,10,10],[1,10,10]],tolerances:[0.001,0.001,0.001],max_cells:192)
m_duration = m_multi_slowdown.curve_axis_motion_duration(interval:[0,1],directions:[[1,0,0],[0,1,0]],limits:[[4,10,10],[1,10,10]],tolerances:[0.001,0.001,0.001],max_cells:192)
m_deadline = m_duration.curve_axis_motion_deadline(interval:[0,1],directions:[[1,0,0],[0,1,0]],limits:[[4,10,10],[1,10,10]],tolerances:[0.001,0.001,0.001],max_cells:192,deadline:1.5)
show m_deadline.surface_extrude(vector:[0,2mm,0]).tessellate(8,2)
