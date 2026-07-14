<!-- Задача C: горизонтальная шкала стадий пайплайна (декодирование/апскейл/интерполяция/кодирование). -->
<script lang="ts">
  import { STAGE_LABELS, STAGE_ORDER } from "../lib/stores.svelte";
  import type { Stage } from "../lib/types";
  import Icon from "./Icon.svelte";

  interface Props {
    stage: Stage | null;
    completed: Stage[];
    showInterpolate: boolean;
  }
  let { stage, completed, showInterpolate }: Props = $props();

  let stages = $derived(STAGE_ORDER.filter((s) => showInterpolate || s !== "interpolate"));

  function statusOf(s: Stage): "done" | "active" | "pending" {
    if (completed.includes(s)) return "done";
    if (stage === s) return "active";
    return "pending";
  }
</script>

<ol class="stage-bar">
  {#each stages as s (s)}
    {@const status = statusOf(s)}
    <li class="stage" data-status={status}>
      <span class="dot">
        {#if status === "done"}
          <Icon name="check" size={11} />
        {/if}
      </span>
      <span class="label">{STAGE_LABELS[s]}</span>
    </li>
  {/each}
</ol>

<style>
  .stage-bar {
    display: flex;
    list-style: none;
    gap: var(--space-2);
  }

  .stage {
    flex: 1;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 10px 12px;
    border-radius: var(--radius-md);
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 600;
    min-width: 0;
    transition:
      border-color var(--dur) var(--ease),
      color var(--dur) var(--ease),
      background var(--dur) var(--ease);
  }

  .stage[data-status="active"] {
    border-color: var(--accent);
    color: var(--text);
    background: rgba(var(--accent-rgb), 0.08);
  }

  .stage[data-status="done"] {
    color: var(--ok);
    border-color: rgba(var(--ok-rgb), 0.4);
  }

  .dot {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 1.5px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    color: inherit;
  }

  .stage[data-status="active"] .dot {
    border-color: var(--accent);
    animation: blink 1.4s ease-in-out infinite;
  }

  .stage[data-status="done"] .dot {
    border-color: currentColor;
  }

  @keyframes blink {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.35;
    }
  }

  .label {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
