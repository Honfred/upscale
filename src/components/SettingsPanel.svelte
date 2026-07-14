<!-- Задача C: настройки джобы + оценка места (debounce через app.updateSettings). -->
<script lang="ts">
  import { app } from "../lib/stores.svelte";
  import { formatBytes } from "../lib/format";
  import Toggle from "./Toggle.svelte";
  import Icon from "./Icon.svelte";
  import type { Codec, Container } from "../lib/types";

  const resolutionOptions: { value: number; label: string }[] = [
    { value: 3840, label: "4K" },
    { value: 2560, label: "2K" },
  ];
  const fpsOptions: { value: number | null; label: string }[] = [
    { value: null, label: "Оригинал" },
    { value: 60, label: "60" },
  ];
  const codecOptions: { value: Codec; label: string }[] = [
    { value: "hevc", label: "HEVC" },
    { value: "h264", label: "H.264" },
    { value: "av1", label: "AV1" },
  ];
  const containerOptions: { value: Container; label: string }[] = [
    { value: "mkv", label: "MKV" },
    { value: "mp4", label: "MP4" },
  ];

  function onCqInput(e: Event) {
    const value = Number((e.currentTarget as HTMLInputElement).value);
    app.updateSettings({ cq: value });
  }

  function onKeepIntermediateChange(e: Event) {
    const checked = (e.currentTarget as HTMLInputElement).checked;
    app.updateSettings({ keepIntermediate: checked });
  }
</script>

<div class="settings-panel">
  <div class="field">
    <span class="label">Разрешение</span>
    <Toggle
      options={resolutionOptions}
      value={app.settings.targetWidth}
      onchange={(v) => app.updateSettings({ targetWidth: v })}
      ariaLabel="Целевое разрешение"
    />
  </div>

  <div class="field">
    <span class="label">Частота кадров</span>
    <Toggle
      options={fpsOptions}
      value={app.settings.targetFps}
      onchange={(v) => app.updateSettings({ targetFps: v })}
      ariaLabel="Целевой fps"
    />
  </div>

  <div class="field">
    <span class="label">Кодек</span>
    <Toggle
      options={codecOptions}
      value={app.settings.codec}
      onchange={(v) => app.updateSettings({ codec: v })}
      ariaLabel="Видеокодек"
    />
  </div>

  <details class="advanced">
    <summary>
      <span class="chevron"><Icon name="arrow" size={11} /></span>
      Дополнительно
    </summary>
    <div class="advanced-body">
      <div class="field">
        <span class="label">
          Качество (CQ): <span class="tabular-nums cq-value">{app.settings.cq}</span>
        </span>
        <input
          class="slider"
          type="range"
          min="10"
          max="28"
          step="1"
          value={app.settings.cq}
          oninput={onCqInput}
          aria-label="Constant quality"
        />
        <span class="hint">Меньше значение — выше качество и больше размер файла</span>
      </div>

      <div class="field">
        <span class="label">Контейнер</span>
        <Toggle
          options={containerOptions}
          value={app.settings.container}
          onchange={(v) => app.updateSettings({ container: v })}
          ariaLabel="Контейнер"
        />
      </div>

      <label class="checkbox">
        <input
          type="checkbox"
          checked={app.settings.keepIntermediate}
          onchange={onKeepIntermediateChange}
        />
        Сохранять промежуточные файлы
      </label>
    </div>
  </details>

  <div
    class="estimate"
    data-state={app.estimate ? (app.estimate.sufficient ? "ok" : "warn") : "pending"}
  >
    {#if app.estimate}
      <span class="tabular-nums line">
        {app.estimate.outWidth}×{app.estimate.outHeight} · масштаб ×{app.estimate.scale} · до
        {formatBytes(app.estimate.tempPeakBytes)} временных файлов
      </span>
      {#if !app.estimate.sufficient}
        <span class="line warn-text">
          <Icon name="warning" size={13} />
          Недостаточно места на диске (доступно {formatBytes(app.estimate.freeBytes)})
        </span>
      {/if}
    {:else if app.estimating}
      <span class="line hint">Считаем оценку места на диске…</span>
    {:else}
      <span class="line hint">Оценка появится после выбора настроек.</span>
    {/if}
  </div>
</div>

<style>
  .settings-panel {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-4) var(--space-5);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .label {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .cq-value {
    color: var(--text);
    text-transform: none;
    letter-spacing: normal;
  }

  .hint {
    font-size: 12px;
    color: var(--text-muted);
  }

  .slider {
    width: 100%;
    accent-color: var(--accent);
  }

  .checkbox {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: 13px;
    color: var(--text);
    cursor: pointer;
  }

  .checkbox input {
    accent-color: var(--accent);
  }

  .advanced {
    border-top: 1px solid var(--border);
    padding-top: var(--space-3);
  }

  .advanced summary {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    list-style: none;
    cursor: pointer;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-muted);
  }

  .advanced summary::-webkit-details-marker {
    display: none;
  }

  .advanced summary:hover {
    color: var(--text);
  }

  .chevron {
    display: flex;
    transition: transform var(--dur) var(--ease);
  }

  .advanced[open] .chevron {
    transform: rotate(90deg);
  }

  .advanced-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding-top: var(--space-4);
  }

  .estimate {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding-top: var(--space-3);
    border-top: 1px solid var(--border);
  }

  .line {
    font-size: 12px;
    color: var(--text-muted);
  }

  .estimate[data-state="ok"] .line:first-child {
    color: var(--text);
  }

  .warn-text {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--warn);
    font-weight: 600;
  }
</style>
