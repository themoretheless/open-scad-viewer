// @rush/1
profile = ellipse_arc(center:[5mm,0mm,0mm],axis_u:[5mm,0mm,0mm],axis_v:[0mm,0.7mm,0mm],start_degrees:0deg,sweep_degrees:360deg)
wing = wing_loft_surface(inputs:[profile,profile,profile],wing_stations:[[0mm,0deg,8deg,7deg,1],[20mm,5deg,8deg,7deg,0.85],[50mm,12deg,8deg,7deg,0.6]],wing_tip:"open",tip_height_mm:0mm,fold_grid:64)
show wing.tessellate(16,16)
