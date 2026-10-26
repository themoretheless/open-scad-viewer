# Repair regression corpus

These small ASCII STL/OFF meshes were created specifically for this repository by its contributors. They reproduce deterministic versions of defects seen in triangle-soup exports: unwelded vertices, collapsed faces, reversed winding, missing faces, edges with three incident fans, and pinched vertices.

No third-party mesh data is included. The fixtures are distributed under the repository's MIT license; see `LICENSE` at the repository root. Coordinates are in arbitrary model units. Every test uses an exact weld (`epsilon = 0`) and compares repair statistics and residual topology with the recorded expectations in `repair_corpus.rs`.
