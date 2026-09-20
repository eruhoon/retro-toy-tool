<script lang="ts">
  import type { DeviceProfile, GameItem, ScrapedGame, ScraperSettings } from '../types';
  import { searchGameMetadata, downloadAndUploadScrapedImage, downloadAndUploadScrapedVideo } from '../api';
  import {
    Search,
    X,
    Sparkles,
    Check,
    Loader2,
    Calendar,
    Tag,
    Building2,
    Star,
    Image as ImageIcon,
    Users,
    Globe,
    AlertCircle,
    CheckCircle2,
    Settings,
    Film,
    Play,
    Video,
  } from 'lucide-svelte';

  let {
    isOpen = $bindable(false),
    game = $bindable(null),
    device = null,
    systemId = null,
    romsPathOverride = undefined,
    scraperSettings = $bindable(),
    onOpenSettings = () => {},
    onApplied = () => {},
  } = $props<{
    isOpen: boolean;
    game: GameItem | null;
    device: DeviceProfile | null;
    systemId: string | null;
    romsPathOverride?: string;
    scraperSettings: ScraperSettings;
    onOpenSettings: () => void;
    onApplied: (msg: string) => void;
  }>();

  let searchQuery = $state('');
  let selectedSource = $state<'auto' | 'screenscraper' | 'steam' | 'dlsite' | 'wikipedia' | 'rawg'>('auto');


  let isSearching = $state(false);
  let searchResults = $state<ScrapedGame[]>([]);
  let selectedCandidate = $state<ScrapedGame | null>(null);
  let isApplying = $state(false);
  let applyingMsg = $state('');
  let searchError = $state<string | null>(null);

  // Field overwrite choices
  let applyTitle = $state(true);
  let applyDesc = $state(true);
  let applyGenre = $state(true);
  let applyDate = $state(true);
  let applyDev = $state(true);
  let applyRating = $state(true);
  let applyImage = $state(true);
  let applyVideo = $state(true);

  // Preview tab: image or video
  let previewTab = $state<'image' | 'video'>('image');

  function formatFileSize(bytes?: number | null): string {
    if (!bytes || bytes <= 0) return '';
    if (bytes >= 1024 * 1024) {
      return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    }
    return `${Math.round(bytes / 1024)} KB`;
  }

  function selectCandidate(item: ScrapedGame) {
    selectedCandidate = item;
    if (!item.cover_url && item.video_url) {
      previewTab = 'video';
    } else if (previewTab === 'video' && !item.video_url) {
      previewTab = 'image';
    }
  }

  // Common ROM / archive file extensions to safely remove without damaging titles with dots (e.g. "D.P", "v1.0")
  const ROM_EXTENSIONS = /\.(zip|7z|rar|tar|gz|iso|cue|bin|img|nes|fds|smc|sfc|smd|gen|gba|gbc|gb|nds|n64|z64|v64|chd|pbp|cso|wbfs|rvz|sh|exe|apk|m3u|desktop)$/i;
  // DLsite product codes: RJ, VJ, BJ followed by 6 to 8 digits (e.g. RJ123456, RJ01234567)
  const DLSITE_CODE_REGEX = /\b([RBV]J\d{6,8})\b/i;

  function extractBestSearchQuery(g: GameItem): { query: string; isDlsiteCode: boolean } {
    const rawFile = g.filename || '';
    const rawPath = g.path || '';
    const rawName = g.name || '';

    // 1. Check if DLsite RJ/VJ/BJ product code exists anywhere in filename, path or name
    const dlsiteMatch = rawFile.match(DLSITE_CODE_REGEX) ||
                        rawPath.match(DLSITE_CODE_REGEX) ||
                        rawName.match(DLSITE_CODE_REGEX);

    if (dlsiteMatch) {
      return { query: dlsiteMatch[1].toUpperCase(), isDlsiteCode: true };
    }

    // 2. Default search is based on the ROM file (filename/path)
    let romTarget = rawFile;

    // If filename is generic (e.g. game.sh, start.sh, launch.sh, default.exe) or empty, use path
    if (!romTarget || /^(game|start|launch|run|play|default|main)\.(sh|exe|bin|bat)$/i.test(romTarget)) {
      if (rawPath) {
        romTarget = rawPath;
      }
    }

    // Fallback to name only if ROM file information is missing
    if (!romTarget) {
      romTarget = rawName;
    }

    return { query: cleanTitle(romTarget), isDlsiteCode: false };
  }

  function cleanTitle(raw: string): string {
    let s = (raw || '').trim();

    // 1. Strip directory path (or use parent folder name if filename is generic)
    const lastSlash = Math.max(s.lastIndexOf('/'), s.lastIndexOf('\\'));
    if (lastSlash >= 0) {
      const fileName = s.substring(lastSlash + 1);
      const parentDir = s.substring(0, lastSlash).split(/[/\\]/).pop() || '';
      if (/^(game|start|launch|run|play|default|main)\.(sh|exe|bin|bat)$/i.test(fileName) && parentDir) {
        s = parentDir;
      } else {
        s = fileName;
      }
    }

    // 2. Strip only known rom/archive extensions (preserve "D.P", "K.O.F", "v1.2")
    s = s.replace(ROM_EXTENSIONS, '');

    // 3. Remove parentheses tags: (USA), (Japan), (En,Ja)
    s = s.replace(/\([^)]*\)/g, ' ');

    // 4. Remove brackets [!], [b1], etc., EXCEPT if it's an RJ code
    s = s.replace(/\[([^\]]*)\]/g, (match, inner) => {
      if (DLSITE_CODE_REGEX.test(inner)) {
        return match;
      }
      return ' ';
    });

    // 5. Replace underscores and dashes with space
    s = s.replace(/[_\-+]/g, ' ');

    // 6. Collapse whitespace
    return s.replace(/\s+/g, ' ').trim();
  }

  let lastOpenedGameKey = $state<string | null>(null);

  // Auto clean query on game change / open (Runs ONCE when opening for a game, does not overwrite user typing)
  $effect(() => {
    if (!isOpen) {
      lastOpenedGameKey = null;
      return;
    }

    const currentKey = game ? (game.path || game.filename || game.name) : null;
    if (isOpen && game && lastOpenedGameKey !== currentKey) {
      lastOpenedGameKey = currentKey;
      const { query: initialQuery, isDlsiteCode } = extractBestSearchQuery(game);
      searchQuery = initialQuery;

      if (isDlsiteCode) {
        selectedSource = 'dlsite';
      } else {
        selectedSource = scraperSettings.default_source || 'auto';
      }

      selectedCandidate = null;
      searchError = null;
      handleSearch();
    }
  });

  async function handleSearch() {
    if (!searchQuery.trim()) return;

    isSearching = true;
    searchError = null;
    selectedCandidate = null;
    searchResults = [];

    try {
      const results = await searchGameMetadata(
        searchQuery.trim(),
        systemId || undefined,
        scraperSettings,
        selectedSource
      );
      searchResults = results;
      if (results.length > 0) {
        selectCandidate(results[0]);
      }
    } catch (err: any) {
      console.error('검색 실패:', err);
      searchError = err?.message || '검색 중 오류가 발생했습니다.';
    } finally {
      isSearching = false;
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      handleSearch();
    }
  }

  async function handleApply() {
    if (!game || !selectedCandidate) return;

    isApplying = true;
    applyingMsg = '메타데이터 적용 중...';

    try {
      // 1. Overwrite selected text fields
      if (applyTitle && selectedCandidate.name) {
        game.name = selectedCandidate.name;
      }
      if (applyDesc && selectedCandidate.desc) {
        game.desc = selectedCandidate.desc;
      }
      if (applyGenre && selectedCandidate.genre) {
        game.genre = selectedCandidate.genre;
      }
      if (applyDate && selectedCandidate.releasedate) {
        game.releasedate = selectedCandidate.releasedate;
      }
      if (applyDev) {
        if (selectedCandidate.developer) game.developer = selectedCandidate.developer;
        if (selectedCandidate.publisher) game.publisher = selectedCandidate.publisher;
      }
      if (applyRating && selectedCandidate.rating !== null && selectedCandidate.rating !== undefined) {
        game.rating = selectedCandidate.rating;
      }
      if (selectedCandidate.players) {
        game.players = selectedCandidate.players;
      }

      // 2. Download and upload image if requested and present
      if (applyImage && selectedCandidate.cover_url && device && systemId) {
        applyingMsg = '표지 이미지 다운로드 및 기기 전송 중...';
        const savedRelPath = await downloadAndUploadScrapedImage(
          device,
          systemId,
          game.filename,
          selectedCandidate.cover_url,
          romsPathOverride
        );
        game.image = savedRelPath;
      }

      // 3. Download and upload video if requested and present
      if (applyVideo && selectedCandidate.video_url && device && systemId) {
        applyingMsg = '미리보기 영상 다운로드 및 기기 전송 중...';
        const savedVideoRelPath = await downloadAndUploadScrapedVideo(
          device,
          systemId,
          game.filename,
          selectedCandidate.video_url,
          romsPathOverride
        );
        game.video = savedVideoRelPath;
      }

      // 3. Upgrade status from unregistered to registered
      if (game.status === 'unregistered') {
        game.status = 'registered';
      }

      onApplied(`'${game.name}' 정보가 적용되었습니다. 상단 '저장' 버튼으로 기기에 반영하세요.`);
      isOpen = false;
    } catch (err: any) {
      console.error('정보 적용 실패:', err);
      alert('정보 적용 중 오류: ' + (err?.message || err));
    } finally {
      isApplying = false;
      applyingMsg = '';
    }
  }
