//! Audio timeline (expanded in the audio phase).

use anyhow::Result;
use std::io::Write;
use std::path::Path;

pub const SAMPLE_RATE: u32 = 48_000;

/// Placeholder audio track; filled in by the audio module.
#[derive(Clone, Debug, Default)]
pub struct AudioTrack {}

impl AudioTrack {
    pub fn end_time(&self) -> f64 {
        0.0
    }
    pub fn is_empty(&self) -> bool {
        true
    }
    pub fn mix(&self, sample_rate: u32, duration: f64) -> Vec<f32> {
        vec![0.0; (duration * sample_rate as f64) as usize * 2]
    }
}

/// Write interleaved f32 samples as 16-bit PCM WAV.
pub fn write_wav(path: &Path, samples: &[f32], sample_rate: u32, channels: u16) -> Result<()> {
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    let data_len = (samples.len() * 2) as u32;
    f.write_all(b"RIFF")?;
    f.write_all(&(36 + data_len).to_le_bytes())?;
    f.write_all(b"WAVEfmt ")?;
    f.write_all(&16u32.to_le_bytes())?;
    f.write_all(&1u16.to_le_bytes())?;
    f.write_all(&channels.to_le_bytes())?;
    f.write_all(&sample_rate.to_le_bytes())?;
    f.write_all(&(sample_rate * channels as u32 * 2).to_le_bytes())?;
    f.write_all(&(channels * 2).to_le_bytes())?;
    f.write_all(&16u16.to_le_bytes())?;
    f.write_all(b"data")?;
    f.write_all(&data_len.to_le_bytes())?;
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0).round() as i16;
        f.write_all(&v.to_le_bytes())?;
    }
    Ok(())
}
