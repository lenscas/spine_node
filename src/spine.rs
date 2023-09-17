use std::{borrow::Cow, cell::RefCell, path::PathBuf, rc::Rc, sync::Arc};

use macroquad::{
    miniquad::CullFace,
    prelude::{Mat4, Vec2},
    texture::Texture2D,
};
use rusty_spine::{
    controller::{SkeletonController, SkeletonControllerSettings},
    draw::{ColorSpace, CullDirection},
    AnimationStateData, Atlas, SkeletonBinary, SkeletonData, SkeletonJson, SpineError,
};

use crate::{setup_rusty_spine::add_to_cache, SpineSkeletonBytes, SpineSkeletonPath, Token};

#[derive(Clone)]
pub struct AnimationOptions {
    pub looping: bool,
    pub animation_name: String,
}

struct SpineCreationOptions {
    position: Vec2,
    scale: f32,
    skin: Option<String>,
    backface_culling: bool,
    animation: Option<AnimationOptions>,
    skeleton_data: SkeletonData,
    atlas: Arc<Atlas>,
}

#[derive(Clone)]
pub struct LoadSpineFromFile<'a> {
    pub atlas_path: String,
    pub skeleton_path: SpineSkeletonPath<'a>,
    pub animation: Option<AnimationOptions>,
    pub position: Vec2,
    pub scale: f32,
    pub skin: Option<String>,
    pub backface_culling: bool,
}
#[derive(Clone)]
pub struct LoadSpineFromBytes<'a> {
    pub atlas: Cow<'a, [u8]>,
    pub dir_path: Cow<'a, str>,
    pub skeleton_data: SpineSkeletonBytes<'a>,
    pub animation: Option<AnimationOptions>,
    pub position: Vec2,
    pub scale: f32,
    pub skin: Option<String>,
    pub backface_culling: bool,
    pub preloaded_texture: Option<Texture2D>,
}

#[derive(Clone)]
pub struct Spine {
    pub(crate) controller: Rc<RefCell<SkeletonController>>,
    pub(crate) world: Mat4,
    pub(crate) cull_face: CullFace,
}

impl Spine {
    fn new(info: SpineCreationOptions, _token: crate::Token) -> Result<Self, SpineError> {
        let skeleton_data = Arc::new(info.skeleton_data);
        let premultiplied_alpha = info.atlas.pages().any(|v| v.pma());
        let animation_state_data = Arc::new(AnimationStateData::new(skeleton_data.clone()));
        let mut controller = SkeletonController::new(skeleton_data, animation_state_data)
            .with_settings(SkeletonControllerSettings {
                premultiplied_alpha,
                cull_direction: CullDirection::CounterClockwise,
                color_space: ColorSpace::SRGB,
            });
        if let Some(animation) = info.animation {
            controller.animation_state.set_animation_by_name(
                0,
                &animation.animation_name,
                animation.looping,
            )?;
        }
        if let Some(skin) = info.skin {
            controller.skeleton.set_skin_by_name(&skin)?;
        }
        controller.settings.premultiplied_alpha = premultiplied_alpha;
        Ok(Self {
            controller: Rc::new(RefCell::new(controller)),
            world: Mat4::from_translation(info.position.extend(0.))
                * Mat4::from_scale(Vec2::splat(info.scale).extend(1.)),
            cull_face: match info.backface_culling {
                false => CullFace::Nothing,
                true => CullFace::Back,
            },
        })
    }
    pub fn load_from_bytes(info: LoadSpineFromBytes, token: Token) -> Result<Self, SpineError> {
        if let Some(preloaded_texture) = info.preloaded_texture {
            let res: Vec<_> = info
                .atlas
                .iter()
                .copied()
                .take_while(|v| *v != b'\n')
                .collect();
            let name = String::from_utf8_lossy(&res);
            let mut full_path = PathBuf::new();
            full_path.push(info.dir_path.as_ref());
            full_path.push(name.as_ref());
            println!("{:?}", full_path);
            add_to_cache(full_path, preloaded_texture);
        }
        let atlas = Arc::new(Atlas::new(info.atlas.as_ref(), info.dir_path.as_ref())?);
        let skeleton_data = match info.skeleton_data {
            SpineSkeletonBytes::Binary(bytes) => {
                let skeleton_binary = SkeletonBinary::new(atlas.clone());
                skeleton_binary.read_skeleton_data(bytes.as_ref())?
            }
            SpineSkeletonBytes::Json(bytes) => {
                let skeleton_json = SkeletonJson::new(atlas.clone());
                skeleton_json.read_skeleton_data(bytes.as_ref())?
            }
        };
        Self::new(
            SpineCreationOptions {
                position: info.position,
                scale: info.scale,
                skin: info.skin,
                backface_culling: info.backface_culling,
                animation: info.animation,
                skeleton_data,
                atlas,
            },
            token,
        )
    }
    pub fn load(info: LoadSpineFromFile, token: crate::Token) -> Result<Self, SpineError> {
        let atlas = Arc::new(Atlas::new_from_file(info.atlas_path)?);

        let skeleton_data = match info.skeleton_path {
            SpineSkeletonPath::Binary(path) => {
                let skeleton_binary = SkeletonBinary::new(atlas.clone());
                skeleton_binary.read_skeleton_data_file(path.as_ref())?
            }
            SpineSkeletonPath::Json(path) => {
                let skeleton_json = SkeletonJson::new(atlas.clone());
                skeleton_json.read_skeleton_data_file(path.as_ref())?
            }
        };
        Self::new(
            SpineCreationOptions {
                position: info.position,
                scale: info.scale,
                skin: info.skin,
                backface_culling: info.backface_culling,
                animation: info.animation,
                skeleton_data,
                atlas,
            },
            token,
        )
    }
}
