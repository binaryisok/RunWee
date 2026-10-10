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

pub mod color;
pub mod math;
pub mod rate;

/// Everything needed to write scenes.
pub mod prelude {
    pub use crate::color::*;
    pub use crate::math::*;
    pub use crate::rate::*;
}
