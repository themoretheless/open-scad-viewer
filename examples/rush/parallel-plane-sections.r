// @rush/1
base = line_curve(start:[0mm,0mm,0mm],end:[2mm,0mm,0mm])
sheet = base.surface_extrude(vector:[0mm,2mm,2mm])
levels = sheet.parallel_plane_sections(plane_normal:[0,0,1],plane_offsets:[0.5mm,1mm,3mm],tolerance_mm:0.05mm,max_work:100000)
section = levels.parallel_section_piece(plane_normal:[0,0,1],plane_offsets:[0.5mm,1mm,3mm],tolerance_mm:0.05mm,max_work:100000,section_index:1,piece:0)
show section.surface_extrude(vector:[0mm,0mm,1mm]).tessellate(12,4)
