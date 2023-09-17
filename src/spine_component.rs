use std::path::PathBuf;

use macroquad::{
    miniquad::{Bindings, BlendState, BufferSource, BufferType, BufferUsage, UniformsSource},
    prelude::{warn, Vec2},
    texture::Texture2D,
    window::get_internal_gl,
};
use monad_quad::{
    components::{asyncs::AsyncState, Context},
    Component,
};
use rusty_spine::Color;

use crate::{
    animation_event::AnimationEvent,
    blend_states::{BlendStates, GetBlendStates},
    create_pipeline,
    setup_rusty_spine::get_cache_item,
    shader, SpineState,
};

const MAX_MESH_VERTICES: usize = 10000;
const MAX_MESH_INDICES: usize = 5000;

#[repr(C)]
struct Vertex {
    position: Vec2,
    uv: Vec2,
    color: Color,
    dark_color: Color,
}

pub struct AnimationStateWithData<T> {
    pub animation_state: SpineState,
    pub extra_data: T,
}

pub struct SpineAnimation<Func> {
    func: Func,
}

impl<T, Func: Fn(AnimationEvent, &mut AnimationStateWithData<T>)>
    Component<&AnimationStateWithData<T>, &mut AnimationStateWithData<T>> for SpineAnimation<Func>
{
    type Input = Func;

    fn instantiate(func: Self::Input) -> Self
    where
        Self: Sized,
    {
        Self { func }
    }

    fn process<'c>(
        &mut self,
        context: &Context,
        state: &'c mut AnimationStateWithData<T>,
    ) -> &'c mut AnimationStateWithData<T> {
        state.animation_state.process_loading();
        let delta = context.get_delta();
        state
            .animation_state
            .spine
            .borrow_mut()
            .controller
            .borrow_mut()
            .update(delta);
        if state.animation_state.events.borrow().is_empty() {
            return state;
        }
        let temp_events = Vec::new();
        let mut events = state.animation_state.events.replace(temp_events);
        for event in events.drain(0..) {
            (self.func)(event, state)
        }
        state.animation_state.events.replace(events);
        state
    }

    fn render(&self, context: &Context, props: &AnimationStateWithData<T>) {
        let renderables = props
            .animation_state
            .spine
            .borrow()
            .controller
            .borrow_mut()
            .combined_renderables();
        let gl_context = unsafe { get_internal_gl().quad_context };
        // Create bindings that can be re-used for rendering Spine meshes
        while renderables.len() > props.animation_state.bindings.borrow().len() {
            let buffer_source = BufferSource::empty::<Vertex>(MAX_MESH_VERTICES);
            let vertex_buffer =
                gl_context.new_buffer(BufferType::VertexBuffer, BufferUsage::Stream, buffer_source);
            let buffer_source = BufferSource::empty::<u16>(MAX_MESH_INDICES);
            let index_buffer =
                gl_context.new_buffer(BufferType::IndexBuffer, BufferUsage::Stream, buffer_source);

            props.animation_state.bindings.borrow_mut().push(Bindings {
                vertex_buffers: vec![vertex_buffer],
                index_buffer,
                images: vec![],
            });
        }

        // Delete textures that are no longer used. The delete call needs to happen here, before
        // rendering, or it may not actually delete the texture.
        props
            .animation_state
            .texture_delete_queue
            .delete_previous(gl_context);

        // Apply backface culling only if this skeleton needs it
        //gl_context.set_cull_face(props.spine.cull_face);

        let view = props.animation_state.view(context);
        let ctx = gl_context;
        let mut last_blend_state: Option<(BlendState, BlendState)> = None;
        let mut pipeline = props.animation_state.pipeline;
        for (renderable, bindings) in renderables
            .into_iter()
            .zip(props.animation_state.bindings.borrow_mut().iter_mut())
        {
            // Set blend state based on this renderable's blend mode
            let BlendStates {
                alpha_blend,
                color_blend,
            } = renderable.blend_mode.get_blend_states(
                props
                    .animation_state
                    .spine
                    .borrow()
                    .controller
                    .borrow_mut()
                    .settings
                    .premultiplied_alpha,
            );
            let change_pipeline = match last_blend_state {
                None => true,
                Some(x) if x.0 != alpha_blend || x.1 != color_blend => true,
                Some(_) => false,
            };
            if change_pipeline {
                last_blend_state = Some((alpha_blend, color_blend));
                pipeline =
                    create_pipeline(ctx, &props.animation_state.spine.borrow(), last_blend_state)
                        .unwrap();
            }
            ctx.apply_pipeline(&pipeline);

            // Create the vertex and index buffers for miniquad
            let mut vertices = vec![];
            for vertex_index in 0..renderable.vertices.len() {
                vertices.push(Vertex {
                    position: Vec2 {
                        x: renderable.vertices[vertex_index][0],
                        y: renderable.vertices[vertex_index][1],
                    },
                    uv: Vec2 {
                        x: renderable.uvs[vertex_index][0],
                        y: renderable.uvs[vertex_index][1],
                    },
                    color: Color::from(renderable.colors[vertex_index]),
                    dark_color: Color::from(renderable.dark_colors[vertex_index]),
                });
            }
            ctx.buffer_update(bindings.vertex_buffers[0], BufferSource::slice(&vertices));
            //bindings.vertex_buffers[0].update(ctx, &vertices);
            ctx.buffer_update(
                bindings.index_buffer,
                BufferSource::slice(&renderable.indices),
            );
            //bindings.index_buffer.update(ctx, &renderable.indices);

            // If there is no attachment (and therefore no texture), skip rendering this renderable
            let Some(attachment_renderer_object) = renderable.attachment_renderer_object else { continue };
            if attachment_renderer_object.is_null() {
                warn!("SkeletonCombinedRenderable.attachment_renderer_object is Some(null).");
                warn!("This often indicates that the callbacks aren't setup properly.");
                warn!("Skipping for now.");
                continue;
            }
            // Load textures if they haven't been loaded already
            let spine_texture =
                unsafe { &mut *(attachment_renderer_object as *mut AsyncState<PathBuf>) };
            let texture = match spine_texture.get_value() {
                None => {
                    println!("is not loaded");
                    continue;
                }
                Some(x) => x.to_owned(),
            };
            let texture = get_cache_item(&texture, |v| v.map(Texture2D::raw_miniquad_id).unwrap());
            bindings.images = vec![texture];
            // Draw this renderable

            ctx.apply_bindings(bindings);

            ctx.apply_uniforms(UniformsSource::table(&shader::Uniforms {
                world: props.animation_state.spine.borrow().world,
                view,
            }));
            ctx.draw(0, renderable.indices.len() as i32, 1);
        }

        // End frame
        ctx.end_render_pass();
    }
}
