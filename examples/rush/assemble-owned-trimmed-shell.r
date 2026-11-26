// @rush/1
source = brep_box(min:[0mm,0mm,0mm],max:[3mm,2mm,4mm])
shell = source.assemble_owned_trimmed_shell(tolerance_mm:0.000001mm,tolerance_uv:0.00000001,agreement_cells:10000,trim_pairs:10000,trim_cells:10000,trim_domain_cells:10000,regularity_cells:10000)
show shell.brep_tessellate(4)
