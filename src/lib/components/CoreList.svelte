<script lang="ts">
  import type { InstalledCore } from '../types';
  import { isCoreMatchingSystem } from '../utils/coreUtils';
  import {
    Search,
    Cpu,
    RefreshCw,
    ChevronsRight,
    CheckCircle2,
    Layers,
    Tag,
    FileCode,
  } from 'lucide-svelte';

  let {
    cores = [],
    isLoading = false,
    selectedSystemId = null,
    systemName = '',
    isSidebarOpen = true,
    onToggleSidebar = () => {},
    onRefresh = () => {},
  } = $props<{
    cores: InstalledCore[];
    isLoading: boolean;
    selectedSystemId: string | null;
    systemName?: string;
    isSidebarOpen?: boolean;
    onToggleSidebar?: () => void;
    onRefresh: () => void;
  }>();

  let searchQuery = $state('');

  // 현재 기종의 기본 코어인지 확인
  function isDefaultCore(core: InstalledCore): boolean {
    if (!selectedSystemId) return false;
    const sysId = selectedSystemId.toLowerCase();
    return core.is_default_for.map((s) => s.toLowerCase()).includes(sysId);
  }

  // 현재 선택된 기종에 일치하는 코어 목록만 추출
  let systemCores = $derived(
    cores.filter((c: InstalledCore) => isCoreMatchingSystem(c, selectedSystemId))
  );

  // 검색어 필터링된 코어 목록
  let filteredCores = $derived(
    systemCores.filter((c: InstalledCore) => {
      if (!searchQuery.trim()) return true;
      const q = searchQuery.toLowerCase().trim();
      const matchesName = c.display_name.toLowerCase().includes(q) || c.core_name.toLowerCase().includes(q);
      const matchesSystem = c.system_name.toLowerCase().includes(q);
      const matchesFile = c.file_name.toLowerCase().includes(q);
      const matchesId = c.id.toLowerCase().includes(q);
      const matchesExt = c.supported_extensions.some((ext) => ext.toLowerCase().includes(q.replace(/^\./, '')));
      return matchesName || matchesSystem || matchesFile || matchesId || matchesExt;
    })
  );
</script>

