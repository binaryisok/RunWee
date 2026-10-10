//! # RunWee
//!
//! Ultra-efficient STEM animation engine: manim-style mathematical animation
//! combined with Remotion-style composition, transitions, audio and captions.

#![allow(
    clippy::excessive_precision,
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::new_without_default
)]

pub mod anim;
pub mod audio;
pub mod captions;
pub mod color;
pub mod effects;
pub mod math;
pub mod mobject;
pub mod output;
pub mod path;
pub mod rate;
pub mod render;
pub mod scene;
pub mod shapes;
pub mod style;

/// Everything needed to write scenes.
pub mod prelude {
    pub use crate::anim::*;
    pub use crate::color::*;
    pub use crate::math::*;
    pub use crate::mobject::*;
    pub use crate::output::*;
    pub use crate::path::*;
    pub use crate::rate::*;
    pub use crate::render::*;
    pub use crate::scene::*;
    pub use crate::shapes::*;
    pub use crate::style::*;
}
