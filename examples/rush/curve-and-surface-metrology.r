// @rush/1
edge = line_curve(start:[0mm,0mm,0mm],end:[6mm,8mm,0mm])
measured = edge.curve_length_bounds(tolerance_mm:0.001mm,max_cells:512)
station = measured.curve_station_at_length(distance_mm:5mm,tolerance_mm:0.001mm,max_cells:2048)
divided = station.curve_divide_by_length(segments:4,tolerance_mm:0.001mm,max_cells:4096)
wall = divided.surface_extrude(vector:[0mm,0mm,2mm])
area = wall.surface_area_bounds(tolerance_mm2:0.001mm*1mm,max_cells:128)
show area.tessellate(8,4)
