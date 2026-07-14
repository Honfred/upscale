<!-- Задача C: idle-экран — drag&drop + выбор файла через диалог. -->
<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { open } from "@tauri-apps/plugin-dialog";
  import { app, VIDEO_EXTENSIONS } from "../lib/stores.svelte";
  import Icon from "./Icon.svelte";
  import Banner from "./Banner.svelte";

  let unlisten: (() => void) | undefined;

  function hasAllowedExt(path: string): boolean {
    const dot = path.lastIndexOf(".");
    if (dot === -1) return false;
    return VIDEO_EXTENSIONS.includes(path.slice(dot + 1).toLowerCase());
  }

  async function pickFile() {
    const path = await open({
      title: "Выберите видеофайл",
      multiple: false,
      directory: false,
      filters: [{ name: "Видео", extensions: VIDEO_EXTENSIONS }],
    });
    if (typeof path === "string") {
      await app.selectSource(path);
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      void pickFile();
    }
  }

  onMount(() => {
    let cancelled = false;
    getCurrentWebview()
      .onDragDropEvent((event) => {
        const p = event.payload;
        if (p.type === "enter" || p.type === "over") {
          app.setDropActive(true);
        } else if (p.type === "leave") {
          app.setDropActive(false);
        } else if (p.type === "drop") {
          app.setDropActive(false);
          const path = p.paths[0];
          if (!path) return;
          if (!hasAllowedExt(path)) {
            app.setIdleError(
              "Неподдерживаемый формат файла. Ожидается видео: " +
                VIDEO_EXTENSIONS.join(", ").toUpperCase() +
                ".",
            );
            return;
          }
          void app.selectSource(path);
        }
      })
      .then((fn) => {
        if (cancelled) {
          fn();
        } else {
          unlisten = fn;
        }
      });
    return () => {
      cancelled = true;
    };
  });

  onDestroy(() => {
    unlisten?.();
  });
</script>

<div class="dropzone-wrap">
  <div
    class="dropzone"
    class:active={app.dropActive}
    role="button"
    tabindex="0"
    aria-label="Выбрать видеофайл или перетащить сюда"
    onclick={pickFile}
    onkeydown={onKeydown}
    ondragover={(e) => e.preventDefault()}
    ondrop={(e) => e.preventDefault()}
  >
    <div class="icon"><Icon name="upload" size={30} /></div>
    <p class="title">Перетащите видеофайл сюда</p>
    <p class="hint">или нажмите, чтобы выбрать файл</p>
    <p class="formats">{VIDEO_EXTENSIONS.join(" · ").toUpperCase()}</p>
  </div>

  {#if app.idleError}
    <Banner kind="error" message={app.idleError} />
  {/if}
</div>

<style>
  .dropzone-wrap {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .dropzone {
    border: 1.5px dashed var(--border);
    border-radius: var(--radius-lg);
    padding: 72px 24px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    text-align: center;
    cursor: pointer;
    color: var(--text-muted);
    transition:
      border-color var(--dur) var(--ease),
      background var(--dur) var(--ease),
      color var(--dur) var(--ease);
  }

  .dropzone:hover {
    border-color: var(--accent);
    color: var(--text);
  }

  .dropzone.active {
    border-color: var(--accent);
    border-style: solid;
    background: rgba(var(--accent-rgb), 0.08);
    color: var(--text);
    animation: pulse 1.1s ease-in-out infinite;
  }

  .icon {
    color: var(--accent);
    margin-bottom: var(--space-2);
  }

  .title {
    font-size: 16px;
    font-weight: 600;
    color: var(--text);
  }

  .hint {
    font-size: 13px;
  }

  .formats {
    font-size: 11px;
    letter-spacing: 0.04em;
    color: var(--text-muted);
    margin-top: var(--space-2);
  }

  @keyframes pulse {
    0%,
    100% {
      box-shadow: 0 0 0 0 rgba(var(--accent-rgb), 0.35);
    }
    50% {
      box-shadow: 0 0 0 8px rgba(var(--accent-rgb), 0);
    }
  }
</style>
