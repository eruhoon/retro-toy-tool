<script lang="ts">
  import { DEVICE_PRESETS, type DevicePreset, type DeviceProfile, type ScraperSettings, type ScreenScraperAccountStatus, type StorageLocation } from '../types';
  import { testConnection, testScreenscraperAccount } from '../api';
  import { saveScraperSettings } from '../storage';
  import {
    CheckCircle,
    AlertCircle,
    Loader2,
    X,
    Plus,
    Trash2,
    Cpu,
    RefreshCw,
    HardDrive,
    Smartphone,
    Globe,
    Key,
    User,
    Eye,
    EyeOff,
    Settings,
    Gamepad2,
  } from 'lucide-svelte';

  let {
    isOpen = $bindable(false),
    activeTab = $bindable<'device' | 'scraper'>('device'),
    profiles = $bindable([]),
    activeDevice = $bindable(null),
    settings = $bindable(),
    onDeviceConnected = () => {},
    onSavedScraperSettings = () => {},
  } = $props<{
    isOpen: boolean;
    activeTab?: 'device' | 'scraper';
    profiles: DeviceProfile[];
    activeDevice: DeviceProfile | null;
    settings?: ScraperSettings;
    onDeviceConnected?: (device: DeviceProfile) => void;
    onSavedScraperSettings?: () => void;
  }>();

  // ------------------------------------
  // Tab 1: Device Management State
  // ------------------------------------
  let selectedPreset = $state<DevicePreset['id']>('knulli');
  let name = $state('내 Knulli 게임기');
  let host = $state('');
  let port = $state(22);
  let username = $state('root');
  let password = $state('linux');
  let romsPath = $state('/userdata/roms');

  let detectedStorages = $state<StorageLocation[]>([]);
  let isTestingDevice = $state(false);
  let testDeviceSuccess = $state<boolean | null>(null);
  let testDeviceMsg = $state('');
  let editingId = $state<string | null>(null);

  let romPathHint = $derived.by(() => {
    switch (selectedPreset) {
      case 'rocknix':
        return 'ROCKNIX는 보통 /storage/roms(내장) 또는 /media/.../roms(외장) 입니다.';
      case 'batocera':
        return 'Batocera는 보통 /userdata/roms(내장) 또는 /media/.../roms(외장) 입니다.';
      case 'retropie':
        return 'RetroPie는 보통 /home/pi/RetroPie/roms(내장) 또는 /media/usb0/RetroPie/roms(외장) 입니다.';
      case 'custom':
        return '기기의 EmulationStation ROMs 디렉토리 절대 경로를 입력하세요. (예: /roms, /storage/roms)';
      case 'knulli':
      default:
        return 'Knulli는 보통 /userdata/roms(내장) 또는 /userdata/mount/.../roms(외장) 입니다.';
    }
  });

  function applyPreset(presetId: DevicePreset['id']) {
    selectedPreset = presetId;
    const p = DEVICE_PRESETS.find((x) => x.id === presetId);
    if (p) {
      if (!editingId) {
        name = p.name;
      }
      port = p.defaultPort;
      username = p.defaultUser;
      password = p.defaultPass;
      romsPath = p.defaultRomsPath;
    }
  }

  function startNewProfile() {
    editingId = null;
    applyPreset('knulli');
    name = '새 Knulli 기기';
    host = '';
    testDeviceSuccess = null;
    testDeviceMsg = '';
    detectedStorages = [];
  }

  function editProfile(p: DeviceProfile) {
    editingId = p.id;
    name = p.name;
    selectedPreset = p.device_type;
    host = p.host;
    port = p.port;
    username = p.username;
    password = p.password;
    romsPath = p.roms_path;
    testDeviceSuccess = null;
    testDeviceMsg = '';
    detectedStorages = [];
  }

  function deleteProfile(id: string) {
    profiles = profiles.filter((p: DeviceProfile) => p.id !== id);
    if (editingId === id) {
      startNewProfile();
    }
    if (activeDevice?.id === id) {
      activeDevice = null;
    }
  }

  async function handleTestConnection() {
    if (!host.trim()) {
      testDeviceSuccess = false;
      testDeviceMsg = 'IP 주소를 입력하세요.';
      return;
    }
    isTestingDevice = true;
    testDeviceSuccess = null;
    testDeviceMsg = '';

    try {
      const res = await testConnection(host, port, username, password, romsPath);
      if (res.success) {
        testDeviceSuccess = true;
        detectedStorages = res.storages || [];
        if (res.detected_roms_path && res.detected_roms_path !== romsPath) {
          romsPath = res.detected_roms_path;
          testDeviceMsg = `연결 성공! 기기의 ROM 경로(${res.os_name || ''})를 자동으로 감지했습니다: ${res.detected_roms_path}`;
        } else {
          testDeviceMsg = `연결 성공! (${res.detected_roms_path || romsPath})`;
        }
      } else {
        testDeviceSuccess = false;
        testDeviceMsg = res.message;
        detectedStorages = [];
      }
    } catch (err: any) {
      testDeviceSuccess = false;
      testDeviceMsg = typeof err === 'string' ? err : err?.message || '연결 실패';
      detectedStorages = [];
    } finally {
      isTestingDevice = false;
    }
  }

  function handleSaveAndConnect() {
    if (!host.trim() || !name.trim()) return;

    const device: DeviceProfile = {
      id: editingId || 'dev_' + Date.now(),
      name: name.trim(),
      device_type: selectedPreset,
      host: host.trim(),
      port,
      username: username.trim(),
      password,
      roms_path: romsPath.trim() || '/userdata/roms',
    };

    if (editingId) {
      profiles = profiles.map((p: DeviceProfile) => (p.id === editingId ? device : p));
    } else {
      profiles = [...profiles, device];
    }

    activeDevice = device;
    isOpen = false;
    onDeviceConnected(device);
  }

  // ------------------------------------
  // Tab 2: Scraper Settings State
  // ------------------------------------
  let scraperUsername = $state('');
  let scraperPassword = $state('');
  let defaultSource = $state<'auto' | 'screenscraper' | 'steam' | 'dlsite' | 'wikipedia' | 'rawg'>('auto');
  let showScraperPassword = $state(false);

  let isTestingScraper = $state(false);
  let testScraperResult = $state<ScreenScraperAccountStatus | null>(null);
  let saveScraperSuccessMsg = $state('');

  // Sync scraper state when modal opens
  $effect(() => {
    if (isOpen && settings) {
      scraperUsername = settings.screenscraper_username;
      scraperPassword = settings.screenscraper_password;
      defaultSource = settings.default_source || 'auto';
      testScraperResult = null;
      saveScraperSuccessMsg = '';
    }
  });

  async function handleTestScraperAccount() {
    if (!scraperUsername.trim() || !scraperPassword.trim()) {
      testScraperResult = {
        valid: false,
        message: 'ScreenScraper 아이디와 비밀번호를 모두 입력해주세요.',
      };
      return;
    }

    isTestingScraper = true;
    testScraperResult = null;
    try {
      const res = await testScreenscraperAccount(scraperUsername.trim(), scraperPassword.trim());
      testScraperResult = res;
    } catch (err: any) {
      testScraperResult = {
        valid: false,
        message: '연결 실패: ' + (err?.message || err),
      };
    } finally {
      isTestingScraper = false;
    }
  }

  function handleSaveScraper() {
    if (!settings) return;
    settings.screenscraper_username = scraperUsername.trim();
    settings.screenscraper_password = scraperPassword.trim();
    settings.default_source = defaultSource;
    saveScraperSettings(settings);

    saveScraperSuccessMsg = '스크래퍼 설정이 안전하게 저장되었습니다.';
    onSavedScraperSettings();
    setTimeout(() => {
      isOpen = false;
    }, 600);
  }
