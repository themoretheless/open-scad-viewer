# Rejected 32-workflow control

Clean mesh84 A/A failed: median ratio 0.9836073, 95% CI [0.9592616, 1.0221198], relative width 1.06553 > 1.06. No A/B occurred. All hashes were unchanged. Increasing averaging did not establish stability.

Observer cost was 110–120 ms per poll. Separate read-only timing identified two ps traversals as roughly 95 ms. Poll-overlap blocks had median absolute log ratio 0.0560 vs 0.0439 without overlap; this is suggestive, not causal proof. Total CPU idle also fell to 53–70% during part of the window, without recognized compiler/test rivals; aggregate AGX includes the fixture and cannot identify another GPU client.

Replace two ps traversals with one plus native read-only proc_pidpath identity lookup. Five local collection samples: old 93.9–95.8 ms, new 55.9–58.3 ms, same 845 processes and same three recognized rivals. Record cumulative CPU time per process for attribution of any further failure. These are observer-cost measurements, not evidence of SDF acceleration. No criterion or workload changes. Start all A/A from zero; preserve both rejected series.
