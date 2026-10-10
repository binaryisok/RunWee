//! Captions (expanded in the captions phase).

use crate::render::RenderConfig;

#[derive(Clone, Debug, Default)]
pub struct CaptionTrack {}

impl CaptionTrack {
    pub fn draw(
        &self,
        _t: f64,
        _pm: &mut tiny_skia::Pixmap,
        _cfg: RenderConfig,
        _frame_height: f64,
    ) {
    }
    pub fn is_empty(&self) -> bool {
        true
    }
    pub fn to_srt(&self) -> String {
        String::new()
    }
}

pub fn srt_to_vtt(srt: &str) -> Option<String> {
    Some(format!("WEBVTT\n\n{}", srt.replace(',', ".")))
}
