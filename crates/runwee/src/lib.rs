//! # RunWee
//!
//! Ultra-efficient STEM animation engine: manim-style mathematical animation
//! combined with Remotion-style composition, transitions, audio and captions.

#![allow(
    clippy::excessive_precision,
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::new_without_default,
    clippy::needless_range_loop
)]

pub mod anim;
pub mod assets;
pub mod audio;
pub mod biology;
pub mod boolean;
pub mod captions;
pub mod chemistry;
pub mod color;
pub mod compose;
pub mod deco;
pub mod effects;
pub mod extra;
pub mod fonts;
pub mod graph;
pub mod math;
pub mod mobject;
pub mod output;
pub mod path;
pub mod physics;
pub mod rate;
pub mod render;
pub mod scene;
pub mod sfx;
pub mod shapes;
pub mod sim;
pub mod style;
pub mod tex;
pub mod text;
pub mod three_d;
pub mod transition;
pub mod util;
pub mod voice;

/// Everything needed to write scenes.
pub mod prelude {
    pub use crate::anim::*;
    pub use crate::assets::*;
    pub use crate::audio::{AudioClip, AudioData};
    pub use crate::biology;
    pub use crate::boolean::*;
    pub use crate::captions::{CaptionLook, CaptionStyle, Cue};
    pub use crate::chemistry;
    pub use crate::color::*;
    pub use crate::compose::*;
    pub use crate::deco::*;
    pub use crate::effects::Effect;
    pub use crate::extra::*;
    pub use crate::graph::*;
    pub use crate::math::*;
    pub use crate::mobject::*;
    pub use crate::output::*;
    pub use crate::path::*;
    pub use crate::physics;
    pub use crate::rate::*;
    pub use crate::render::*;
    pub use crate::scene::*;
    pub use crate::sfx;
    pub use crate::shapes::*;
    pub use crate::sim;
    pub use crate::style::*;
    pub use crate::tex::{chem, tex, tex_parts, tex_sized, tex_text, MathTex};
    pub use crate::text::*;
    pub use crate::three_d::*;
    pub use crate::transition::{Dir, Presentation, Transition};
    pub use crate::util::*;
    pub use crate::voice::*;
}
