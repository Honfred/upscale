// Задача C: стейт-машина экранов приложения (idle -> selected -> processing -> result/error).
// Единая реактивная точка правды на классе с полями $state — безопасный паттерн для
// шаринга состояния между .svelte-компонентами (мутация полей, без переприсваивания
// самого экспортируемого биндинга).
import { DEFAULT_SETTINGS } from "./types";
import type {
  DiskEstimate,
  JobDone,
  JobError,
  JobEvent,
  SourceInfo,
  Stage,
  SystemInfo,
  UpscaleSettings,
} from "./types";
import * as api from "./api";
import { listenJob } from "./events";

export type Screen = "idle" | "selected" | "processing" | "result" | "error";

/** Разрешённые расширения видео (совпадают с фильтром диалога и drag&drop). */
export const VIDEO_EXTENSIONS = ["mkv", "mp4", "avi", "webm", "mov", "ts", "m2ts"];

/** Порядок отображения стадий в StageBar (без "concat" — она визуально часть кодирования). */
export const STAGE_ORDER: Stage[] = ["decode", "upscale", "interpolate", "encode"];

export const STAGE_LABELS: Record<Stage, string> = {
  decode: "Декодирование",
  upscale: "Апскейл",
  interpolate: "Интерполяция",
  encode: "Кодирование",
  concat: "Кодирование",
};

export interface ProgressState {
  stage: Stage | null;
  segmentIndex: number;
  totalSegments: number;
  stageProgress: number;
  overallProgress: number;
  fpsNow: number | null;
  etaSeconds: number | null;
  framesDone: number;
  framesTotal: number;
  completedStages: Stage[];
  warning: string | null;
}

export interface ResultState {
  outputPath: string;
  elapsedSec: number;
  outputBytes: number;
}

export interface ErrorState {
  message: string;
  stage: Stage | null;
  recoverable: boolean;
}

function emptyProgress(): ProgressState {
  return {
    stage: null,
    segmentIndex: 0,
    totalSegments: 0,
    stageProgress: 0,
    overallProgress: 0,
    fpsNow: null,
    etaSeconds: null,
    framesDone: 0,
    framesTotal: 0,
    completedStages: [],
    warning: null,
  };
}

function messageOf(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  if (e && typeof e === "object" && "message" in e) {
    return String((e as { message: unknown }).message);
  }
  return "Неизвестная ошибка";
}

class AppStore {
  screen = $state<Screen>("idle");
  source = $state<SourceInfo | null>(null);
  settings = $state<UpscaleSettings>({ ...DEFAULT_SETTINGS });
  estimate = $state<DiskEstimate | null>(null);
  estimating = $state(false);
  system = $state<SystemInfo | null>(null);
  dropActive = $state(false);
  idleError = $state<string | null>(null);
  jobId = $state<string | null>(null);
  progress = $state<ProgressState>(emptyProgress());
  result = $state<ResultState | null>(null);
  jobError = $state<ErrorState | null>(null);

  private estimateTimer: ReturnType<typeof setTimeout> | undefined;
  private estimateSeq = 0;
  private unlistenJobEvents: (() => void) | null = null;

  /** Можно ли запускать обработку при текущей оценке. */
  get canStart(): boolean {
    return !!this.estimate && this.estimate.sufficient && !this.estimating;
  }

  /** Показывать ли баннер о недостающих компонентах. */
  get systemWarning(): boolean {
    return (
      this.system !== null &&
      (!this.system.ffmpegOk || !this.system.realesrganOk || !this.system.rifeOk)
    );
  }

  setDropActive(v: boolean) {
    this.dropActive = v;
  }

  setIdleError(msg: string | null) {
    this.idleError = msg;
  }

  /** Проверка source-файла через probe_source; при ошибке остаёмся в idle. */
  async selectSource(path: string) {
    this.idleError = null;
    try {
      const info = await api.probeSource(path);
      this.source = info;
      this.settings = { ...DEFAULT_SETTINGS };
      this.estimate = null;
      this.screen = "selected";
      this.scheduleEstimate();
    } catch (e) {
      this.idleError = messageOf(e);
    }
  }

  /** Частичное обновление настроек + переоценка (debounce). */
  updateSettings(patch: Partial<UpscaleSettings>) {
    Object.assign(this.settings, patch);
    this.scheduleEstimate();
  }

  private scheduleEstimate() {
    if (this.estimateTimer) clearTimeout(this.estimateTimer);
    this.estimateTimer = setTimeout(() => {
      void this.runEstimate();
    }, 300);
  }

  private async runEstimate() {
    if (!this.source) return;
    const seq = ++this.estimateSeq;
    this.estimating = true;
    try {
      const snapshot: UpscaleSettings = { ...this.settings };
      const res = await api.estimateJob(this.source, snapshot);
      if (seq === this.estimateSeq) {
        this.estimate = res;
      }
    } catch {
      if (seq === this.estimateSeq) {
        this.estimate = null;
      }
    } finally {
      if (seq === this.estimateSeq) {
        this.estimating = false;
      }
    }
  }

