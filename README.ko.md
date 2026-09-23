# retro-toy-tool

[English](README.md) | **한국어**

Tauri v2 + Svelte 5 기반의 레트로 게임 기기 ROM 및 메타데이터 관리 도구입니다.

## 주요 기능
- **SSH/SFTP 원격 연결**: Knulli, Batocera, ROCKNIX, RetroPie 등 다양한 레트로 핸드헬드 OS 지원
- **ROM 스캔 & gamelist.xml 관리**: 기기 내 ROM 파일 감지 및 EmulationStation 메타데이터 파싱/저장
- **게임 메타데이터 스크래핑**: ScreenScraper API, Steam, DLsite, 위키백과 등 다양한 데이터베이스를 활용한 이미지, 비디오, 설명 스크래핑
- **미디어 & 메타데이터 편집기**: 박스아트, 타이틀 스크린, 비디오 프리뷰 및 게임 정보 직접 수정/업로드

## 개발 환경 설정
- Node.js (v22 권장, `.nvmrc` 제공)
- Rust (Cargo)
- pnpm

```bash
# 의존성 설치
pnpm install

# 데스크톱 앱 실행 (Tauri + Svelte dev)
pnpm tauri dev

# 빌드
pnpm tauri build
```

## 버전 관리 및 자동 릴리즈 배포
`main` 브랜치에 코드를 푸시할 때 **마이너(Minor) 이상의 버전 변경**(`0.1.0` -> `0.2.0`, `1.0.0` 등)이 감지되면 GitHub Actions가 자동으로 macOS(DMG), Windows(EXE), Linux(DEB/AppImage)를 빌드하여 GitHub Release에 게시합니다.

```bash
# 마이너 버전 올리기 (예: 0.1.0 -> 0.2.0)
pnpm bump:minor

# 메이저 버전 올리기 (예: 0.1.0 -> 1.0.0)
pnpm bump:major

# 패치 버전 올리기 (0.1.0 -> 0.1.1, 마이너 미만이므로 CI 릴리즈 빌드는 스킵됨)
pnpm bump:patch
```

> **참고**: `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` 3개 파일의 버전이 동시에 안전하게 업데이트됩니다. 변경사항을 커밋 후 `main`에 푸시하면 자동 배포가 시작됩니다.

## 면책 조항 (Disclaimer)
> ⚠️ **주의**: 본 프로젝트의 개발 및 유지보수 과정에는 AI 코딩 어시스턴트가 적극적으로 활용되고 있습니다.

- AI 생성 및 보조 코드가 포함되어 있어 예기치 않은 버그, 비정상적인 동작, 기기 환경에 따른 호환성 문제가 발생할 수 있습니다.
- 기기 내 ROM 파일, 세이브 파일, `gamelist.xml` 및 미디어 파일 변경 시 데이터 유실 위험이 있을 수 있으니, **작업 전 기기 및 SD 카드의 중요 데이터를 반드시 백업**하시기 바랍니다.
- 본 소프트웨어는 있는 그대로(AS-IS) 제공되며, 본 도구의 사용으로 인해 발생하는 기기 오작동, 데이터 손실 또는 기타 손해에 대해 개발자는 어떠한 책임도 지지 않습니다.
