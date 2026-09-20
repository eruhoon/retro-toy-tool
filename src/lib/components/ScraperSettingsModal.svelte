<script lang="ts">
  import type { ScraperSettings, ScreenScraperAccountStatus } from '../types';
  import { testScreenscraperAccount } from '../api';
  import { saveScraperSettings } from '../storage';
  import { X, Globe, CheckCircle, AlertCircle, Loader2, Key, User, Eye, EyeOff } from 'lucide-svelte';

  let {
    isOpen = $bindable(false),
    settings = $bindable(),
    onSaved = () => {},
  } = $props<{
    isOpen: boolean;
    settings: ScraperSettings;
    onSaved?: () => void;
  }>();

  let username = $state('');
  let password = $state('');
  let defaultSource = $state<'auto' | 'screenscraper' | 'steam' | 'dlsite' | 'wikipedia' | 'rawg'>('auto');
  let showPassword = $state(false);


  let isTesting = $state(false);
  let testResult = $state<ScreenScraperAccountStatus | null>(null);
  let saveSuccessMsg = $state('');

  // Sync state when modal opens
  $effect(() => {
    if (isOpen) {
      username = settings.screenscraper_username;
      password = settings.screenscraper_password;
      defaultSource = settings.default_source || 'auto';
      testResult = null;
      saveSuccessMsg = '';
    }
  });

  async function handleTestAccount() {
    if (!username.trim() || !password.trim()) {
      testResult = {
        valid: false,
        message: 'ScreenScraper 아이디와 비밀번호를 모두 입력해주세요.',
      };
      return;
    }

    isTesting = true;
    testResult = null;
    try {
      const res = await testScreenscraperAccount(username.trim(), password.trim());
      testResult = res;
    } catch (err: any) {
      testResult = {
        valid: false,
        message: '연결 실패: ' + (err?.message || err),
      };
    } finally {
      isTesting = false;
    }
  }

  function handleSave() {
    settings.screenscraper_username = username.trim();
    settings.screenscraper_password = password.trim();
    settings.default_source = defaultSource;
    saveScraperSettings(settings);

    saveSuccessMsg = '설정이 안전하게 저장되었습니다.';
    onSaved();
    setTimeout(() => {
      isOpen = false;
    }, 600);
  }
</script>

