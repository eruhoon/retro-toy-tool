<script lang="ts">
  import type { DeviceProfile } from '../types';
  import { Settings, RefreshCw, Save, Sparkles, Loader2, Trash2, Globe } from 'lucide-svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';

  let {
    activeDevice = $bindable(null),
    profiles = [],
    selectedSystemId = null,
    isDirty = false,
    isSaving = false,
    isLoading = false,
    onOpenSettings = () => {},
    onOpenDeviceModal = () => {},
    onDeviceSelect = () => {},
    onRefresh = () => {},
    onSaveGamelist = () => {},
    onBatchRegister = () => {},
    onBatchCleanMissing = () => {},
    onOpenScraperSettings = () => {},
  } = $props<{
    activeDevice: DeviceProfile | null;
    profiles: DeviceProfile[];
    selectedSystemId: string | null;
    isDirty: boolean;
    isSaving: boolean;
    isLoading: boolean;
    onOpenSettings?: () => void;
    onOpenDeviceModal?: () => void;
    onDeviceSelect: (id: string) => void;
    onRefresh: () => void;
    onSaveGamelist: () => void;
    onBatchRegister: () => void;
    onBatchCleanMissing?: () => void;
    onOpenScraperSettings?: () => void;
  }>();

  $effect(() => {
    if (activeDevice?.name) {
      const title = `Toy Manager - 오래가자 내 ${activeDevice.name}야~`;
      document.title = title;
      try {
        getCurrentWindow().setTitle(title).catch(() => {});
      } catch {
        // ignore
      }
    } else {
      const title = 'Toy Manager';
      document.title = title;
      try {
        getCurrentWindow().setTitle(title).catch(() => {});
      } catch {
        // ignore
      }
    }
  });

</script>

