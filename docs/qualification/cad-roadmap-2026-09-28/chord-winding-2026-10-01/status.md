# Represented chord winding and fill rules

Added integer winding at a finite XY point for an exactly connected represented closed chord chain, plus oriented multi-loop aggregation. Crossing ownership uses half-open Y intervals; side predicates use outward interval orientation. Boundary points, degenerate edges and unresolved sides return errors instead of a guessed fill. Nonzero and even-odd fill rules are explicit. Loops and edges have bounded input counts.

Four native tests passed: square reversal/outside/boundary; opposite winding in bowtie lobes; double-wound contour distinguishing nonzero/even-odd; outer boundary with clockwise hole and counterclockwise island. Boundary points of holes are refused.

Open: proving interior witnesses for directed boundary walks, containment hierarchy and final contour selection/reconstruction. Winding of represented chords does not certify the original rational offset topology or deviation after trimming. Native-only candidate; no new trimmed command is published.
