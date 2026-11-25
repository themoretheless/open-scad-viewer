// @rush/1
// The patch satisfies z=x*y in millimetre coordinates.
patch = bezier_surface(points:[[[0,0,0],[0,1mm,0]],[[1mm,0,0],[1mm,1mm,1mm]]])
audit = patch.surface_quadric_support(quadric:[[0,0.5,0,0],[0.5,0,0,0],[0,0,0,-0.5],[0,0,-0.5,0]],max_work:1000000,max_cells:4096)
show audit.tessellate(12,12)
