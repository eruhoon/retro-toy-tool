<script lang="ts">
  import { Download, X, ArrowUpCircle, Loader2, CheckCircle } from 'lucide-svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';

  let {
    isOpen = $bindable(false),
    version = '',
    body = null,
    currentVersion = '',
    onDismiss = () => {},
  } = $props<{
    isOpen: boolean;
    version: string;
    body: string | null;
    currentVersion: string;
    onDismiss?: () => void;
  }>();

  let isDownloading = $state(false);
  let isFinished = $state(false);
  let downloadedBytes = $state(0);
  let totalBytes = $state<number | null>(null);
  let errorMessage = $state<string | null>(null);

  let progressPercent = $derived(
    totalBytes && totalBytes > 0
      ? Math.round((downloadedBytes / totalBytes) * 100)
      : null
  );

  let unlistenProgress: (() => void) | null = null;
  let unlistenFinished: (() => void) | null = null;

  onMount(async () => {
    unlistenProgress = await listen<{ chunkLength: number; contentLength: number | null }>(
      'update-progress',
      (event) => {
        downloadedBytes += event.payload.chunkLength;
        if (event.payload.contentLength) {
          totalBytes = event.payload.contentLength;
        }
      }
    );
    unlistenFinished = await listen('update-finished', () => {
      isFinished = true;
    });
  });

  onDestroy(() => {
    unlistenProgress?.();
    unlistenFinished?.();
  });

  async function handleInstall() {
    isDownloading = true;
    errorMessage = null;
    downloadedBytes = 0;
    totalBytes = null;
    try {
      await invoke('install_update');
    } catch (e: any) {
      errorMessage = String(e?.message || e);
      isDownloading = false;
    }
  }

  function handleDismiss() {
    if (isDownloading) return;
    isOpen = false;
    onDismiss();
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }
</script>

