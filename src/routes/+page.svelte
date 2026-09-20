<script lang="ts">
  import { onMount } from 'svelte';
  import type { DeviceProfile, GameItem, ScraperSettings, StorageLocation, SystemPlatform } from '$lib/types';
  import { getSystems, getSystemGames, saveSystemGames, detectStorages } from '$lib/api';
  import {
    loadProfiles,
    saveProfiles,
    loadActiveDeviceId,
    saveActiveDeviceId,
    loadScraperSettings,
    saveScraperSettings,
  } from '$lib/storage';

  import Header from '$lib/components/Header.svelte';
  import SystemSidebar from '$lib/components/SystemSidebar.svelte';
  import GameList from '$lib/components/GameList.svelte';
  import MetadataEditor from '$lib/components/MetadataEditor.svelte';
  import SettingsModal from '$lib/components/SettingsModal.svelte';
  import ScraperModal from '$lib/components/ScraperModal.svelte';
  import { Gamepad, Gamepad2, Plus, Cpu, AlertTriangle, CheckCircle } from 'lucide-svelte';

  let profiles = $state<DeviceProfile[]>([]);
  let activeDevice = $state<DeviceProfile | null>(null);

  let isSettingsModalOpen = $state(false);
  let settingsModalTab = $state<'device' | 'scraper'>('device');

  function openSettings(tab: 'device' | 'scraper' = 'device') {
    settingsModalTab = tab;
    isSettingsModalOpen = true;
  }

  let scraperSettings = $state<ScraperSettings>(loadScraperSettings());
  let isScraperModalOpen = $state(false);

  let storages = $state<StorageLocation[]>([]);
  let activeStoragePath = $state<string>('');

  let systems = $state<SystemPlatform[]>([]);
  let selectedSystemId = $state<string | null>(null);
  let isSidebarOpen = $state(true);
  let selectedSystem = $derived(systems.find((s) => s.id === selectedSystemId));
  let selectedSystemName = $derived(selectedSystem ? selectedSystem.name : (selectedSystemId || ''));
  let games = $state<GameItem[]>([]);
  let selectedGame = $state<GameItem | null>(null);

  let isConnectingDevice = $state(false);
  let connectingDeviceName = $state<string | null>(null);
  let isLoadingSystems = $state(false);
  let isLoadingGames = $state(false);
  let isSaving = $state(false);
  let isDirty = $state(false);

  let toastMessage = $state<string | null>(null);
  let toastType = $state<'success' | 'error' | 'info'>('info');

  function showToast(msg: string, type: 'success' | 'error' | 'info' = 'info') {
    toastMessage = msg;
    toastType = type;
    setTimeout(() => {
      if (toastMessage === msg) toastMessage = null;
    }, 4000);
  }

  onMount(() => {
    profiles = loadProfiles();
    const lastId = loadActiveDeviceId();
    if (lastId) {
      const found = profiles.find((p) => p.id === lastId);
      if (found) {
        connectToDevice(found);
        return;
      }
    }
    if (profiles.length === 0) {
      openSettings('device');
    } else {
      connectToDevice(profiles[0]);
    }
  });

  // Watch profiles change and sync to storage
  $effect(() => {
    saveProfiles(profiles);
  });

  async function connectToDevice(device: DeviceProfile) {
    isConnectingDevice = true;
    connectingDeviceName = device.name;
    activeDevice = device;
    saveActiveDeviceId(device.id);
    selectedSystemId = null;
    games = [];
    selectedGame = null;
    isDirty = false;

    isLoadingSystems = true;
    try {
      // 1. Auto-detect available storages (internal & external SD card)
      const detected = await detectStorages(device);
      storages = detected;

      // 2. Use configured roms_path, or first detected storage, or /userdata/roms
      activeStoragePath = device.roms_path || (detected[0]?.path ?? '/userdata/roms');

      systems = await getSystems(device, activeStoragePath);
      const activeLabel = storages.find((s) => s.path === activeStoragePath)?.label || activeStoragePath;
      showToast(`'${device.name}'에 연결되었습니다. [${activeLabel}]`, 'success');

      // Auto select first system with games if available
      const firstWithRoms = systems.find((s) => s.rom_count > 0);
      if (firstWithRoms) {
        await selectSystem(firstWithRoms.id);
      } else if (systems.length > 0) {
        await selectSystem(systems[0].id);
      }
    } catch (err: any) {
      console.error('플랫폼 로드 실패:', err);
      showToast('게임기 접속 실패: ' + (err?.message || err), 'error');
      systems = [];
    } finally {
      isLoadingSystems = false;
      isConnectingDevice = false;
      connectingDeviceName = null;
    }
  }

  async function handleStorageSelect(path: string) {
    if (path === activeStoragePath || !activeDevice) return;
    if (isDirty) {
      const confirmLeave = confirm('수정된 메타데이터가 저장되지 않았습니다. 저장하지 않고 이동하시겠습니까?');
      if (!confirmLeave) return;
    }

    const stObj = storages.find((s) => s.path === path);
    isConnectingDevice = true;
    connectingDeviceName = `${activeDevice.name} [${stObj?.label || '저장소'}]`;

    activeStoragePath = path;
    selectedSystemId = null;
    games = [];
    selectedGame = null;
    isDirty = false;
    isLoadingSystems = true;

    try {
      systems = await getSystems(activeDevice, activeStoragePath);
      showToast(`저장소가 '${stObj?.label || path}'(으)로 전환되었습니다.`, 'info');

      const firstWithRoms = systems.find((s) => s.rom_count > 0);
      if (firstWithRoms) {
        await selectSystem(firstWithRoms.id);
      } else if (systems.length > 0) {
        await selectSystem(systems[0].id);
      }
    } catch (err: any) {
      console.error('저장소 전환 실패:', err);
      showToast('저장소 로드 실패: ' + (err?.message || err), 'error');
    } finally {
      isLoadingSystems = false;
      isConnectingDevice = false;
      connectingDeviceName = null;
    }
  }

  function disconnectDevice() {
    if (isDirty) {
      const confirmLeave = confirm('수정된 메타데이터가 저장되지 않았습니다. 연결을 해제하시겠습니까?');
      if (!confirmLeave) return;
    }
    activeDevice = null;
    saveActiveDeviceId(null);
    selectedSystemId = null;
    systems = [];
    games = [];
    selectedGame = null;
    isDirty = false;
    storages = [];
    activeStoragePath = '';
    showToast('기기 연결이 해제되었습니다.', 'info');
  }

  function handleDeviceSelect(id: string) {
    if (!id) {
      disconnectDevice();
      return;
    }
    const dev = profiles.find((p) => p.id === id);
    if (dev) {
      connectToDevice(dev);
    }
  }

  async function selectSystem(systemId: string) {
    if (!activeDevice) return;
    if (isDirty) {
      const confirmLeave = confirm('수정된 메타데이터가 저장되지 않았습니다. 저장하지 않고 이동하시겠습니까?');
      if (!confirmLeave) return;
    }

    selectedSystemId = systemId;
    selectedGame = null;
    isDirty = false;
    isLoadingGames = true;

    try {
      games = await getSystemGames(activeDevice, systemId, activeStoragePath);
      if (games.length > 0) {
        selectedGame = games[0];
      }

      // Reconcile sidebar platform counts with ground truth games
      const sysIdx = systems.findIndex((s) => s.id === systemId);
      if (sysIdx !== -1) {
        const romCount = games.filter((g) => g.status === 'registered' || g.status === 'unregistered').length;
        const unregCount = games.filter((g) => g.status === 'unregistered').length;
        const missingImgCount = games.filter((g) => !g.image || g.image.trim() === '').length;
        systems[sysIdx].rom_count = romCount;
        systems[sysIdx].unregistered_count = unregCount;
        systems[sysIdx].missing_image_count = missingImgCount;
        systems = [...systems];
      }
    } catch (err: any) {
      console.error('게임 로드 실패:', err);
      showToast('게임 목록 로드 실패: ' + (err?.message || err), 'error');
      games = [];
    } finally {
      isLoadingGames = false;
    }
  }

  function handleGameUpdated() {
    isDirty = true;
  }

  function handleBatchRegister() {
    let count = 0;
    games = games.map((g) => {
      if (g.status === 'unregistered') {
        count++;
        return { ...g, status: 'registered' };
      }
      return g;
    });

    if (count > 0) {
      isDirty = true;
      showToast(`${count}개의 미등록 ROM을 gamelist.xml 등록 목록에 추가했습니다. 상단 '저장' 버튼을 눌러 적용하세요.`, 'info');
    } else {
      showToast('미등록 상태인 ROM이 없습니다.', 'info');
    }
  }

  function handleBatchCleanMissing() {
    const missingCount = games.filter((g) => g.status === 'missing').length;
    if (missingCount === 0) {
      showToast('누락된 엔트리가 없습니다.', 'info');
      return;
    }

    games = games.filter((g) => g.status !== 'missing');
    if (selectedGame && selectedGame.status === 'missing') {
      selectedGame = games[0] || null;
    }
    isDirty = true;
    showToast(`${missingCount}개의 미아(누락) 메타데이터를 제거했습니다. 상단 '저장' 버튼을 눌러 게임기에 반영하세요.`, 'info');
  }

  function handleDeleteGame(path: string) {
    games = games.filter((g) => g.path !== path);
    if (selectedGame?.path === path) {
      selectedGame = games[0] || null;
    }
    isDirty = true;
    showToast(`선택한 엔트리를 목록에서 제거했습니다. 상단 '저장'을 눌러 반영하세요.`, 'info');
  }

  async function handleSaveGamelist() {
    if (!activeDevice || !selectedSystemId) return;

    isSaving = true;
    try {
      // Filter out missing games from save if desired, or keep registered
      const toSave = games.filter((g) => g.status === 'registered');
      await saveSystemGames(activeDevice, selectedSystemId, toSave, activeStoragePath);
      isDirty = false;
      showToast(`'${selectedSystemId}'의 gamelist.xml이 기기에 안전하게 저장되었습니다 (백업 생성됨).`, 'success');

      // Refresh system stats
      if (activeDevice) {
        const updatedSystems = await getSystems(activeDevice, activeStoragePath);
        systems = updatedSystems;
      }
    } catch (err: any) {
      console.error('저장 실패:', err);
      showToast('gamelist.xml 저장 실패: ' + (err?.message || err), 'error');
    } finally {
      isSaving = false;
    }
  }

  function handleRefresh() {
    if (activeDevice && selectedSystemId) {
      selectSystem(selectedSystemId);
    }
  }
