# G1 v76: unresolved WebKit memory failures

Run [37641272884](https://github.com/themoretheless/open-scad-viewer/actions/runs/37641272884), exact source `9ec787a4c4fb5376125a0c183b73e85ce08d8296`, completed with 129 passed working fragments and three WebKit memory failures. These retained supervisor reports were downloaded from the three run-specific GitHub artifacts. No pass is inferred from them.

Endpoint RSS drift: 175525888, 155308032, 200757248 bytes; limit 67108864. Slopes: 516624.9691428571, 381042.0720601504, 399779.33184962405 bytes/job; limit 131072. Original warmup 50, measured 500, sample spacing 25 and no forced GC were preserved.

The cause is not proven. Existing WebKit worker termination issues are investigation leads, not attribution of these failures: https://bugs.webkit.org/show_bug.cgi?id=313389 . The frozen browser revision needs its own reproduction and verification.
