use std::{cell::RefCell, rc::Rc};

use macroquad::prelude::{vec2, RED};
use monad_quad::{
    components::{
        asyncs::{AsyncComp, AsyncState, WithLoading},
        logic::{Comp, StateFull},
        render::Background,
    },
    Component,
};

use spine_node::{
    setup_runtime, AnimationEvent, AnimationOptions, AnimationStateWithData, LoadSpineFromBytes,
    Spine, SpineAnimation, SpineState,
};

struct MainState {
    spine_state: AsyncState<SpineState>,
}

#[macroquad::main("basic spine example")]
async fn main() {
    //We are first going to create a SpineState
    let loading = load_spine();
    let state = MainState {
        //put the future into the async state from monad_quad so it can be easily used in the scene graph
        spine_state: AsyncState::new_loading(loading),
    };
    StateFull::new_from(state)
        .render(
            //push the future along and either render our character rig or a red background depending on
            //if it is loaded already or not
            WithLoading::new(
                |v: &MainState| v.spine_state.to_owned(),
                animation_node(),
                Comp::<_, Background>::map_in(|_| RED).map_out(|_, _| ()),
            ),
        )
        .await;
}

fn animation_node() -> impl for<'c> Component<
    &'c (Rc<RefCell<SpineState>>, bool, &'c MainState), //render state
    &'c mut (Rc<RefCell<SpineState>>, bool, &'c mut MainState), //process state
> {
    AsyncComp::<
        SpineState,
        AnimationStateWithData<()>,
        SpineAnimation<fn(AnimationEvent, &mut _)>,
    >::map_in(|x, _, _: &MainState| AnimationStateWithData {
        animation_state: x.borrow().to_owned(),
        extra_data: (),
    })
    .map_out_with(
        |event, state| {
            if let AnimationEvent::Complete { track_entry: _ } = event {
                let skin = state.animation_state.get_skin(|v| {
                    v.map(|v| v.name().to_string())
                        .unwrap_or_else(|| "no_mask".to_string())
                });
                println!("{}", skin);
                if skin == "no_mask" {
                    state.animation_state.set_skin_by_name("with_mask").unwrap()
                } else {
                    state.animation_state.set_skin_by_name("no_mask").unwrap()
                }
            } else {
                println!("{:?}", event);
            };
        },
        |x, v| {
            v.spine_state.to_done(x.animation_state);
        },
    )
}

async fn load_spine() -> SpineState {
    let token = setup_runtime();
    let base_path = "./examples/asset_exports/".to_string();
    let atlas = macroquad::file::load_file(&(base_path.clone() + "skeleton.atlas"))
        .await
        .unwrap();
    let skeleton = macroquad::file::load_file(&(base_path.clone() + "skeleton.json"))
        .await
        .unwrap();
    let texture = macroquad::texture::load_texture(&(base_path.clone() + "skeleton.png"))
        .await
        .unwrap();
    let spine = Spine::load_from_bytes(
        LoadSpineFromBytes {
            atlas: atlas.into(),
            dir_path: base_path.into(),
            skeleton_data: spine_node::SpineSkeletonBytes::Json(skeleton.into()),
            animation: Some(AnimationOptions {
                looping: true,
                animation_name: "wag_tail".to_string(),
            }),
            position: vec2(0., -220.),
            scale: 0.5,
            skin: Some("no_mask".to_string()),
            backface_culling: true,
            preloaded_texture: Some(texture),
        },
        token.clone(),
    )
    .unwrap();
    SpineState::new(spine, token).unwrap()
}
