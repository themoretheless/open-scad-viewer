// @rush/1
a = bezier_curve(points:[[0,0,0],[1mm,0,0]])
result = a.oriented_box_path_clearance(domain:[0,1],oriented_boxes:[[[0.5,0,0],[0,1,0],[-1,0,0],[0,0,1],[0.2,0.1,0.1]]],radius_mm:0.05,parameter_tolerance:0.005,max_cells:1000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
