# Partial rational wall: live profiling observation

The maximum-work partial-carrier native test remained live at PID 80612,
using approximately one CPU core and 159.8 MiB physical footprint.
A one-second macOS sample was saved to
`/tmp/partial-rational-wall-sample-20261010.txt`.

The sample contains only two observations of the active test thread; it cannot
support percentages or a claim of a dominant hotspot. Both reach
`qualify_trimmed_minimum_aligned_chords`. One reaches the test-only direct
`inspect_self_chords` differential check at `source_profile_shell_wall.rs:544`,
then transverse-correction derivatives and higher derivative bounds. Another
reaches the later pair pass at line 745. This confirms active numerical work,
not a lock wait.

The test compares cached and direct decisions and checks cached work does not
increase. That direct work is additional test overhead, separate from the
reported certificate budget. It must be accounted for before interpreting test
wall-clock time as production performance. No code or proof policy was changed
based on this sparse sample. The running maximum-work baseline is preserved.
