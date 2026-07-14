// Типизированные обёртки над Tauri invoke. КОНТРАКТ: имена команд и формы
// аргументов соответствуют #[tauri::command] в src-tauri/src/commands.rs.
import { invoke } from "@tauri-apps/api/core";
import type {
  DiskEstimate,
  JobStatus,
  SourceInfo,
  SystemInfo,
  UpscaleSettings,
} from "./types";

export function probeSource(path: string): Promise<SourceInfo> {
  return invoke("probe_source", { path });
}

export function estimateJob(
  source: SourceInfo,
  settings: UpscaleSettings,
): Promise<DiskEstimate> {
  return invoke("estimate_job", { source, settings });
}

/** Возвращает jobId; прогресс приходит событиями (см. events.ts). */
export function startJob(
  source: SourceInfo,
  settings: UpscaleSettings,
): Promise<string> {
  return invoke("start_job", { source, settings });
}

export function cancelJob(jobId: string): Promise<void> {
  return invoke("cancel_job", { jobId });
}

export function getJobStatus(jobId: string): Promise<JobStatus> {
  return invoke("get_job_status", { jobId });
}

export function systemCheck(): Promise<SystemInfo> {
  return invoke("system_check");
}

export function revealOutput(path: string): Promise<void> {
  return invoke("reveal_output", { path });
}
