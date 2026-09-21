<script lang="ts">
  import type { DeviceProfile, GameItem } from '../types';
  import { fetchRemoteImage, uploadGameImage, fetchRemoteVideo, uploadGameVideo } from '../api';
  import {
    Image as ImageIcon,
    Upload,
    Star,
    EyeOff,
    CheckCircle,
    AlertCircle,
    AlertTriangle,
    Calendar,
    Users,
    Tag,
    Building2,
    Loader2,
    Sparkles,
    Trash2,
    Film,
    Play,
  } from 'lucide-svelte';

  let {
    game = $bindable(null),
    device = null,
    systemId = null,
    romsPathOverride = undefined,
    onGameUpdated = () => {},
    onDeleteGame = () => {},
    onOpenScraper = () => {},
  } = $props<{
    game: GameItem | null;
    device: DeviceProfile | null;
    systemId: string | null;
    romsPathOverride?: string;
    onGameUpdated: () => void;
    onDeleteGame?: (path: string) => void;
    onOpenScraper?: () => void;
  }>();

  let previewImageUrl = $state<string | null>(null);
  let isLoadingImage = $state(false);
  let isUploadingImage = $state(false);

  let previewVideoUrl = $state<string | null>(null);
  let isLoadingVideo = $state(false);
  let isUploadingVideo = $state(false);

  let uploadMsg = $state('');
  let activeMediaTab = $state<'image' | 'video'>('image');

  let lastGamePath = $state<string | null>(null);

  function decodeHtml(text?: string | null): string {
    if (!text) return '';
    if (!text.includes('&')) return text;
    try {
      const doc = new DOMParser().parseFromString(text, 'text/html');
      return doc.documentElement.textContent || text;
    } catch {
      return text
        .replace(/&quot;/g, '"')
        .replace(/&#039;/g, "'")
        .replace(/&#39;/g, "'")
        .replace(/&amp;/g, '&')
        .replace(/&lt;/g, '<')
        .replace(/&gt;/g, '>');
    }
  }

  // Watch game change and load remote image / video if present
  $effect(() => {
    const currentPath = game?.path || null;
    if (currentPath !== lastGamePath) {
      lastGamePath = currentPath;
      previewImageUrl = null;
      previewVideoUrl = null;
      if (game) {
        if (game.name && game.name.includes('&')) game.name = decodeHtml(game.name);
        if (game.desc && game.desc.includes('&')) game.desc = decodeHtml(game.desc);
        if (game.developer && game.developer.includes('&')) game.developer = decodeHtml(game.developer);
        if (game.publisher && game.publisher.includes('&')) game.publisher = decodeHtml(game.publisher);
        if (game.genre && game.genre.includes('&')) game.genre = decodeHtml(game.genre);
      }
      if (game?.video && !game?.image) {
        activeMediaTab = 'video';
      }
    }

    if (game && device && systemId && game.image && game.image.trim() !== '') {
      if (!previewImageUrl && !isLoadingImage) {
        loadRemoteImage(game.image);
      }
    } else {
      previewImageUrl = null;
    }

    if (game && device && systemId && game.video && game.video.trim() !== '') {
      if (activeMediaTab === 'video' && !previewVideoUrl && !isLoadingVideo) {
        loadRemoteVideo(game.video);
      }
    } else {
      previewVideoUrl = null;
    }
  });

  async function loadRemoteImage(imagePath: string) {
    if (!device || !systemId) return;
    isLoadingImage = true;
    try {
      const dataUri = await fetchRemoteImage(device, systemId, imagePath, romsPathOverride);
      previewImageUrl = dataUri;
    } catch (err) {
      console.warn('이미지 로드 실패:', err);
      previewImageUrl = null;
    } finally {
      isLoadingImage = false;
    }
  }

  async function loadRemoteVideo(videoPath: string) {
    if (!device || !systemId) return;
    isLoadingVideo = true;
    try {
      const dataUri = await fetchRemoteVideo(device, systemId, videoPath, romsPathOverride);
      previewVideoUrl = dataUri;
    } catch (err) {
      console.warn('비디오 로드 실패:', err);
      previewVideoUrl = null;
    } finally {
      isLoadingVideo = false;
    }
  }

  function switchMediaTab(tab: 'image' | 'video') {
    activeMediaTab = tab;
    if (tab === 'video' && game?.video && !previewVideoUrl) {
      loadRemoteVideo(game.video);
    } else if (tab === 'image' && game?.image && !previewImageUrl) {
      loadRemoteImage(game.image);
    }
  }

  async function handleFileInput(e: Event) {
    const target = e.target as HTMLInputElement;
    const file = target.files?.[0];
    if (!file) return;
    await processImageUpload(file);
    target.value = '';
  }

  async function handleVideoFileInput(e: Event) {
    const target = e.target as HTMLInputElement;
    const file = target.files?.[0];
    if (!file) return;
    await processVideoUpload(file);
    target.value = '';
  }

  async function handleDrop(e: DragEvent) {
    e.preventDefault();
    const file = e.dataTransfer?.files?.[0];
    if (!file) return;
    if (file.type.startsWith('video/') || file.name.endsWith('.mp4') || file.name.endsWith('.webm')) {
      await processVideoUpload(file);
    } else if (file.type.startsWith('image/')) {
      await processImageUpload(file);
    }
  }

  async function processImageUpload(file: File) {
    if (!device || !systemId || !game) return;

    isUploadingImage = true;
    uploadMsg = '이미지 전송 중...';

    try {
      const reader = new FileReader();
      const base64Promise = new Promise<string>((resolve, reject) => {
        reader.onload = () => resolve(reader.result as string);
        reader.onerror = reject;
      });
      reader.readAsDataURL(file);
      const base64Data = await base64Promise;

      // Clean filename for remote save: e.g. "Game (USA)-image.png"
      const ext = file.name.split('.').pop() || 'png';
      let base = game.filename || '';
      const lastSlash = Math.max(base.lastIndexOf('/'), base.lastIndexOf('\\'));
      if (lastSlash >= 0) {
        base = base.substring(lastSlash + 1);
      }
      const baseName = base.substring(0, base.lastIndexOf('.')) || base;
      const remoteFileName = `${baseName}-image.${ext}`;

      const savedRelPath = await uploadGameImage(device, systemId, remoteFileName, base64Data, romsPathOverride);

      // Update game
      game.image = savedRelPath;
      if (game.status === 'unregistered') {
        game.status = 'registered';
      }
      previewImageUrl = base64Data;
      activeMediaTab = 'image';
      uploadMsg = '이미지 업로드 성공!';
      onGameUpdated();
    } catch (err: any) {
      console.error('이미지 업로드 실패:', err);
      uploadMsg = '업로드 실패: ' + (err?.message || err);
    } finally {
      isUploadingImage = false;
      setTimeout(() => (uploadMsg = ''), 3000);
    }
  }

  async function processVideoUpload(file: File) {
    if (!device || !systemId || !game) return;

    isUploadingVideo = true;
    uploadMsg = '비디오 전송 중 (용량에 따라 수 초 소요될 수 있습니다)...';

    try {
      const reader = new FileReader();
      const base64Promise = new Promise<string>((resolve, reject) => {
        reader.onload = () => resolve(reader.result as string);
        reader.onerror = reject;
      });
      reader.readAsDataURL(file);
      const base64Data = await base64Promise;

      const ext = file.name.split('.').pop() || 'mp4';
      let base = game.filename || '';
      const lastSlash = Math.max(base.lastIndexOf('/'), base.lastIndexOf('\\'));
      if (lastSlash >= 0) {
        base = base.substring(lastSlash + 1);
      }
      const baseName = base.substring(0, base.lastIndexOf('.')) || base;
      const remoteFileName = `${baseName}-video.${ext}`;

      const savedRelPath = await uploadGameVideo(device, systemId, remoteFileName, base64Data, romsPathOverride);

      game.video = savedRelPath;
      if (game.status === 'unregistered') {
        game.status = 'registered';
      }
      previewVideoUrl = base64Data;
      activeMediaTab = 'video';
      uploadMsg = '비디오 업로드 성공!';
      onGameUpdated();
    } catch (err: any) {
      console.error('비디오 업로드 실패:', err);
      uploadMsg = '비디오 업로드 실패: ' + (err?.message || err);
    } finally {
      isUploadingVideo = false;
      setTimeout(() => (uploadMsg = ''), 4000);
    }
  }

  function registerThisGame() {
    if (!game) return;
    game.status = 'registered';
    onGameUpdated();
  }

  // Date helper: converts "19911101T000000" -> "1991-11-01"
  function getDisplayDate(raw?: string | null): string {
    if (!raw) return '';
    const clean = raw.replace(/[^0-9]/g, '');
    if (clean.length >= 8) {
      return `${clean.substring(0, 4)}-${clean.substring(4, 6)}-${clean.substring(6, 8)}`;
    }
    if (clean.length >= 4) {
      return clean.substring(0, 4);
    }
    return raw;
  }

  function handleDateChange(e: Event) {
    if (!game) return;
    const val = (e.target as HTMLInputElement).value.trim();
    if (!val) {
      game.releasedate = null;
    } else {
      const digits = val.replace(/[^0-9]/g, '');
      if (digits.length === 8) {
        game.releasedate = `${digits}T000000`;
      } else if (digits.length === 4) {
        game.releasedate = `${digits}0101T000000`;
      } else {
        game.releasedate = val;
      }
    }
    onGameUpdated();
  }
</script>

<aside class="metadata-inspector" ondragover={(e) => e.preventDefault()} ondrop={handleDrop}>
  {#if !game}
    <div class="empty-selection">
      <div class="empty-icon-wrap">
        <ImageIcon size={36} />
      </div>
      <p>목록에서 게임을 선택하면<br />상세 메타데이터를 확인 및 편집할 수 있습니다.</p>
    </div>
  {:else}
    <!-- Top Fixed Area: Actions, Notices & Boxart Image -->
    <div class="inspector-fixed-top">
      <!-- Scrape Online Action Button -->

      <div class="scrape-action-box">
        <button class="btn-primary scrape-main-btn" onclick={onOpenScraper}>
          <Sparkles size={15} />
          <span>온라인 스크랩</span>
        </button>
        <span class="scrape-tip">ScreenScraper / 위키백과 / RAWG 검색</span>
      </div>

      <!-- Unregistered notice bar -->
      {#if game.status === 'unregistered'}
        <div class="unregistered-banner">
          <div class="banner-text">
            <AlertCircle size={15} />
            <span>이 파일은 아직 gamelist.xml에 등록되지 않았습니다.</span>
          </div>

          <button class="btn-primary mini-btn" onclick={registerThisGame}>
            <Sparkles size={13} /> 메타데이터로 등록
          </button>
        </div>
      {/if}

      <!-- Missing file (orphan metadata) notice bar -->
      {#if game.status === 'missing'}
        <div class="missing-banner">
          <div class="banner-text">
            <AlertTriangle size={15} />
            <span>실제 파일이 없는 미아 메타데이터입니다.</span>
          </div>
          <button class="btn-danger mini-btn" onclick={() => onDeleteGame(game.path)}>
            <Trash2 size={13} /> gamelist.xml에서 제거
          </button>
        </div>
      {/if}

      <!-- Boxart / Video Media Area -->
      <div class="media-preview-box">
        <div class="media-tabs-header">
          <button
            type="button"
            class="media-tab-btn"
            class:active={activeMediaTab === 'image'}
            onclick={() => switchMediaTab('image')}
          >
            <ImageIcon size={13} />
            <span>표지 이미지</span>
            {#if game.image}
              <span class="media-badge has-media">등록됨</span>
            {/if}
          </button>
          <button
            type="button"
            class="media-tab-btn video-tab"
            class:active={activeMediaTab === 'video'}
            onclick={() => switchMediaTab('video')}
          >
            <Film size={13} />
            <span>플레이 영상</span>
            {#if game.video}
              <span class="media-badge video-badge has-media">등록됨</span>
            {/if}
          </button>
        </div>

        {#if activeMediaTab === 'video'}
          <div class="video-wrapper">
            {#if isLoadingVideo}
              <div class="img-loading">
                <Loader2 class="spin" size={24} />
                <span>기기에서 비디오 로드 중...</span>
              </div>
            {:else if previewVideoUrl}
              <video
                src={previewVideoUrl}
                controls
                autoplay
                muted
                loop
                playsinline
                class="editor-video-player"
              >
                <track kind="captions" />
              </video>
            {:else if game.video}
              <div class="no-video">
                <Film size={32} />
                <span>영상을 재생할 수 없습니다</span>
                <span class="video-path-hint">{game.video}</span>
                <button
                  type="button"
                  class="btn-secondary mini-btn"
                  onclick={() => loadRemoteVideo(game.video!)}
                >
                  다시 시도
                </button>
              </div>
            {:else}
              <div class="no-video empty-video">
                <Film size={34} />
                <span class="empty-video-title">등록된 플레이 영상이 없습니다</span>
                <span class="empty-video-hint">PC에서 MP4 영상을 업로드하거나 온라인 스크랩으로 다운로드할 수 있습니다.</span>
                <div class="empty-video-btns">
                  <label class="btn-primary mini-btn empty-upload-label">
                    <Upload size={12} /> 동영상 업로드
                    <input
                      type="file"
                      accept="video/mp4,video/webm"
                      style="display: none;"
                      onchange={handleVideoFileInput}
                      disabled={isUploadingVideo}
                    />
                  </label>
                  <button type="button" class="btn-secondary mini-btn" onclick={onOpenScraper}>
                    <Sparkles size={12} /> 스크랩 검색
                  </button>
                </div>
              </div>
            {/if}
          </div>
        {:else}
          <div class="image-wrapper">
            {#if isLoadingImage}
              <div class="img-loading">
                <Loader2 class="spin" size={24} />
                <span>이미지 로드 중...</span>
              </div>
            {:else if previewImageUrl}
              <img src={previewImageUrl} alt={game.name} />
            {:else}
              <div class="no-image">
                <ImageIcon size={36} />
                <span>이미지 없음 (드래그하여 등록)</span>
              </div>
            {/if}
          </div>
        {/if}

        <div class="image-actions">
          <div class="action-btn-group">
            {#if activeMediaTab === 'image'}
              <label class="btn-secondary upload-btn">
                {#if isUploadingImage}
                  <Loader2 class="spin" size={13} /> 전송 중...
                {:else}
                  <Upload size={13} /> 이미지 교체 / 업로드
                {/if}
                <input
                  type="file"
                  accept="image/*"
                  style="display: none;"
                  onchange={handleFileInput}
                  disabled={isUploadingImage}
                />
              </label>
              {#if game.image}
                <button
                  type="button"
                  class="btn-secondary upload-btn unlink-media-btn"
                  onclick={() => {
                    if (!game) return;
                    game.image = null;
                    previewImageUrl = null;
                    onGameUpdated();
                  }}
                  title="이미지 연결 해제"
                >
                  연결 해제
                </button>
              {/if}
            {:else}
              <label class="btn-secondary upload-btn video-upload-btn">
                {#if isUploadingVideo}
                  <Loader2 class="spin" size={13} /> 전송 중...
                {:else}
                  <Upload size={13} /> 동영상 교체 / 업로드
                {/if}
                <input
                  type="file"
                  accept="video/mp4,video/webm"
                  style="display: none;"
                  onchange={handleVideoFileInput}
                  disabled={isUploadingVideo}
                />
              </label>
              {#if game.video}
                <button
                  type="button"
                  class="btn-secondary upload-btn unlink-media-btn"
                  onclick={() => {
                    if (!game) return;
                    game.video = null;
                    previewVideoUrl = null;
                    onGameUpdated();
                  }}
                  title="동영상 연결 해제"
                >
                  연결 해제
                </button>
              {/if}
            {/if}
          </div>
          {#if uploadMsg}
            <span class="upload-status">{uploadMsg}</span>
          {/if}
        </div>
      </div>
    </div>

    <!-- Scrollable ROM Metadata Form Section -->
    <div class="inspector-form-scroll">
      <div class="form-body">

        <div class="field">
          <label for="f-name">게임 제목 (Name)</label>
          <input
            id="f-name"
            type="text"
            bind:value={game.name}
            oninput={onGameUpdated}
            placeholder="게임의 표시 이름"
          />
        </div>

        <div class="field">
          <label for="f-file">ROM 파일 및 위치</label>
          <div id="f-file" class="readonly-field file-field-wrap">
            <span class="pure-file">{game.filename}</span>
            {#if game.path && game.path.replace(/^\.\//, '') !== game.filename}
              <span class="subpath-label" title="기기 상대 경로">📁 {game.path}</span>
            {/if}
          </div>
        </div>

        {#if game.video}
          <div class="field">
            <label for="f-video"><Film size={13} /> 미리보기 영상 경로 (Video)</label>
            <div class="video-field-wrap">
              <input
                id="f-video"
                type="text"
                bind:value={game.video}
                oninput={onGameUpdated}
                placeholder="예: ./videos/game-video.mp4"
              />
              <button
                type="button"
                class="mini-btn-remove"
                title="동영상 연결 해제"
                onclick={() => { game.video = null; previewVideoUrl = null; activeMediaTab = 'image'; onGameUpdated(); }}
              >
                연결 해제
              </button>
            </div>
          </div>
        {/if}

        <div class="field">
          <label for="f-desc">게임 설명 (Description)</label>
          <textarea
            id="f-desc"
            bind:value={game.desc}
            oninput={onGameUpdated}
            placeholder="스토리, 특징, 조작법 등 설명..."
            rows="3"
          ></textarea>
        </div>

        <div class="field">
          <label for="f-genre"><Tag size={13} /> 장르 (Genre)</label>
          <input
            id="f-genre"
            type="text"
            bind:value={game.genre}
            oninput={onGameUpdated}
            placeholder="예: Role Playing Game, Action"
          />
        </div>

        <div class="grid-2">
          <div class="field">
            <label for="f-date"><Calendar size={13} /> 출시일 (YYYY-MM-DD)</label>
            <input
              id="f-date"
              type="text"
              value={getDisplayDate(game.releasedate)}
              oninput={handleDateChange}
              placeholder="1991-11-01"
            />
          </div>

          <div class="field">
            <label for="f-rating"><Star size={13} /> 평점 (0.0 ~ 1.0)</label>
            <input
              id="f-rating"
              type="number"
              step="0.05"
              min="0"
              max="1"
              bind:value={game.rating}
              oninput={onGameUpdated}
              placeholder="0.75"
            />
          </div>
        </div>

        <div class="grid-2">
          <div class="field">
            <label for="f-dev"><Building2 size={13} /> 개발사 (Developer)</label>
            <input
              id="f-dev"
              type="text"
              bind:value={game.developer}
              oninput={onGameUpdated}
              placeholder="제작사"
            />
          </div>

          <div class="field">
            <label for="f-pub"><Building2 size={13} /> 배급사 (Publisher)</label>
            <input
              id="f-pub"
              type="text"
              bind:value={game.publisher}
              oninput={onGameUpdated}
              placeholder="유통사"
            />
          </div>
        </div>

        <div class="field">
          <label for="f-players"><Users size={13} /> 플레이 인원 (Players)</label>
          <input
            id="f-players"
            type="text"
            bind:value={game.players}
            oninput={onGameUpdated}
            placeholder="예: 1 또는 1-2"
          />
        </div>

        <div class="toggles-row">
          <label class="toggle-card" class:checked={game.favorite}>
            <input
              type="checkbox"
              bind:checked={game.favorite}
              onchange={onGameUpdated}
            />
            <Star size={15} class={game.favorite ? 'star-filled' : ''} />
            <span>즐겨찾기</span>
          </label>

          <label class="toggle-card" class:checked={game.hidden}>
            <input
              type="checkbox"
              bind:checked={game.hidden}
              onchange={onGameUpdated}
            />
            <EyeOff size={15} />
            <span>숨김 처리</span>
          </label>
        </div>
      </div>
    </div>
  {/if}
</aside>

<style lang="scss">
  @use '../../styles/variables' as *;

  .metadata-inspector {
    width: 390px;
    background: $bg-secondary;
    border-left: 1px solid $border-color;
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    height: 100%;
    overflow: hidden;
  }

  .empty-selection {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    padding: 30px;
    text-align: center;
    color: $text-muted;
    gap: 16px;

    .empty-icon-wrap {
      width: 64px;
      height: 64px;
      border-radius: $radius-full;
      background: $bg-tertiary;
      display: flex;
      align-items: center;
      justify-content: center;
      color: $text-disabled;
    }

    p {
      font-size: 13px;
      line-height: 1.6;
    }
  }

  .inspector-fixed-top {
    flex-shrink: 0;
    padding: 14px 16px 12px 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    border-bottom: 1px solid $border-color;
    background: $bg-secondary;
  }

  .inspector-form-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 14px 16px 24px 16px;
    display: flex;
    flex-direction: column;
  }

  .scrape-action-box {

    display: flex;
    flex-direction: column;
    gap: 5px;

    .scrape-main-btn {
      width: 100%;
      padding: 10px 14px;
      font-size: 13px;
      font-weight: 600;
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 8px;
      background: linear-gradient(135deg, #6366f1, #8b5cf6);
      box-shadow: 0 2px 10px rgba(99, 102, 241, 0.3);
      cursor: pointer;
      border: none;
      border-radius: $radius-sm;
      color: white;
      transition: all 0.15s ease;

      &:hover {
        background: linear-gradient(135deg, #4f46e5, #7c3aed);
        box-shadow: 0 4px 14px rgba(99, 102, 241, 0.45);
        transform: translateY(-1px);
      }
    }

    .scrape-tip {
      font-size: 11px;
      color: $text-muted;
      text-align: center;
    }
  }

  .unregistered-banner {

    background: rgba(245, 158, 11, 0.1);
    border: 1px solid rgba(245, 158, 11, 0.3);
    border-radius: $radius-md;
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;

    .banner-text {
      display: flex;
      align-items: center;
      gap: 6px;
      font-size: 11.5px;
      color: #fbbf24;
      font-weight: 500;
    }

    .mini-btn {
      padding: 4px 8px;
      font-size: 11.5px;
      align-self: flex-start;
    }
  }

  .missing-banner {
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.3);
    border-radius: $radius-md;
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;

    .banner-text {
      display: flex;
      align-items: center;
      gap: 6px;
      font-size: 11.5px;
      color: #f87171;
      font-weight: 500;
    }

    .mini-btn {
      padding: 4px 8px;
      font-size: 11.5px;
      align-self: flex-start;
    }
  }

  .media-preview-box {
    background: $bg-primary;
    border: 1px solid $border-color;
    border-radius: $radius-md;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    flex-shrink: 0;

    .media-tabs-header {
      display: flex;
      background: $bg-secondary;
      border-bottom: 1px solid $border-color;
      padding: 4px;
      gap: 4px;

      .media-tab-btn {
        flex: 1;
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 6px;
        padding: 5px 8px;
        font-size: 11.5px;
        font-weight: 500;
        background: transparent;
        border: 1px solid transparent;
        border-radius: $radius-sm;
        color: $text-secondary;
        cursor: pointer;
        transition: all 0.15s ease;

        &:hover {
          color: $text-primary;
          background: rgba(255, 255, 255, 0.04);
        }

        &.active {
          background: $bg-tertiary;
          color: $text-primary;
          border-color: $border-light;
          font-weight: 600;
        }

        &.video-tab.active {
          color: #d8b4fe;
          border-color: rgba(168, 85, 247, 0.35);
          background: rgba(168, 85, 247, 0.12);
        }

        .media-badge {
          font-size: 10px;
          padding: 1px 5px;
          border-radius: 10px;
          background: rgba(255, 255, 255, 0.08);
          color: $text-muted;

          &.has-media {
            background: rgba(16, 185, 129, 0.15);
            color: #34d399;
          }

          &.video-badge.has-media {
            background: rgba(168, 85, 247, 0.2);
            color: #c084fc;
          }
        }
      }
    }

    .video-wrapper {
      height: 220px;
      background: #000;
      display: flex;
      align-items: center;
      justify-content: center;
      position: relative;
      overflow: hidden;
      flex-shrink: 0;

      .editor-video-player {
        width: 100%;
        height: 100%;
        max-height: 220px;
        object-fit: contain;
        background: #000;
        outline: none;
      }

      .no-video {
        display: flex;
        flex-direction: column;
        align-items: center;
        text-align: center;
        padding: 16px;
        gap: 6px;
        color: $text-muted;
        font-size: 12px;

        .video-path-hint {
          font-size: 10.5px;
          color: $text-secondary;
          word-break: break-all;
          max-width: 90%;
        }

        &.empty-video {
          gap: 6px;

          .empty-video-title {
            font-size: 12.5px;
            font-weight: 500;
            color: $text-secondary;
          }

          .empty-video-hint {
            font-size: 11px;
            color: $text-muted;
            max-width: 240px;
            line-height: 1.4;
          }

          .empty-video-btns {
            display: flex;
            align-items: center;
            gap: 8px;
            margin-top: 6px;

            .empty-upload-label {
              cursor: pointer;
            }
          }
        }

        .mini-btn {
          padding: 4px 8px;
          font-size: 11px;
        }
      }
    }

    .image-wrapper {
      height: 220px;
      background: #090b10;
      display: flex;
      align-items: center;
      justify-content: center;
      position: relative;
      overflow: hidden;
      flex-shrink: 0;

      img {
        max-width: 100%;
        max-height: 100%;
        object-fit: contain;
      }

      .img-loading, .no-image {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 8px;
        color: $text-muted;
        font-size: 12px;
      }
    }

    .image-actions {
      padding: 8px 10px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      background: $bg-tertiary;
      border-top: 1px solid $border-color;

      .action-btn-group {
        display: flex;
        align-items: center;
        gap: 6px;
      }

      .upload-btn {
        padding: 5px 10px;
        font-size: 11.5px;
        cursor: pointer;
        display: inline-flex;
        align-items: center;
        gap: 5px;
      }

      .video-upload-btn {
        background: rgba(168, 85, 247, 0.12);
        color: #d8b4fe;
        border-color: rgba(168, 85, 247, 0.3);

        &:hover {
          background: rgba(168, 85, 247, 0.2);
          color: #f3e8ff;
        }
      }

      .unlink-media-btn {
        font-size: 11px;
        padding: 5px 8px;
        color: #f87171;
        border-color: rgba(239, 68, 68, 0.25);
        background: transparent;

        &:hover {
          background: rgba(239, 68, 68, 0.12);
          color: #fca5a5;
        }
      }

      .upload-status {
        font-size: 11px;
        color: #34d399;
      }
    }
  }

  .form-body {
    display: flex;
    flex-direction: column;
    gap: 12px;

    .field {
      display: flex;
      flex-direction: column;
      gap: 4px;

      .video-field-wrap {
        display: flex;
        align-items: center;
        gap: 6px;

        input {
          flex: 1;
        }

        .mini-btn-remove {
          padding: 4px 8px;
          font-size: 11px;
          color: #f87171;
          white-space: nowrap;
          background: transparent;
          border: 1px solid rgba(239, 68, 68, 0.3);
          border-radius: $radius-sm;
          cursor: pointer;

          &:hover {
            background: rgba(239, 68, 68, 0.15);
          }
        }
      }

      label {
        font-size: 11.5px;
        color: $text-secondary;
        font-weight: 600;
        display: inline-flex;
        align-items: center;
        gap: 5px;
        letter-spacing: -0.1px;

        :global(svg) {
          color: $text-muted;
          flex-shrink: 0;
        }
      }

      input, textarea {
        font-size: 12.5px;
        padding: 7px 10px;
        width: 100%;
        background-color: $bg-primary;
        border: 1px solid $border-color;
        border-radius: $radius-sm;

        &:focus {
          border-color: $accent-color;
          box-shadow: 0 0 0 2px $accent-glow;
        }
      }

      .readonly-field {
        font-size: 11px;
        font-family: $font-mono;
        background: $bg-primary;
        padding: 7px 10px;
        border-radius: $radius-sm;
        color: $text-muted;
        border: 1px solid $border-color;
        word-break: break-all;

        &.file-field-wrap {
          display: flex;
          flex-direction: column;
          gap: 4px;

          .pure-file {
            color: $text-primary;
            font-weight: 500;
          }

          .subpath-label {
            font-size: 10px;
            color: $accent-light;
            opacity: 0.85;
          }
        }
      }
    }

    .grid-2 {
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 10px;
    }

    .toggles-row {
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 10px;
      margin-top: 4px;

      .toggle-card {
        display: flex;
        align-items: center;
        gap: 8px;
        background: $bg-primary;
        border: 1px solid $border-color;
        padding: 8px 12px;
        border-radius: $radius-sm;
        cursor: pointer;
        font-size: 12px;
        font-weight: 500;
        color: $text-secondary;
        transition: all 0.15s ease;

        input {
          display: none;
        }

        &:hover {
          border-color: $border-light;
        }

        &.checked {
          background: rgba(99, 102, 241, 0.12);
          border-color: $accent-color;
          color: white;

          :global(.star-filled) {
            color: #f43f5e;
            fill: #f43f5e;
          }
        }
      }
    }
  }
</style>
