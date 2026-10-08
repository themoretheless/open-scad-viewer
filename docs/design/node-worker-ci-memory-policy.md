# Linux CI allocator policy for disposable qualification workers

The Linux Node check jobs declare `MALLOC_ARENA_MAX=2` before Node starts.
This bounds glibc allocator arenas rather than changing the memory probe:
four warm-up jobs, 24 measured jobs, worker termination/join, three GC passes,
32 MiB RSS growth and all original slope/heap/external limits remain unchanged.
The workflow saves a separate probe receipt alongside provider-readiness data.

glibc documents this as a process-wide arena-count limit independent of core
count: https://sourceware.org/glibc/manual/latest/html_node/Memory-Allocation-Tunables.html

Exact PR38 Node 22.23.3 checks failed twice with RSS growth of 58,433,536 and
48,926,720 bytes. Joined-worker counts and bounded JavaScript heap do not prove
that the native allocator returned every page. This policy does not establish
the cause of those failures or certify default-allocator production memory.

A local Linux ARM64 Node 22.23.3 experiment used the exact locked source and
WASM49. Liftoff-only compilation alone grew RSS by 38,928,384 bytes. Two fresh
Liftoff-only probes with two arenas grew RSS by -397,312 and 26,202,112 bytes;
all 28 workers joined, no supervisor quarantine occurred, and heap growth was
166,160 bytes. These ARM64 observations are diagnostic evidence, not an x86 CI
pass. x86 emulation failed inside esbuild's Go runtime and supplies no valid
memory measurement. The final policy must pass real Linux x86 CI on the exact
pushed commit. Node's existing compilation policy is unchanged.

No geometry source, WASM artifact, proof budget, historical ownRust record or
G0/G1 row result changes. The broader clean G1 matrix remains unqualified.
