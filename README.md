# retro-toy-tool

Tauri v2 + Svelte 5 기반의 레트로 게임 기기 ROM 및 메타데이터 관리 도구입니다.

## 주요 기능
- **SSH/SFTP 원격 연결**: Knulli, Batocera, ROCKNIX, RetroPie 등 다양한 레트로 핸드헬드 OS 지원
- **ROM 스캔 & gamelist.xml 관리**: 기기 내 ROM 파일 감지 및 EmulationStation 메타데이터 파싱/저장
- **게임 메타데이터 스크래핑**: ScreenScraper API 및 온라인 데이터베이스를 활용한 이미지, 비디오, 설명 스크래핑
- **미디어 & 메타데이터 편집기**: 박스아트, 타이틀 스크린, 비디오 프리뷰 및 게임 정보 직접 수정/업로드

## 개발 환경 설정
- Node.js (pnpm 권장)
- Rust (Cargo)
- Tauri CLI v2

```bash
# 의존성 설치
pnpm install

# 데스크톱 앱 실행 (Tauri + Svelte dev)
pnpm tauri dev

# 빌드
pnpm tauri build
```
