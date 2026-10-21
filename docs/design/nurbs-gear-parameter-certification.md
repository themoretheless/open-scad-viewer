# Inclusive nominal gear flank bounds

The native Rust parameter owner `involute::derive_gear_parameter_bounds` encloses the ideal involute parameters derived from exact binary64 module, tooth count, degree pressure angle, clearance and backlash. It uses Machin's identity for pi, outward alternating-series arctangent bounds, Taylor sine/cosine bounds, outward interval arithmetic and bracketed square roots. Below-base root radii have an exactly zero roll start; uncertainty near the base circle is retained.

`involute::certify_quintic_parameter_box` bounds the actual saved quintic poles against every ideal involute in independently justified radius, roll and placement boxes. It includes parameter derivation uncertainty, stored pole error, rotation/reflection and the continuous Hermite remainder. Numerical sampling is an independent regression oracle, not the proof.

`brep_core::gear::gear_with_report` computes this bound for both flanks of every tooth. `flank_rounding_certified` is true only if every bound succeeds within the existing 1e-6 budget. The reported `flank_deviation` then includes the full nominal parameter bound. A refused family proof preserves the legacy candidate with the flag false and the separate stored-parameter certificate; it never silently authorizes a nominal-geometry certificate. All nonfinite spec fields are rejected before authoring.

This does not certify root transitions, undercut geometry, profile embedding, whole-solid topology or saved helical wall rounding. Consequently the four gear catalog entries remain planned. Spur, helical, herringbone and internal fixtures verify saved incidence and flank flags, without treating closed incidence as geometric solid validity. The native count remains 189/200.
