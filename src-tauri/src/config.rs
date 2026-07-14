//! Настройки джобы и производные вычисления. КОНТРАКТ с src/lib/types.ts.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Codec {
    Hevc,
    H264,
    Av1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Container {
    Mkv,
    Mp4,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpscaleSettings {
    /// Целевая ширина (напр. 3840); масштаб модели подбирается автоматически.
    pub target_width: u32,
    /// None = оставить исходный fps (стадия интерполяции пропускается).
    pub target_fps: Option<f32>,
    pub codec: Codec,
    /// NVENC constant quality 0..51, дефолт 19.
    pub cq: u8,
    pub container: Container,
    /// None = авто-подбор в estimate (диапазон 6..20 с).
    pub segment_seconds: Option<u32>,
    pub keep_intermediate: bool,
    /// None = рядом с исходником.
    pub output_dir: Option<PathBuf>,
    /// None = app cache dir.
    pub temp_dir: Option<PathBuf>,
}

impl UpscaleSettings {
    /// Масштаб модели realesr-animevideov3: clamp(ceil(target/source), 2, 4).
    pub fn scale_for(&self, source_width: u32) -> u32 {
        let s = (self.target_width + source_width - 1) / source_width.max(1);
        s.clamp(2, 4)
    }
}

/// NVENC preset зафиксирован (баланс скорость/качество для 4070 Super).
pub const NVENC_PRESET: &str = "p5";
/// Tile size для ncnn-vulkan на 12GB VRAM при 4K-выводе.
pub const NCNN_TILE: u32 = 256;
/// Модель Real-ESRGAN для аниме-видео.
pub const ESRGAN_MODEL: &str = "realesr-animevideov3";
/// Модель RIFE (поддерживает произвольное число выходных кадров, -n).
pub const RIFE_MODEL_DIR: &str = "rife-v4.6";