{#if isOpen}
  <div class="updater-overlay" role="dialog" aria-modal="true" aria-label="업데이트 알림">
    <div class="updater-modal">
      <div class="modal-header">
        <div class="title-row">
          <ArrowUpCircle size={20} class="update-icon" />
          <h2>새 버전이 있습니다!</h2>
        </div>
        {#if !isDownloading}
          <button class="close-btn" onclick={handleDismiss} title="나중에">
            <X size={16} />
          </button>
        {/if}
      </div>

      <div class="version-row">
        <div class="ver-badge current">
          <span class="ver-label">현재</span>
          <span class="ver-num">v{currentVersion}</span>
        </div>
        <span class="arrow">→</span>
        <div class="ver-badge new">
          <span class="ver-label">최신</span>
          <span class="ver-num">v{version}</span>
        </div>
      </div>

      {#if body}
        <div class="release-notes">
          <p class="notes-label">릴리즈 노트</p>
          <pre class="notes-body">{body}</pre>
        </div>
      {/if}

      {#if errorMessage}
        <div class="error-msg">
          <span>⚠ 업데이트 실패: {errorMessage}</span>
        </div>
      {/if}

      {#if isDownloading && !isFinished}
        <div class="progress-section">
          <div class="progress-bar-wrap">
            <div
              class="progress-bar-fill"
              style="width: {progressPercent !== null ? progressPercent + '%' : '0%'}"
              class:indeterminate={progressPercent === null}
            ></div>
          </div>
          <div class="progress-info">
            <Loader2 size={13} class="spin" />
            {#if progressPercent !== null}
              <span>{progressPercent}% — {formatBytes(downloadedBytes)}{totalBytes ? ` / ${formatBytes(totalBytes)}` : ''}</span>
            {:else}
              <span>다운로드 중... {formatBytes(downloadedBytes)}</span>
            {/if}
          </div>
        </div>
      {/if}

      {#if isFinished}
        <div class="finish-msg">
          <CheckCircle size={15} />
          <span>설치 완료! 앱이 재시작됩니다...</span>
        </div>
      {/if}

      <div class="modal-footer">
        {#if !isDownloading}
          <button class="btn-secondary" onclick={handleDismiss}>나중에</button>
          <button class="btn-primary install-btn" onclick={handleInstall}>
            <Download size={14} />
            지금 업데이트
          </button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style lang="scss">
  @use '../../styles/variables' as *;

  .updater-overlay {
    position: fixed;
    inset: 0;
    z-index: 99998;
    background: rgba(10, 14, 26, 0.7);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    animation: fade-in 0.2s ease;
  }

  .updater-modal {
    background: $bg-secondary;
    border: 1px solid rgba(99, 102, 241, 0.4);
    border-radius: $radius-lg;
    padding: 28px;
    width: 460px;
    max-width: 90vw;
    display: flex;
    flex-direction: column;
    gap: 18px;
    box-shadow: 0 24px 48px rgba(0, 0, 0, 0.6), 0 0 24px rgba(99, 102, 241, 0.15);
    animation: pop-in 0.22s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;

    .title-row {
      display: flex;
      align-items: center;
      gap: 10px;

      :global(.update-icon) {
        color: #818cf8;
      }

      h2 {
        font-size: 16px;
        font-weight: 700;
        color: $text-primary;
        margin: 0;
      }
    }

    .close-btn {
      background: none;
      border: none;
      color: $text-muted;
      cursor: pointer;
      padding: 4px;
      border-radius: $radius-sm;
      display: flex;
      align-items: center;
      &:hover { color: $text-primary; background: $bg-card; }
    }
  }

  .version-row {
    display: flex;
    align-items: center;
    gap: 12px;

    .ver-badge {
      display: flex;
      flex-direction: column;
      align-items: center;
      padding: 8px 18px;
      border-radius: $radius-md;
      gap: 2px;

      .ver-label {
        font-size: 10px;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.5px;
        opacity: 0.7;
      }

      .ver-num {
        font-size: 15px;
        font-weight: 700;
        font-family: monospace;
      }

      &.current {
        background: rgba(255, 255, 255, 0.04);
        border: 1px solid $border-color;
        color: $text-secondary;
      }

      &.new {
        background: rgba(99, 102, 241, 0.12);
        border: 1px solid rgba(99, 102, 241, 0.35);
        color: #818cf8;
      }
    }

    .arrow {
      font-size: 18px;
      color: $text-muted;
    }
  }

  .release-notes {
    max-height: 180px;
    overflow-y: auto;

    .notes-label {
      font-size: 11px;
      font-weight: 600;
      color: $text-muted;
      text-transform: uppercase;
      letter-spacing: 0.5px;
      margin-bottom: 8px;
    }

    .notes-body {
      font-size: 12.5px;
      color: $text-secondary;
      line-height: 1.6;
      white-space: pre-wrap;
      word-break: break-word;
      background: $bg-primary;
      border: 1px solid $border-color;
      border-radius: $radius-sm;
      padding: 10px 12px;
      margin: 0;
      font-family: inherit;
    }
  }

  .error-msg {
    font-size: 12.5px;
    color: #f87171;
    background: rgba(239, 68, 68, 0.08);
    border: 1px solid rgba(239, 68, 68, 0.3);
    border-radius: $radius-sm;
    padding: 10px 14px;
  }

  .progress-section {
    display: flex;
    flex-direction: column;
    gap: 8px;

    .progress-bar-wrap {
      height: 6px;
      background: rgba(255, 255, 255, 0.06);
      border-radius: 99px;
      overflow: hidden;

      .progress-bar-fill {
        height: 100%;
        background: linear-gradient(90deg, #6366f1, #a855f7);
        border-radius: 99px;
        transition: width 0.3s ease;

        &.indeterminate {
          width: 40% !important;
          animation: indeterminate-slide 1.4s ease-in-out infinite;
        }
      }
    }

    .progress-info {
      display: flex;
      align-items: center;
      gap: 6px;
      font-size: 12px;
      color: $text-muted;

      :global(.spin) {
        animation: spin 1s linear infinite;
      }
    }
  }

  .finish-msg {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: #34d399;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;

    .btn-secondary, .btn-primary {
      padding: 8px 18px;
      font-size: 13px;
    }

    .install-btn {
      display: flex;
      align-items: center;
      gap: 6px;
    }
  }

  @keyframes fade-in {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  @keyframes pop-in {
    from { opacity: 0; transform: scale(0.94) translateY(8px); }
    to { opacity: 1; transform: scale(1) translateY(0); }
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  @keyframes indeterminate-slide {
    0% { transform: translateX(-100%); }
    100% { transform: translateX(300%); }
  }
</style>
