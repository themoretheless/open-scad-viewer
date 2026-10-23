import base from '../../../../vite.config'
import {mergeConfig} from 'vite'
export default mergeConfig(base,{define:{__VUE_OPTIONS_API__:false}})
