// @rush/1
profile = cst_spline_profile(chord_mm:2mm,n1:0.5,n2:1,upper_coefficients:[0.2,0.3,0.1],lower_coefficients:[-0.15,-0.2,-0.1],samples:64,degree:3,control_count:24)
show profile.surface_extrude(vector:[0,0,1mm]).tessellate(64,4)
