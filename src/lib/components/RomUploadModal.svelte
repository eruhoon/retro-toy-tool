<script lang="ts">
  import type { RomUploadProgressPayload, RomUploadResult } from '../types';
  import { HardDriveUpload, CheckCircle2, AlertCircle, X, File, Loader2 } from 'lucide-svelte';

  let {
    isOpen = false,
    systemName = '',
    deviceName = '',
    progress = null,
    result = null,
    isUploading = false,
    onClose = () => {},
  } = $props<{
    isOpen: boolean;
    systemName: string;
    deviceName: string;
    progress: RomUploadProgressPayload | null;
    result: RomUploadResult | null;
    isUploading: boolean;
    onClose: () => void;
  }>();

  function formatBytes(bytes: number): string {
    if (!bytes || bytes <= 0) return '0 B';
    if (bytes < 1024) return bytes + ' B';
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
    if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
    return (bytes / (1024 * 1024 * 1024)).toFixed(2) + ' GB';
  }

  function formatSpeed(bps: number): string {
    if (!bps || bps <= 0) return '-';
    if (bps < 1024 * 1024) return (bps / 1024).toFixed(1) + ' KB/s';
    return (bps / (1024 * 1024)).toFixed(1) + ' MB/s';
  }

  let filePercent = $derived.by(() => {
    if (!progress || progress.total_bytes === 0) return 0;
    const p = (progress.current_bytes / progress.total_bytes) * 100;
    return Math.min(100, Math.max(0, p));
  });

  let overallPercent = $derived.by(() => {
    if (!progress || progress.overall_total_bytes === 0) return 0;
    const p = (progress.overall_current_bytes / progress.overall_total_bytes) * 100;
    return Math.min(100, Math.max(0, p));
  });
</script>

