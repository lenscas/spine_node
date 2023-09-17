use macroquad::{
    miniquad::{
        BlendState, BufferLayout, Pipeline, RenderingBackend, VertexAttribute, VertexFormat,
    },
    prelude::{PipelineParams, ShaderError, ShaderSource},
};

use crate::{shader, Spine};

pub(crate) fn create_pipeline(
    context: &mut dyn RenderingBackend,
    spine: &Spine,
    blends: Option<(BlendState, BlendState)>,
) -> Result<Pipeline, ShaderError> {
    let source = ShaderSource::Glsl {
        vertex: shader::VERTEX,
        fragment: shader::FRAGMENT,
    };
    create_pipeline_with_shader(context, spine, blends, source)
}

pub(crate) fn create_pipeline_with_shader(
    context: &mut dyn RenderingBackend,
    spine: &Spine,
    blends: Option<(BlendState, BlendState)>,
    source: ShaderSource,
) -> Result<Pipeline, ShaderError> {
    let shader = context.new_shader(source, shader::meta())?;
    let mut default_params = PipelineParams::default();
    default_params.alpha_blend = blends
        .map(|v| Some(v.0))
        .unwrap_or(default_params.alpha_blend);
    default_params.color_blend = blends
        .map(|v| Some(v.1))
        .unwrap_or(default_params.color_blend);
    Ok(context.new_pipeline_with_params(
        &[BufferLayout::default()],
        &[
            VertexAttribute::new("position", VertexFormat::Float2),
            VertexAttribute::new("uv", VertexFormat::Float2),
            VertexAttribute::new("color", VertexFormat::Float4),
            VertexAttribute::new("dark_color", VertexFormat::Float4),
        ],
        shader,
        PipelineParams {
            cull_face: spine.cull_face,
            ..default_params
        },
    ))
}