</script>

{#if isOpen}
  <div class="modal-backdrop" onclick={() => (isOpen = false)} role="presentation">
    <div class="settings-dialog" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
      <!-- Modal Header with integrated Tabs -->
      <header class="dialog-header">
        <div class="header-left">
          <div class="settings-title">
            <Settings size={20} class="title-icon" />
            <h2>환경 설정</h2>
          </div>

          <nav class="nav-tabs" aria-label="설정 탭">
            <button
              type="button"
              class="nav-tab-btn"
              class:active={activeTab === 'device'}
              onclick={() => (activeTab = 'device')}
            >
              <Gamepad2 size={16} />
              <span>게임기 연결 ({profiles.length})</span>
            </button>

            <button
              type="button"
              class="nav-tab-btn"
              class:active={activeTab === 'scraper'}
              onclick={() => (activeTab = 'scraper')}
            >
              <Globe size={16} />
              <span>온라인 스크래퍼</span>
            </button>
          </nav>
        </div>

        <button class="close-btn" onclick={() => (isOpen = false)} title="닫기">
          <X size={18} />
        </button>
      </header>

      <!-- Modal Body -->
      <div class="dialog-body">
        {#if activeTab === 'device'}
          <!-- 1. DEVICE SETTINGS TAB -->
          <div class="device-tab-content">
            <!-- Left: Device List -->
            <div class="device-sidebar">
              <div class="sidebar-top">
                <h3>등록된 기기 ({profiles.length})</h3>
                <button class="btn-secondary sm-btn" onclick={startNewProfile}>
                  <Plus size={13} /> 새 기기
                </button>
              </div>

              <div class="device-list-scroll">
                {#if profiles.length === 0}
                  <div class="empty-hint">등록된 기기가 없습니다. 우측에서 정보를 입력하고 연결해보세요.</div>
                {:else}
                  {#each profiles as p}
                    <div
                      class="device-card"
                      class:active={editingId === p.id}
                      class:connected={activeDevice?.id === p.id}
                      onclick={() => editProfile(p)}
                      role="button"
                      tabindex="0"
                    >
                      <div class="device-info">
                        <div class="device-name">
                          {p.name}
                          {#if activeDevice?.id === p.id}
                            <span class="badge-connected">연결됨</span>
                          {/if}
                        </div>
                        <div class="device-sub">{p.host}:{p.port} ({p.device_type})</div>
                      </div>
                      <button
                        class="delete-btn"
                        title="기기 삭제"
                        onclick={(e) => {
                          e.stopPropagation();
                          deleteProfile(p.id);
                        }}
                      >
                        <Trash2 size={13} />
                      </button>
                    </div>
                  {/each}
                {/if}
              </div>
            </div>

            <!-- Right: Device Configuration Form -->
            <div class="device-form-container">
              <div class="form-scroll">
                <div class="preset-section">
                  <label class="section-label">기기 OS 프리셋 선택</label>
                  <div class="preset-pills">
                    {#each DEVICE_PRESETS as preset}
                      <button
                        type="button"
                        class="preset-pill"
                        class:selected={selectedPreset === preset.id}
                        onclick={() => applyPreset(preset.id)}
                      >
                        {preset.name}
                      </button>
                    {/each}
                  </div>
                </div>

                <div class="form-grid">
                  <div class="field full">
                    <label for="dev-name">기기 별명 (프로필 이름)</label>
                    <input id="dev-name" type="text" bind:value={name} placeholder="예: 거실용 Knulli RG40XX" />
                  </div>

                  <div class="field ip-field">
                    <label for="dev-host">IP 주소 / 호스트</label>
                    <input id="dev-host" type="text" bind:value={host} placeholder="예: 192.168.0.50 또는 knulli.local" />
                  </div>

                  <div class="field port-field">
                    <label for="dev-port">SSH 포트</label>
                    <input id="dev-port" type="number" bind:value={port} min="1" max="65535" />
                  </div>

                  <div class="field">
                    <label for="dev-user">아이디 (User)</label>
                    <input id="dev-user" type="text" bind:value={username} placeholder="root" />
                  </div>

                  <div class="field">
                    <label for="dev-pass">비밀번호 (Password)</label>
                    <input id="dev-pass" type="password" bind:value={password} placeholder="기본 비밀번호" />
                  </div>

                  <div class="field full">
                    <label for="dev-roms">ROMs 디렉토리 경로 (EmulationStation/ES-DE)</label>
                    <input id="dev-roms" type="text" bind:value={romsPath} placeholder="/userdata/roms 또는 /storage/roms" />
                    <span class="field-hint">{romPathHint}</span>

                    {#if detectedStorages.length > 0}
                      <div class="detected-storages-list">
                        <span class="detected-label">감지된 저장소 (클릭하여 선택):</span>
                        <div class="detected-pills">
                          {#each detectedStorages as st}
                            <button
                              type="button"
                              class="detected-pill-btn"
                              class:selected={romsPath === st.path}
                              onclick={() => (romsPath = st.path)}
                              title={st.path}
                            >
                              {#if st.storage_type === 'external'}
                                <HardDrive size={13} />
                              {:else}
                                <Smartphone size={13} />
                              {/if}
                              <span>{st.label}</span>
                            </button>
                          {/each}
                        </div>
                      </div>
                    {/if}
                  </div>
                </div>

                {#if testDeviceSuccess !== null || isTestingDevice}
                  <div
                    class="test-result-box"
                    class:success={testDeviceSuccess === true}
                    class:error={testDeviceSuccess === false}
                    class:loading={isTestingDevice}
                  >
                    {#if isTestingDevice}
                      <Loader2 class="spin" size={16} />
                      <span>게임기에 연결을 시도하고 ROM 경로를 탐색하는 중...</span>
                    {:else if testDeviceSuccess === true}
                      <CheckCircle size={16} />
                      <span>{testDeviceMsg}</span>
                    {:else}
                      <AlertCircle size={16} />
                      <span>{testDeviceMsg}</span>
                    {/if}
                  </div>
                {/if}
              </div>

              <!-- Footer for Device Tab -->
              <footer class="tab-footer">
                <button
                  type="button"
                  class="btn-secondary"
                  disabled={isTestingDevice || !host.trim()}
                  onclick={handleTestConnection}
                >
                  {#if isTestingDevice}
                    <Loader2 class="spin" size={14} /> 테스트 중...
                  {:else}
                    <RefreshCw size={14} /> 연결 테스트 & 경로 탐색
                  {/if}
                </button>

                <button
                  type="button"
                  class="btn-primary"
                  disabled={isTestingDevice || !host.trim() || !name.trim()}
                  onclick={handleSaveAndConnect}
                >
                  저장하고 게임기 접속
                </button>
              </footer>
            </div>
          </div>
        {:else if activeTab === 'scraper'}
          <!-- 2. SCRAPER SETTINGS TAB -->
          <div class="scraper-tab-content">
            <div class="scraper-scroll">
              <!-- ScreenScraper Account Section -->
              <section class="section-box">
                <div class="section-title">
                  <span class="badge-source">공식 표준 DB</span>
                  <h4>ScreenScraper.fr 계정 연동</h4>
                </div>
                <p class="section-desc">
                  Batocera, Knulli 등 리눅스 에뮬레이터 표준 데이터베이스입니다. 계정(무료)이 없더라도 위키백과 및 공개 DB를 통해 검색할 수 있습니다.
                </p>

                <div class="field-grid">
                  <div class="field">
                    <label for="ss-user">ScreenScraper 아이디 (User ID)</label>
                    <div class="input-with-icon">
                      <span class="left-icon"><User size={15} /></span>
                      <input
                        id="ss-user"
                        type="text"
                        bind:value={scraperUsername}
                        placeholder="예: retro_gamer"
                      />
                    </div>
                  </div>

                  <div class="field">
                    <label for="ss-pass">비밀번호 (Password)</label>
                    <div class="input-with-icon">
                      <span class="left-icon"><Key size={15} /></span>
                      <input
                        id="ss-pass"
                        type={showScraperPassword ? 'text' : 'password'}
                        bind:value={scraperPassword}
                        placeholder="비밀번호 입력"
                      />
                      <button
                        type="button"
                        class="toggle-eye"
                        onclick={() => (showScraperPassword = !showScraperPassword)}
                        title={showScraperPassword ? '비밀번호 숨기기' : '비밀번호 보기'}
                      >
                        {#if showScraperPassword}
                          <EyeOff size={14} />
                        {:else}
                          <Eye size={14} />
                        {/if}
                      </button>
                    </div>
                  </div>
                </div>

                <div class="test-row">
                  <button
                    type="button"
                    class="btn-secondary test-btn"
                    onclick={handleTestScraperAccount}
                    disabled={isTestingScraper}
                  >
                    {#if isTestingScraper}
                      <Loader2 size={13} class="spin" /> 계정 확인 중...
                    {:else}
                      ScreenScraper 계정 연결 테스트
                    {/if}
                  </button>

                  {#if testScraperResult}
                    <div class="test-feedback" class:success={testScraperResult.valid} class:error={!testScraperResult.valid}>
                      {#if testScraperResult.valid}
                        <CheckCircle size={14} />
                      {:else}
                        <AlertCircle size={14} />
                      {/if}
                      <span>{testScraperResult.message}</span>
                    </div>
                  {/if}
                </div>
              </section>

              <!-- Default Provider Selection -->
              <section class="section-box">
                <div class="section-title">
                  <h4>기본 검색 모드</h4>
                </div>
                <div class="source-pills">
                  <label class="source-pill" class:active={defaultSource === 'auto'}>
                    <input type="radio" bind:group={defaultSource} value="auto" />
                    <span class="pill-name">자동 통합 검색 (권장)</span>
                    <span class="pill-sub">ScreenScraper + Steam + DLsite + 위키백과 + RAWG</span>
                  </label>

                  <label class="source-pill" class:active={defaultSource === 'screenscraper'}>
                    <input type="radio" bind:group={defaultSource} value="screenscraper" />
                    <span class="pill-name">ScreenScraper 전용</span>
                    <span class="pill-sub">에뮬레이터 공식 DB 우선 검색</span>
                  </label>

                  <label class="source-pill" class:active={defaultSource === 'steam'}>
                    <input type="radio" bind:group={defaultSource} value="steam" />
                    <span class="pill-name">Steam Store (스팀)</span>
                    <span class="pill-sub">스팀 정식 타이틀, 한글 설명, 600x900 세로 포스터 표지</span>
                  </label>

                  <label class="source-pill" class:active={defaultSource === 'dlsite'}>
                    <input type="radio" bind:group={defaultSource} value="dlsite" />
                    <span class="pill-name">DLsite (동인/인디)</span>
                    <span class="pill-sub">RJ 품번 자동 인식 및 일어/한국어/영문판 동시 통합 검색</span>
                  </label>

                  <label class="source-pill" class:active={defaultSource === 'wikipedia'}>
                    <input type="radio" bind:group={defaultSource} value="wikipedia" />
                    <span class="pill-name">한국어 위키백과</span>
                    <span class="pill-sub">한국어 줄거리 및 설명 우선 검색</span>
                  </label>
                </div>
              </section>
            </div>

            <!-- Footer for Scraper Tab -->
            <footer class="tab-footer scraper-footer">
              {#if saveScraperSuccessMsg}
                <div class="saved-msg">
                  <CheckCircle size={14} /> {saveScraperSuccessMsg}
                </div>
              {/if}
              <div class="footer-actions">
                <button type="button" class="btn-secondary" onclick={() => (isOpen = false)}>닫기</button>
                <button type="button" class="btn-primary" onclick={handleSaveScraper}>스크래퍼 설정 저장</button>
              </div>
            </footer>
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style lang="scss">
  @use '../../styles/variables' as *;

  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(5px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    padding: 20px;
    animation: fade-in 0.15s ease-out;
  }

  .settings-dialog {
    width: 920px;
    max-width: 95vw;
    height: 650px;
    max-height: 90vh;
    background: $bg-secondary;
    border: 1px solid $border-color;
    border-radius: $radius-lg;
    box-shadow: 0 24px 48px rgba(0, 0, 0, 0.6);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: pop-up 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .dialog-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 16px 0 20px;
    height: 56px;
    background: $bg-tertiary;
    border-bottom: 1px solid $border-color;
    flex-shrink: 0;

    .header-left {
      display: flex;
      align-items: center;
      gap: 24px;
      height: 100%;

      .settings-title {
        display: flex;
        align-items: center;
        gap: 8px;
        color: $text-primary;

        :global(.title-icon) {
          color: $accent-color;
        }

        h2 {
          font-size: 16px;
          font-weight: 700;
          margin: 0;
          white-space: nowrap;
        }
      }

      .nav-tabs {
        display: flex;
        align-items: center;
        gap: 4px;
        height: 100%;

        .nav-tab-btn {
          display: flex;
          align-items: center;
          gap: 6px;
          height: 34px;
          padding: 0 12px;
          border-radius: $radius-sm;
          border: 1px solid transparent;
          background: transparent;
          color: $text-secondary;
          font-size: 13px;
          font-weight: 500;
          cursor: pointer;
          transition: all 0.15s ease;

          &:hover {
            color: $text-primary;
            background: rgba(255, 255, 255, 0.05);
          }

          &.active {
            color: #818cf8;
            background: rgba(99, 102, 241, 0.15);
            border-color: rgba(99, 102, 241, 0.35);
            font-weight: 600;
          }
        }
      }
    }

    .close-btn {
      width: 32px;
      height: 32px;
      display: flex;
      align-items: center;
      justify-content: center;
      border-radius: $radius-sm;
      background: transparent;
      border: none;
      color: $text-muted;
      cursor: pointer;
      transition: all 0.12s ease;

      &:hover {
        background: $bg-hover;
        color: $text-primary;
      }
    }
  }

  .dialog-body {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  /* -------------------------------------------
     1. Device Tab Styles
     ------------------------------------------- */
  .device-tab-content {
    display: flex;
    height: 100%;
    overflow: hidden;

    .device-sidebar {
      width: 250px;
      background: rgba(0, 0, 0, 0.15);
      border-right: 1px solid $border-color;
      display: flex;
      flex-direction: column;
      flex-shrink: 0;

      .sidebar-top {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 12px 14px;
        border-bottom: 1px solid $border-color;

        h3 {
          font-size: 12px;
          font-weight: 600;
          color: $text-secondary;
          margin: 0;
        }

        .sm-btn {
          font-size: 11px;
          padding: 4px 8px;
          display: flex;
          align-items: center;
          gap: 3px;
        }
      }

      .device-list-scroll {
        flex: 1;
        overflow-y: auto;
        padding: 8px;
        display: flex;
        flex-direction: column;
        gap: 6px;

        .empty-hint {
          padding: 24px 12px;
          font-size: 11.5px;
          color: $text-muted;
          text-align: center;
          line-height: 1.5;
        }

        .device-card {
          padding: 9px 12px;
          background: $bg-tertiary;
          border: 1px solid $border-color;
          border-radius: $radius-sm;
          cursor: pointer;
          display: flex;
          align-items: center;
          justify-content: space-between;
          transition: all 0.15s ease;

          &:hover {
            border-color: rgba(99, 102, 241, 0.4);
            background: $bg-hover;
          }

          &.active {
            border-color: #6366f1;
            background: rgba(99, 102, 241, 0.12);
          }

          &.connected {
            border-left: 3px solid #10b981;
          }

          .device-info {
            overflow: hidden;

            .device-name {
              font-size: 12.5px;
              font-weight: 600;
              color: $text-primary;
              white-space: nowrap;
              overflow: hidden;
              text-overflow: ellipsis;
              display: flex;
              align-items: center;
              gap: 6px;

              .badge-connected {
                font-size: 9.5px;
                padding: 1px 4px;
                border-radius: 3px;
                background: rgba(16, 185, 129, 0.2);
                color: #34d399;
                font-weight: 700;
              }
            }

            .device-sub {
              font-size: 10.5px;
              color: $text-muted;
              margin-top: 2px;
              font-family: $font-mono;
              white-space: nowrap;
              overflow: hidden;
              text-overflow: ellipsis;
            }
          }

          .delete-btn {
            background: transparent;
            border: none;
            color: $text-muted;
            padding: 4px;
            border-radius: 3px;
            cursor: pointer;
            opacity: 0.6;
            transition: all 0.12s ease;

            &:hover {
              color: #f87171;
              background: rgba(239, 68, 68, 0.15);
              opacity: 1;
            }
          }
        }
      }
    }

    .device-form-container {
      flex: 1;
      display: flex;
      flex-direction: column;
      overflow: hidden;

      .form-scroll {
        flex: 1;
        overflow-y: auto;
        padding: 18px 24px;
      }
    }
  }

  /* Preset Pills */
  .preset-section {
    margin-bottom: 16px;

    .section-label {
      display: block;
      font-size: 11px;
      font-weight: 600;
      color: $text-muted;
      margin-bottom: 8px;
    }

    .preset-pills {
      display: flex;
      flex-wrap: wrap;
      gap: 6px;

      .preset-pill {
        padding: 5px 11px;
        font-size: 11.5px;
        font-weight: 500;
        background: $bg-tertiary;
        border: 1px solid $border-color;
        color: $text-secondary;
        border-radius: $radius-sm;
        cursor: pointer;
        transition: all 0.12s ease;

        &:hover {
          color: $text-primary;
          border-color: rgba(99, 102, 241, 0.4);
        }

        &.selected {
          background: rgba(99, 102, 241, 0.2);
          border-color: #6366f1;
          color: #818cf8;
          font-weight: 600;
        }
      }
    }
  }

  /* Form Grid */
  .form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;

    .field {
      display: flex;
      flex-direction: column;
      gap: 5px;

      &.full {
        grid-column: 1 / -1;
      }

      &.ip-field {
        grid-column: span 1;
      }

      &.port-field {
        grid-column: span 1;
      }

      label {
        font-size: 11.5px;
        font-weight: 600;
        color: $text-secondary;
      }

      input {
        width: 100%;
        padding: 7px 10px;
        font-size: 12.5px;
      }

      .field-hint {
        font-size: 11px;
        color: $text-muted;
        margin-top: 2px;
      }
    }
  }

  /* Detected Storage Pills */
  .detected-storages-list {
    margin-top: 8px;
    background: rgba(0, 0, 0, 0.2);
    border: 1px dashed $border-color;
    border-radius: $radius-sm;
    padding: 8px 10px;

    .detected-label {
      display: block;
      font-size: 10.5px;
      color: $text-muted;
      margin-bottom: 6px;
    }

    .detected-pills {
      display: flex;
      flex-wrap: wrap;
      gap: 6px;

      .detected-pill-btn {
        display: inline-flex;
        align-items: center;
        gap: 5px;
        padding: 4px 8px;
        font-size: 11px;
        background: $bg-tertiary;
        border: 1px solid $border-color;
        color: $text-secondary;
        border-radius: $radius-sm;
        cursor: pointer;
        transition: all 0.12s ease;

        &:hover {
          border-color: #6366f1;
          color: $text-primary;
        }

        &.selected {
          background: rgba(99, 102, 241, 0.2);
          border-color: #6366f1;
          color: #818cf8;
          font-weight: 600;
        }
      }
    }
  }

  /* Test Box */
  .test-result-box {
    margin-top: 14px;
    padding: 10px 14px;
    border-radius: $radius-sm;
    font-size: 12px;
    display: flex;
    align-items: center;
    gap: 8px;

    &.loading {
      background: rgba(99, 102, 241, 0.1);
      border: 1px solid rgba(99, 102, 241, 0.3);
      color: #818cf8;
    }

    &.success {
      background: rgba(16, 185, 129, 0.12);
      border: 1px solid rgba(16, 185, 129, 0.35);
      color: #34d399;
    }

    &.error {
      background: rgba(239, 68, 68, 0.12);
      border: 1px solid rgba(239, 68, 68, 0.35);
      color: #f87171;
    }
  }

  /* -------------------------------------------
     2. Scraper Tab Styles
     ------------------------------------------- */
  .scraper-tab-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;

    .scraper-scroll {
      flex: 1;
      overflow-y: auto;
      padding: 20px 24px;
      display: flex;
      flex-direction: column;
      gap: 16px;
    }
  }

  .section-box {
    background: $bg-tertiary;
    border: 1px solid $border-color;
    border-radius: $radius-md;
    padding: 18px 20px;

    .section-title {
      display: flex;
      align-items: center;
      gap: 8px;
      margin-bottom: 6px;

      h4 {
        font-size: 13.5px;
        font-weight: 600;
        color: $text-primary;
        margin: 0;
      }

      .badge-source {
        font-size: 10px;
        font-weight: 700;
        background: rgba(99, 102, 241, 0.2);
        color: #818cf8;
        padding: 2px 6px;
        border-radius: 4px;
      }
    }

    .section-desc {
      font-size: 12px;
      color: $text-secondary;
      line-height: 1.5;
      margin: 0 0 14px 0;
    }
  }

  .input-with-icon {
    position: relative;
    display: flex;
    align-items: center;

    .left-icon {
      position: absolute;
      left: 10px;
      color: $text-muted;
      pointer-events: none;
      display: flex;
    }

    input {
      width: 100%;
      padding-left: 32px;
      padding-right: 32px;
    }

    .toggle-eye {
      position: absolute;
      right: 8px;
      background: transparent;
      border: none;
      color: $text-muted;
      cursor: pointer;
      display: flex;
      padding: 4px;

      &:hover {
        color: $text-primary;
      }
    }
  }

  .test-row {
    margin-top: 14px;
    display: flex;
    align-items: center;
    gap: 12px;

    .test-btn {
      font-size: 12px;
      padding: 6px 12px;
      display: inline-flex;
      align-items: center;
      gap: 6px;
    }

    .test-feedback {
      display: flex;
      align-items: center;
      gap: 6px;
      font-size: 12px;

      &.success {
        color: #34d399;
      }

      &.error {
        color: #f87171;
      }
    }
  }

  .source-pills {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;

    .source-pill {
      display: flex;
      flex-direction: column;
      gap: 3px;
      padding: 10px 12px;
      background: $bg-secondary;
      border: 1px solid $border-color;
      border-radius: $radius-sm;
      cursor: pointer;
      position: relative;
      transition: all 0.12s ease;

      input {
        position: absolute;
        opacity: 0;
      }

      .pill-name {
        font-size: 12px;
        font-weight: 600;
        color: $text-primary;
      }

      .pill-sub {
        font-size: 10.5px;
        color: $text-muted;
      }

      &:hover {
        border-color: rgba(99, 102, 241, 0.4);
      }

      &.active {
        border-color: #6366f1;
        background: rgba(99, 102, 241, 0.12);

        .pill-name {
          color: #818cf8;
        }
      }
    }
  }

  /* Tab Footers */
  .tab-footer {
    padding: 12px 20px;
    border-top: 1px solid $border-color;
    background: $bg-tertiary;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 10px;
    flex-shrink: 0;

    &.scraper-footer {
      justify-content: space-between;

      .saved-msg {
        display: flex;
        align-items: center;
        gap: 6px;
        font-size: 12px;
        color: #34d399;
      }

      .footer-actions {
        display: flex;
        align-items: center;
        gap: 8px;
        margin-left: auto;
      }
    }
  }

  @keyframes fade-in {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  @keyframes pop-up {
    from {
      opacity: 0;
      transform: scale(0.96) translateY(8px);
    }
    to {
      opacity: 1;
      transform: scale(1) translateY(0);
    }
  }
</style>
