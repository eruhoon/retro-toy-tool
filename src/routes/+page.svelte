<script lang="ts">
  import { onMount } from 'svelte';
  import type {
    DeviceProfile,
    GameItem,
    InstalledCore,
    RomUploadProgressPayload,
    RomUploadResult,
    ScraperSettings,
    StorageLocation,
    SystemPlatform,
  } from '$lib/types';
  import {
    getSystems,
    getSystemGames,
    saveSystemGames,
    detectStorages,
    uploadRomFiles,
    onRomUploadProgress,
    getInstalledCores,
    pingDevice,
  } from '$lib/api';
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
  import CoreList from '$lib/components/CoreList.svelte';
  import MetadataEditor from '$lib/components/MetadataEditor.svelte';
  import SettingsModal from '$lib/components/SettingsModal.svelte';
  import ScraperModal from '$lib/components/ScraperModal.svelte';
  import RomUploadModal from '$lib/components/RomUploadModal.svelte';
  import UpdaterModal from '$lib/components/UpdaterModal.svelte';
  import { isCoreMatchingSystem } from '$lib/utils/coreUtils';
  import { invoke } from '@tauri-apps/api/core';
  import { Gamepad, Gamepad2, Plus, Cpu, AlertTriangle, CheckCircle, RefreshCw } from 'lucide-svelte';


  let profiles = $state<DeviceProfile[]>([]);
  let activeDevice = $state<DeviceProfile | null>(null);
  let devicePingStatus = $state<Record<string, 'checking' | 'online' | 'offline'>>({});
  let isPingingDevices = $state(false);

  async function pingAllDevices() {
    if (profiles.length === 0) return;
    isPingingDevices = true;
    
    // Set all to checking
    const initial: Record<string, 'checking' | 'online' | 'offline'> = { ...devicePingStatus };
    for (const p of profiles) {
      initial[p.id] = 'checking';
    }
    devicePingStatus = initial;

    // Ping concurrently
    await Promise.all(
      profiles.map(async (p) => {
        const ok = await pingDevice(p.host, p.port);
        devicePingStatus = {
          ...devicePingStatus,
          [p.id]: ok ? 'online' : 'offline',
        };
      })
    );
    isPingingDevices = false;
  }

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

  // ROM File Upload State
  let isUploadModalOpen = $state(false);
  let isUploadingRoms = $state(false);
  let uploadProgress = $state<RomUploadProgressPayload | null>(null);
  let uploadResult = $state<RomUploadResult | null>(null);

  // Main View Tab State ('roms' | 'cores')
  let mainViewTab = $state<'roms' | 'cores'>('roms');
  let installedCores = $state<InstalledCore[]>([]);
  let isLoadingCores = $state(false);

  let matchedCoresCount = $derived.by(() => {
    if (!selectedSystemId) return installedCores.length;
    return installedCores.filter((c) => isCoreMatchingSystem(c, selectedSystemId)).length;
  });

  async function loadCores(force = false) {
    if (!activeDevice) return;
    if (!force && installedCores.length > 0) return;

    isLoadingCores = true;
    try {
      const list = await getInstalledCores(activeDevice);
      installedCores = list;
    } catch (err: any) {
      console.error('에뮬레이터 코어 로드 실패:', err);
      showToast('에뮬레이터 코어 조회 실패: ' + (err?.message || err), 'error');
    } finally {
      isLoadingCores = false;
    }
  }

  function switchMainTab(tab: 'roms' | 'cores') {
    mainViewTab = tab;
    if (tab === 'cores' && installedCores.length === 0) {
      loadCores();
    }
  }

  // Auto-updater State
  let isUpdaterModalOpen = $state(false);
  let updateVersion = $state('');
  let updateBody = $state<string | null>(null);
  let updateCurrentVersion = $state('');
  let hasUpdate = $state(false);

  function showToast(msg: string, type: 'success' | 'error' | 'info' = 'info') {

    toastMessage = msg;
    toastType = type;
    setTimeout(() => {
      if (toastMessage === msg) toastMessage = null;
    }, 4000);
  }

  async function handleUploadFiles(paths: string[]) {
    if (!activeDevice) {
      showToast('연결된 기기가 없습니다.', 'error');
      return;
    }
    if (!selectedSystemId) {
      showToast('ROM을 업로드할 콘솔 플랫폼을 먼저 선택해주세요.', 'error');
      return;
    }
    if (!paths || paths.length === 0) return;

    isUploadModalOpen = true;
    isUploadingRoms = true;
    uploadProgress = null;
    uploadResult = null;

    try {
      const res = await uploadRomFiles(activeDevice, selectedSystemId, paths, activeStoragePath);
      uploadResult = res;

      // Automatically refresh game list & system rom counts
      games = await getSystemGames(activeDevice, selectedSystemId, activeStoragePath);
      systems = await getSystems(activeDevice, activeStoragePath);

      if (res.failed_files.length === 0) {
        showToast(`${res.success_count}개 ROM 파일 복사 완료`, 'success');
      } else {
        showToast(`${res.success_count}개 성공, ${res.failed_files.length}개 실패`, 'error');
      }
    } catch (err: any) {
      console.error('ROM 업로드 실패:', err);
      uploadResult = {
        success_count: 0,
        failed_files: [String(err?.message || err)],
        message: '업로드 중 오류가 발생했습니다: ' + (err?.message || err),
      };
      showToast('ROM 복사 실패: ' + (err?.message || err), 'error');
    } finally {
      isUploadingRoms = false;
    }
  }

  onMount(() => {
    let unlistenProgress: (() => void) | null = null;
    onRomUploadProgress((payload) => {
      uploadProgress = payload;
    }).then((unlisten) => {
      unlistenProgress = unlisten;
    }).catch((err) => {
      console.warn('onRomUploadProgress 리스너 등록 실패:', err);
    });

    // Check for app updates silently on startup
    invoke<{ version: string; body: string | null; current_version: string } | null>('check_update')
      .then((info) => {
        if (info) {
          updateVersion = info.version;
          updateBody = info.body;
          updateCurrentVersion = info.current_version;
          hasUpdate = true;
          isUpdaterModalOpen = true;
        }
      })
      .catch((err) => {
        console.warn('업데이트 확인 실패:', err);
      });

    profiles = loadProfiles();
    pingAllDevices();

    return () => {
      if (unlistenProgress) unlistenProgress();
    };
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
    installedCores = [];
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

      // Preload installed cores in background
      loadCores(true).catch(() => {});
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
    if (activeDevice) {
      if (mainViewTab === 'cores') {
        loadCores(true);
      } else if (selectedSystemId) {
        selectSystem(selectedSystemId);
      }
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
    {hasUpdate}
    onOpenSettings={() => openSettings('device')}
    onDeviceSelect={handleDeviceSelect}
    onRefresh={handleRefresh}
    onSaveGamelist={handleSaveGamelist}
    onBatchRegister={handleBatchRegister}
    onBatchCleanMissing={handleBatchCleanMissing}
    onOpenUpdater={() => (isUpdaterModalOpen = true)}
  />

  <main class="main-content">
    {#if !activeDevice}
      <div class="no-device-screen">
        <div class="welcome-box">
          <div class="device-icon">
            <Cpu size={44} />
          </div>
          <h2>게임기 연결</h2>
          <p>
            원격으로 연결할 게임기를 선택하거나 새 게임기를 추가하세요.
          </p>

          {#if profiles.length > 0}
            <div class="saved-devices-container">
              <div class="saved-devices-header">
                <span class="saved-devices-title">등록된 기기 목록</span>
                <button
                  class="btn-refresh-ping"
                  disabled={isPingingDevices}
                  onclick={pingAllDevices}
                  title="기기 연결 상태 다시 확인"
                >
                  <RefreshCw size={12} class={isPingingDevices ? 'spin' : ''} />
                  <span>상태 새로고침</span>
                </button>
              </div>
              <div class="saved-devices-list">
                {#each profiles as p}
                  {@const status = devicePingStatus[p.id] || 'checking'}
                  <button
                    class="saved-device-item"
                    disabled={isConnectingDevice}
                    onclick={() => connectToDevice(p)}
                  >
                    <div class="saved-device-meta">
                      <div class="saved-device-name">
                        <span
                          class="saved-device-dot {status}"
                          title={status === 'online' ? '기기 응답 있음 (연결 가능)' : status === 'offline' ? '기기 응답 없음 (전원 꺼짐 또는 IP 다름)' : '연결 확인 중...'}
                        ></span>
                        <strong>{p.name}</strong>
                        <span class="saved-device-tag">{p.device_type}</span>
                        {#if status === 'online'}
                          <span class="device-status-badge online">연결 가능</span>
                        {:else if status === 'offline'}
                          <span class="device-status-badge offline">오프라인</span>
                        {:else}
                          <span class="device-status-badge checking">확인 중...</span>
                        {/if}
                      </div>
                      <div class="saved-device-sub">{p.host}:{p.port} • {p.roms_path}</div>
                    </div>
                    <span class="connect-action-text">연결 &gt;</span>
                  </button>
                {/each}
              </div>
            </div>
          {/if}

          <button
            class="btn-secondary add-device-btn"
            disabled={isConnectingDevice}
            onclick={() => openSettings('device')}
          >
            <Plus size={16} /> 새 게임기 등록 및 설정
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

        <!-- 2. Central & Right Workspace with View Tabs -->
        <div class="central-workspace">
          <!-- Main Tab Switcher -->
          <div class="main-tab-bar">
            <button
              class="main-tab-btn"
              class:active={mainViewTab === 'roms'}
              onclick={() => switchMainTab('roms')}
            >
              <Gamepad2 size={15} />
              <span>게임 ROM ({selectedSystem ? selectedSystem.rom_count : games.length})</span>
            </button>

            <button
              class="main-tab-btn"
              class:active={mainViewTab === 'cores'}
              onclick={() => switchMainTab('cores')}
            >
              <Cpu size={15} />
              <span>
                에뮬 코어
                {#if selectedSystemId}
                  <span class="tab-count-badge">({matchedCoresCount})</span>
                {:else if installedCores.length > 0}
                  <span class="tab-count-badge">({installedCores.length})</span>
                {/if}
              </span>
            </button>
          </div>

          <!-- Tab Content Area -->
          <div class="tab-view-container">
            {#if mainViewTab === 'roms'}
              <div class="roms-view-pane">
                {#if selectedSystemId}
                  <GameList
                    {games}
                    bind:selectedGame
                    isLoading={isLoadingGames}
                    isSidebarOpen={isSidebarOpen}
                    systemName={selectedSystemName}
                    hasSelectedSystem={!!selectedSystemId}
                    onToggleSidebar={() => (isSidebarOpen = !isSidebarOpen)}
                    onSelectGame={(g) => (selectedGame = g)}
                    onUploadFiles={handleUploadFiles}
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

                <!-- Right Metadata Inspector -->
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
            {:else}
              <div class="cores-view-pane">
                <CoreList
                  cores={installedCores}
                  isLoading={isLoadingCores}
                  selectedSystemId={selectedSystemId}
                  systemName={selectedSystemName}
                  isSidebarOpen={isSidebarOpen}
                  onToggleSidebar={() => (isSidebarOpen = !isSidebarOpen)}
                  onRefresh={() => loadCores(true)}
                />
              </div>
            {/if}
          </div>
        </div>
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

  <!-- ROM Upload Progress Modal -->
  <RomUploadModal
    isOpen={isUploadModalOpen}
    systemName={selectedSystemName}
    deviceName={activeDevice?.name || '에뮬레이터 기기'}
    progress={uploadProgress}
    result={uploadResult}
    isUploading={isUploadingRoms}
    onClose={() => {
      isUploadModalOpen = false;
      uploadProgress = null;
      uploadResult = null;
    }}
  />

  <!-- Auto-updater Modal -->
  <UpdaterModal
    bind:isOpen={isUpdaterModalOpen}
    version={updateVersion}
    body={updateBody}
    currentVersion={updateCurrentVersion}
    onDismiss={() => { isUpdaterModalOpen = false; }}
  />


  <!-- Device Loading Blocking Overlay -->
  {#if isConnectingDevice}
    <div class="loading-overlay" role="alert" aria-busy="true" aria-live="polite">
      <div class="loading-card">
        <div class="circular-spinner">
          <svg viewBox="0 0 50 50" class="spinner-svg">
            <defs>
              <linearGradient id="spinner-grad" x1="0%" y1="0%" x2="100%" y2="100%">
                <stop offset="0%" stop-color="#818cf8" />
                <stop offset="100%" stop-color="#c084fc" />
              </linearGradient>
            </defs>
            <circle class="spinner-bg" cx="25" cy="25" r="20" fill="none" stroke-width="3.5" />
            <circle class="spinner-bar" cx="25" cy="25" r="20" fill="none" stroke-width="3.5" stroke="url(#spinner-grad)" />
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

  .central-workspace {
    flex: 1;
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background: $bg-primary;
  }

  .main-tab-bar {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 0 16px;
    background: $bg-secondary;
    border-bottom: 1px solid $border-color;
    flex-shrink: 0;

    .main-tab-btn {
      display: flex;
      align-items: center;
      gap: 7px;
      padding: 10px 18px;
      background: transparent;
      border: none;
      border-radius: 0; /* 라운딩 완전 제거 */
      border-bottom: 2px solid transparent;
      margin-bottom: -1px; /* 바닥 경계선과 정확히 일치 */
      color: $text-secondary;
      font-size: 13px;
      font-weight: 500;
      cursor: pointer;
      transition: color 0.15s ease, border-color 0.15s ease;

      &:hover {
        color: $text-primary;
        background: rgba(255, 255, 255, 0.02);
      }

      &.active {
        color: $accent-color;
        border-bottom-color: $accent-color;
        font-weight: 600;
      }

      .tab-count-badge {
        font-size: 11.5px;
        opacity: 0.85;
      }
    }
  }


  .tab-view-container {
    flex: 1;
    display: flex;
    overflow: hidden;
    height: 100%;
  }

  .roms-view-pane {
    flex: 1;
    display: flex;
    width: 100%;
    height: 100%;
    overflow: hidden;
  }

  .cores-view-pane {
    flex: 1;
    display: flex;
    width: 100%;
    height: 100%;
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

      .saved-devices-container {
        width: 100%;
        display: flex;
        flex-direction: column;
        gap: 8px;
        margin-top: 6px;

        .saved-devices-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 0 4px;

          .saved-devices-title {
            font-size: 11px;
            text-transform: uppercase;
            letter-spacing: 0.05em;
            color: $text-muted;
            font-weight: 600;
          }

          .btn-refresh-ping {
            background: none;
            border: none;
            display: flex;
            align-items: center;
            gap: 4px;
            font-size: 11px;
            color: $text-muted;
            cursor: pointer;
            padding: 2px 6px;
            border-radius: $radius-sm;
            transition: all 0.15s ease;

            &:hover:not(:disabled) {
              color: $accent-color;
              background: rgba(255, 255, 255, 0.05);
            }

            &:disabled {
              opacity: 0.6;
              cursor: not-allowed;
            }

            :global(.spin) {
              animation: spin 1s linear infinite;
            }
          }
        }

        .saved-devices-list {
          display: flex;
          flex-direction: column;
          gap: 6px;
          max-height: 240px;
          overflow-y: auto;
        }

        .saved-device-item {
          display: flex;
          align-items: center;
          justify-content: space-between;
          width: 100%;
          padding: 12px 14px;
          background: $bg-tertiary;
          border: 1px solid $border-color;
          border-radius: $radius-md;
          cursor: pointer;
          text-align: left;
          transition: background-color 0.15s ease, border-color 0.15s ease;

          &:hover {
            border-color: $accent-color;
            background: $bg-card;

            .connect-action-text {
              color: $accent-light;
            }
          }

          .saved-device-meta {
            display: flex;
            flex-direction: column;
            gap: 3px;

            .saved-device-name {
              display: flex;
              align-items: center;
              gap: 8px;
              color: $text-primary;
              font-size: 14px;

              .saved-device-dot {
                width: 8px;
                height: 8px;
                border-radius: $radius-full;
                background: $text-disabled;
                flex-shrink: 0;
                transition: background 0.2s ease, box-shadow 0.2s ease;

                &.online {
                  background: #10b981;
                  box-shadow: 0 0 8px rgba(16, 185, 129, 0.6);
                }

                &.offline {
                  background: #ef4444;
                  opacity: 0.6;
                }

                &.checking {
                  background: #f59e0b;
                  animation: pulse-dot 1.2s infinite ease-in-out;
                }
              }

              .saved-device-tag {
                font-size: 10.5px;
                padding: 1px 6px;
                border-radius: $radius-sm;
                background: rgba(255, 255, 255, 0.08);
                color: $text-secondary;
                text-transform: uppercase;
                font-weight: 500;
              }

              .device-status-badge {
                font-size: 10px;
                padding: 1px 6px;
                border-radius: $radius-sm;
                font-weight: 500;

                &.online {
                  background: rgba(16, 185, 129, 0.15);
                  color: #34d399;
                  border: 1px solid rgba(16, 185, 129, 0.25);
                }

                &.offline {
                  background: rgba(239, 68, 68, 0.12);
                  color: #f87171;
                  border: 1px solid rgba(239, 68, 68, 0.2);
                }

                &.checking {
                  background: rgba(245, 158, 11, 0.12);
                  color: #fbbf24;
                  border: 1px solid rgba(245, 158, 11, 0.2);
                }
              }
            }

            .saved-device-sub {
              font-size: 11.5px;
              color: $text-muted;
              font-family: $font-mono;
              padding-left: 16px;
            }
          }

          .connect-action-text {
            font-size: 12.5px;
            font-weight: 600;
            color: $accent-color;
            transition: color 0.15s ease;
            flex-shrink: 0;
            margin-left: 12px;
          }
        }
      }

      .add-device-btn {
        width: 100%;
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 8px;
        padding: 11px 16px;
        font-size: 13.5px;
        border-style: dashed;
        border-color: $border-light;
        margin-top: 4px;

        &:hover {
          border-color: $accent-color;
          color: $text-primary;
        }
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
      width: 76px;
      height: 76px;
      display: flex;
      align-items: center;
      justify-content: center;
      border-radius: 50%;
      background: radial-gradient(circle, rgba(99, 102, 241, 0.12) 0%, transparent 70%);
      box-shadow: 0 0 24px rgba(99, 102, 241, 0.25);

      .spinner-svg {
        width: 100%;
        height: 100%;
        animation: rotate 1.8s linear infinite;
        transform-origin: center;
        overflow: visible;
        border-radius: 50%;

        .spinner-bg {
          stroke: rgba(255, 255, 255, 0.08);
        }

        .spinner-bar {
          stroke-linecap: round;
          animation: dash 1.6s ease-in-out infinite;
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
        pointer-events: none;
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
  @keyframes pulse-dot {
    0%, 100% {
      opacity: 1;
      transform: scale(1);
    }
    50% {
      opacity: 0.4;
      transform: scale(0.85);
    }
  }

  @keyframes spin {
    100% {
      transform: rotate(360deg);
    }
  }
</style>
