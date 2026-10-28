use crate::valid::ModuleInfo;
use crate::{Module, ShaderStage};

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub shader_model: (u8, u8),
}

#[derive(Debug, Clone, Default)]
pub struct PipelineOptions {
    pub entry_point: Option<(ShaderStage, String)>,
}

pub fn write_string(
    module: &Module,
    _info: &ModuleInfo,
    _options: &Options,
    _pipeline_options: &PipelineOptions,
) -> Result<String, String> {
    Ok(module.raw_source.clone())
}
