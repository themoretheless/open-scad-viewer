// @rush/1
panel = bezier_surface(points:[[[0,0,0],[0,1mm,0]],[[1mm,0,0],[1mm,1mm,0]]])
cells = panel.surface_umbilic_cells(max_work:1000000)
regions = cells.surface_umbilic_regions(max_depth:4,max_work:1000000)
comparison = regions.surface_gap_comparison(gap_level_per_mm2:1/1mm/1mm,max_work:1000000)
ranges = comparison.surface_gap_ranges(max_work:1000000)
map = ranges.surface_gap_regions(gap_level_per_mm2:1/1mm/1mm,max_depth:4,max_work:1000000)
show map.tessellate(12,12)