<div class="core-list-panel">
  <!-- 상단 컨트롤 툴바 -->
  <div class="core-toolbar">
    <div class="toolbar-left">
      {#if !isSidebarOpen}
        <button
          class="toggle-sidebar-btn"
          onclick={onToggleSidebar}
          title="기종 사이드바 열기"
        >
          <span class="system-name-label">{systemName || '기종 목록'}</span>
          <ChevronsRight size={14} class="arrow-icon" />
        </button>
      {/if}

      <div class="search-wrap">
        <Search size={14} class="search-icon" />
        <input
          type="text"
          placeholder="코어 이름, 지원 확장자(예: chd, zip) 검색..."
          bind:value={searchQuery}
        />
        {#if searchQuery}
          <button class="clear-search-btn" onclick={() => (searchQuery = '')}>✕</button>
        {/if}
      </div>
    </div>

    <div class="toolbar-right">
      <div class="count-summary">
        {#if selectedSystemId}
          <span class="highlight-summary">
            <strong>{systemName || selectedSystemId}</strong> 코어: <strong>{filteredCores.length}</strong>개
          </span>
        {:else}
          <span>설치 코어: <strong>{filteredCores.length}</strong>개</span>
        {/if}
      </div>

      <button
        class="btn-secondary refresh-btn"
        disabled={isLoading}
        title="기기 내 에뮬레이터 코어 재스캔"
        onclick={onRefresh}
      >
        <RefreshCw size={14} class={isLoading ? 'spin' : ''} />
        <span>새로고침</span>
      </button>
    </div>
  </div>

  <!-- 코어 카드 리스트 본문 -->
  <div class="core-list-body">
    {#if isLoading}
      <div class="state-message">
        <div class="spinner"></div>
        <span>기기 내 설치된 에뮬레이터 코어 및 메타데이터 스캔 중...</span>
      </div>
    {:else if cores.length === 0}
      <div class="state-message">
        <Cpu size={44} class="empty-icon" />
        <h3>기기에서 에뮬레이터 코어를 찾지 못했습니다.</h3>
        <p>
          기기의 SSH 연결 상태를 확인하거나 아래의 새로고침 버튼을 눌러 다시 스캔해 보세요.<br />
          (Knulli, Batocera, ROCKNIX, RetroPie, ArkOS, muOS 등 지원)
        </p>
        <button class="btn-primary reset-view-btn" onclick={onRefresh} disabled={isLoading}>
          <RefreshCw size={14} class={isLoading ? 'spin' : ''} />
          <span>기기 코어 다시 스캔</span>
        </button>
      </div>
    {:else if !selectedSystemId}
      <div class="state-message">
        <Cpu size={44} class="empty-icon" />
        <h3>좌측에서 플랫폼(기종)을 선택하세요.</h3>
        <p>선택하신 플랫폼을 구동하는 데 사용할 수 있는 에뮬레이터 코어 목록이 여기에 표시됩니다.</p>
      </div>
    {:else if systemCores.length === 0}
      <div class="state-message">
        <Cpu size={44} class="empty-icon" />
        <h3>'{systemName || selectedSystemId}' 기종에 호환되는 에뮬레이터 코어가 없습니다.</h3>
        <p>
          해당 기종이 독립 실행형(Standalone) 에뮬레이터를 사용 중이거나 다른 이름으로 등록되었을 수 있습니다.
        </p>
      </div>
    {:else if filteredCores.length === 0}
      <div class="state-message">
        <Cpu size={40} class="empty-icon" />
        <h3>검색 조건에 맞는 에뮬레이터 코어가 없습니다.</h3>
        <p>검색어를 변경하거나 필터를 초기화해 보세요.</p>
        {#if searchQuery}
          <button class="btn-secondary reset-view-btn" onclick={() => (searchQuery = '')}>
            검색어 초기화
          </button>
        {/if}
      </div>
    {:else}
      <div class="core-grid">
        {#each filteredCores as core (core.id + core.file_name)}
          {@const isDefault = isDefaultCore(core)}
          <div class="core-card" class:is-default={isDefault}>
            <div class="card-header">
              <div class="core-title-wrap">
                <div class="core-badge-row">
                  <span class="system-tag">{core.system_name}</span>
                  {#if isDefault}
                    <span class="default-badge" title="현재 기종(batocera.conf)의 기본 실행 코어로 설정되어 있습니다">
                      <CheckCircle2 size={12} />
                      현재 기본 코어
                    </span>
                  {/if}
                </div>
                <h3 class="core-name" title={core.display_name}>{core.display_name}</h3>
              </div>

              <!-- 미래 코어 교체 버튼 자리 (2단계 대비) -->
              <div class="card-actions">
                {#if isDefault}
                  <div class="active-status-pill">
                    <span>활성</span>
                  </div>
                {:else if selectedSystemId && isCoreMatchingSystem(core, selectedSystemId)}
                  <button
                    class="btn-switch-core"
                    title="추후 업데이트에서 기본 코어로 바로 적용할 수 있도록 지원될 예정입니다"
                  >
                    기본 코어로 지정
                  </button>
                {/if}
              </div>
            </div>

            <!-- 파일명 및 핵심 정보 -->
            <div class="card-meta">
              <div class="meta-item">
                <FileCode size={13} class="meta-icon" />
                <code class="file-code">{core.file_name}</code>
              </div>
              {#if core.license}
                <div class="meta-item">
                  <span class="meta-label">라이선스:</span>
                  <span class="meta-val">{core.license}</span>
                </div>
              {/if}
              {#if core.authors}
                <div class="meta-item">
                  <span class="meta-label">제작:</span>
                  <span class="meta-val author-val" title={core.authors}>{core.authors}</span>
                </div>
              {/if}
            </div>

            <!-- 지원 확장자 태그들 -->
            <div class="card-extensions">
              <span class="ext-title">
                <Tag size={12} />
                지원 확장자 ({core.supported_extensions.length})
              </span>
              <div class="ext-chips">
                {#if core.supported_extensions.length > 0}
                  {#each core.supported_extensions as ext}
                    <span class="ext-chip">.{ext}</span>
                  {/each}
                {:else}
                  <span class="ext-empty">확장자 정보 없음</span>
                {/if}
              </div>
            </div>

            <!-- 지원 기종 태그 (es_systems 기반) -->
            {#if core.supported_systems.length > 0}
              <div class="card-supported-systems">
                <span class="sys-title">
                  <Layers size={12} />
                  호환 기종:
                </span>
                <div class="sys-tags">
                  {#each core.supported_systems as sys}
                    <span class="sys-tag" class:highlight={sys.toLowerCase() === selectedSystemId?.toLowerCase()}>
                      {sys}
                    </span>
                  {/each}
                </div>
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style lang="scss">
  @use '../../styles/variables' as *;

  .core-list-panel {
    flex: 1;
    display: flex;
    flex-direction: column;
    height: 100%;
    background: $bg-primary;
    overflow: hidden;
  }

  .core-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 16px;
    background: $bg-secondary;
    border-bottom: 1px solid $border-color;

    .toolbar-left {
      display: flex;
      align-items: center;
      gap: 10px;
      flex: 1;
      max-width: 540px;
    }

    .toggle-sidebar-btn {
      display: flex;
      align-items: center;
      gap: 6px;
      padding: 6px 10px;
      background: $bg-tertiary;
      border: 1px solid $border-color;
      border-radius: $radius-sm;
      color: $text-secondary;
      cursor: pointer;
      font-size: 12px;

      &:hover {
        background: $border-color;
        color: $text-primary;
      }
    }

    .search-wrap {
      position: relative;
      flex: 1;
      display: flex;
      align-items: center;

      .search-icon {
        position: absolute;
        left: 10px;
        color: $text-muted;
      }

      input {
        width: 100%;
        padding: 7px 28px 7px 32px;
        background: $bg-primary;
        border: 1px solid $border-color;
        border-radius: $radius-sm;
        color: $text-primary;
        font-size: 13px;
        outline: none;
        transition: border-color 0.15s ease;

        &:focus {
          border-color: $accent-color;
        }

        &::placeholder {
          color: $text-muted;
          font-size: 12.5px;
        }
      }

      .clear-search-btn {
        position: absolute;
        right: 8px;
        background: none;
        border: none;
        color: $text-muted;
        cursor: pointer;
        font-size: 12px;

        &:hover {
          color: $text-primary;
        }
      }
    }

    .toolbar-right {
      display: flex;
      align-items: center;
      gap: 12px;

      .count-summary {
        font-size: 12.5px;
        color: $text-muted;

        strong {
          color: $text-primary;
        }

        .highlight-summary {
          color: $accent-color;
          strong {
            color: #fff;
          }
        }
      }

      .refresh-btn {
        display: flex;
        align-items: center;
        gap: 6px;
        padding: 6px 12px;
        font-size: 12.5px;
        border-radius: $radius-sm;
      }
    }
  }


  .core-list-body {
    flex: 1;
    overflow-y: auto;
    padding: 16px;

    .state-message {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      min-height: 280px;
      gap: 12px;
      text-align: center;
      color: $text-muted;

      .empty-icon {
        opacity: 0.4;
      }

      h3 {
        font-size: 16px;
        color: $text-secondary;
      }

      p {
        font-size: 13px;
        max-width: 380px;
        line-height: 1.5;
      }

      .reset-view-btn {
        margin-top: 8px;
        font-size: 13px;
      }

      .spinner {
        width: 32px;
        height: 32px;
        border: 3px solid rgba(255, 255, 255, 0.1);
        border-top-color: $accent-color;
        border-radius: 50%;
        animation: spin 0.8s linear infinite;
      }
    }
  }

  .core-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));
    gap: 14px;
  }

  .core-card {
    background: $bg-secondary;
    border: 1px solid $border-color;
    border-radius: $radius-md;
    padding: 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    transition: transform 0.15s ease, border-color 0.15s ease, box-shadow 0.15s ease;

    &:hover {
      border-color: rgba(255, 255, 255, 0.2);
      box-shadow: $shadow-md;
    }

    &.is-default {
      border-color: rgba(34, 197, 94, 0.5);
      background: linear-gradient(180deg, rgba(34, 197, 94, 0.05) 0%, $bg-secondary 100%);
    }

    .card-header {
      display: flex;
      align-items: flex-start;
      justify-content: space-between;
      gap: 10px;

      .core-title-wrap {
        display: flex;
        flex-direction: column;
        gap: 4px;
      }

      .core-badge-row {
        display: flex;
        align-items: center;
        gap: 6px;
      }

      .system-tag {
        font-size: 11px;
        font-weight: 600;
        text-transform: uppercase;
        color: $accent-color;
        letter-spacing: 0.5px;
      }

      .default-badge {
        display: inline-flex;
        align-items: center;
        gap: 4px;
        padding: 2px 7px;
        background: rgba(34, 197, 94, 0.15);
        color: #4ade80;
        border: 1px solid rgba(34, 197, 94, 0.3);
        border-radius: $radius-full;
        font-size: 10.5px;
        font-weight: 600;
      }

      .core-name {
        font-size: 15px;
        font-weight: 600;
        color: $text-primary;
        margin: 0;
        line-height: 1.3;
      }

      .card-actions {
        flex-shrink: 0;

        .active-status-pill {
          padding: 3px 8px;
          background: rgba(34, 197, 94, 0.2);
          color: #4ade80;
          border-radius: $radius-sm;
          font-size: 11px;
          font-weight: 600;
        }

        .btn-switch-core {
          padding: 4px 8px;
          background: $bg-tertiary;
          border: 1px solid $border-color;
          border-radius: $radius-sm;
          color: $text-secondary;
          font-size: 11.5px;
          cursor: pointer;
          transition: all 0.15s ease;

          &:hover {
            background: $accent-color;
            border-color: $accent-color;
            color: #fff;
          }
        }
      }
    }

    .card-meta {
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      gap: 12px;
      font-size: 12px;
      color: $text-muted;
      padding-bottom: 6px;
      border-bottom: 1px dashed rgba(255, 255, 255, 0.08);

      .meta-item {
        display: flex;
        align-items: center;
        gap: 4px;
      }

      .file-code {
        font-family: monospace;
        font-size: 11.5px;
        color: #93c5fd;
        background: rgba(59, 130, 246, 0.1);
        padding: 1px 5px;
        border-radius: 3px;
      }

      .meta-label {
        color: $text-muted;
      }

      .meta-val {
        color: $text-secondary;

        &.author-val {
          max-width: 130px;
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
        }
      }
    }

    .card-extensions {
      display: flex;
      flex-direction: column;
      gap: 6px;

      .ext-title {
        display: flex;
        align-items: center;
        gap: 4px;
        font-size: 11px;
        font-weight: 600;
        color: $text-muted;
      }

      .ext-chips {
        display: flex;
        flex-wrap: wrap;
        gap: 4px;

        .ext-chip {
          padding: 1px 6px;
          background: rgba(255, 255, 255, 0.05);
          border: 1px solid rgba(255, 255, 255, 0.08);
          border-radius: 4px;
          font-family: monospace;
          font-size: 11px;
          color: $text-secondary;
        }

        .ext-empty {
          font-size: 11px;
          color: $text-muted;
          font-style: italic;
        }
      }
    }

    .card-supported-systems {
      display: flex;
      align-items: center;
      gap: 6px;
      font-size: 11px;
      color: $text-muted;
      margin-top: 2px;

      .sys-title {
        display: flex;
        align-items: center;
        gap: 4px;
        flex-shrink: 0;
      }

      .sys-tags {
        display: flex;
        flex-wrap: wrap;
        gap: 4px;

        .sys-tag {
          padding: 1px 5px;
          background: $bg-tertiary;
          border-radius: 3px;
          font-size: 10.5px;
          color: $text-muted;

          &.highlight {
            background: rgba(99, 102, 241, 0.2);
            color: #a5b4fc;
            font-weight: 600;
          }
        }
      }
    }
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .spin {
    animation: spin 1s linear infinite;
  }
</style>
