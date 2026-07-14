<!-- Задача C: набор маленьких инлайн-SVG иконок (stroke, без заливки, без emoji). -->
<script lang="ts">
  interface Props {
    name: "upload" | "check" | "folder" | "close" | "play" | "warning" | "arrow";
    size?: number;
  }
  let { name, size = 18 }: Props = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.75"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  class="icon"
>
  {#if name === "upload"}
    <path d="M12 15.5V4M12 4l-4.5 4.5M12 4l4.5 4.5" />
    <path d="M4.5 15.5v3a1.5 1.5 0 0 0 1.5 1.5h12a1.5 1.5 0 0 0 1.5-1.5v-3" />
  {:else if name === "check"}
    <path d="M4 12.5l5 5L20 6.5" />
  {:else if name === "folder"}
    <path
      d="M3.5 7.25a1 1 0 0 1 1-1h4.75l1.75 2h8.5a1 1 0 0 1 1 1v8.5a1 1 0 0 1-1 1h-15a1 1 0 0 1-1-1v-10.5z"
    />
  {:else if name === "close"}
    <path d="M5.5 5.5l13 13M18.5 5.5l-13 13" />
  {:else if name === "play"}
    <path d="M6.5 4.75v14.5l12.5-7.25L6.5 4.75z" />
  {:else if name === "warning"}
    <path d="M12 3.25l9.25 16.5H2.75L12 3.25z" />
    <path d="M12 10v4" />
    <path d="M12 17.25h0.01" />
  {:else if name === "arrow"}
    <path d="M4 12h14.5M13 6.5l6 5.5-6 5.5" />
  {/if}
</svg>

<style>
  .icon {
    flex-shrink: 0;
    display: block;
  }
</style>
