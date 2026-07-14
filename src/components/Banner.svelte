<!-- Задача C: универсальный инлайн-баннер (предупреждение/ошибка/инфо). -->
<script lang="ts">
  import Icon from "./Icon.svelte";

  interface Props {
    kind?: "warning" | "error" | "info";
    message: string;
  }
  let { kind = "warning", message }: Props = $props();
</script>

<div class="banner" data-kind={kind} role={kind === "error" ? "alert" : "status"}>
  <Icon name="warning" size={15} />
  <span class="selectable">{message}</span>
</div>

<style>
  .banner {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 10px 14px;
    border-radius: var(--radius-md);
    font-size: 13px;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text-muted);
  }

  .banner[data-kind="warning"] {
    color: var(--warn);
    border-color: rgba(var(--warn-rgb), 0.35);
    background: rgba(var(--warn-rgb), 0.08);
  }

  .banner[data-kind="error"] {
    color: var(--err);
    border-color: rgba(var(--err-rgb), 0.35);
    background: rgba(var(--err-rgb), 0.08);
  }
</style>
