# Browser multitab qualification

Two actual CUA browser tabs on port 5198 shared the same controlled document. A 1 mm X translation in the first tab triggered a visible conflict in the second. Both documents were exported through the UI. Loading the saved version matched the writer document structurally. Undo restored the entire prior local document; Redo restored the loaded document; reload retained it. Undo/Redo writes triggered a visible conflict in the first tab.

All four whole-document comparisons passed, including B-rep geometry and IDs. JSON key order is ignored; array order is preserved. This verifies this sequential conflict scenario. Simultaneous commit races, crashes and corrupted storage remain unqualified. The user document on port 5195 was untouched.