{#if isOpen}
  <div class="modal-backdrop" onclick={() => (isOpen = false)} role="presentation">
    <div class="modal-dialog" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
      <header class="modal-header">
        <div class="title-wrap">
          <div class="icon-wrap">
            <Globe size={18} />
          </div>
          <div>
            <h3>온라인 스크래퍼 옵션 설정</h3>
            <p>게임 정보(타이틀, 표지, 장르, 설명)를 수집할 온라인 데이터 소스를 관리합니다.</p>
          </div>
        </div>
        <button class="btn-icon" onclick={() => (isOpen = false)}>
          <X size={16} />
        </button>
      </header>

      <div class="modal-body">
        <!-- ScreenScraper Account Section -->
        <section class="section-box">
          <div class="section-title">
            <span class="badge-source">공식 표준</span>
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
                  bind:value={username}
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
                  type={showPassword ? 'text' : 'password'}
                  bind:value={password}
                  placeholder="비밀번호 입력"
                />
                <button
                  type="button"
                  class="toggle-eye"
                  onclick={() => (showPassword = !showPassword)}
                  title={showPassword ? '비밀번호 숨기기' : '비밀번호 보기'}
                >
                  {#if showPassword}
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
              class="btn-secondary test-btn"
              onclick={handleTestAccount}
              disabled={isTesting}
            >
              {#if isTesting}
                <Loader2 size={13} class="spin" /> 계정 확인 중...
              {:else}
                ScreenScraper 계정 연결 테스트
              {/if}
            </button>

            {#if testResult}
              <div class="test-feedback" class:success={testResult.valid} class:error={!testResult.valid}>
                {#if testResult.valid}
                  <CheckCircle size={14} />
                {:else}
                  <AlertCircle size={14} />
                {/if}
                <span>{testResult.message}</span>
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

      <footer class="modal-footer">
        {#if saveSuccessMsg}
          <div class="saved-msg">
            <CheckCircle size={14} /> {saveSuccessMsg}
          </div>
        {/if}
        <div class="footer-actions">
          <button class="btn-secondary" onclick={() => (isOpen = false)}>취소</button>
          <button class="btn-primary" onclick={handleSave}>설정 저장</button>
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
    width: 90%;
    max-width: 580px;
    box-shadow: $shadow-lg;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: zoom-in 0.15s ease-out;
  }

  .modal-header {
    padding: 16px 20px;
    border-bottom: 1px solid $border-color;
    display: flex;
    align-items: center;
    justify-content: space-between;

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
        margin-top: 2px;
      }
    }
  }

  .modal-body {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    max-height: 70vh;
    overflow-y: auto;
  }

  .section-box {
    background: $bg-primary;
    border: 1px solid $border-color;
    border-radius: $radius-md;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;

    .section-title {
      display: flex;
      align-items: center;
      gap: 8px;

      h4 {
        font-size: 13.5px;
        font-weight: 600;
      }

      .badge-source {
        font-size: 10px;
        padding: 2px 6px;
        border-radius: 4px;
        background: rgba(16, 185, 129, 0.15);
        color: #34d399;
        font-weight: 600;
      }
    }

    .section-desc {
      font-size: 12px;
      color: $text-secondary;
      line-height: 1.5;
    }
  }

  .field-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;

    label {
      font-size: 11.5px;
      color: $text-secondary;
      font-weight: 500;
    }

    .input-with-icon {
      position: relative;
      width: 100%;
      display: flex;
      align-items: center;

      .left-icon {
        position: absolute;
        left: 10px;
        color: $text-muted;
        pointer-events: none;
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 1;
      }

      input {
        width: 100%;
        box-sizing: border-box;
        padding: 8px 36px 8px 32px;
        font-size: 12.5px;
        background: $bg-secondary;
        border: 1px solid $border-color;
        border-radius: $radius-sm;

        &:focus {
          border-color: $accent-color;
        }
      }

      .toggle-eye {
        position: absolute;
        right: 8px;
        width: 24px;
        height: 24px;
        background: none;
        border: none;
        color: $text-muted;
        cursor: pointer;
        padding: 0;
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 2;

        &:hover {
          color: $text-primary;
        }

        :global(svg) {
          color: inherit;
        }
      }
    }

  }

  .test-row {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
    margin-top: 4px;

    .test-btn {
      font-size: 11.5px;
      padding: 6px 12px;
    }

    .test-feedback {
      display: flex;
      align-items: center;
      gap: 6px;
      font-size: 11.5px;
      font-weight: 500;

      &.success {
        color: #34d399;
      }

      &.error {
        color: #f87171;
      }
    }
  }

  .source-pills {
    display: flex;
    flex-direction: column;
    gap: 8px;

    .source-pill {
      display: flex;
      flex-direction: column;
      gap: 2px;
      padding: 10px 14px;
      border: 1px solid $border-color;
      border-radius: $radius-sm;
      cursor: pointer;
      background: $bg-secondary;
      transition: all 0.15s;

      input {
        display: none;
      }

      .pill-name {
        font-size: 12.5px;
        font-weight: 600;
        color: $text-primary;
      }

      .pill-sub {
        font-size: 11px;
        color: $text-muted;
      }

      &:hover {
        border-color: $border-light;
      }

      &.active {
        border-color: $accent-color;
        background: rgba(99, 102, 241, 0.1);

        .pill-name {
          color: $accent-light;
        }
      }
    }
  }

  .modal-footer {
    padding: 14px 20px;
    border-top: 1px solid $border-color;
    display: flex;
    align-items: center;
    justify-content: space-between;

    .saved-msg {
      display: flex;
      align-items: center;
      gap: 6px;
      font-size: 12px;
      color: #34d399;
      font-weight: 500;
    }

    .footer-actions {
      display: flex;
      align-items: center;
      gap: 8px;
      margin-left: auto;
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