  /** Запуск обработки; при ошибке старта переходим в экран ошибки. */
  async beginJob() {
    if (!this.source || !this.canStart) return;
    try {
      const id = await api.startJob(this.source, { ...this.settings });
      this.jobId = id;
      this.progress = emptyProgress();
      this.result = null;
      this.jobError = null;
      this.screen = "processing";
    } catch (e) {
      this.jobError = { message: messageOf(e), stage: null, recoverable: true };
      this.screen = "error";
    }
  }

  /** Отмена текущей джобы; UI возвращается сразу, не дожидаясь события бэкенда. */
  async requestCancel() {
    if (!this.jobId) return;
    const id = this.jobId;
    this.jobId = null; // дальнейшие события этой джобы игнорируются (см. handleJob*)
    this.screen = "selected";
    try {
      await api.cancelJob(id);
    } catch {
      // отмена — best effort, UI уже вернулся к настройкам
    }
  }

  /** Полный сброс к экрану выбора файла. */
  resetToIdle() {
    if (this.estimateTimer) clearTimeout(this.estimateTimer);
    this.estimateSeq++;
    this.screen = "idle";
    this.source = null;
    this.settings = { ...DEFAULT_SETTINGS };
    this.estimate = null;
    this.estimating = false;
    this.jobId = null;
    this.progress = emptyProgress();
    this.result = null;
    this.jobError = null;
    this.idleError = null;
  }

  /** Из экрана ошибки — назад к настройкам, если ошибка восстановимая. */
  errorBack() {
    if (this.jobError?.recoverable && this.source) {
      this.screen = "selected";
      this.jobError = null;
    } else {
      this.resetToIdle();
    }
  }

  async revealOutput() {
    if (!this.result) return;
    try {
      await api.revealOutput(this.result.outputPath);
    } catch {
      // best effort — не блокируем экран результата
    }
  }

  async checkSystem() {
    try {
      this.system = await api.systemCheck();
    } catch {
      this.system = {
        vulkanOk: false,
        gpuName: null,
        ffmpegOk: false,
        realesrganOk: false,
        rifeOk: false,
        nvencCodecs: [],
      };
    }
  }

  /** Подписка на события job:// — вызывается один раз при монтировании App. */
  async startJobEventsSubscription() {
    if (this.unlistenJobEvents) return;
    this.unlistenJobEvents = await listenJob({
      onProgress: (e) => this.handleJobEvent(e),
      onDone: (e) => this.handleJobDone(e),
      onError: (e) => this.handleJobError(e),
    });
  }

  stopJobEventsSubscription() {
    this.unlistenJobEvents?.();
    this.unlistenJobEvents = null;
  }

  private handleJobEvent(e: JobEvent) {
    if (!this.jobId || e.jobId !== this.jobId) return;
    if (e.kind === "started") {
      this.progress.totalSegments = e.totalSegments;
      this.progress.framesTotal = e.totalFrames;
    } else if (e.kind === "stage") {
      // "concat" визуально считаем частью кодирования, чтобы StageBar оставался 4-элементным
      const display: Stage = e.stage === "concat" ? "encode" : e.stage;
      const idx = STAGE_ORDER.indexOf(display);
      if (idx > 0) {
        for (const s of STAGE_ORDER.slice(0, idx)) {
          if (!this.progress.completedStages.includes(s)) {
            this.progress.completedStages.push(s);
          }
        }
      }
      this.progress.stage = display;
      this.progress.segmentIndex = e.segmentIndex;
      this.progress.totalSegments = e.totalSegments;
      this.progress.stageProgress = e.stageProgress;
      this.progress.overallProgress = e.overallProgress;
      this.progress.fpsNow = e.fpsNow;
      this.progress.etaSeconds = e.etaSeconds;
      this.progress.framesDone = e.framesDone;
      this.progress.framesTotal = e.framesTotal;
    } else if (e.kind === "segment_done") {
      this.progress.completedStages = [];
    } else if (e.kind === "warning") {
      this.progress.warning = e.message;
    }
  }

  private handleJobDone(e: JobDone) {
    if (!this.jobId || e.jobId !== this.jobId) return;
    this.result = {
      outputPath: e.outputPath,
      elapsedSec: e.elapsedSec,
      outputBytes: e.outputBytes,
    };
    this.jobId = null;
    this.screen = "result";
  }

  private handleJobError(e: JobError) {
    if (!this.jobId || e.jobId !== this.jobId) return;
    this.jobError = { message: e.message, stage: e.stage, recoverable: e.recoverable };
    this.jobId = null;
    this.screen = "error";
  }
}

export const app = new AppStore();
