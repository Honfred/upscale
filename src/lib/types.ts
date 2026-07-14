// Зеркало Rust-структур из src-tauri/src/{config,events,probe,commands}.rs.
// КОНТРАКТ: при изменении Rust-типов этот файл обновляется синхронно.

export type Codec = "hevc" | "h264" | "av1";
export type Container = "mkv" | "mp4";

export interface UpscaleSettings {
  /** Целевая ширина, напр. 3840. Масштаб (x2/x3/x4) подбирается автоматически. */
  targetWidth: number;
  /** null = оставить исходный fps (без интерполяции). */
  targetFps: number | null;
  codec: Codec;
  /** NVENC constant quality, 0..51, дефолт 19 (меньше = лучше). */
  cq: number;
  container: Container;
  /** null = авто-подбор по свободному месту (6..20 c). */
  segmentSeconds: number | null;
  keepIntermediate: boolean;
  /** null = рядом с исходником. */
  outputDir: string | null;
  /** null = системный кэш приложения. */
  tempDir: string | null;
}

export const DEFAULT_SETTINGS: UpscaleSettings = {
  targetWidth: 3840,
  targetFps: 60,
  codec: "hevc",
  cq: 19,
  container: "mkv",
  segmentSeconds: null,
  keepIntermediate: false,
  outputDir: null,
  tempDir: null,
};

export interface SourceInfo {
  path: string;
  width: number;
  height: number;
  fps: number;
  durationSec: number;
  frameCount: number;
  hasAudio: boolean;
  subtitleStreams: number[];
  codecName: string;
  pixFmt: string;
}

export interface DiskEstimate {
  tempPeakBytes: number;
  tempTotalWritten: number;
  outputBytesEst: number;
  freeBytes: number;
  sufficient: boolean;
  /** Фактический сегмент в секундах, который будет использован. */
  segmentSeconds: number;
  /** Фактический масштаб модели (2/3/4). */
  scale: number;
  /** Фактическое выходное разрешение. */
  outWidth: number;
  outHeight: number;
}

export interface SystemInfo {
  vulkanOk: boolean;
  gpuName: string | null;
  ffmpegOk: boolean;
  realesrganOk: boolean;
  rifeOk: boolean;
  nvencCodecs: Codec[];
}

export type Stage = "decode" | "upscale" | "interpolate" | "encode" | "concat";

// ---- События (канал job://progress, job://done, job://error) ----

export type JobEvent =
  | {
      kind: "started";
      jobId: string;
      totalSegments: number;
      totalFrames: number;
    }
  | {
      kind: "stage";
      jobId: string;
      stage: Stage;
      segmentIndex: number;
      totalSegments: number;
      stageProgress: number;
      overallProgress: number;
      fpsNow: number | null;
      etaSeconds: number | null;
      framesDone: number;
      framesTotal: number;
    }
  | { kind: "segment_done"; jobId: string; segmentIndex: number }
  | { kind: "warning"; jobId: string; message: string };

export interface JobDone {
  jobId: string;
  outputPath: string;
  elapsedSec: number;
  outputBytes: number;
}

export interface JobError {
  jobId: string;
  stage: Stage | null;
  message: string;
  recoverable: boolean;
}

export type JobState = "running" | "done" | "error" | "cancelled";

export interface JobStatus {
  jobId: string;
  state: JobState;
  overallProgress: number;
}

export interface AppErrorPayload {
  code: string;
  message: string;
}
