/** Keep token boundaries and intra-line anchors used by shader variants. */
export function compactWgslLiterals(source) {
  return source.replace(/(\/\*\s*wgsl\s*\*\/\s*)`([^`]*)`/g, (literal, marker, body) => {
    // Interpolated/escaped strings need a JS parser; leave them untouched.
    if (body.includes('${') || body.includes('\\')) return literal
    // Block comments can nest in WGSL; avoid interpreting their line comments.
    if (body.includes('/*')) return literal
    const uncommented=body.replace(/\/\/[^\r\n]*/g, '')
    if (/["']/.test(uncommented)) return literal
    const compact = uncommented.split('\n').map(line => line.trim()).filter(Boolean).join('\n')
    return marker + '`' + compact + '`'
  })
}
export function compactWgslPlugin() {
  return {name:'compact-static-wgsl',apply:'build',enforce:'pre',transform(source,id){
    if (!/\/src\/services\/(shaders\/generated\/sources|solidGpuView)\.ts$/.test(id)) return
    return {code:compactWgslLiterals(source),map:null}
  }}
}
