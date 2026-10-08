# Bounded chord splitting graph

The graph splits represented source chords at proper crossings and shares one constructed vertex between each crossing pair. Edges retain original edge index/domain, normalized cut-parameter enclosures and a positional construction bound from endpoint errors. This bound concerns represented chord geometry and does not replace the original offset approximation certificate.

All pair diagnostics must be complete, with no contacts, unresolved overlaps or degenerate edges. Multiple cuts on one edge require provably separated parameter intervals. Vertex and edge budgets refuse excessive construction. Contacts, coincident multiway crossings and uncertain cut ordering still require reconciliation; they are not silently dropped.

Three native tests passed: bowtie graph connectivity/error bounds; multiple ordered cuts on one chord; unchanged simple chain and atomic resource/overlap refusal.

Open: contact/overlap reconciliation, directed face traversal, winding selection, final region admission and integration with offset application. No trimmed offset or region topology is certified by this graph yet.
