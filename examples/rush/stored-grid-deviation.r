// @rush/1
panel = bezier_surface(points:[[[0,0,0],[0,1mm,0]],[[1mm,0,0],[1mm,1mm,0]]])
cell = panel.surface_cell_interpolation_error(uv_domain:[[0,1],[0,1]],max_cells:16)
triangle = cell.surface_triangle_interpolation_error(triangle_uv:[[0,0],[1,0],[1,1]],vertices:[[0,0,0],[1mm,0,0],[1mm,1mm,0.125mm]],max_cells:16)
grid = triangle.surface_grid_interpolation_error(u_lines:[0,1],v_lines:[0,1],vertices:[[0,0,0],[0,1mm,0],[1mm,0,0],[1mm,1mm,0.125mm]],max_cells:16)
show grid.tessellate(8,8)