</script>

<div class="app-layout">
  <Header
    {activeDevice}
    {profiles}
    {selectedSystemId}
    {isDirty}
    {isSaving}
    isLoading={isConnectingDevice || isLoadingSystems || isLoadingGames}
    onOpenSettings={() => openSettings('device')}
    onDeviceSelect={handleDeviceSelect}
    onRefresh={handleRefresh}
    onSaveGamelist={handleSaveGamelist}
    onBatchRegister={handleBatchRegister}
    onBatchCleanMissing={handleBatchCleanMissing}
  />

  <main class="main-content">
    {#if !activeDevice}
      <div class="no-device-screen">
        <div class="welcome-box">
          <div class="device-icon">
            <Cpu size={48} />
          </div>
          <h2>리눅스 게임기를 연결해주세요</h2>
          <p>
            Knulli, ROCKNIX, Batocera 등 리눅스 기반 게임기의 IP와 계정을 입력하면<br />
            ES-DE의 ROM 디렉토리와 gamelist.xml 메타데이터를 원격으로 편리하게 관리할 수 있습니다.
          </p>
          <button class="btn-primary start-btn" onclick={() => openSettings('device')}>
            <Plus size={16} /> 게임기 프리셋 설정 및 연결
          </button>
        </div>
      </div>
    {:else}
      <div class="workspace-grid">
        <!-- 1. Systems Sidebar -->
        <SystemSidebar
          {systems}
          bind:selectedSystemId
          isLoading={isLoadingSystems}
          {storages}
          {activeStoragePath}
          isOpen={isSidebarOpen}
          onToggle={() => (isSidebarOpen = !isSidebarOpen)}
          onSelect={selectSystem}
          onSelectStorage={handleStorageSelect}
        />

        <!-- 2. Central Game List -->
        {#if selectedSystemId}
          <GameList
            {games}
            bind:selectedGame
            isLoading={isLoadingGames}
            isSidebarOpen={isSidebarOpen}
            systemName={selectedSystemName}
            onToggleSidebar={() => (isSidebarOpen = !isSidebarOpen)}
            onSelectGame={(g) => (selectedGame = g)}
          />
        {:else}
          <div class="no-system-selected">
            <Gamepad size={40} />
            <p>좌측에서 관리할 에뮬레이터 플랫폼을 선택하세요.</p>
            {#if !isSidebarOpen}
              <button class="btn-secondary open-fallback-btn" onclick={() => (isSidebarOpen = true)}>
                콘솔 플랫폼 목록 열기 &gt;&gt;
              </button>
            {/if}
          </div>
        {/if}

        <!-- 3. Right Metadata Inspector -->
        <MetadataEditor
          bind:game={selectedGame}
          device={activeDevice}
          systemId={selectedSystemId}
          romsPathOverride={activeStoragePath}
          onGameUpdated={handleGameUpdated}
          onDeleteGame={handleDeleteGame}
          onOpenScraper={() => (isScraperModalOpen = true)}
        />
      </div>
    {/if}
  </main>

  <!-- Scraper Search Modal -->
  <ScraperModal
    bind:isOpen={isScraperModalOpen}
    bind:game={selectedGame}
    device={activeDevice}
    systemId={selectedSystemId}
    romsPathOverride={activeStoragePath}
    bind:scraperSettings
    onOpenSettings={() => openSettings('scraper')}
    onApplied={(msg) => {
      isDirty = true;
      showToast(msg, 'success');
    }}
  />

  <!-- Unified Settings Modal (Device & Scraper Tabs) -->
  <SettingsModal
    bind:isOpen={isSettingsModalOpen}
    bind:activeTab={settingsModalTab}
    bind:profiles
    bind:activeDevice
    bind:settings={scraperSettings}
    onDeviceConnected={connectToDevice}
    onSavedScraperSettings={() => showToast('스크래퍼 설정이 저장되었습니다.', 'info')}
  />


  <!-- Device Loading Blocking Overlay -->
  {#if isConnectingDevice}
    <div class="loading-overlay" role="alert" aria-busy="true" aria-live="polite">
      <div class="loading-card">
        <div class="circular-spinner">
          <svg viewBox="0 0 50 50" class="spinner-svg">
            <circle class="spinner-bg" cx="25" cy="25" r="20" fill="none" stroke-width="4" />
            <circle class="spinner-bar" cx="25" cy="25" r="20" fill="none" stroke-width="4" />
          </svg>
          <div class="spinner-inner-icon">
            <Gamepad2 size={22} />
          </div>
        </div>
        <div class="loading-text">
          <h3>기기 연결 및 데이터 로딩 중...</h3>
          {#if connectingDeviceName}
            <p><strong>'{connectingDeviceName}'</strong>에 연결하고 플랫폼 목록을 불러오는 중입니다.</p>
          {:else}
            <p>잠시만 기다려 주세요.</p>
          {/if}
        </div>
      </div>
    </div>
  {/if}

  <!-- Toast Message -->
  {#if toastMessage}
    <div class="toast-popup" class:success={toastType === 'success'} class:error={toastType === 'error'}>
      {#if toastType === 'success'}
        <CheckCircle size={16} />
      {:else if toastType === 'error'}
        <AlertTriangle size={16} />
      {/if}
      <span>{toastMessage}</span>
    </div>
  {/if}
</div>

<style lang="scss">
  @use '../styles/variables' as *;

  .app-layout {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
    background: $bg-primary;
  }

  .main-content {
    flex: 1;
    overflow: hidden;
    position: relative;
  }

  .workspace-grid {
    display: flex;
    height: 100%;
    width: 100%;
    overflow: hidden;
  }

  .no-device-screen {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    padding: 20px;

    .welcome-box {
      max-width: 520px;
      text-align: center;
      background: $bg-secondary;
      border: 1px solid $border-color;
      border-radius: $radius-lg;
      padding: 40px 30px;
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 16px;
      box-shadow: $shadow-lg;

      .device-icon {
        width: 72px;
        height: 72px;
        border-radius: $radius-full;
        background: $accent-glow;
        color: $accent-color;
        display: flex;
        align-items: center;
        justify-content: center;
      }

      h2 {
        font-size: 20px;
        font-weight: 700;
      }

      p {
        color: $text-secondary;
        font-size: 13.5px;
        line-height: 1.6;
      }

      .start-btn {
        margin-top: 10px;
        padding: 10px 20px;
        font-size: 14px;
      }
    }
  }

  .no-system-selected {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    color: $text-muted;
    font-size: 13px;

    .open-fallback-btn {
      margin-top: 4px;
      font-size: 12.5px;
      padding: 8px 16px;
    }
  }

  .toast-popup {
    position: fixed;
    bottom: 24px;
    right: 24px;
    background: $bg-card;
    color: $text-primary;
    border: 1px solid $border-light;
    padding: 12px 18px;
    border-radius: $radius-md;
    box-shadow: $shadow-lg;
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
    z-index: 2000;
    animation: slide-up 0.2s ease-out;

    &.success {
      border-color: rgba(16, 185, 129, 0.5);
      color: #34d399;
    }

    &.error {
      border-color: rgba(239, 68, 68, 0.5);
      color: #f87171;
    }
  }

  @keyframes slide-up {
    from {
      opacity: 0;
      transform: translateY(12px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .loading-overlay {
    position: fixed;
    inset: 0;
    z-index: 99999;
    background: rgba(10, 14, 26, 0.75);
    backdrop-filter: blur(6px);
    -webkit-backdrop-filter: blur(6px);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: wait;
    user-select: none;
    animation: overlay-fade 0.2s ease-out;

    .loading-card {
      background: $bg-secondary;
      border: 1px solid rgba(99, 102, 241, 0.35);
      border-radius: $radius-lg;
      padding: 36px 44px;
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 20px;
      box-shadow: 0 24px 48px rgba(0, 0, 0, 0.6), 0 0 24px rgba(99, 102, 241, 0.2);
      text-align: center;
      max-width: 420px;
      animation: card-pop 0.25s cubic-bezier(0.16, 1, 0.3, 1);
    }

    .circular-spinner {
      position: relative;
      width: 72px;
      height: 72px;
      display: flex;
      align-items: center;
      justify-content: center;

      .spinner-svg {
        width: 100%;
        height: 100%;
        animation: rotate 1.8s linear infinite;
        transform-origin: center;

        .spinner-bg {
          stroke: rgba(255, 255, 255, 0.08);
        }

        .spinner-bar {
          stroke: #818cf8;
          stroke-linecap: round;
          animation: dash 1.6s ease-in-out infinite;
          filter: drop-shadow(0 0 6px rgba(99, 102, 241, 0.6));
        }
      }

      .spinner-inner-icon {
        position: absolute;
        inset: 0;
        display: flex;
        align-items: center;
        justify-content: center;
        color: #a5b4fc;
        animation: pulse-icon 1.8s ease-in-out infinite alternate;
      }
    }

    .loading-text {
      display: flex;
      flex-direction: column;
      gap: 8px;

      h3 {
        font-size: 16px;
        font-weight: 700;
        color: $text-primary;
        margin: 0;
      }

      p {
        font-size: 13px;
        color: $text-secondary;
        margin: 0;
        line-height: 1.5;

        strong {
          color: #a5b4fc;
        }
      }
    }
  }

  @keyframes rotate {
    100% {
      transform: rotate(360deg);
    }
  }

  @keyframes dash {
    0% {
      stroke-dasharray: 1, 126;
      stroke-dashoffset: 0;
    }
    50% {
      stroke-dasharray: 95, 126;
      stroke-dashoffset: -30;
    }
    100% {
      stroke-dasharray: 1, 126;
      stroke-dashoffset: -126;
    }
  }

  @keyframes pulse-icon {
    0% {
      transform: scale(0.92);
      opacity: 0.75;
    }
    100% {
      transform: scale(1.06);
      opacity: 1;
    }
  }

  @keyframes overlay-fade {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  @keyframes card-pop {
    from {
      opacity: 0;
      transform: scale(0.92) translateY(8px);
    }
    to {
      opacity: 1;
      transform: scale(1) translateY(0);
    }
  }
</style>
