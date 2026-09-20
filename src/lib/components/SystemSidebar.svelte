<script lang="ts">
  import type { StorageLocation, SystemPlatform } from '../types';
  import { Search, Folder, ImageOff, AlertCircle, Eye, EyeOff, Filter, HardDrive, Smartphone } from 'lucide-svelte';

  let {
    systems = [],
    selectedSystemId = $bindable(null),
    isLoading = false,
    storages = [],
    activeStoragePath = '',
    isOpen = true,
    onSelect = () => {},
    onSelectStorage = () => {},
    onToggle = () => {},
  } = $props<{
    systems: SystemPlatform[];
    selectedSystemId: string | null;
    isLoading: boolean;
    storages?: StorageLocation[];
    activeStoragePath?: string;
    isOpen?: boolean;
    onSelect: (systemId: string) => void;
    onSelectStorage?: (path: string) => void;
    onToggle?: () => void;
  }>();

  const IGNORED_SYSTEM_IDS = new Set([
    'bios', 'savestates', 'saves', 'themes', 'theme', 'downloads', 'download',
    'backups', 'backup', 'cheats', 'music', 'sound', 'sounds', 'overlays',
    'shaders', 'system', 'records', 'recordings', 'logs', 'decorations',
    'extra', 'configs', 'config', 'tools', 'package', 'packages', 'retroarch',
    'kodi', 'splash', 'lost+found', 'images', 'media', 'videos', 'covers',
    'screenshots', 'wheels', 'titles', 'fanart', 'manuals'
  ]);

  let searchQuery = $state('');
  let hideEmpty = $state(true); // Default to hiding empty platforms

  let validSystems = $derived(
    systems.filter((s: SystemPlatform) => !IGNORED_SYSTEM_IDS.has(s.id.toLowerCase()))
  );

  let emptyCount = $derived(validSystems.filter((s: SystemPlatform) => s.rom_count === 0 && s.metadata_count === 0).length);

  let filteredSystems = $derived(
    validSystems.filter((s: SystemPlatform) => {
      const matchesSearch =
        s.id.toLowerCase().includes(searchQuery.toLowerCase()) ||
        s.name.toLowerCase().includes(searchQuery.toLowerCase());

      if (!matchesSearch) return false;

      // Only hide empty platforms if user is NOT searching
      if (!searchQuery.trim() && hideEmpty && s.rom_count === 0 && s.metadata_count === 0) {
        return false;
      }

      return true;
    })
  );

  let totalRoms = $derived(validSystems.reduce((acc: number, s: SystemPlatform) => acc + s.rom_count, 0));
  let totalUnregistered = $derived(validSystems.reduce((acc: number, s: SystemPlatform) => acc + s.unregistered_count, 0));
  let nonEmptyCount = $derived(validSystems.filter((s: SystemPlatform) => s.rom_count > 0 || s.metadata_count > 0).length);
</script>

