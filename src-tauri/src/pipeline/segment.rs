//! Сегментация исходного видео по кадрам (без накопления дрейфа времени)
//! и вспомогательные преобразования кадр <-> временная метка.

/// Один сегмент обработки: диапазон кадров исходного видео.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Segment {
    pub index: u32,
    pub start_frame: u64,
    pub frame_count: u64,
}

impl Segment {
    pub fn end_frame(&self) -> u64 {
        self.start_frame + self.frame_count
    }
}

/// Делит `frame_count` кадров на сегменты по `segment_seconds` секунд (в
/// кадрах: round(segment_seconds * fps)). Последний сегмент забирает остаток.
/// sum(frame_count_i) всегда равен исходному frame_count — границы считаются
/// в целых кадрах, поэтому дрейфа не возникает.
pub fn compute_segments(frame_count: u64, fps: f32, segment_seconds: u32) -> Vec<Segment> {
    if frame_count == 0 {
        return Vec::new();
    }

    let segment_frames = ((segment_seconds as f64) * (fps as f64)).round().max(1.0) as u64;

    let mut segments = Vec::new();
    let mut start = 0u64;
    let mut index = 0u32;

    while start < frame_count {
        let remaining = frame_count - start;
        let n = segment_frames.min(remaining);
        segments.push(Segment {
            index,
            start_frame: start,
            frame_count: n,
        });
        start += n;
        index += 1;
    }

    segments
}

/// Тайм-код начала сегмента в секундах (для ffmpeg -ss на входном seek).
pub fn start_timestamp(start_frame: u64, fps: f32) -> f64 {
    start_frame as f64 / fps as f64
}

/// Форматирует тайм-код с 6 знаками после запятой (формат, понятный ffmpeg -ss).
pub fn format_timestamp(ts: f64) -> String {
    format!("{ts:.6}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segments_cover_whole_video_exactly() {
        // 24 fps, 10s segments, 543 кадров (нецелое число сегментов).
        let segments = compute_segments(543, 24.0, 10);
        let total: u64 = segments.iter().map(|s| s.frame_count).sum();
        assert_eq!(total, 543);

        // Границы непрерывны.
        let mut expected_start = 0u64;
        for seg in &segments {
            assert_eq!(seg.start_frame, expected_start);
            expected_start += seg.frame_count;
        }
        assert_eq!(expected_start, 543);
    }

    #[test]
    fn last_segment_takes_remainder() {
        // segment_frames = round(10*24) = 240. 543 = 240*2 + 63.
        let segments = compute_segments(543, 24.0, 10);
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].frame_count, 240);
        assert_eq!(segments[1].frame_count, 240);
        assert_eq!(segments[2].frame_count, 63);
    }

    #[test]
    fn exact_multiple_has_no_tiny_tail_segment() {
        // 480 = 240*2 ровно - последний сегмент не должен быть нулевым лишним.
        let segments = compute_segments(480, 24.0, 10);
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].frame_count, 240);
        assert_eq!(segments[1].frame_count, 240);
    }

    #[test]
    fn single_short_video_is_one_segment() {
        let segments = compute_segments(50, 24.0, 10);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].frame_count, 50);
    }

    #[test]
    fn vfr_fractional_fps_rounds_segment_frames_consistently() {
        // 23.976 fps (NTSC), 8с сегменты -> round(8*23.976) = 192.
        let segments = compute_segments(1000, 23.976, 8);
        let total: u64 = segments.iter().map(|s| s.frame_count).sum();
        assert_eq!(total, 1000);
        assert_eq!(segments[0].frame_count, 192);
    }

    #[test]
    fn empty_video_has_no_segments() {
        assert!(compute_segments(0, 24.0, 10).is_empty());
    }

    #[test]
    fn start_timestamp_matches_frame_over_fps() {
        assert!((start_timestamp(240, 24.0) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn format_timestamp_has_six_decimals() {
        assert_eq!(format_timestamp(10.0), "10.000000");
        assert_eq!(format_timestamp(1.5), "1.500000");
    }
}
