// Подписка на события прогресса джобы. КОНТРАКТ: имена каналов соответствуют
// src-tauri/src/events.rs.
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { JobDone, JobError, JobEvent } from "./types";

export const EV_PROGRESS = "job://progress";
export const EV_DONE = "job://done";
export const EV_ERROR = "job://error";

export interface JobListeners {
  onProgress: (e: JobEvent) => void;
  onDone: (e: JobDone) => void;
  onError: (e: JobError) => void;
}

/** Подписывается на все три канала; возвращает функцию отписки. */
export async function listenJob(l: JobListeners): Promise<UnlistenFn> {
  const unsubs = await Promise.all([
    listen<JobEvent>(EV_PROGRESS, (e) => l.onProgress(e.payload)),
    listen<JobDone>(EV_DONE, (e) => l.onDone(e.payload)),
    listen<JobError>(EV_ERROR, (e) => l.onError(e.payload)),
  ]);
  return () => unsubs.forEach((u) => u());
}