</script>

{#if isOpen}
  <div class="modal-backdrop" onclick={() => (isOpen = false)} role="presentation">
    <div class="modal-dialog" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
      <!-- Header -->
      <header class="modal-header">
        <div class="title-wrap">
          <div class="icon-wrap">
            <Sparkles size={18} />
          </div>
          <div>
            <h3>온라인 스크랩</h3>
            <p>온라인 데이터베이스에서 게임 정보와 커버 이미지를 검색하고 적용합니다.</p>
          </div>
        </div>

        <div class="header-actions">
          <button class="btn-ghost settings-btn" onclick={onOpenSettings} title="스크래퍼 계정 설정">
            <Settings size={15} />
            <span>설정</span>
          </button>
          <button class="btn-icon" onclick={() => (isOpen = false)}>
            <X size={16} />
          </button>
        </div>
      </header>

      <!-- Search Bar -->
      <div class="search-bar-wrap">
        <div class="search-input-box">
          <Search size={16} class="search-icon" />
          <input
            type="text"
            bind:value={searchQuery}
            onkeydown={handleKeyDown}
            placeholder="게임 제목, DLsite 품번(예: RJ123456), 스팀/위키 검색..."
          />
          {#if searchQuery}
            <button
              type="button"
              class="clear-input-btn"
              onclick={() => (searchQuery = '')}
              title="검색어 지우기"
            >
              <X size={14} />
            </button>
          {/if}
          <button class="btn-primary search-btn" onclick={handleSearch} disabled={isSearching}>
            {#if isSearching}
              <Loader2 size={14} class="spin" />
            {:else}
              검색
            {/if}
          </button>
        </div>

        <!-- Source Selector Tabs -->
        <div class="source-tabs">
          <button
            class="tab-btn"
            class:active={selectedSource === 'auto'}
            onclick={() => { selectedSource = 'auto'; handleSearch(); }}
          >
            전체 통합 검색
          </button>
          <button
            class="tab-btn"
            class:active={selectedSource === 'screenscraper'}
            onclick={() => { selectedSource = 'screenscraper'; handleSearch(); }}
          >
            ScreenScraper
          </button>
          <button
            class="tab-btn"
            class:active={selectedSource === 'steam'}
            onclick={() => { selectedSource = 'steam'; handleSearch(); }}
          >
            Steam (스팀)
          </button>
          <button
            class="tab-btn"
            class:active={selectedSource === 'dlsite'}
            title="DLsite (일어 원문, 한국어판, 영문판 동시 통합 검색)"
            onclick={() => { selectedSource = 'dlsite'; handleSearch(); }}
          >
            DLsite
          </button>
          <button
            class="tab-btn"
            class:active={selectedSource === 'wikipedia'}
            onclick={() => { selectedSource = 'wikipedia'; handleSearch(); }}
          >
            한국어 위키백과
          </button>

          <button
            class="tab-btn"
            class:active={selectedSource === 'rawg'}
            onclick={() => { selectedSource = 'rawg'; handleSearch(); }}
          >
            RAWG
          </button>
        </div>
      </div>

      <!-- Main Content Area: Left Candidate List & Right Preview -->
      <div class="modal-body">
        {#if isSearching}
          <div class="state-view">
            <Loader2 size={32} class="spin" />
            <p>온라인 데이터베이스에서 게임 정보를 긁어오는 중입니다...</p>
          </div>
        {:else if searchError}
          <div class="state-view error">
            <AlertCircle size={32} />
            <p>{searchError}</p>
            <button class="btn-secondary" onclick={handleSearch}>다시 시도</button>
          </div>
        {:else if searchResults.length === 0}
          <div class="state-view">
            <Search size={32} />
            <p>검색 결과가 없습니다. 검색어를 변경하거나 소스를 바꿔보세요.</p>
          </div>
        {:else}
          <div class="results-grid">
            <!-- Candidate List Column -->
            <div class="candidates-col">
              <div class="col-title">검색된 결과 ({searchResults.length}건) - 원하는 항목을 선택하세요</div>
              <div class="candidate-list">
                {#each searchResults as item}
                  <div
                    class="candidate-card"
                    class:selected={selectedCandidate?.id === item.id}
                    onclick={() => selectCandidate(item)}
                    role="button"
                    tabindex="0"
                  >
                    <div class="card-thumb">
                      {#if item.cover_url}
                        <img src={item.cover_url} alt={item.name} loading="lazy" />
                      {:else}
                        <ImageIcon size={24} />
                      {/if}
                      {#if item.video_url}
                        <div class="thumb-video-badge" title="영상 제공">
                          <Play size={9} fill="currentColor" />
                        </div>
                      {/if}
                    </div>

                    <div class="card-info">
                      <div class="card-title-row">
                        <span class="card-title">{item.name}</span>
                        <div class="card-badges">
                          {#if item.cover_size}
                            <span class="media-badge img" title="커버 이미지 용량">
                              <ImageIcon size={10} /> {formatFileSize(item.cover_size)}
                            </span>
                          {/if}
                          {#if item.video_url}
                            <span class="media-badge video" title="미리보기 영상 용량">
                              <Play size={9} fill="currentColor" /> {formatFileSize(item.video_size) || '영상'}
                            </span>
                          {/if}
                          <span class="source-tag">{item.source}</span>
                        </div>
                      </div>
                      <div class="card-meta">
                        {#if item.releasedate}
                          <span>{item.releasedate.substring(0, 4)}년</span>
                        {/if}
                        {#if item.genre}
                          <span>• {item.genre}</span>
                        {/if}
                        {#if item.developer}
                          <span>• {item.developer}</span>
                        {/if}
                      </div>
                      {#if item.desc}
                        <p class="card-desc">{item.desc}</p>
                      {/if}
                    </div>

                    <div class="check-mark">
                      {#if selectedCandidate?.id === item.id}
                        <Check size={16} />
                      {/if}
                    </div>
                  </div>
                {/each}
              </div>
            </div>

            <!-- Detail & Overwrite Options Column -->
            {#if selectedCandidate}
              <div class="preview-col">
                <div class="col-title">선택한 정보 미리보기 & 적용 필드</div>

                <div class="preview-card">
                  {#if selectedCandidate.cover_url || selectedCandidate.video_url}
                    <div class="preview-media-section">
                      {#if selectedCandidate.cover_url && selectedCandidate.video_url}
                        <div class="media-tab-bar">
                          <button
                            type="button"
                            class="media-tab-btn"
                            class:active={previewTab === 'image'}
                            onclick={() => (previewTab = 'image')}
                          >
                            <ImageIcon size={12} />
                            <span>표지 이미지</span>
                            {#if selectedCandidate.cover_size}
                              <span class="tab-size-tag">{formatFileSize(selectedCandidate.cover_size)}</span>
                            {/if}
                          </button>
                          <button
                            type="button"
                            class="media-tab-btn"
                            class:active={previewTab === 'video'}
                            onclick={() => (previewTab = 'video')}
                          >
                            <Play size={11} fill="currentColor" />
                            <span>미리보기 영상</span>
                            {#if selectedCandidate.video_size}
                              <span class="tab-size-tag video">{formatFileSize(selectedCandidate.video_size)}</span>
                            {/if}
                          </button>
                        </div>
                      {/if}

                      <div class="preview-media-viewer">
                        {#if previewTab === 'video' && selectedCandidate.video_url}
                          <video
                            src={selectedCandidate.video_url}
                            controls
                            autoplay
                            muted
                            loop
                            playsinline
                            class="preview-video-player"
                          >
                            <track kind="captions" />
                          </video>
                        {:else if selectedCandidate.cover_url}
                          <div class="preview-image-wrap">
                            <img src={selectedCandidate.cover_url} alt={selectedCandidate.name} />
                            {#if selectedCandidate.cover_size}
                              <div class="preview-size-badge">
                                <ImageIcon size={11} /> {formatFileSize(selectedCandidate.cover_size)}
                              </div>
                            {/if}
                          </div>
                        {:else if selectedCandidate.video_url}
                          <video
                            src={selectedCandidate.video_url}
                            controls
                            autoplay
                            muted
                            loop
                            playsinline
                            class="preview-video-player"
                          >
                            <track kind="captions" />
                          </video>
                        {/if}
                      </div>
                    </div>
                  {/if}

                  <div class="preview-details">
                    <h4>{selectedCandidate.name}</h4>
                    <div class="preview-tags">
                      <span class="tag source">{selectedCandidate.source}</span>
                      {#if selectedCandidate.genre}
                        <span class="tag"><Tag size={11} /> {selectedCandidate.genre}</span>
                      {/if}
                      {#if selectedCandidate.releasedate}
                        <span class="tag"><Calendar size={11} /> {selectedCandidate.releasedate.substring(0, 10)}</span>
                      {/if}
                      {#if selectedCandidate.developer}
                        <span class="tag"><Building2 size={11} /> {selectedCandidate.developer}</span>
                      {/if}
                      {#if selectedCandidate.rating}
                        <span class="tag rating"><Star size={11} /> {Math.round(selectedCandidate.rating * 100)}%</span>
                      {/if}
                    </div>

                    {#if selectedCandidate.desc}
                      <p class="preview-desc">{selectedCandidate.desc}</p>
                    {/if}
                  </div>
                </div>

                <!-- Overwrite Checkboxes -->
                <div class="apply-options">
                  <div class="options-header">적용할 항목 선택:</div>
                  <div class="checkbox-grid">
                    <label class="check-label">
                      <input type="checkbox" bind:checked={applyTitle} />
                      <span>게임 제목</span>
                    </label>
                    <label class="check-label">
                      <input type="checkbox" bind:checked={applyDesc} />
                      <span>줄거리 설명</span>
                    </label>
                    <label class="check-label">
                      <input type="checkbox" bind:checked={applyGenre} />
                      <span>장르</span>
                    </label>
                    <label class="check-label">
                      <input type="checkbox" bind:checked={applyDate} />
                      <span>출시일</span>
                    </label>
                    <label class="check-label">
                      <input type="checkbox" bind:checked={applyDev} />
                      <span>개발/배급사</span>
                    </label>
                    <label class="check-label">
                      <input type="checkbox" bind:checked={applyRating} />
                      <span>평점</span>
                    </label>
                    <label class="check-label full">
                      <input type="checkbox" bind:checked={applyImage} disabled={!selectedCandidate.cover_url} />
                      <span class="apply-label-content">
                        <span>박스아트 이미지 (기기 SSH로 자동 업로드)</span>
                        {#if selectedCandidate.cover_size}
                          <span class="apply-size-pill img">{formatFileSize(selectedCandidate.cover_size)}</span>
                        {/if}
                      </span>
                    </label>
                    {#if selectedCandidate.video_url}
                      <label class="check-label full video-highlight">
                        <input type="checkbox" bind:checked={applyVideo} />
                        <span class="apply-label-content">
                          <span>미리보기 동영상 (.mp4 기기 SSH로 자동 업로드)</span>
                          {#if selectedCandidate.video_size}
                            <span class="apply-size-pill video">{formatFileSize(selectedCandidate.video_size)}</span>
                          {/if}
                        </span>
                      </label>
                    {/if}
                  </div>
                </div>
              </div>
            {/if}
          </div>
        {/if}
      </div>

      <!-- Footer -->
      <footer class="modal-footer">
        {#if applyingMsg}
          <div class="applying-status">
            <Loader2 size={14} class="spin" />
            <span>{applyingMsg}</span>
          </div>
        {/if}

        <div class="footer-actions">
          <button class="btn-secondary" onclick={() => (isOpen = false)} disabled={isApplying}>
            취소
          </button>
          <button
            class="btn-primary apply-btn"
            onclick={handleApply}
            disabled={!selectedCandidate || isApplying}
          >
            {#if isApplying}
              <Loader2 size={14} class="spin" /> 적용 중...
            {:else}
              <CheckCircle2 size={15} /> 선택한 정보 적용하기
            {/if}
          </button>
        </div>
      </footer>
    </div>
  </div>
{/if}

<style lang="scss">
  @use '../../styles/variables' as *;

  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal-dialog {
    background: $bg-secondary;
    border: 1px solid $border-light;
    border-radius: $radius-lg;
    width: 95%;
    max-width: 920px;
    height: 85vh;
    box-shadow: $shadow-lg;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: zoom-in 0.15s ease-out;
  }

  .modal-header {
    padding: 14px 20px;
    border-bottom: 1px solid $border-color;
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-shrink: 0;

    .title-wrap {
      display: flex;
      align-items: center;
      gap: 12px;

      .icon-wrap {
        width: 36px;
        height: 36px;
        border-radius: $radius-md;
        background: rgba(99, 102, 241, 0.15);
        color: $accent-color;
        display: flex;
        align-items: center;
        justify-content: center;
      }

      h3 {
        font-size: 15px;
        font-weight: 600;
      }

      p {
        font-size: 11.5px;
        color: $text-muted;
        margin-top: 1px;
      }
    }

    .header-actions {
      display: flex;
      align-items: center;
      gap: 8px;

      .settings-btn {
        display: flex;
        align-items: center;
        gap: 6px;
        font-size: 12px;
        padding: 5px 10px;
      }
    }
  }

  .search-bar-wrap {
    padding: 12px 20px;
    background: $bg-primary;
    border-bottom: 1px solid $border-color;
    display: flex;
    flex-direction: column;
    gap: 10px;
    flex-shrink: 0;

    .search-input-box {
      position: relative;
      display: flex;
      align-items: center;
      gap: 8px;

      :global(.search-icon) {
        position: absolute;
        left: 12px;
        color: $text-muted;
        pointer-events: none;
      }

      input {
        flex: 1;
        padding: 9px 36px 9px 38px;
        font-size: 13px;
        background: $bg-secondary;
        border: 1px solid $border-color;
        border-radius: $radius-sm;

        &:focus {
          border-color: $accent-color;
        }
      }

      .clear-input-btn {
        position: absolute;
        right: 90px;
        background: transparent;
        border: none;
        color: $text-muted;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 4px;
        border-radius: 50%;
        transition: all 0.12s ease;

        &:hover {
          background: rgba(255, 255, 255, 0.1);
          color: $text-primary;
        }
      }

      .search-btn {
        padding: 9px 18px;
        font-size: 13px;
      }
    }

    .source-tabs {
      display: flex;
      gap: 6px;

      .tab-btn {
        background: none;
        border: 1px solid $border-color;
        color: $text-secondary;
        padding: 4px 10px;
        font-size: 11.5px;
        border-radius: $radius-sm;
        cursor: pointer;
        transition: all 0.15s;

        &:hover {
          color: $text-primary;
          border-color: $border-light;
        }

        &.active {
          background: rgba(99, 102, 241, 0.15);
          border-color: $accent-color;
          color: $accent-light;
          font-weight: 500;
        }
      }
    }
  }

  .modal-body {
    flex: 1;
    overflow: hidden;
    display: flex;
    background: $bg-primary;
  }

  .state-view {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    color: $text-muted;
    font-size: 13px;
    padding: 30px;
    text-align: center;

    &.error {
      color: #f87171;
    }
  }

  .results-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    width: 100%;
    height: 100%;
    overflow: hidden;
  }

  .col-title {
    font-size: 11.5px;
    font-weight: 600;
    color: $text-secondary;
    padding: 10px 14px;
    border-bottom: 1px solid $border-color;
    background: $bg-secondary;
  }

  .candidates-col {
    display: flex;
    flex-direction: column;
    height: 100%;
    border-right: 1px solid $border-color;
    overflow: hidden;

    .candidate-list {
      flex: 1;
      overflow-y: auto;
      padding: 10px;
      display: flex;
      flex-direction: column;
      gap: 8px;
    }
  }

  .candidate-card {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px;
    background: $bg-secondary;
    border: 1px solid $border-color;
    border-radius: $radius-sm;
    cursor: pointer;
    transition: all 0.15s ease;

    &:hover {
      border-color: $border-light;
      background: rgba(255, 255, 255, 0.02);
    }

    &.selected {
      border-color: $accent-color;
      background: rgba(99, 102, 241, 0.08);

      .check-mark {
        color: $accent-color;
      }
    }

    .card-thumb {
      position: relative;
      width: 48px;
      height: 60px;
      background: #000;
      border-radius: 4px;
      overflow: hidden;
      flex-shrink: 0;
      display: flex;
      align-items: center;
      justify-content: center;
      color: $text-muted;

      img {
        width: 100%;
        height: 100%;
        object-fit: cover;
      }

      .thumb-video-badge {
        position: absolute;
        bottom: 2px;
        right: 2px;
        background: rgba(0, 0, 0, 0.75);
        color: #c084fc;
        border-radius: 3px;
        padding: 2px 3px;
        display: flex;
        align-items: center;
        justify-content: center;
        box-shadow: 0 1px 3px rgba(0, 0, 0, 0.4);
      }
    }

    .card-info {
      flex: 1;
      min-width: 0;
      display: flex;
      flex-direction: column;
      gap: 3px;

      .card-title-row {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 6px;

        .card-title {
          font-size: 13px;
          font-weight: 600;
          color: $text-primary;
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
        }

        .card-badges {
          display: flex;
          align-items: center;
          gap: 4px;
          flex-shrink: 0;

          .media-badge {
            display: inline-flex;
            align-items: center;
            gap: 3px;
            font-size: 9.5px;
            font-weight: 600;
            padding: 1px 5px;
            border-radius: 3px;

            &.img {
              background: rgba(59, 130, 246, 0.15);
              color: #93c5fd;
              border: 1px solid rgba(59, 130, 246, 0.3);
            }

            &.video {
              background: rgba(168, 85, 247, 0.15);
              color: #d8b4fe;
              border: 1px solid rgba(168, 85, 247, 0.3);
            }
          }

          .source-tag {
            font-size: 10px;
            padding: 1px 5px;
            border-radius: 3px;
            background: $bg-tertiary;
            color: $text-muted;
          }
        }
      }

      .card-meta {
        font-size: 11px;
        color: $text-secondary;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
      }

      .card-desc {
        font-size: 11px;
        color: $text-muted;
        display: -webkit-box;
        -webkit-line-clamp: 2;
        line-clamp: 2;
        -webkit-box-orient: vertical;
        overflow: hidden;
        line-height: 1.4;

      }
    }

    .check-mark {
      width: 20px;
      display: flex;
      align-items: center;
      justify-content: center;
      color: transparent;
    }
  }

  .preview-col {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow-y: auto;
    padding: 14px;
    gap: 14px;

    .preview-card {
      background: $bg-secondary;
      border: 1px solid $border-color;
      border-radius: $radius-md;
      padding: 14px;
      display: flex;
      flex-direction: column;
      gap: 12px;

      .preview-media-section {
        display: flex;
        flex-direction: column;
        gap: 8px;

        .media-tab-bar {
          display: flex;
          align-items: center;
          gap: 6px;
          background: rgba(0, 0, 0, 0.25);
          padding: 3px;
          border-radius: $radius-sm;
          border: 1px solid $border-color;

          .media-tab-btn {
            flex: 1;
            display: inline-flex;
            align-items: center;
            justify-content: center;
            gap: 5px;
            padding: 5px 8px;
            border-radius: 4px;
            font-size: 11.5px;
            font-weight: 500;
            background: transparent;
            border: 1px solid transparent;
            color: $text-secondary;
            cursor: pointer;
            transition: all 0.15s ease;

            &:hover {
              color: $text-primary;
              background: rgba(255, 255, 255, 0.04);
            }

            &.active {
              background: $bg-secondary;
              color: $text-primary;
              border-color: $border-light;
              box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
            }

            .tab-size-tag {
              font-size: 10px;
              padding: 0 4px;
              border-radius: 3px;
              background: rgba(59, 130, 246, 0.2);
              color: #93c5fd;

              &.video {
                background: rgba(168, 85, 247, 0.2);
                color: #d8b4fe;
              }
            }
          }
        }

        .preview-media-viewer {
          display: flex;
          align-items: center;
          justify-content: center;
          background: #090b10;
          border-radius: $radius-sm;
          overflow: hidden;
          min-height: 160px;
          max-height: 220px;
          border: 1px solid $border-color;

          .preview-video-player {
            width: 100%;
            max-height: 220px;
            background: #000;
            outline: none;
          }

          .preview-image-wrap {
            position: relative;
            width: 100%;
            height: 170px;
            display: flex;
            align-items: center;
            justify-content: center;

            img {
              max-height: 100%;
              max-width: 100%;
              object-fit: contain;
            }

            .preview-size-badge {
              position: absolute;
              bottom: 6px;
              right: 8px;
              display: inline-flex;
              align-items: center;
              gap: 4px;
              background: rgba(0, 0, 0, 0.75);
              backdrop-filter: blur(4px);
              color: #e2e8f0;
              font-size: 10px;
              font-weight: 600;
              padding: 2px 6px;
              border-radius: 4px;
              border: 1px solid rgba(255, 255, 255, 0.15);
            }
          }
        }
      }

      .preview-details {
        display: flex;
        flex-direction: column;
        gap: 8px;

        h4 {
          font-size: 15px;
          font-weight: 700;
        }

        .preview-tags {
          display: flex;
          flex-wrap: wrap;
          gap: 6px;

          .tag {
            font-size: 11px;
            display: inline-flex;
            align-items: center;
            gap: 4px;
            background: $bg-tertiary;
            padding: 3px 7px;
            border-radius: 4px;
            color: $text-secondary;

            &.rating {
              color: #fbbf24;
            }

            &.source {
              background: rgba(99, 102, 241, 0.18);
              color: $accent-light;
              font-weight: 600;
              border: 1px solid rgba(99, 102, 241, 0.3);
            }
          }
        }

        .preview-desc {
          font-size: 12px;
          color: $text-secondary;
          line-height: 1.5;
          max-height: 100px;
          overflow-y: auto;
        }
      }
    }

    .apply-options {
      background: $bg-secondary;
      border: 1px solid $border-color;
      border-radius: $radius-md;
      padding: 12px 14px;
      display: flex;
      flex-direction: column;
      gap: 10px;

      .options-header {
        font-size: 12px;
        font-weight: 600;
        color: $text-primary;
      }

      .checkbox-grid {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 8px;

        .check-label {
          display: flex;
          align-items: center;
          gap: 6px;
          font-size: 12px;
          color: $text-secondary;
          cursor: pointer;

          &.full {
            grid-column: 1 / -1;
            margin-top: 4px;
            font-weight: 500;
            color: $accent-light;

            &.video-highlight {
              color: #d8b4fe;
            }
          }

          .apply-label-content {
            display: flex;
            align-items: center;
            justify-content: space-between;
            width: 100%;

            .apply-size-pill {
              font-size: 10px;
              font-weight: 600;
              padding: 1px 6px;
              border-radius: 4px;

              &.img {
                background: rgba(59, 130, 246, 0.18);
                color: #93c5fd;
                border: 1px solid rgba(59, 130, 246, 0.3);
              }

              &.video {
                background: rgba(168, 85, 247, 0.18);
                color: #e9d5ff;
                border: 1px solid rgba(168, 85, 247, 0.3);
              }
            }
          }

          input {
            cursor: pointer;
          }
        }
      }
    }
  }

  .modal-footer {
    padding: 12px 20px;
    border-top: 1px solid $border-color;
    background: $bg-secondary;
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-shrink: 0;

    .applying-status {
      display: flex;
      align-items: center;
      gap: 8px;
      font-size: 12px;
      color: $accent-light;
    }

    .footer-actions {
      display: flex;
      align-items: center;
      gap: 8px;
      margin-left: auto;

      .apply-btn {
        display: flex;
        align-items: center;
        gap: 6px;
        font-size: 13px;
        padding: 8px 16px;
      }
    }
  }

  @keyframes zoom-in {
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
