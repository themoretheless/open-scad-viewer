/** Dependency-light probe; the compiler itself stays behind a lazy import. */
export const isModelGraphText = (source: string) => /^\s*\/\/\s*@modelgraph-text\/1\b/.test(source)
