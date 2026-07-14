<!-- Задача C: сегмент-контрол (используется для разрешения/fps/кодека/контейнера). -->
<script lang="ts" generics="T extends string | number | null">
  interface Option {
    value: T;
    label: string;
  }
  interface Props {
    options: Option[];
    value: T;
    onchange: (v: T) => void;
    ariaLabel?: string;
  }
  let { options, value, onchange, ariaLabel }: Props = $props();
</script>

<div class="toggle" role="group" aria-label={ariaLabel}>
  {#each options as opt (String(opt.value))}
    <button
      type="button"
      class="toggle-btn"
      class:active={opt.value === value}
      aria-pressed={opt.value === value}
      onclick={() => onchange(opt.value)}
    >
      {opt.label}
    </button>
  {/each}
</div>

<style>
  .toggle {
    display: flex;
    gap: 4px;
    padding: 4px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }

  .toggle-btn {
    flex: 1;
    padding: 8px 12px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    border-radius: var(--radius-sm);
    font-size: 13px;
    font-weight: 600;
    transition:
      background var(--dur-fast) var(--ease),
      color var(--dur-fast) var(--ease);
  }

  .toggle-btn:hover:not(.active) {
    color: var(--text);
    background: var(--surface-2);
  }

  .toggle-btn.active {
    background: linear-gradient(135deg, var(--accent), var(--accent-2));
    color: #0b0c10;
  }
</style>
