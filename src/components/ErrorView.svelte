<!-- Задача C: экран ошибки — сообщение, назад/новое видео. -->
<script lang="ts">
  import { app, STAGE_LABELS } from "../lib/stores.svelte";
  import Icon from "./Icon.svelte";
</script>

<div class="error-view">
  <div class="badge">
    <Icon name="warning" size={26} />
  </div>
  <h2>Не удалось выполнить обработку</h2>

  <p class="message selectable">{app.jobError?.message}</p>
  {#if app.jobError?.stage}
    <p class="stage-hint">Этап: {STAGE_LABELS[app.jobError.stage]}</p>
  {/if}

  <div class="actions">
    {#if app.jobError?.recoverable && app.source}
      <button class="btn ghost" onclick={() => app.errorBack()}>← Назад</button>
    {/if}
    <button class="btn primary" onclick={() => app.resetToIdle()}>Новое видео</button>
  </div>
</div>

<style>
  .error-view {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: var(--space-3);
    padding: var(--space-6) var(--space-5);
  }

  .badge {
    width: 56px;
    height: 56px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--err);
    border: 1.5px solid rgba(var(--err-rgb), 0.4);
    background: rgba(var(--err-rgb), 0.08);
    margin-bottom: var(--space-2);
  }

  .message {
    font-size: 13px;
    color: var(--text-muted);
    max-width: 480px;
  }

  .stage-hint {
    font-size: 12px;
    color: var(--text-muted);
  }

  .actions {
    display: flex;
    gap: var(--space-3);
    margin-top: var(--space-5);
  }
</style>