<aside class="sidebar" class:collapsed={!isOpen}>
  <div class="sidebar-header">
    {#if storages && storages.length > 1}
      <div class="storage-selector">
        <span class="storage-label">저장소:</span>
        <div class="storage-pills">
          {#each storages as st}
            <button
              class="storage-pill"
              class:active={activeStoragePath === st.path}
              onclick={() => onSelectStorage(st.path)}
              title={`${st.label}\n${st.path}`}
            >
              {#if st.storage_type === 'external'}
                <HardDrive size={12} />
              {:else}
                <Smartphone size={12} />
              {/if}
              <span>{st.storage_type === 'external' ? '외장 SD카드' : '내장 메모리'}</span>
            </button>
          {/each}
        </div>
      </div>
    {/if}

    <div class="search-row">
      <div class="search-box">
        <Search size={14} class="search-icon" />
        <input
          type="text"
          placeholder="기종 검색 (예: gba)..."
          bind:value={searchQuery}
        />
      </div>

      <button
        class="toggle-empty-btn"
        class:active={hideEmpty}
        title={hideEmpty ? "0개 플랫폼 숨김 활성화 (클릭 시 전체 표시)" : "모든 플랫폼 표시 중 (클릭 시 0개 숨김)"}
        onclick={() => (hideEmpty = !hideEmpty)}
      >
        {#if hideEmpty}
          <EyeOff size={14} />
          <span>0개 숨김 ({emptyCount})</span>
        {:else}
          <Eye size={14} />
          <span>전체 표시</span>
        {/if}
      </button>
    </div>

    <div class="summary-bar">
      <span>표시 {filteredSystems.length} / 총 {systems.length}개</span>
      <span class="sep">•</span>
      <span>{totalRoms}개 ROM</span>
      {#if totalUnregistered > 0}
        <span class="sep">•</span>
        <span class="unregistered-highlight" title="미등록된 롬">{totalUnregistered}개 미등록</span>
      {/if}
    </div>
  </div>

  <div class="system-list">
    {#if isLoading && systems.length === 0}
      <div class="loading-state">
        <span>기기 플랫폼 목록 조회 중...</span>
      </div>
    {:else if filteredSystems.length === 0}
      <div class="empty-state">
        <span>검색 결과가 없습니다.</span>
      </div>
    {:else}
      {#each filteredSystems as s}
        <div
          class="system-item"
          class:active={selectedSystemId === s.id}
          onclick={() => onSelect(s.id)}
          role="button"
          tabindex="0"
        >
          <div class="system-main">
            <div class="system-name">{s.name}</div>
            <div class="system-id">{s.id}</div>
          </div>

          <div class="system-badges">
            {#if s.unregistered_count > 0}
              <span class="badge unregistered" title={`미등록 ROM: ${s.unregistered_count}개`}>
                <AlertCircle size={11} />
                {s.unregistered_count}
              </span>
            {/if}

            {#if s.missing_image_count > 0}
              <span class="badge missing-img" title={`이미지 누락: ${s.missing_image_count}개`}>
                <ImageOff size={11} />
              </span>
            {/if}

            <span class="badge count-badge" title={`전체 ROM: ${s.rom_count}개`}>
              {s.rom_count}
            </span>
          </div>
        </div>
      {/each}
    {/if}
  </div>
</aside>

<style lang="scss">
  @use '../../styles/variables' as *;

  .sidebar {
    width: 270px;
    background: $bg-secondary;
    border-right: 1px solid $border-color;
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    height: 100%;
    overflow: hidden;
    transition: margin-left 0.25s cubic-bezier(0.4, 0, 0.2, 1),
                border-color 0.2s ease,
                opacity 0.2s ease;

    &.collapsed {
      margin-left: -270px;
      border-right-color: transparent;
      opacity: 0;
      pointer-events: none;
    }
  }

  .sidebar-header {
    padding: 12px;
    border-bottom: 1px solid $border-color;
    display: flex;
    flex-direction: column;
    gap: 8px;

    .storage-selector {
      display: flex;
      align-items: center;
      gap: 8px;
      padding-bottom: 2px;

      .storage-label {
        font-size: 11px;
        font-weight: 600;
        color: $text-muted;
        white-space: nowrap;
      }

      .storage-pills {
        display: flex;
        gap: 6px;
        flex: 1;

        .storage-pill {
          flex: 1;
          display: flex;
          align-items: center;
          justify-content: center;
          gap: 5px;
          padding: 5px 8px;
          font-size: 11px;
          font-weight: 500;
          background: $bg-tertiary;
          border: 1px solid $border-color;
          color: $text-secondary;
          border-radius: $radius-sm;
          cursor: pointer;
          transition: all 0.15s ease;

          &:hover {
            background: $bg-hover;
            color: $text-primary;
          }

          &.active {
            background: rgba(99, 102, 241, 0.2);
            border-color: #6366f1;
            color: #818cf8;
            font-weight: 600;
          }
        }
      }
    }

    .search-row {
      display: flex;
      align-items: center;
      gap: 6px;

      .search-box {
        position: relative;
        display: flex;
        align-items: center;
        flex: 1;

        :global(.search-icon) {
          position: absolute;
          left: 10px;
          color: $text-muted;
          pointer-events: none;
        }

        input {
          width: 100%;
          padding-left: 32px;
          padding-top: 6px;
          padding-bottom: 6px;
          font-size: 12px;
        }
      }

      .toggle-empty-btn {
        background: $bg-tertiary;
        border: 1px solid $border-color;
        color: $text-secondary;
        padding: 5px 8px;
        font-size: 11px;
        border-radius: $radius-sm;
        display: flex;
        align-items: center;
        gap: 4px;
        white-space: nowrap;
        cursor: pointer;
        transition: all 0.12s ease;

        &:hover {
          background: $bg-hover;
          color: $text-primary;
        }

        &.active {
          background: rgba(99, 102, 241, 0.15);
          border-color: rgba(99, 102, 241, 0.35);
          color: #818cf8;
          font-weight: 500;
        }
      }
    }

    .summary-bar {
      font-size: 11px;
      color: $text-muted;
      display: flex;
      align-items: center;
      gap: 4px;
      padding: 0 2px;

      .sep {
        opacity: 0.5;
      }

      .unregistered-highlight {
        color: #fbbf24;
        font-weight: 600;
      }
    }
  }

  .system-list {
    flex: 1;
    overflow-y: auto;
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .loading-state, .empty-state {
    padding: 30px 16px;
    text-align: center;
    color: $text-muted;
    font-size: 12px;
  }

  .system-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    border-radius: $radius-sm;
    cursor: pointer;
    transition: all 0.12s ease;
    border: 1px solid transparent;

    &:hover {
      background: $bg-tertiary;
    }

    &.active {
      background: $bg-card;
      border-color: $accent-color;

      .system-main .system-name {
        color: white;
        font-weight: 600;
      }
    }

    .system-main {
      min-width: 0;
      flex: 1;

      .system-name {
        font-size: 12.5px;
        font-weight: 500;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        color: $text-primary;
      }

      .system-id {
        font-size: 10px;
        color: $text-muted;
        font-family: $font-mono;
        margin-top: 1px;
      }
    }

    .system-badges {
      display: flex;
      align-items: center;
      gap: 4px;
      margin-left: 8px;

      .badge {
        font-size: 10.5px;
        padding: 1px 6px;

        &.count-badge {
          background: $bg-hover;
          color: $text-secondary;
          border-radius: $radius-sm;
        }

        &.missing-img {
          color: $text-muted;
          background: rgba(255, 255, 255, 0.05);
          padding: 2px 4px;
        }
      }
    }
  }
</style>
