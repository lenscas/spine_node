use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
};

use macroquad::{
    logging::error,
    miniquad::{MipmapFilterMode, RenderingBackend, TextureWrap},
    prelude::Color,
    texture::{FilterMode, Image, Texture2D},
    window::get_internal_gl,
};
use monad_quad::components::asyncs::AsyncState;
use rusty_spine::atlas::{AtlasFilter, AtlasWrap};

#[derive(Clone)]
pub struct Token {
    queue: Arc<Mutex<Vec<Texture2D>>>,
}
impl Token {
    pub(crate) fn delete_previous(&self, gl_context: &mut dyn RenderingBackend) {
        for texture_delete in self.queue.lock().unwrap().drain(..) {
            gl_context.delete_texture(texture_delete.raw_miniquad_id());
        }
    }
}

static QUEUE: OnceLock<Token> = OnceLock::new();
static CACHE: OnceLock<Arc<Mutex<HashMap<PathBuf, Texture2D>>>> = OnceLock::new();

pub fn get_cache_item<V>(a: &Path, callback: impl FnOnce(Option<&Texture2D>) -> V) -> V {
    let cache = CACHE.get_or_init(Default::default).lock().unwrap();
    let res = cache.get(a);
    callback(res)
}
pub fn get_cache_mut<V>(callback: impl FnOnce(&mut HashMap<PathBuf, Texture2D>) -> V) -> V {
    let mut cache = CACHE.get_or_init(Default::default).lock().unwrap();
    callback(&mut cache)
}
pub fn add_to_cache(key: PathBuf, value: Texture2D) -> Option<Texture2D> {
    let mut cache = CACHE.get_or_init(Default::default).lock().unwrap();
    cache.insert(key, value)
}

fn prepare_texture(
    texture: Texture2D,
    mag_filter: FilterMode,
    min_filter: FilterMode,
    x_wrap: TextureWrap,
    y_wrap: TextureWrap,
) {
    let ctx = unsafe { get_internal_gl() };
    let texture = texture.raw_miniquad_id();
    ctx.quad_context
        .texture_set_filter(texture, FilterMode::Linear, MipmapFilterMode::None);
    ctx.quad_context.texture_set_mag_filter(texture, mag_filter);
    ctx.quad_context
        .texture_set_min_filter(texture, min_filter, MipmapFilterMode::None);
    ctx.quad_context.texture_set_wrap(texture, x_wrap, y_wrap);
}

pub async fn load_texture_or_fallback(path: &str) -> Texture2D {
    macroquad::texture::load_texture(path)
        .await
        .unwrap_or_else(|e| {
            error!("Could not load texture at {}.\nError: {e}", path);
            Texture2D::from_image(&Image::gen_image_color(1, 1, Color::new(1., 0., 1., 1.)))
        })
}

/// Creates a [crate::Token] without checking if the callbacks for rusty_spine are setup correctly.
///
/// # Safety
///
/// In order for rusty_spine (and thus spine_node) to work properly
/// [rusty_spine::extension::set_create_texture_cb] and [rusty_spine::extension::set_dispose_texture_cb] has to be called first
///
/// This is normally done through [crate::setup_runtime]. However,
/// if you want to add your own behavior instead of the default one then
/// you can use this function to prevent [crate::setup_runtime]
/// from setting up the callback again.
///
/// It is unsafe as calling it without the working callbacks will result in some
/// values being `null` when they shouldn't be. With limited testing it appears that this crate
/// handles that correctly at time of writing
/// but that isn't a guarantee nor does it mean that [rusty_spine]
/// doesn't run into UB or similar.
///
pub unsafe fn unchecked_assume_runtime_created() -> Token {
    QUEUE
        .get_or_init(|| Token {
            queue: Arc::new(Mutex::new(Vec::new())),
        })
        .to_owned()
}

fn setup_spine() -> Token {
    rusty_spine::extension::set_create_texture_cb(|atlas_page, path| {
        fn convert_filter(filter: AtlasFilter) -> FilterMode {
            match filter {
                AtlasFilter::Linear => FilterMode::Linear,
                AtlasFilter::Nearest => FilterMode::Nearest,
                filter => {
                    println!("Unsupported texture filter mode: {filter:?}");
                    FilterMode::Linear
                }
            }
        }
        fn convert_wrap(wrap: AtlasWrap) -> TextureWrap {
            match wrap {
                AtlasWrap::ClampToEdge => TextureWrap::Clamp,
                AtlasWrap::MirroredRepeat => TextureWrap::Mirror,
                AtlasWrap::Repeat => TextureWrap::Repeat,
                wrap => {
                    println!("Unsupported texture wrap mode: {wrap:?}");
                    TextureWrap::Clamp
                }
            }
        }
        let min_filter = convert_filter(atlas_page.min_filter());
        let mag_filter = convert_filter(atlas_page.mag_filter());
        let x_wrap = convert_wrap(atlas_page.u_wrap());
        let y_wrap = convert_wrap(atlas_page.v_wrap());
        let path = path.to_owned();
        let mut full_path = PathBuf::new();
        full_path.push(&path);
        let value = if get_cache_mut(|v| v.contains_key(&full_path)) {
            let texture = get_cache_item(&full_path, |v| v.cloned().unwrap());
            prepare_texture(texture, mag_filter, min_filter, x_wrap, y_wrap);
            AsyncState::new_done(full_path)
        } else {
            AsyncState::new_loading(async move {
                let texture = load_texture_or_fallback(&path).await;
                add_to_cache(full_path.clone(), texture.clone());
                prepare_texture(texture, mag_filter, min_filter, x_wrap, y_wrap);
                full_path
            })
        };
        atlas_page.renderer_object().set(value)
    });
    let token = unsafe { unchecked_assume_runtime_created() };
    //let texture_delete_queue_cb = token.queue.clone();
    rusty_spine::extension::set_dispose_texture_cb(move |atlas_page| unsafe {
        atlas_page
            .renderer_object()
            .dispose::<AsyncState<PathBuf>>()
    });
    token
}

pub fn setup_runtime() -> Token {
    QUEUE
        .get()
        .map(ToOwned::to_owned)
        .unwrap_or_else(setup_spine)
}
