<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';

  type SleepStatus = 'enabled' | 'disabled';
  type ChangeOutcome = 'applied' | 'cancelled';
  type ErrorKind = 'sync' | 'update';

  let sleepStatus = $state<SleepStatus | null>(null);
  let errorMessage = $state<string | null>(null);
  let errorKind = $state<ErrorKind | null>(null);
  let isSyncing = $state(false);
  let isChanging = $state(false);
  let refreshPromise: Promise<void> | null = null;
  let previewStatus: SleepStatus = 'enabled';

  const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  function applyStatus(status: SleepStatus, preserveUpdateError = false) {
    sleepStatus = status;
    if (!preserveUpdateError || errorKind !== 'update') {
      errorMessage = null;
      errorKind = null;
    }
  }

  function applySyncError(message: string) {
    errorMessage = message;
    errorKind = 'sync';
  }

  function applyUpdateError(message: string) {
    errorMessage = message;
    errorKind = 'update';
  }

  function refreshState(preserveUpdateError = false): Promise<void> {
    if (refreshPromise) return refreshPromise;

    refreshPromise = (async () => {
      isSyncing = true;
      try {
        if (import.meta.env.DEV && !isTauri) {
          await new Promise((resolve) => setTimeout(resolve, 120));
          applyStatus(previewStatus, preserveUpdateError);
        } else {
          const status = await invoke<SleepStatus>('get_sleep_state');
          applyStatus(status, preserveUpdateError);
        }
      } catch (error) {
        applySyncError(String(error));
      } finally {
        isSyncing = false;
        refreshPromise = null;
      }
    })();

    return refreshPromise;
  }

  async function toggleSleepState() {
    if (isChanging) return;
    if (!sleepStatus) {
      await refreshState();
      return;
    }

    const disabled = sleepStatus === 'enabled';
    isChanging = true;
    errorMessage = null;
    errorKind = null;

    let preserveUpdateError = false;
    try {
      if (import.meta.env.DEV && !isTauri) {
        await new Promise((resolve) => setTimeout(resolve, 300));
        previewStatus = disabled ? 'disabled' : 'enabled';
      } else {
        await invoke<ChangeOutcome>('set_sleep_disabled', { disabled });
      }
    } catch (error) {
      applyUpdateError(String(error));
      preserveUpdateError = true;
    } finally {
      await refreshState(preserveUpdateError);
      isChanging = false;
    }
  }

  const buttonLabel = $derived.by(() => {
    if (!sleepStatus) return errorMessage ? 'RETRY' : '…';
    return sleepStatus === 'disabled' ? 'ON' : 'OFF';
  });

  const headerStatus = $derived.by(() => {
    if (isChanging) return 'UPDATING…';
    if (!sleepStatus && isSyncing) return 'CHECKING…';
    if (errorKind === 'update') return 'UPDATE FAILED';
    if (errorKind === 'sync' && sleepStatus) return 'SYNC FAILED';
    if (errorMessage) return 'ERROR';
    return null;
  });

  const actionLabel = $derived.by(() => {
    if (!sleepStatus) return errorMessage ? '状態取得を再試行' : '状態を確認中';
    return sleepStatus === 'disabled' ? 'Lid Awakeをオフにする' : 'Lid Awakeをオンにする';
  });

  onMount(() => {
    void refreshState();
    if (!isTauri) return;

    const unlistenUpdated = listen<SleepStatus>('sleep-state-updated', (event) => {
      applyStatus(event.payload, errorKind === 'update');
    });
    const unlistenError = listen<string>('sleep-state-error', (event) => {
      applySyncError(event.payload);
    });

    return () => {
      void unlistenUpdated.then((unlisten) => unlisten());
      void unlistenError.then((unlisten) => unlisten());
    };
  });

  function handleDragStart(event: MouseEvent) {
    if (event.button !== 0 || !isTauri) return;
    event.preventDefault();
    void getCurrentWindow().startDragging();
  }
</script>

<main class="widget">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <header class="widget-header" onmousedown={handleDragStart}>
    <div class="widget-title">
      <span
        class="status-dot"
        class:active={sleepStatus === 'disabled' && !errorMessage}
        class:error={Boolean(errorMessage)}
        class:checking={!sleepStatus && !errorMessage}
      ></span>
      LID AWAKE
    </div>
    {#if headerStatus}
      <div
        class="header-status"
        class:error={Boolean(errorMessage) && !isChanging && !isSyncing}
        aria-live="polite"
      >
        {#if isChanging || isSyncing}
          <span class="header-spinner" aria-hidden="true"></span>
        {/if}
        {headerStatus}
      </div>
    {/if}
  </header>

  <section class="content-area" aria-live="polite">
    <button
      type="button"
      class="state-button"
      class:on={sleepStatus === 'disabled'}
      class:off={sleepStatus === 'enabled'}
      class:unknown={!sleepStatus}
      class:error={Boolean(errorMessage)}
      disabled={isChanging || (!sleepStatus && isSyncing)}
      aria-pressed={sleepStatus === 'disabled'}
      aria-busy={isChanging || isSyncing}
      aria-label={actionLabel}
      aria-describedby={errorMessage ? 'status-detail' : undefined}
      title={errorMessage ?? actionLabel}
      onclick={toggleSleepState}
    >
      <span class="toggle-track" aria-hidden="true">
        <span class="toggle-knob"></span>
      </span>
      <span class="toggle-label">{buttonLabel}</span>
    </button>
    {#if errorMessage}
      <span class="visually-hidden" id="status-detail">{errorMessage}</span>
    {/if}
  </section>
</main>
