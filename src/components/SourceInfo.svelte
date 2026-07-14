<!-- Задача C: карточка с информацией об исходном файле и целевых параметрах. -->
<script lang="ts">
  import { app } from "../lib/stores.svelte";
  import { basename, formatDuration, formatFps } from "../lib/format";
  import Icon from "./Icon.svelte";

  let targetWidth = $derived(app.estimate?.outWidth ?? app.settings.targetWidth);
  let targetHeight = $derived(
    app.estimate?.outHeight ??
      (app.source ? Math.round((app.source.height * app.settings.targetWidth) / app.source.width) : 0),
  );
</script>

{#if app.source}
  <div class="source-info">
    <p class="filename selectable" title={app.source.path}>{basename(app.source.path)}</p>
    <div class="rows">
      <div class="row">
        <span class="key">Разрешение</span>
        <span class="value tabular-nums">
          {app.source.width}×{app.source.height}
          <Icon name="arrow" size={13} />
          {targetWidth}×{targetHeight}
        </span>
      </div>
      <div class="row">
        <span class="key">Частота кадров</span>
        <span class="value tabular-nums">
          {formatFps(app.source.fps)}
          <Icon name="arrow" size={13} />
          {app.settings.targetFps !== null ? formatFps(app.settings.targetFps) : "оригинал"}
        </span>
      </div>
      <div class="row">
        <span class="key">Длительность</span>
        <span class="value tabular-nums">{formatDuration(app.source.durationSec)}</span>
      </div>
      <div class="row">
        <span class="key">Кодек</span>
        <span class="value">{app.source.codecName.toUpperCase()}</span>
      </div>
    </div>
  </div>
{/if}

<style>
  .source-info {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-4) var(--space-5);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }

  .filename {
    font-size: 15px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .rows {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 13px;
  }

  .key {
    color: var(--text-muted);
  }

  .value {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--text);
  }

  .value :global(.icon) {
    color: var(--text-muted);
  }
</style>
