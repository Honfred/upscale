<!-- Задача C: экран результата — успешное завершение обработки. -->
<script lang="ts">
  import { app } from "../lib/stores.svelte";
  import { basename, formatBytes, formatDuration } from "../lib/format";
  import Icon from "./Icon.svelte";
</script>

<div class="result">
  <div class="badge">
    <Icon name="check" size={30} />
  </div>
  <h2>Готово</h2>

  {#if app.result}
    <p class="path selectable" title={app.result.outputPath}>{basename(app.result.outputPath)}</p>
    <div class="stats">
      <div class="stat">
        <span class="key">Размер</span>
        <span class="value tabular-nums">{formatBytes(app.result.outputBytes)}</span>
      </div>
      <div class="stat">
        <span class="key">Время обработки</span>
        <span class="value tabular-nums">{formatDuration(app.result.elapsedSec)}</span>
      </div>
    </div>
  {/if}

  <div class="actions">
    <button class="btn ghost" onclick={() => app.revealOutput()}>
      <Icon name="folder" size={15} />
      Открыть папку
    </button>
    <button class="btn primary" onclick={() => app.resetToIdle()}>Новое видео</button>
  </div>
</div>

<style>
  .result {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: var(--space-3);
    padding: var(--space-6) var(--space-5);
  }

  .badge {
    width: 64px;
    height: 64px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--bg);
    background: linear-gradient(135deg, var(--accent), var(--accent-2));
    margin-bottom: var(--space-2);
  }

  .path {
    font-size: 13px;
    color: var(--text-muted);
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .stats {
    display: flex;
    gap: var(--space-6);
    margin-top: var(--space-3);
  }

  .stat {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .key {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
  }

  .value {
    font-size: 18px;
    font-weight: 600;
  }

  .actions {
    display: flex;
    gap: var(--space-3);
    margin-top: var(--space-5);
  }
</style>
