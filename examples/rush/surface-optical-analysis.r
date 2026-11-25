// @rush/1
sphere = sphere_surface(center:[0mm,0mm,0mm],radius:1mm)
highlights = sphere.surface_highlight_lines(lights:[[0,0,1]],view_direction:[0,0,1],level:0.17,grid:[32,32],kink_angle_rad:0.35)
reflections = highlights.surface_reflection_lines(light:[0,0,1],view_direction:[0,0,1],level:0.17,grid:[32,32],jump_threshold:200)
bands = reflections.surface_isophote_analysis(light:[0,0,1],cos_levels:[0.23,0.47,0.71],grid:[32,32],uneven_tolerance:0.9)
show bands.tessellate(16,32)
