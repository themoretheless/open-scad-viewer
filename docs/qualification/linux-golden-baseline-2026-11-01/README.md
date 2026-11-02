# Historical Linux golden replay

All 43 historical hashes from source `6dff3e94a2b0eb56f7024d6c2faf9766f6badbf3` reproduce exactly. The same compiled executable ran under Debian glibc 2.36, Ubuntu 24.04 glibc 2.39 and Ubuntu 26.04 glibc 2.43. Complete canonical payloads are byte-identical in all three executions.

[receipt.json](receipt.json) records the executable and fixture hashes, every case hash and the scope. `canonical-payloads.tar.gz` retains the complete payloads; compiler/libc details and runtime logs are adjacent compressed files. The only test instrumentation saves canonical payloads before hashing. The original exact comparison remains active; no golden value or tolerance changed.

For these historical cases, switching runtime glibc alone does not explain the failed current-source CI hashes. This does not prove compiler-host reproducibility, validate the changed algorithms or qualify a current artifact. The same-host comparison with current sources remains pending.
