mod animation_event;
mod blend_states;
mod create_pipeline;
mod setup_rusty_spine;
mod shader;
mod spine;
mod spine_component;

use std::borrow::Cow;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

pub use animation_event::AnimationEvent;
use create_pipeline::create_pipeline;
use macroquad::miniquad::{Bindings, Pipeline};
use macroquad::prelude::Mat4;
use macroquad::prelude::ShaderError;
use macroquad::window::get_internal_gl;
use monad_quad::components::asyncs::AsyncState;
use monad_quad::components::Context;
use rusty_spine::c_interface::CTmpMut;
use rusty_spine::{Animation, AnimationState, Skeleton, Skin, SpineError, TrackEntry};
pub use setup_rusty_spine::{setup_runtime, unchecked_assume_runtime_created, Token};
pub use spine::{AnimationOptions, LoadSpineFromBytes, LoadSpineFromFile, Spine};
pub use spine_component::{AnimationStateWithData, SpineAnimation};

#[derive(Clone)]
pub enum SpineSkeletonPath<'a> {
    Binary(Cow<'a, str>),
    Json(Cow<'a, str>),
}
#[derive(Clone)]
pub enum SpineSkeletonBytes<'a> {
    Binary(Cow<'a, [u8]>),
    Json(Cow<'a, [u8]>),
}

#[derive(Clone)]
pub struct SpineState {
    spine: RefCell<Spine>,
    pipeline: Pipeline,
    bindings: RefCell<Vec<Bindings>>,
    texture_delete_queue: Token,
    pub(crate) events: Rc<RefCell<Vec<AnimationEvent>>>,
}

impl SpineState {
    pub fn new(spine: Spine, texture_delete_queue: Token) -> Result<Self, ShaderError> {
        let context = unsafe { get_internal_gl() };
        let pipeline = create_pipeline(context.quad_context, &spine, None)?;
        let events = Rc::new(RefCell::new(Vec::new()));
        let moved_events = events.clone();
        spine
            .controller
            .borrow_mut()
            .animation_state
            .set_listener(move |_, y| moved_events.borrow_mut().push(y.into()));
        Ok(Self {
            events,
            spine: RefCell::new(spine),
            pipeline,
            bindings: RefCell::new(vec![]),
            texture_delete_queue,
        })
    }
    pub fn new_spine(self, spine: Spine) -> Result<Self, ShaderError> {
        Self::new(spine, self.texture_delete_queue)
    }
    fn view(&self, context: &Context) -> Mat4 {
        Mat4::orthographic_rh_gl(
            context.viewport_size().x * -0.5,
            context.viewport_size().x * 0.5,
            context.viewport_size().y * -0.5,
            context.viewport_size().y * 0.5,
            0.,
            1.,
        )
    }
    pub fn set_animation_by_name(
        &mut self,
        track_index: usize,
        animation_name: &str,
        looping: bool,
    ) -> Result<(), SpineError> {
        self.set_animation_by_name_with_cb(track_index, animation_name, looping, |x| x.map(|_| ()))
    }
    pub fn set_animation_by_name_with_cb<T, E>(
        &mut self,
        track_index: usize,
        animation_name: &str,
        looping: bool,
        cb: impl FnOnce(Result<CTmpMut<AnimationState, TrackEntry>, SpineError>) -> Result<T, E>,
    ) -> Result<T, E> {
        let spine = self.spine.borrow();
        let mut borrowed_controller = spine.controller.borrow_mut();
        let x = borrowed_controller.animation_state.set_animation_by_name(
            track_index,
            animation_name,
            looping,
        );
        cb(x)
    }
    pub fn set_animation(&mut self, track_index: usize, animation: &Animation, looping: bool) {
        self.set_animation_with_cb(track_index, animation, looping, |_| ())
    }
    pub fn set_animation_with_cb<T>(
        &mut self,
        track_index: usize,
        animation: &Animation,
        looping: bool,
        cb: impl FnOnce(CTmpMut<AnimationState, TrackEntry>) -> T,
    ) -> T {
        let spine = self.spine.borrow();
        let mut borrowed_controller = spine.controller.borrow_mut();
        let x = borrowed_controller
            .animation_state
            .set_animation(track_index, animation, looping);
        cb(x)
    }
    pub fn set_skin_by_name(&mut self, skin_name: &str) -> Result<(), SpineError> {
        let spine = self.spine.borrow();
        let mut controller = spine.controller.borrow_mut();
        let res = controller.skeleton.set_skin_by_name(skin_name);
        controller.skeleton.set_slots_to_setup_pose();
        res
    }
    pub fn get_skin<T>(
        &self,
        cb: impl FnOnce(Option<rusty_spine::c_interface::CTmpRef<Skeleton, Skin>>) -> T,
    ) -> T {
        let spine = self.spine.borrow();
        let controller = spine.controller.borrow_mut();
        cb(controller.skeleton.skin())
    }
    pub(crate) fn process_loading(&self) {
        let renderables = self
            .spine
            .borrow()
            .controller
            .borrow_mut()
            .combined_renderables();
        for renderable in renderables {
            let Some(attachment_renderer_object) = renderable.attachment_renderer_object else { continue };
            let spine_texture =
                unsafe { &mut *(attachment_renderer_object as *mut AsyncState<PathBuf>) };
            spine_texture.process();
        }
    }
    pub fn is_fully_loaded(&self) -> bool {
        self.spine
            .borrow()
            .controller
            .borrow_mut()
            .combined_renderables()
            .iter()
            .all(|v| {
                let Some(attachment_renderer_object) = v.attachment_renderer_object else { return true };
                let spine_texture =
                    unsafe { &mut *(attachment_renderer_object as *mut AsyncState<PathBuf>) };
                spine_texture.is_loaded()
            })
    }
}
