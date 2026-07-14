// Задача C: форматирование чисел/времени для UI. Только представление, без бизнес-логики.

/** Байты -> человекочитаемая строка (КБ/МБ/ГБ). */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return "—";
  const gb = bytes / 1024 ** 3;
  if (gb >= 1) return `${gb.toFixed(gb >= 10 ? 1 : 2)} ГБ`;
  const mb = bytes / 1024 ** 2;
  if (mb >= 1) return `${mb.toFixed(mb >= 10 ? 0 : 1)} МБ`;
  const kb = bytes / 1024;
  return `${Math.max(1, Math.round(kb))} КБ`;
}

/** Секунды -> "чч:мм:сс". */
export function formatDuration(totalSec: number): string {
  if (!Number.isFinite(totalSec) || totalSec < 0) return "00:00:00";
  const rounded = Math.round(totalSec);
  const s = rounded % 60;
  const m = Math.floor(rounded / 60) % 60;
  const h = Math.floor(rounded / 3600);
  const pad = (n: number) => n.toString().padStart(2, "0");
  return `${pad(h)}:${pad(m)}:${pad(s)}`;
}

/** Секунды до завершения -> "~12 мин" / "~45 сек" / "~1 ч 5 мин". */
export function formatEta(seconds: number | null): string {
  if (seconds == null || !Number.isFinite(seconds) || seconds < 0) return "—";
  if (seconds < 60) return `~${Math.max(1, Math.round(seconds))} сек`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `~${minutes} мин`;
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  return m === 0 ? `~${h} ч` : `~${h} ч ${m} мин`;
}

/** Кадры в секунду -> "24.0 кадр/с". */
export function formatFps(fps: number | null): string {
  if (fps == null || !Number.isFinite(fps)) return "—";
  return `${fps.toFixed(fps < 10 ? 1 : 0)} кадр/с`;
}

/** Имя файла из полного пути (без учёта ОС-специфичных нюансов). */
export function basename(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}
