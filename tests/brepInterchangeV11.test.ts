import {describe,expect,it} from 'vitest'
import {createBrepBox} from '../src/services/geometry/brep'
import {exportDirectIgesV2,importDirectIgesV2} from '../src/services/cadIges'
import {exportDirectStepV4,importDirectStepV4} from '../src/services/cadNurbsStep'

describe('V11 direct interchange successors',()=>{
  it('roundtrips an IGES graph without reconstruction',()=>{
    const source=createBrepBox([0,0,0],[2,3,4])
    const exported=exportDirectIgesV2(source)
    expect(exported.text.split('\n').filter(Boolean).every(line=>line.length===80)).toBe(true)
    expect(exported.text).toContain('     186')
    expect(exported.text).toContain('     514')
    const imported=importDirectIgesV2(exported.text)
    expect(imported.identity.preserved).toBe(true)
    expect(imported.model.topologyIds?.faces).toEqual(source.topologyIds?.faces)
    expect(imported.model.faces).toHaveLength(6)
  })

  it('exposes the STEP /4 graph successor without changing /3',()=>{
    const source=createBrepBox([0,0,0],[1,1,1])
    const exported=exportDirectStepV4(source)
    expect(exported.certificate.capability).toBe('step-interchange/4')
    const imported=importDirectStepV4(exported.text)
    expect(imported.identity.preserved).toBe(true)
    expect(imported.model.topologyIds?.edges).toEqual(source.topologyIds?.edges)
  })

  it('refuses malformed and unsupported roots',()=>{
    expect(()=>importDirectIgesV2('not an 80-column IGES exchange')).toThrow()
    expect(()=>importDirectStepV4('ISO-10303-21; DATA; #1=FACETED_BREP(); ENDSEC; END-ISO-10303-21;')).toThrow()
  })
})
