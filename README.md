# retro-toy-tool

**English** | [한국어](README.ko.md)

A modern desktop ROM and metadata manager for retro handheld gaming devices, built with **Tauri v2** and **Svelte 5**.

---

## ✨ Features

- **SSH / SFTP Remote Connection**: Seamlessly connects to devices running popular retro handheld OSes, such as Knulli, Batocera, ROCKNIX, RetroPie, and more.
- **ROM Scanning & `gamelist.xml` Management**: Automatically detects ROM files and parses, updates, and saves EmulationStation XML metadata directly on the device.
- **Multi-Source Scraping**: Scrapes game metadata, high-resolution box arts, title screens, video previews, and synopses from ScreenScraper, Steam, DLsite, Wikipedia, and RAWG.
- **Media & Metadata Editor**: Easily edit game titles, descriptions, release dates, genres, and upload custom images or videos.

---

## 🛠️ Development Setup

### Prerequisites

- **Node.js**: v22 or later (`.nvmrc` provided)
- **Package Manager**: [pnpm](https://pnpm.io/)
- **Rust & Cargo**: Latest stable toolchain
- **Platform Dependencies**:
  - **Linux**: Standard Tauri v2 packages (`libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, etc.)
  - **macOS / Windows**: Standard build tools (Xcode CLI / MSVC C++ Build Tools)

### Getting Started

```bash
# 1. Install frontend dependencies
pnpm install

# 2. Run in desktop development mode (Vite + Tauri)
pnpm tauri dev

# 3. Build production binaries locally
pnpm tauri build
```

---

## 🚀 Versioning & Automated Releases

This repository is configured with GitHub Actions to automatically build and publish multi-platform releases (**macOS DMG**, **Windows EXE**, **Linux DEB/AppImage**) when a **minor or major version bump** is pushed to the `main` branch.

```bash
# Bump minor version (e.g., 0.1.0 -> 0.2.0) - Triggers automated CI release
pnpm bump:minor

# Bump major version (e.g., 0.1.0 -> 1.0.0) - Triggers automated CI release
pnpm bump:major

# Bump patch version (e.g., 0.1.0 -> 0.1.1) - Skips CI release build
pnpm bump:patch
```

> **Note**: The bump scripts keep `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml` synchronized automatically. After bumping, commit and push to `main` to start the release process.

---

## ⚠️ Disclaimer

> **Important**: This project actively utilizes AI coding assistants for development and code refactoring.

- Code generated or assisted by AI may contain unforeseen bugs, unintended behaviors, or compatibility issues with specific device firmware and network environments.
- Modifying ROMs, save files, `gamelist.xml`, or media assets over SFTP carries inherent risk of data loss. **Always back up your SD card and critical save files before using this tool.**
- This software is distributed on an **"AS IS"** BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. The developers assume no liability for device malfunction, bricking, data loss, or other damages incurred through its use.

---

## 📄 License

This project is licensed under the MIT License.
