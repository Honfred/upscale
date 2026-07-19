<!-- Задача C: корневой компонент — стейт-машина экранов (idle -> selected -> processing -> result/error). -->
<script lang="ts">
  import { onMount } from "svelte";
  import { fly } from "svelte/transition";
  import { app } from "./lib/stores.svelte";
  import DropZone from "./components/DropZone.svelte";
  import SourceInfo from "./components/SourceInfo.svelte";
  import SettingsPanel from "./components/SettingsPanel.svelte";
  import ProgressCard from "./components/ProgressCard.svelte";
  import ResultView from "./components/ResultView.svelte";
  import ErrorView from "./components/ErrorView.svelte";
  import Banner from "./components/Banner.svelte";
  import Icon from "./components/Icon.svelte";

  const APP_VERSION = "0.1.4";

  onMount(() => {
    void app.checkSystem();
    void app.startJobEventsSubscription();
    return () => app.stopJobEventsSubscription();
  });
</script>

<div class="app-shell">
  <header class="app-header">
    <span class="logo">AnimeUpscale</span>
    <span class="version">v{APP_VERSION}</span>
  </header>

  {#if app.systemWarning}
    <div class="banner-slot">
      <Banner kind="warning" message="Компоненты не найдены — переустановите приложение." />
    </div>
  {/if}

  <main class="app-main">
    {#if app.screen === "idle"}
      <div class="screen" in:fly={{ y: 8, duration: 200 }}>
        <DropZone />
      </div>
    {:else if app.screen === "selected"}
      <div class="screen" in:fly={{ y: 8, duration: 200 }}>
        <SourceInfo />
        <SettingsPanel />
        <div class="selected-actions">
          <button class="btn ghost" onclick={() => app.resetToIdle()}>← другой файл</button>
          <button class="btn primary block" disabled={!app.canStart} onclick={() => app.beginJob()}>
            <Icon name="play" size={15} />
            Начать обработку
          </button>
        </div>
      </div>
    {:else if app.screen === "processing"}
      <div class="screen" in:fly={{ y: 8, duration: 200 }}>
        <ProgressCard />
      </div>
    {:else if app.screen === "result"}
      <div class="screen" in:fly={{ y: 8, duration: 200 }}>
        <ResultView />
      </div>
    {:else}
      <div class="screen" in:fly={{ y: 8, duration: 200 }}>
        <ErrorView />
      </div>
    {/if}
  </main>
</div>

<style>
  .app-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  .app-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-4) var(--space-6);
    flex-shrink: 0;
  }

  .logo {
    font-size: 15px;
    color: var(--text-muted);
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .version {
    font-size: 12px;
    color: var(--text-muted);
  }

  .banner-slot {
    padding: 0 var(--space-6) var(--space-3);
    flex-shrink: 0;
  }

  .app-main {
    flex: 1;
    display: flex;
    justify-content: center;
    overflow-y: auto;
  }

  .screen {
    width: 100%;
    max-width: 640px;
    padding: var(--space-3) var(--space-6) var(--space-6);
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }

  .selected-actions {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .selected-actions .btn.block {
    flex: 1;
  }
</style>
