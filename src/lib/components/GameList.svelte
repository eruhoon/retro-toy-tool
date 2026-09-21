<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
  import type { GameItem, GameStatus } from '../types';
  import {
    Search,
    Star,
    Image,
    ImageOff,
    AlertCircle,
    FileText,
    CheckCircle2,
    ChevronsRight,
    ChevronsLeft,
    Film,
    FolderUp,
  } from 'lucide-svelte';

  let {
    games = [],
    selectedGame = $bindable(null),
    isLoading = false,
    isSidebarOpen = true,
    systemName = '',
    hasSelectedSystem = false,
    onSelectGame = () => {},
    onToggleSidebar = () => {},
    onUploadFiles = () => {},
  } = $props<{
    games: GameItem[];
    selectedGame: GameItem | null;
    isLoading: boolean;
    isSidebarOpen?: boolean;
    systemName?: string;
    hasSelectedSystem?: boolean;
    onSelectGame: (game: GameItem) => void;
    onToggleSidebar?: () => void;
    onUploadFiles?: (paths: string[]) => void;
  }>();

  let listContainerEl = $state<HTMLDivElement | null>(null);
  let isDragOver = $state(false);

  onMount(() => {
    let unlistenFn: (() => void) | null = null;
    try {
      const appWindow = getCurrentWebviewWindow();
      appWindow.onDragDropEvent((event) => {
        if (event.payload.type === 'over') {
          const { x, y } = event.payload.position;
          if (listContainerEl) {
            const rect = listContainerEl.getBoundingClientRect();
            if (x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom) {
              isDragOver = true;
              return;
            }
          }
          isDragOver = false;
        } else if (event.payload.type === 'drop') {
          const { x, y } = event.payload.position;
          const paths = event.payload.paths;
          let isInside = false;
          if (listContainerEl) {
            const rect = listContainerEl.getBoundingClientRect();
            isInside = x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom;
          }
          isDragOver = false;
          if (isInside && paths && paths.length > 0) {
            onUploadFiles(paths);
          }
        } else if (event.payload.type === 'leave') {
          isDragOver = false;
        }
      }).then((unlisten) => {
        unlistenFn = unlisten;
      });
    } catch (err) {
      console.warn('onDragDropEvent failed to register:', err);
    }

    return () => {
      if (unlistenFn) unlistenFn();
    };
  });

  type FilterType = 'all' | 'unregistered' | 'missing_image' | 'favorite';
  let activeFilter = $state<FilterType>('all');
  let searchQuery = $state('');

  let filteredGames = $derived(
    games.filter((g: GameItem) => {
      // 1. Text filter
      const matchesText =
        g.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        g.filename.toLowerCase().includes(searchQuery.toLowerCase()) ||
        (g.developer && g.developer.toLowerCase().includes(searchQuery.toLowerCase())) ||
        (g.genre && g.genre.toLowerCase().includes(searchQuery.toLowerCase()));

      if (!matchesText) return false;

      // 2. Tab filter
      if (activeFilter === 'unregistered') {
        return g.status === 'unregistered';
      }
      if (activeFilter === 'missing_image') {
        return !g.image || g.image.trim() === '';
      }
      if (activeFilter === 'favorite') {
        return g.favorite;
      }
      return true;
    })
  );

  function formatFileSize(bytes: number): string {
    if (!bytes || bytes === 0) return '-';
    if (bytes < 1024) return bytes + ' B';
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
    return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
  }

  function formatYear(releasedate?: string | null): string {
    if (!releasedate || releasedate.length < 4) return '';
    return releasedate.substring(0, 4);
  }

  function getSubfolder(p: string | undefined): string | null {
    if (!p) return null;
    const clean = p.replace(/^\.\//, '');
    const idx = clean.lastIndexOf('/');
    if (idx > 0) {
      return clean.substring(0, idx) + '/';
    }
    return null;
  }
</script>

<div class="game-list-container" bind:this={listContainerEl}>
  {#if isDragOver}
    <div class="drag-drop-overlay" class:error={!hasSelectedSystem}>
      <div class="drag-drop-card">
        <FolderUp size={44} class="drop-icon" />
        {#if hasSelectedSystem}
          <h3>[{systemName}] 폴더로 ROM 파일 복사</h3>
          <p>여기에 파일을 놓으면 기기의 해당 폴더로 자동 전송됩니다.</p>
          <span class="sub-note">단일/다중 파일 및 폴더 드롭 지원</span>
        {:else}
          <h3 class="warn-title">콘솔 플랫폼을 먼저 선택해주세요</h3>
          <p>좌측 목록에서 롬을 넣을 콘솔 플랫폼을 선택한 후 드롭해주세요.</p>
        {/if}
      </div>
    </div>
  {/if}

  <div class="toolbar">
    <div class="toolbar-left">
      {#if isSidebarOpen}
        <button
          class="sidebar-toggle-btn collapse"
          onclick={onToggleSidebar}
          title="콘솔 플랫폼 목록 접기 (서랍 닫기)"
        >
          <ChevronsLeft size={16} />
        </button>
      {:else}
        <button
          class="sidebar-toggle-btn restore"
          onclick={onToggleSidebar}
          title="콘솔 플랫폼 목록 펼치기 (원복)"
        >
          <span class="system-name-label">{systemName || '콘솔 목록'}</span>
          <ChevronsRight size={14} class="arrow-icon" />
        </button>
      {/if}

      <div class="search-wrap">
        <Search size={14} class="search-icon" />
        <input
          type="text"
          placeholder="게임 제목, 파일명, 장르 검색..."
          bind:value={searchQuery}
        />
      </div>
    </div>

    <div class="filter-tabs">
      <button
        class="tab-btn"
        class:active={activeFilter === 'all'}
        onclick={() => (activeFilter = 'all')}
      >
        전체 ({games.length})
      </button>

      <button
        class="tab-btn warn"
        class:active={activeFilter === 'unregistered'}
        onclick={() => (activeFilter = 'unregistered')}
      >
        미등록 ({games.filter((g: GameItem) => g.status === 'unregistered').length})
      </button>

      <button
        class="tab-btn"
        class:active={activeFilter === 'missing_image'}
        onclick={() => (activeFilter = 'missing_image')}
      >
        이미지 누락 ({games.filter((g: GameItem) => !g.image || g.image.trim() === '').length})
      </button>

      <button
        class="tab-btn fav"
        class:active={activeFilter === 'favorite'}
        onclick={() => (activeFilter = 'favorite')}
      >
        즐겨찾기 ({games.filter((g: GameItem) => g.favorite).length})
      </button>
    </div>
  </div>

  <div class="list-body">
    {#if isLoading}
      <div class="center-message">
        <div class="spinner"></div>
        <span>ROM 및 gamelist.xml 로딩 중...</span>
      </div>
    {:else if filteredGames.length === 0}
      <div class="center-message">
        <FileText size={32} class="empty-icon" />
        <span>해당 조건에 맞는 게임이 없습니다.</span>
      </div>
    {:else}
      <div class="game-table">
        <div class="table-header">
          <div class="col col-fav">★</div>
          <div class="col col-name">제목 / 파일명</div>
          <div class="col col-ox" title="실제 ROM 파일이 기기에 존재하는지 여부">ROM</div>
          <div class="col col-ox" title="gamelist.xml에 메타데이터가 등록되어 있는지 여부">메타</div>
          <div class="col col-status">상태</div>
          <div class="col col-genre">장르</div>
          <div class="col col-year">연도</div>
          <div class="col col-size">크기</div>
        </div>

        <div class="table-rows">
          {#each filteredGames as game}
            <div
              class="table-row"
              class:selected={selectedGame?.path === game.path}
              class:unregistered-row={game.status === 'unregistered'}
              onclick={() => onSelectGame(game)}
              role="button"
              tabindex="0"
            >
              <div class="col col-fav">
                {#if game.favorite}
                  <Star size={13} class="star-active" />
                {/if}
              </div>

              <div class="col col-name">
                <div class="title-line">
                  {#if game.image}
                    <span class="has-img-icon" title="이미지 등록됨"><Image size={13} /></span>
                  {:else}
                    <span class="no-img-icon" title="이미지 누락"><ImageOff size={13} /></span>
                  {/if}
                  {#if game.video}
                    <span class="has-video-icon" title="동영상 등록됨"><Film size={12} /></span>
                  {/if}
                  <span class="game-title">{game.name}</span>
                </div>
                <div class="filename-line">
                  {#if getSubfolder(game.path)}
                    <span class="subfolder-tag">{getSubfolder(game.path)}</span>
                  {/if}
                  {game.filename}
                </div>
              </div>

              <!-- ROM File Exists: O / X -->
              <div class="col col-ox">
                {#if game.status === 'registered' || game.status === 'unregistered'}
                  <span class="ox-badge yes" title="실제 ROM 파일 존재">O</span>
                {:else}
                  <span class="ox-badge no" title="ROM 파일 없음 (누락)">X</span>
                {/if}
              </div>

              <!-- Metadata Exists: O / X -->
              <div class="col col-ox">
                {#if game.status === 'registered' || game.status === 'missing'}
                  <span class="ox-badge yes" title="gamelist.xml 메타데이터 있음">O</span>
                {:else}
                  <span class="ox-badge no" title="메타데이터 없음 (미등록)">X</span>
                {/if}
              </div>

              <div class="col col-status">
                {#if game.status === 'registered'}
                  <span class="badge registered">
                    <CheckCircle2 size={11} /> 등록됨
                  </span>
                {:else if game.status === 'unregistered'}
                  <span class="badge unregistered">
                    <AlertCircle size={11} /> 미등록
                  </span>
                {:else}
                  <span class="badge missing">
                    누락
                  </span>
                {/if}
              </div>

              <div class="col col-genre">
                <span>{game.genre || '-'}</span>
              </div>

              <div class="col col-year">
                <span>{formatYear(game.releasedate) || '-'}</span>
              </div>

              <div class="col col-size">
                <span>{formatFileSize(game.file_size)}</span>
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</div>

<style lang="scss">
  @use '../../styles/variables' as *;

  .game-list-container {
    position: relative;
    flex: 1;
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background: $bg-primary;
  }

  .drag-drop-overlay {
    position: absolute;
    inset: 6px;
    z-index: 50;
    background: rgba(15, 23, 42, 0.88);
    border: 2px dashed #3b82f6;
    border-radius: 10px;
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
    animation: fade-in-scale 0.15s ease-out;

    &.error {
      border-color: #ef4444;
      background: rgba(40, 10, 15, 0.88);
      .drop-icon {
        color: #ef4444;
      }
    }

    .drag-drop-card {
      display: flex;
      flex-direction: column;
      align-items: center;
      text-align: center;
      padding: 24px 32px;
      max-width: 440px;

      .drop-icon {
        color: #3b82f6;
        margin-bottom: 12px;
        animation: bounce 1.2s infinite ease-in-out;
      }

      h3 {
        margin: 0 0 6px 0;
        font-size: 16px;
        font-weight: 700;
        color: #f8fafc;

        &.warn-title {
          color: #f87171;
        }
      }

      p {
        margin: 0 0 10px 0;
        font-size: 13px;
        color: #94a3b8;
      }

      .sub-note {
        font-size: 11px;
        background: rgba(255, 255, 255, 0.08);
        color: #cbd5e1;
        padding: 3px 8px;
        border-radius: 4px;
      }
    }
  }

  @keyframes bounce {
    0%, 100% {
      transform: translateY(0);
    }
    50% {
      transform: translateY(-8px);
    }
  }

  .toolbar {
    padding: 10px 14px;
    background: $bg-secondary;
    border-bottom: 1px solid $border-color;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;

    .toolbar-left {
      display: flex;
      align-items: center;
      gap: 10px;
      flex: 1;
      min-width: 260px;
    }

    .sidebar-toggle-btn,
    .restore-sidebar-btn {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      cursor: pointer;
      white-space: nowrap;
      transition: all 0.15s ease;
      animation: fade-in-scale 0.2s ease;

      &.collapse {
        padding: 6px 9px;
        background: $bg-tertiary;
        border: 1px solid $border-color;
        border-radius: $radius-sm;
        color: $text-secondary;

        &:hover {
          background: $bg-hover;
          color: $text-primary;
          border-color: rgba(99, 102, 241, 0.45);
        }
      }

      &.restore {
        gap: 6px;
        padding: 6px 12px;
        background: rgba(99, 102, 241, 0.12);
        border: 1px solid rgba(99, 102, 241, 0.35);
        border-radius: $radius-sm;
        color: #818cf8;
        font-size: 12px;
        font-weight: 600;

        .system-name-label {
          max-width: 140px;
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }

        :global(.arrow-icon) {
          transition: transform 0.15s ease;
        }

        &:hover {
          background: rgba(99, 102, 241, 0.22);
          border-color: rgba(99, 102, 241, 0.55);
          color: #a5b4fc;
          transform: translateY(-1px);

          :global(.arrow-icon) {
            transform: translateX(3px);
          }
        }

        &:active {
          transform: translateY(0);
        }
      }
    }

    .search-wrap {
      position: relative;
      flex: 1;
      min-width: 180px;
      max-width: 380px;

      :global(.search-icon) {
        position: absolute;
        left: 10px;
        top: 50%;
        transform: translateY(-50%);
        color: $text-muted;
        pointer-events: none;
      }

      input {
        width: 100%;
        padding-left: 32px;
        padding-top: 6px;
        padding-bottom: 6px;
        font-size: 12.5px;
      }
    }

    .filter-tabs {
      display: flex;
      align-items: center;
      gap: 4px;
      background: $bg-primary;
      padding: 3px;
      border-radius: $radius-sm;
      border: 1px solid $border-color;

      .tab-btn {
        background: transparent;
        border: none;
        color: $text-secondary;
        font-size: 11.5px;
        padding: 4px 10px;
        border-radius: $radius-sm - 2px;
        cursor: pointer;

        &:hover {
          color: $text-primary;
        }

        &.active {
          background: $bg-card;
          color: white;
          font-weight: 600;
          box-shadow: $shadow-sm;
        }

        &.warn.active {
          color: #fbbf24;
        }

        &.fav.active {
          color: #f43f5e;
        }
      }
    }
  }

  .list-body {
    flex: 1;
    overflow-y: auto;
    position: relative;
  }

  .center-message {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    height: 100%;
    color: $text-muted;
    font-size: 13px;

    :global(.empty-icon) {
      opacity: 0.3;
    }
  }

  .spinner {
    width: 24px;
    height: 24px;
    border: 2px solid $border-color;
    border-top-color: $accent-color;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  .game-table {
    display: flex;
    flex-direction: column;
    width: 100%;

    .table-header {
      display: flex;
      align-items: center;
      padding: 8px 14px;
      background: rgba(23, 26, 35, 0.95);
      border-bottom: 1px solid $border-color;
      font-size: 11px;
      font-weight: 600;
      color: $text-muted;
      position: sticky;
      top: 0;
      z-index: 10;
      backdrop-filter: blur(4px);
    }

    .table-rows {
      display: flex;
      flex-direction: column;
    }

    .table-row {
      display: flex;
      align-items: center;
      padding: 8px 14px;
      border-bottom: 1px solid rgba(51, 65, 85, 0.4);
      cursor: pointer;
      transition: background 0.1s ease, border-color 0.1s ease;

      &.unregistered-row {
        background: rgba(245, 158, 11, 0.03);
      }

      &:hover {
        background: $bg-tertiary;
      }

      &.unregistered-row:hover {
        background: rgba(245, 158, 11, 0.1);
      }

      &.selected {
        background: $bg-card;
        border-left: 3px solid $accent-color;

        &:hover {
          background: $bg-hover;
        }

        &.unregistered-row {
          background: rgba(99, 102, 241, 0.18);
          border-left: 3px solid #f59e0b;

          &:hover {
            background: rgba(99, 102, 241, 0.25);
          }
        }
      }
    }

    .col {
      padding: 0 6px;
      font-size: 12px;

      &.col-fav {
        width: 24px;
        text-align: center;
        padding: 0;

        :global(.star-active) {
          color: #f43f5e;
          fill: #f43f5e;
        }
      }

      &.col-name {
        flex: 1;
        min-width: 0;

        .title-line {
          display: flex;
          align-items: center;
          gap: 6px;

          .game-title {
            font-weight: 500;
            color: $text-primary;
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
          }

          :global(.has-img-icon) {
            color: #10b981;
            flex-shrink: 0;
          }

          :global(.has-video-icon) {
            color: #c084fc;
            flex-shrink: 0;
            display: inline-flex;
            align-items: center;
          }

          :global(.no-img-icon) {
            color: $text-muted;
            flex-shrink: 0;
            opacity: 0.5;
          }
        }

        .filename-line {
          font-size: 10.5px;
          color: $text-muted;
          font-family: $font-mono;
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
          margin-top: 1px;

          .subfolder-tag {
            color: $accent-light;
            opacity: 0.85;
            margin-right: 3px;
            font-weight: 500;
          }
        }
      }

      &.col-ox {
        width: 44px;
        text-align: center;
        display: flex;
        justify-content: center;
        align-items: center;

        .ox-badge {
          display: inline-flex;
          align-items: center;
          justify-content: center;
          width: 20px;
          height: 20px;
          border-radius: 50%;
          font-size: 11px;
          font-weight: 700;
          font-family: $font-mono;
          line-height: 1;

          &.yes {
            background: rgba(16, 185, 129, 0.15);
            color: #34d399;
            border: 1px solid rgba(16, 185, 129, 0.4);
          }

          &.no {
            background: rgba(239, 68, 68, 0.15);
            color: #f87171;
            border: 1px solid rgba(239, 68, 68, 0.4);
          }
        }
      }

      &.col-status {
        width: 85px;
      }

      &.col-genre {
        width: 110px;
        color: $text-secondary;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
      }

      &.col-year {
        width: 55px;
        color: $text-muted;
        font-family: $font-mono;
        text-align: center;
      }

      &.col-size {
        width: 70px;
        color: $text-muted;
        font-family: $font-mono;
        font-size: 11px;
        text-align: right;
      }
    }
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  @keyframes fade-in-scale {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }
</style>