{#if isOpen}
  <div class="modal-overlay">
    <div class="modal-card">
      <header class="modal-header">
        <div class="header-title">
          <HardDriveUpload size={18} class="title-icon" />
          <h3>ROM 파일 전송</h3>
          <span class="system-tag">{systemName}</span>
        </div>
        {#if !isUploading}
          <button class="close-btn" onclick={onClose} title="닫기">
            <X size={16} />
          </button>
        {/if}
      </header>

      <div class="modal-body">
        {#if isUploading}
          <!-- In Progress View -->
          <div class="upload-status-box">
            <div class="file-info-row">
              <div class="file-name-wrap">
                <File size={16} class="file-icon" />
                <span class="current-filename" title={progress?.file_name}>
                  {progress?.file_name || '파일 준비 중...'}
                </span>
              </div>
              <div class="file-count-badge">
                {progress?.file_index || 1} / {progress?.total_files || 1} 파일
              </div>
            </div>

            <!-- Current File Progress Bar -->
            <div class="progress-section">
              <div class="progress-header">
                <span class="progress-label">현재 파일 진행률</span>
                <span class="progress-pct">{filePercent.toFixed(1)}%</span>
              </div>
              <div class="progress-track">
                <div class="progress-fill current-fill" style="width: {filePercent}%;"></div>
              </div>
              <div class="progress-meta">
                <span>{formatBytes(progress?.current_bytes || 0)} / {formatBytes(progress?.total_bytes || 0)}</span>
                <span class="speed-text">{formatSpeed(progress?.bytes_per_sec || 0)}</span>
              </div>
            </div>

            <!-- Overall Progress Bar (Shown when multiple files) -->
            {#if (progress?.total_files || 0) > 1}
              <div class="progress-section overall-section">
                <div class="progress-header">
                  <span class="progress-label">전체 복사 진행률 ({overallPercent.toFixed(1)}%)</span>
                  <span class="progress-sub">
                    {formatBytes(progress?.overall_current_bytes || 0)} / {formatBytes(progress?.overall_total_bytes || 0)}
                  </span>
                </div>
                <div class="progress-track overall-track">
                  <div class="progress-fill overall-fill" style="width: {overallPercent}%;"></div>
                </div>
              </div>
            {/if}

            <div class="loading-hint">
              <Loader2 size={14} class="spin" />
              <span>'{deviceName}'(으)로 SFTP 안전 스트리밍 전송 중입니다. 잠시만 기다려주세요...</span>
            </div>
          </div>
        {:else if result}
          <!-- Finished View -->
          <div class="result-box">
            {#if result.failed_files.length === 0}
              <div class="result-banner success">
                <CheckCircle2 size={32} class="result-icon success" />
                <div class="result-texts">
                  <h4>모든 ROM 파일 복사 완료!</h4>
                  <p>{result.message}</p>
                </div>
              </div>
            {:else}
              <div class="result-banner warning">
                <AlertCircle size={32} class="result-icon warning" />
                <div class="result-texts">
                  <h4>일부 파일 전송 실패</h4>
                  <p>{result.message}</p>
                </div>
              </div>

              <div class="failed-list">
                <div class="failed-title">실패한 파일 목록:</div>
                <ul>
                  {#each result.failed_files as failed}
                    <li>{failed}</li>
                  {/each}
                </ul>
              </div>
            {/if}

            <div class="summary-details">
              <div class="summary-item">
                <span class="summary-label">전송 성공</span>
                <span class="summary-val success">{result.success_count}개 파일</span>
              </div>
              <div class="summary-item">
                <span class="summary-label">총 전송 용량</span>
                <span class="summary-val">{formatBytes(progress?.overall_total_bytes || 0)}</span>
              </div>
              <div class="summary-item">
                <span class="summary-label">대상 경로</span>
                <span class="summary-val mono">{systemName} ROM 디렉토리</span>
              </div>
            </div>
          </div>
        {/if}
      </div>

      <footer class="modal-footer">
        {#if !isUploading}
          <button class="btn-primary" onclick={onClose}>확인</button>
        {:else}
          <span class="transferring-note">전송 중 창을 닫지 마세요.</span>
        {/if}
      </footer>
    </div>
  </div>
{/if}

<style lang="scss">
  .modal-overlay {
    position: fixed;
    inset: 0;
    background-color: rgba(0, 0, 0, 0.72);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    animation: fadeIn 0.15s ease-out;
  }

  .modal-card {
    background-color: #1a1e29;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 12px;
    width: 90%;
    max-width: 520px;
    box-shadow: 0 16px 36px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 18px;
    background-color: #151821;
    border-bottom: 1px solid rgba(255, 255, 255, 0.07);

    .header-title {
      display: flex;
      align-items: center;
      gap: 9px;

      .title-icon {
        color: #3b82f6;
      }

      h3 {
        margin: 0;
        font-size: 15px;
        font-weight: 600;
        color: #f1f5f9;
      }

      .system-tag {
        font-size: 11px;
        background-color: rgba(59, 130, 246, 0.15);
        color: #60a5fa;
        padding: 2px 7px;
        border-radius: 4px;
        font-weight: 500;
      }
    }

    .close-btn {
      background: none;
      border: none;
      color: #94a3b8;
      cursor: pointer;
      padding: 4px;
      border-radius: 4px;
      display: flex;
      align-items: center;
      justify-content: center;
      transition: all 0.15s;

      &:hover {
        background-color: rgba(255, 255, 255, 0.08);
        color: #f8fafc;
      }
    }
  }

  .modal-body {
    padding: 20px;
  }

  .upload-status-box {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .file-info-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background-color: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 8px;
    padding: 10px 14px;

    .file-name-wrap {
      display: flex;
      align-items: center;
      gap: 8px;
      min-width: 0;
      flex: 1;

      .file-icon {
        color: #93c5fd;
        flex-shrink: 0;
      }

      .current-filename {
        font-size: 13px;
        font-weight: 500;
        color: #f1f5f9;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
      }
    }

    .file-count-badge {
      font-size: 11px;
      font-weight: 600;
      background-color: rgba(59, 130, 246, 0.2);
      color: #93c5fd;
      padding: 3px 8px;
      border-radius: 12px;
      margin-left: 8px;
      flex-shrink: 0;
    }
  }

  .progress-section {
    display: flex;
    flex-direction: column;
    gap: 6px;

    .progress-header {
      display: flex;
      justify-content: space-between;
      font-size: 12px;

      .progress-label {
        color: #94a3b8;
      }

      .progress-pct {
        font-weight: 600;
        color: #3b82f6;
      }

      .progress-sub {
        color: #64748b;
        font-size: 11px;
      }
    }

    .progress-track {
      width: 100%;
      height: 8px;
      background-color: rgba(255, 255, 255, 0.08);
      border-radius: 4px;
      overflow: hidden;

      .progress-fill {
        height: 100%;
        transition: width 0.12s ease-out;
      }

      .current-fill {
        background: linear-gradient(90deg, #3b82f6, #60a5fa);
      }

      .overall-fill {
        background: linear-gradient(90deg, #10b981, #34d399);
      }
    }

    .progress-meta {
      display: flex;
      justify-content: space-between;
      font-size: 11px;
      color: #94a3b8;

      .speed-text {
        color: #38bdf8;
        font-weight: 600;
      }
    }
  }

  .overall-section {
    border-top: 1px dashed rgba(255, 255, 255, 0.08);
    padding-top: 12px;
  }

  .loading-hint {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11.5px;
    color: #94a3b8;
    margin-top: 4px;

    .spin {
      color: #3b82f6;
      animation: spin 1s linear infinite;
      flex-shrink: 0;
    }
  }

  /* Result View */
  .result-box {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .result-banner {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 16px;
    border-radius: 8px;

    &.success {
      background-color: rgba(16, 185, 129, 0.12);
      border: 1px solid rgba(16, 185, 129, 0.25);

      .result-icon.success {
        color: #10b981;
      }
    }

    &.warning {
      background-color: rgba(245, 158, 11, 0.12);
      border: 1px solid rgba(245, 158, 11, 0.25);

      .result-icon.warning {
        color: #f59e0b;
      }
    }

    .result-texts {
      h4 {
        margin: 0 0 4px 0;
        font-size: 14px;
        font-weight: 600;
        color: #f8fafc;
      }

      p {
        margin: 0;
        font-size: 12px;
        color: #cbd5e1;
      }
    }
  }

  .failed-list {
    background-color: rgba(239, 68, 68, 0.08);
    border: 1px solid rgba(239, 68, 68, 0.2);
    border-radius: 8px;
    padding: 10px 14px;
    font-size: 12px;

    .failed-title {
      font-weight: 600;
      color: #fca5a5;
      margin-bottom: 6px;
    }

    ul {
      margin: 0;
      padding-left: 18px;
      color: #f87171;
      max-height: 100px;
      overflow-y: auto;
    }
  }

  .summary-details {
    background-color: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 8px;
    padding: 10px 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;

    .summary-item {
      display: flex;
      justify-content: space-between;
      font-size: 12px;

      .summary-label {
        color: #94a3b8;
      }

      .summary-val {
        color: #f1f5f9;
        font-weight: 500;

        &.success {
          color: #34d399;
        }

        &.mono {
          font-family: monospace;
          font-size: 11px;
        }
      }
    }
  }

  .modal-footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    padding: 12px 18px;
    background-color: #151821;
    border-top: 1px solid rgba(255, 255, 255, 0.07);

    .transferring-note {
      font-size: 11px;
      color: #64748b;
    }

    .btn-primary {
      background-color: #2563eb;
      color: white;
      border: none;
      padding: 7px 18px;
      border-radius: 6px;
      font-size: 12.5px;
      font-weight: 500;
      cursor: pointer;
      transition: background-color 0.15s;

      &:hover {
        background-color: #1d4ed8;
      }
    }
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
</style>
