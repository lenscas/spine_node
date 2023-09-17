
use macroquad::miniquad::*;
use macroquad::prelude::glam::Mat4;

pub const VERTEX: &str = r#"
        #version 100
        attribute vec2 position;
        attribute vec2 uv;
        attribute vec4 color;
        attribute vec4 dark_color;

        uniform mat4 world;
        uniform mat4 view;

        varying lowp vec2 f_texcoord;
        varying lowp vec4 f_color;
        varying lowp vec4 f_dark_color;

        void main() {
            gl_Position = view * world * vec4(position, 0, 1);
            f_texcoord = uv;
            f_color = color;
            f_dark_color = dark_color;
        }
    "#;

pub const FRAGMENT: &str = r#"
        #version 100
        varying lowp vec2 f_texcoord;
        varying lowp vec4 f_color;
        varying lowp vec4 f_dark_color;

        uniform sampler2D tex;

        void main() {
            lowp vec4 tex_color = texture2D(tex, f_texcoord);
            gl_FragColor = vec4(
                ((tex_color.a - 1.0) * f_dark_color.a + 1.0 - tex_color.rgb) * f_dark_color.rgb + tex_color.rgb * f_color.rgb,
                tex_color.a * f_color.a
            );
        }
    "#;

pub fn meta() -> ShaderMeta {
    ShaderMeta {
        images: vec!["tex".to_string()],
        uniforms: UniformBlockLayout {
            uniforms: vec![
                UniformDesc::new("world", UniformType::Mat4),
                UniformDesc::new("view", UniformType::Mat4),
            ],
        },
    }
}

#[repr(C)]
pub struct Uniforms {
    pub world: Mat4,
    pub view: Mat4,
}
