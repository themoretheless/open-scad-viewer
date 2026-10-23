import {warmGeometryKernel} from '../../src/services/geometry/kernel'
import {authorBrepProfile,booleanBrepProfiles,transformBrepProfile} from '../../src/services/geometry/brepProfile'
import {withRetainedProfile} from '../../src/services/retainedSketchProfile'
export async function retainedLoftFixture(){
 await warmGeometryKernel()
 const profile=booleanBrepProfiles(authorBrepProfile({kind:'circle',radius:3}),authorBrepProfile({kind:'circle',radius:1}),'difference')
 const base=withRetainedProfile({id:'base',name:'Base',closed:true,points:[]},profile)
 const top=withRetainedProfile({id:'top',name:'Top',closed:true,points:[],plane:{origin:[0,0,10],u:[1,0,0],v:[0,1,0]}},transformBrepProfile(profile,[2,0,0,0,0,2,0,0,0,0,1,0,5,7,0,1]))
 return [base,top]
}
