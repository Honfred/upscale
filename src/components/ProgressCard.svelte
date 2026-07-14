<!-- Задача C: processing-экран — общий прогресс, стадии, отмена. -->
<script lang="ts">
  import { app } from "../lib/stores.svelte";
  import { basename, formatEta, formatFps } from "../lib/format";
  import StageBar from "./StageBar.svelte";
  import Banner from "./Banner.svelte";
  import Icon from "./Icon.svelte";

  let confirmingCancel = $state(false);

  function onCancelClick() {
    if (confirmingCancel) {
      confirmingCancel = false;
      void app.requestCancel();
    } else {
      confirmingCancel = true;
    }
  }

  let percent = $derived(Math.round(app.progress.overallProgress * 100));
</script>

<div class="progress-card">
  <div class="head">
    <span class="filename selectable">{app.source ? basename(app.source.path) : ""}</span>
    <span class="percent tabular-nums">{percent}%</span>
  </div>

  <div class="bar-track">
    <div class="bar-fill" style={`width: ${percent}%`}></div>
  </div>

  <p class="meta tabular-nums">
    {#if app.progress.totalSegments > 0}
      Сегмент {app.progress.segmentIndex + 1}/{app.progress.totalSegments}
      · ETA {formatEta(app.progress.etaSeconds)}
      · {formatFps(app.progress.fpsNow)}
    {:else}
      Подготовка…
    {/if}
  </p>

  {#if app.progress.warning}
    <Banner kind="warning" message={app.progress.warning} />
  {/if}

  <StageBar
    stage={app.progress.stage}
    completed={app.progress.completedStages}
    showInterpolate={app.settings.targetFps !== null}
  />

  <div class="actions">
    <button class="btn ghost danger" onclick={onCancelClick}>
      <Icon name="close" size={15} />
      {confirmingCancel ? "Точно отменить?" : "Отменить"}
    </button>
  </div>
</div>

<style>
  .progress-card {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-5);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }

  .head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-3);
  }

  .filename {
    font-size: 14px;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .percent {
    font-size: 40px;
    font-weight: 600;
    line-height: 1;
    flex-shrink: 0;
  }

  .bar-track {
    height: 6px;
    border-radius: 999px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    overflow: hidden;
  }

  .bar-fill {
    height: 100%;
    border-radius: inherit;
    background: linear-gradient(90deg, var(--accent), var(--accent-2));
    transition: width 200ms ease;
  }

  .meta {
    font-size: 13px;
    color: var(--text-muted);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    margin-top: var(--space-2);
  }
</style>