<header class="app-header">
  <div class="brand">
    <div class="logo-icon">
      <img src="/favicon.png" alt="Toy Manager" class="brand-logo-img" />
    </div>
    <div class="logo-text">
      {#if activeDevice?.name}
        <h1 title={`오래가자 내 ${activeDevice.name}야~`}>
          오래가자 내 {activeDevice.name}야~
        </h1>
      {:else}
        <h1 title="Toy Manager">
          Toy Manager
        </h1>
      {/if}
      <span class="version-tag">ES-DE</span>
    </div>
  </div>

  <div class="device-switcher">
    {#if profiles.length > 0}
      <select
        class="device-select"
        value={activeDevice?.id || ''}
        onchange={(e) => onDeviceSelect((e.target as HTMLSelectElement).value)}
      >
        <option value="">선택 안 함</option>
        {#each profiles as p}
          <option value={p.id}>
            {p.name} ({p.host})
          </option>
        {/each}
      </select>
    {/if}

    <button class="btn-secondary device-manage-btn" onclick={onOpenSettings} title="게임기 연결 및 스크래퍼 설정">
      <Settings size={15} />
      <span>설정</span>
    </button>


    {#if activeDevice}
      <div class="status-pill connected" title={activeDevice.roms_path}>
        <span class="status-dot"></span>
        <span class="status-text">{activeDevice.host}</span>
      </div>
    {:else}
      <div class="status-pill disconnected">
        <span class="status-dot"></span>
        <span class="status-text">미연결</span>
      </div>
    {/if}
  </div>

  <div class="header-actions">
    {#if activeDevice && selectedSystemId}
      <button
        class="btn-secondary action-btn"
        disabled={isLoading || isSaving}
        title="기기 데이터 새로고침"
        onclick={onRefresh}
      >
        <RefreshCw size={15} class={isLoading ? 'spin' : ''} />
        <span>새로고침</span>
      </button>

      <button
        class="btn-secondary action-btn sparkles-btn"
        disabled={isLoading || isSaving}
        title="현재 기종의 미등록 ROM들을 파일명 기반으로 기본 메타데이터 등록"
        onclick={onBatchRegister}
      >
        <Sparkles size={15} />
        <span>미등록 ROM 일괄 등록</span>
      </button>

      <button
        class="btn-secondary action-btn clean-btn"
        disabled={isLoading || isSaving}
        title="실제 파일이 없는 미아(누락) 메타데이터 엔트리들을 일괄 제거"
        onclick={onBatchCleanMissing}
      >
        <Trash2 size={14} />
        <span>누락 엔트리 정리</span>
      </button>

      <button
        class="btn-primary action-btn save-btn"
        class:highlight={isDirty}
        disabled={isLoading || isSaving}
        title="gamelist.xml을 게임기에 원격 저장 (자동 백업 .bak 생성)"
        onclick={onSaveGamelist}
      >
        {#if isSaving}
          <Loader2 size={15} class="spin" />
          <span>저장 중...</span>
        {:else}
          <Save size={15} />
          <span>gamelist.xml 저장{#if isDirty}*{/if}</span>
        {/if}
      </button>
    {/if}
  </div>
</header>

<style lang="scss">
  @use '../../styles/variables' as *;

  .app-header {
    height: 52px;
    background: $bg-secondary;
    border-bottom: 1px solid $border-color;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 16px;
    flex-shrink: 0;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;

    .logo-icon {
      width: 32px;
      height: 32px;
      border-radius: $radius-sm;
      display: flex;
      align-items: center;
      justify-content: center;
      box-shadow: 0 2px 8px $accent-glow;
      overflow: hidden;

      .brand-logo-img {
        width: 100%;
        height: 100%;
        object-fit: cover;
      }
    }

    .logo-text {
      display: flex;
      align-items: center;
      gap: 6px;

      h1 {
        font-size: 15px;
        font-weight: 700;
        letter-spacing: -0.3px;
        white-space: nowrap;
        max-width: 420px;
        overflow: hidden;
        text-overflow: ellipsis;
      }

      .version-tag {
        font-size: 10px;
        font-weight: 700;
        background: rgba(99, 102, 241, 0.15);
        color: #818cf8;
        padding: 1px 5px;
        border-radius: $radius-sm;
      }
    }
  }

  .device-switcher {
    display: flex;
    align-items: center;
    gap: 8px;

    .device-select {
      max-width: 220px;
      padding: 6px 10px;
      font-size: 12px;
      cursor: pointer;
    }

    .device-manage-btn {
      padding: 6px 10px;
      font-size: 12px;
    }

    .status-pill {
      display: flex;
      align-items: center;
      gap: 6px;
      padding: 4px 10px;
      border-radius: $radius-full;
      font-size: 11px;
      font-weight: 500;

      .status-dot {
        width: 6px;
        height: 6px;
        border-radius: 50%;
      }

      &.connected {
        background: rgba(16, 185, 129, 0.1);
        color: #34d399;
        border: 1px solid rgba(16, 185, 129, 0.25);
        .status-dot {
          background: #10b981;
          box-shadow: 0 0 6px #10b981;
        }
      }

      &.disconnected {
        background: rgba(239, 68, 68, 0.1);
        color: #f87171;
        border: 1px solid rgba(239, 68, 68, 0.25);
        .status-dot {
          background: #ef4444;
        }
      }
    }
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 8px;

    .action-btn {
      padding: 6px 12px;
      font-size: 12px;
    }

    .sparkles-btn {
      border-color: rgba(245, 158, 11, 0.4);
      color: #fbbf24;
      &:hover {
        background: rgba(245, 158, 11, 0.15);
      }
    }

    .clean-btn {
      border-color: rgba(239, 68, 68, 0.35);
      color: #f87171;
      &:hover {
        background: rgba(239, 68, 68, 0.15);
      }
    }

    .save-btn {
      &.highlight {
        animation: pulse-glow 2s infinite;
      }
    }
  }

  @keyframes pulse-glow {
    0%, 100% {
      box-shadow: 0 0 0 rgba(99, 102, 241, 0);
    }
    50% {
      box-shadow: 0 0 12px $accent-color;
    }
  }
</style>
