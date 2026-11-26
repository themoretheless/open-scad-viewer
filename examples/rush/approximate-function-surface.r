// @rush/1
panel = approximate_function_surface(domain:[0,1,0,1],expressions:["8*u","6*v","u*u+v*v"],second_derivative_bounds:[2mm,2mm],tolerance_mm:0.01mm,max_cells:256)
show panel.tessellate(16,16)
