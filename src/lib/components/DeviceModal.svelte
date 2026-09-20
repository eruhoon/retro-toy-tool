<script lang="ts">
  import { DEVICE_PRESETS, type DevicePreset, type DeviceProfile, type StorageLocation } from '../types';
  import { testConnection } from '../api';
  import { CheckCircle, AlertCircle, Loader2, X, Plus, Trash2, Cpu, RefreshCw, HardDrive, Smartphone } from 'lucide-svelte';

  let {
    isOpen = $bindable(false),
    profiles = $bindable([]),
    activeDevice = $bindable(null),
    onDeviceConnected = () => {},
  } = $props<{
    isOpen: boolean;
    profiles: DeviceProfile[];
    activeDevice: DeviceProfile | null;
    onDeviceConnected?: (device: DeviceProfile) => void;
  }>();

  let selectedPreset = $state<DevicePreset['id']>('knulli');
  let name = $state('내 Knulli 게임기');
  let host = $state('');
  let port = $state(22);
  let username = $state('root');
  let password = $state('linux');
  let romsPath = $state('/userdata/roms');

  let detectedStorages = $state<StorageLocation[]>([]);
  let isTesting = $state(false);
  let testSuccess = $state<boolean | null>(null);
  let testMsg = $state('');
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
    testSuccess = null;
    testMsg = '';
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
    testSuccess = null;
    testMsg = '';
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
      testSuccess = false;
      testMsg = 'IP 주소를 입력하세요.';
      return;
    }
    isTesting = true;
    testSuccess = null;
    testMsg = '';

    try {
      const res = await testConnection(host, port, username, password, romsPath);
      if (res.success) {
        testSuccess = true;
        detectedStorages = res.storages || [];
        if (res.detected_roms_path && res.detected_roms_path !== romsPath) {
          romsPath = res.detected_roms_path;
          testMsg = `연결 성공! 기기의 ROM 경로(${res.os_name || ''})를 자동으로 감지했습니다: ${res.detected_roms_path}`;
        } else {
          testMsg = `연결 성공! (${res.detected_roms_path || romsPath})`;
        }
      } else {
        testSuccess = false;
        testMsg = res.message;
        detectedStorages = [];
      }
    } catch (err: any) {
      testSuccess = false;
      testMsg = typeof err === 'string' ? err : err?.message || '연결 실패';
      detectedStorages = [];
    } finally {
      isTesting = false;
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
</script>

{#if isOpen}
  <div class="modal-backdrop" onclick={() => (isOpen = false)} role="presentation">
    <div class="modal-container" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
      <div class="modal-header">
        <div class="title-wrap">
          <Cpu class="icon" size={22} />
          <h2>게임기 기기 프리셋 & 연결 관리</h2>
        </div>
        <button class="btn-ghost icon-btn" onclick={() => (isOpen = false)}>
          <X size={18} />
        </button>
      </div>

      <div class="modal-body">
        <!-- Sidebar: Saved devices -->
        <div class="device-list-pane">
          <div class="pane-header">
            <h3>등록된 기기 ({profiles.length})</h3>
            <button class="btn-secondary sm-btn" onclick={startNewProfile}>
              <Plus size={14} /> 새 기기
            </button>
          </div>
          <div class="device-items">
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
                    <div class="device-name">{p.name}</div>
                    <div class="device-sub">{p.host}:{p.port} ({p.device_type})</div>
                  </div>
                  <button
                    class="btn-ghost delete-btn"
                    title="기기 삭제"
                    onclick={(e) => {
                      e.stopPropagation();
                      deleteProfile(p.id);
                    }}
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              {/each}
            {/if}
          </div>
        </div>

        <!-- Right: Device configuration form -->
        <div class="device-form-pane">
          <div class="preset-section">
            <label class="form-label">기기 OS 프리셋 선택</label>
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
              <label>기기 별명 (프로필 이름)</label>
              <input type="text" bind:value={name} placeholder="예: 거실용 Knulli RG40XX" />
            </div>

            <div class="field ip-field">
              <label>IP 주소 / 호스트</label>
              <input type="text" bind:value={host} placeholder="예: 192.168.0.50 또는 knulli.local" />
            </div>

            <div class="field port-field">
              <label>SSH 포트</label>
              <input type="number" bind:value={port} min="1" max="65535" />
            </div>

            <div class="field">
              <label>아이디 (User)</label>
              <input type="text" bind:value={username} placeholder="root" />
            </div>

            <div class="field">
              <label>비밀번호 (Password)</label>
              <input type="password" bind:value={password} placeholder="기본 비밀번호" />
            </div>

            <div class="field full">
              <label>ROMs 디렉토리 경로 (EmulationStation/ES-DE)</label>
              <input type="text" bind:value={romsPath} placeholder="/userdata/roms 또는 /storage/roms" />
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

          {#if testSuccess !== null || isTesting}
            <div
              class="test-result-box"
              class:success={testSuccess === true}
              class:error={testSuccess === false}
              class:loading={isTesting}
            >
              {#if isTesting}
                <Loader2 class="spin" size={16} />
                <span>게임기에 연결을 시도하고 ROM 경로를 탐색하는 중...</span>
              {:else if testSuccess === true}
                <CheckCircle size={16} />
                <span>{testMsg}</span>
              {:else}
                <AlertCircle size={16} />
                <span>{testMsg}</span>
              {/if}
            </div>
          {/if}

          <div class="modal-footer">
            <button
              type="button"
              class="btn-secondary"
              disabled={isTesting || !host.trim()}
              onclick={handleTestConnection}
            >
              {#if isTesting}
                <Loader2 class="spin" size={14} /> 테스트 중...
              {:else}
                <RefreshCw size={14} /> 연결 테스트 & 경로 탐색
              {/if}
            </button>

            <button
              type="button"
              class="btn-primary"
              disabled={isTesting || !host.trim() || !name.trim()}
              onclick={handleSaveAndConnect}
            >
              저장하고 게임기 접속
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
{/if}

<style lang="scss">
  @use '../../styles/variables' as *;

  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal-container {
    background: $bg-secondary;
    border: 1px solid $border-color;
    border-radius: $radius-lg;
    width: 820px;
    max-width: 95vw;
    box-shadow: $shadow-lg;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    background: $bg-tertiary;
    border-bottom: 1px solid $border-color;

    .title-wrap {
      display: flex;
      align-items: center;
      gap: 10px;

      .icon {
        color: $accent-color;
      }

      h2 {
        font-size: 16px;
        font-weight: 600;
      }
    }
  }

  .modal-body {
    display: grid;
    grid-template-columns: 260px 1fr;
    min-height: 440px;
  }

  .device-list-pane {
    background: $bg-primary;
    border-right: 1px solid $border-color;
    display: flex;
    flex-direction: column;

    .pane-header {
      padding: 14px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      border-bottom: 1px solid $border-color;

      h3 {
        font-size: 13px;
        color: $text-secondary;
      }

      .sm-btn {
        padding: 4px 8px;
        font-size: 12px;
      }
    }

    .device-items {
      padding: 10px;
      overflow-y: auto;
      flex: 1;
      display: flex;
      flex-direction: column;
      gap: 8px;
    }

    .empty-hint {
      color: $text-muted;
      font-size: 12px;
      text-align: center;
      padding: 20px 10px;
      line-height: 1.5;
    }

    .device-card {
      background: $bg-secondary;
      border: 1px solid $border-color;
      border-radius: $radius-md;
      padding: 10px 12px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      cursor: pointer;
      transition: all 0.15s ease;

      &:hover {
        background: $bg-tertiary;
        border-color: $border-light;
      }

      &.active {
        border-color: $accent-color;
        background: $bg-card;
      }

      &.connected {
        border-left: 3px solid $status-registered;
      }

      .device-info {
        .device-name {
          font-weight: 500;
          font-size: 13px;
        }
        .device-sub {
          font-size: 11px;
          color: $text-muted;
          margin-top: 2px;
        }
      }

      .delete-btn {
        padding: 4px;
        color: $text-muted;
        &:hover {
          color: #ef4444;
        }
      }
    }
  }

  .device-form-pane {
    padding: 20px;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
  }

  .preset-section {
    margin-bottom: 16px;

    .form-label {
      font-size: 12px;
      color: $text-secondary;
      margin-bottom: 8px;
      display: block;
    }

    .preset-pills {
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
    }

    .preset-pill {
      background: $bg-tertiary;
      border: 1px solid $border-color;
      color: $text-secondary;
      padding: 6px 12px;
      border-radius: $radius-full;
      font-size: 12px;
      cursor: pointer;
      transition: all 0.15s ease;

      &:hover {
        border-color: $border-light;
        color: $text-primary;
      }

      &.selected {
        background: $accent-color;
        border-color: $accent-color;
        color: white;
        box-shadow: 0 0 8px $accent-glow;
      }
    }
  }

  .form-grid {
    display: grid;
    grid-template-columns: 1fr 100px;
    gap: 14px;
    margin-bottom: 16px;

    .field {
      display: flex;
      flex-direction: column;
      gap: 6px;

      label {
        font-size: 12px;
        color: $text-secondary;
        font-weight: 500;
      }

      &.full {
        grid-column: 1 / -1;
      }

      .field-hint {
        font-size: 11px;
        color: $text-muted;
      }

      .detected-storages-list {
        margin-top: 8px;
        display: flex;
        flex-direction: column;
        gap: 6px;

        .detected-label {
          font-size: 11px;
          color: $text-muted;
          font-weight: 500;
        }

        .detected-pills {
          display: flex;
          flex-wrap: wrap;
          gap: 6px;

          .detected-pill-btn {
            display: inline-flex;
            align-items: center;
            gap: 5px;
            padding: 5px 10px;
            font-size: 11px;
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

            &.selected {
              background: rgba(99, 102, 241, 0.2);
              border-color: #6366f1;
              color: #818cf8;
              font-weight: 600;
            }
          }
        }
      }
    }
  }

  .test-result-box {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    border-radius: $radius-md;
    font-size: 12px;
    margin-bottom: 16px;

    &.loading {
      background: rgba(99, 102, 241, 0.1);
      color: #818cf8;
      border: 1px solid rgba(99, 102, 241, 0.2);
    }

    &.success {
      background: rgba(16, 185, 129, 0.1);
      color: #34d399;
      border: 1px solid rgba(16, 185, 129, 0.2);
    }

    &.error {
      background: rgba(239, 68, 68, 0.1);
      color: #f87171;
      border: 1px solid rgba(239, 68, 68, 0.2);
    }
  }

  .modal-footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 10px;
    margin-top: auto;
    padding-top: 14px;
    border-top: 1px solid $border-color;
  }

  :global(.spin) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
</style>
