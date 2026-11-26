// @rush/1
body = brep_box([0,0,0],[2,3,4])
checked = body.inspect_shell_orientation(tolerance_uv:0.00000001,max_cells:100000)
show checked.brep_tessellate(3)
