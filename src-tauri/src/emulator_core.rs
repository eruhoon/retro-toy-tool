use std::collections::{HashMap, HashSet};
use crate::models::InstalledCore;
use crate::ssh_client::RemoteSession;

pub async fn fetch_installed_cores(session: &RemoteSession) -> Result<Vec<InstalledCore>, String> {
    // 1. 광범위한 다중 OS 코어 및 info 파일 동적 탐색 쉘 스크립트 (탭 구분자 기반 안전 파싱)
    let scan_cmd = r#"
    CANDIDATE_CORE_DIRS="
        /usr/lib/libretro
        /usr/lib64/libretro
        /storage/.config/retroarch/cores
        /userdata/system/configs/retroarch/cores
        /userdata/bios/cores
        /home/ark/.config/retroarch/cores
        /opt/retroarch/cores
        /opt/retropie/libretrocores
        /opt/retropie/configs/all/retroarch/cores
        /usr/lib/arm-linux-gnueabihf/libretro
        /usr/lib/aarch64-linux-gnu/libretro
        /mnt/mmc/MUOS/emulator/retroarch/cores
        /mnt/sdcard/MUOS/emulator/retroarch/cores
        /mnt/mmc/muos/emulator/retroarch/cores
        /mnt/sdcard/muos/emulator/retroarch/cores
        /opt/muos/emulator/retroarch/cores
        /mnt/vendor/deep/retro/cores
        /mnt/sdcard/retroarch/cores
        /mnt/mmc/retroarch/cores
        /mnt/SDCARD/RetroArch/cores
        /mnt/SDCARD/.retroarch/cores
    "

    CANDIDATE_INFO_DIRS="
        /usr/share/libretro/info
        /storage/.config/retroarch/cores
        /userdata/system/configs/retroarch/cores
        /home/ark/.config/retroarch/cores
        /opt/retropie/configs/all/retroarch/cores
        /usr/share/batocera/datainit/bios/cores
        /mnt/mmc/MUOS/emulator/retroarch/info
        /mnt/sdcard/MUOS/emulator/retroarch/info
    "

    EXISTING_INFO_DIRS=""
    for idir in $CANDIDATE_INFO_DIRS; do
        if [ -d "$idir" ]; then
            EXISTING_INFO_DIRS="$EXISTING_INFO_DIRS $idir"
        fi
    done

    found_cores=""
    collect_from_dir() {
        dir="$1"
        [ -d "$dir" ] || return 0

        # 시스템 공용 라이브러리 오인식을 방지하기 위해 코어 전용 폴더와 일반 lib 폴더 분리 탐색
        case "$dir" in
            */retroarch/cores*|*/cores*|*libretrocores*)
                cores_found=$(find "$dir" -maxdepth 2 -type f \( -name "*libretro*.so" -o -name "*.so" \) 2>/dev/null)
                ;;
            *)
                cores_found=$(find "$dir" -maxdepth 2 -type f -name "*libretro*.so" 2>/dev/null)
                ;;
        esac

        for f in $cores_found; do
            [ -f "$f" ] || continue
            fname=$(basename "$f")

            case " $found_cores " in
                *" $fname "*) continue ;;
            esac
            found_cores="$found_cores $fname"

            base=${fname%_libretro.so}
            base=${base%.so}

            # info 파일 확인 (코어 위치 디렉토리 및 info 후보 디렉토리 탐색)
            inf=""
            for idir in "$dir" $EXISTING_INFO_DIRS; do
                for cand in "${base}_libretro.info" "${fname%.so}.info" "${base}.info"; do
                    if [ -f "$idir/$cand" ]; then
                        inf="$idir/$cand"
                        break 2
                    fi
                done
            done

            if [ -n "$inf" ] && [ -f "$inf" ]; then
                inf_meta=$(awk -F "=" '
                    /^[[:space:]]*display_name[[:space:]]*=/ { d=$2 }
                    /^[[:space:]]*systemname[[:space:]]*=/ { s=$2 }
                    /^[[:space:]]*corename[[:space:]]*=/ { c=$2 }
                    /^[[:space:]]*supported_extensions[[:space:]]*=/ { e=$2 }
                    /^[[:space:]]*authors[[:space:]]*=/ { a=$2 }
                    /^[[:space:]]*license[[:space:]]*=/ { l=$2 }
                    END {
                        gsub(/["\r\n\t]/, "", d); sub(/^[ ]*/, "", d); sub(/[ ]*$/, "", d);
                        gsub(/["\r\n\t]/, "", s); sub(/^[ ]*/, "", s); sub(/[ ]*$/, "", s);
                        gsub(/["\r\n\t]/, "", c); sub(/^[ ]*/, "", c); sub(/[ ]*$/, "", c);
                        gsub(/["\r\n\t]/, "", e); sub(/^[ ]*/, "", e); sub(/[ ]*$/, "", e);
                        gsub(/["\r\n\t]/, "", a); sub(/^[ ]*/, "", a); sub(/[ ]*$/, "", a);
                        gsub(/["\r\n\t]/, "", l); sub(/^[ ]*/, "", l); sub(/[ ]*$/, "", l);
                        printf "%s\t%s\t%s\t%s\t%s\t%s", d, s, c, e, a, l
                    }
                ' "$inf" 2>/dev/null)

                if [ -n "$inf_meta" ]; then
                    printf "CORE\t%s\t%s\t%s\n" "$base" "$fname" "$inf_meta"
                    continue
                fi
            fi

            printf "CORE\t%s\t%s\t\t\t\t\t\t\n" "$base" "$fname"
        done
    }

    for cdir in $CANDIDATE_CORE_DIRS; do
        collect_from_dir "$cdir"
    done

    # 만약 위 후보에서 하나도 못 찾았다면 심층 탐색
    if [ -z "$found_cores" ]; then
        for deep_root in /usr/lib /storage /userdata /opt /home /mnt; do
            [ -d "$deep_root" ] || continue
            for f in $(find "$deep_root" -maxdepth 4 -type f -name "*_libretro.so" 2>/dev/null); do
                [ -f "$f" ] || continue
                fname=$(basename "$f")
                base=${fname%_libretro.so}
                base=${base%.so}
                printf "CORE\t%s\t%s\t\t\t\t\t\t\n" "$base" "$fname"
            done
        done
    fi
    "#;

    let scan_output = session.exec_command(scan_cmd).await.unwrap_or_default();

    // 2. batocera.conf 및 배포판 설정 파일 파싱
    let mut default_cores_by_system: HashMap<String, String> = HashMap::new();
    let mut default_systems_by_core: HashMap<String, Vec<String>> = HashMap::new();

    let conf_candidates = [
        "/userdata/system/batocera.conf",
        "/storage/.config/batocera.conf",
        "/storage/.config/distribution.conf",
    ];

    for cpath in conf_candidates {
        if let Ok(conf_content) = session.read_file_string(cpath).await {
            for line in conf_content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                    continue;
                }
                if let Some((key, val)) = line.split_once('=') {
                    let key = key.trim();
                    let val = val.trim();
                    if let Some((system, prop)) = key.split_once('.') {
                        if prop == "core" && !val.is_empty() {
                            let sys = system.to_lowercase();
                            let core = val.to_lowercase();
                            default_cores_by_system.insert(sys.clone(), core.clone());
                            default_systems_by_core.entry(core).or_default().push(sys);
                        }
                    }
                }
            }
            break;
        }
    }

    // 3. es_systems.cfg XML 파싱 (속성이 포함된 <core default="true"> 등 지원)
    let mut core_to_systems: HashMap<String, HashSet<String>> = HashMap::new();
    let es_cfg_paths = [
        "/var/run/emulationstation/es_systems.cfg",
        "/usr/share/emulationstation/es_systems.cfg",
        "/usr/share/batocera/datainit/system/configs/emulationstation/es_systems.cfg",
        "/etc/emulationstation/es_systems.cfg",
        "/userdata/system/configs/emulationstation/es_systems.cfg",
        "/storage/.config/emulationstation/es_systems.cfg",
        "/etc/emulationstation/es_systems.xml",
        "/opt/retropie/configs/all/emulationstation/es_systems.cfg",
    ];

    for path in es_cfg_paths {
        if let Ok(xml_content) = session.read_file_string(path).await {
            parse_es_systems_cores(&xml_content, &mut core_to_systems, &mut default_systems_by_core);
            if !core_to_systems.is_empty() {
                break;
            }
        }
    }

    // 4. 스캔 결과 라인별 구조화 + 내장 코어 메타데이터 사전 매칭
    let mut cores = Vec::new();
    let mut seen_ids = HashSet::new();

    for line in scan_output.lines() {
        let line = line.trim();
        if !line.starts_with("CORE\t") {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 9 {
            continue;
        }

        let id = parts[1].trim().to_lowercase();
        let file_name = parts[2].trim().to_string();
        let display_name_raw = parts[3].trim();
        let system_name_raw = parts[4].trim();
        let core_name_raw = parts[5].trim();
        let exts_raw = parts[6].trim();
        let authors_raw = parts[7].trim();
        let license_raw = parts[8].trim();

        if seen_ids.contains(&file_name) {
            continue;
        }
        seen_ids.insert(file_name.clone());

        // 내장 코어 사전 조회 (info 파일이 없는 기기를 위한 강력한 Fallback)
        let fallback = get_builtin_core_info(&id);

        let display_name = if !display_name_raw.is_empty() {
            display_name_raw.to_string()
        } else if let Some(fb) = &fallback {
            fb.display_name.to_string()
        } else if !core_name_raw.is_empty() {
            core_name_raw.to_string()
        } else {
            id.clone()
        };

        let core_name = if !core_name_raw.is_empty() {
            core_name_raw.to_string()
        } else if let Some(fb) = &fallback {
            fb.core_name.to_string()
        } else {
            id.clone()
        };

        let system_name = if !system_name_raw.is_empty() {
            system_name_raw.to_string()
        } else if let Some(fb) = &fallback {
            fb.system_name.to_string()
        } else {
            "기타 / 미분류".to_string()
        };

        // 지원 확장자 분리 ("gba|gb|gbc" or "gba, gb")
        let mut supported_extensions: Vec<String> = if !exts_raw.is_empty() {
            exts_raw
                .split(|c| c == '|' || c == ',' || c == ' ')
                .map(|s| s.trim().trim_start_matches('.').to_lowercase())
                .filter(|s| !s.is_empty())
                .collect()
        } else if let Some(fb) = &fallback {
            fb.supported_extensions.iter().map(|s| s.to_string()).collect()
        } else {
            Vec::new()
        };
        supported_extensions.sort();
        supported_extensions.dedup();

        // 지원 기종 목록 매핑 (es_systems.cfg 기반 + 내장 사전)
        let mut supported_systems: Vec<String> = Vec::new();
        if let Some(sys_set) = core_to_systems.get(&id) {
            supported_systems.extend(sys_set.iter().cloned());
        }
        let base_id = id.trim_end_matches("_libretro");
        if base_id != id {
            if let Some(sys_set) = core_to_systems.get(base_id) {
                for s in sys_set {
                    if !supported_systems.contains(s) {
                        supported_systems.push(s.clone());
                    }
                }
            }
        }
        if let Some(fb) = &fallback {
            for sys in fb.supported_systems {
                let s_str = sys.to_string();
                if !supported_systems.contains(&s_str) {
                    supported_systems.push(s_str);
                }
            }
        }

        supported_systems.sort();
        supported_systems.dedup();

        // 기본 코어로 지정된 시스템 목록
        let mut is_default_for: Vec<String> = Vec::new();
        if let Some(sys_list) = default_systems_by_core.get(&id) {
            is_default_for.extend(sys_list.iter().cloned());
        }
        if base_id != id {
            if let Some(sys_list) = default_systems_by_core.get(base_id) {
                for s in sys_list {
                    if !is_default_for.contains(s) {
                        is_default_for.push(s.clone());
                    }
                }
            }
        }
        is_default_for.sort();
        is_default_for.dedup();

        cores.push(InstalledCore {
            id,
            file_name,
            display_name,
            core_name,
            system_name,
            supported_extensions,
            authors: if !authors_raw.is_empty() {
                Some(authors_raw.to_string())
            } else {
                fallback.as_ref().and_then(|f| f.authors.map(|s| s.to_string()))
            },
            license: if !license_raw.is_empty() {
                Some(license_raw.to_string())
            } else {
                fallback.as_ref().and_then(|f| f.license.map(|s| s.to_string()))
            },
            supported_systems,
            is_default_for,
        });
    }

    // 이름순 정렬
    cores.sort_by(|a, b| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()));

    Ok(cores)
}

/// es_systems.cfg XML에서 <system>의 <name>과 <core> 매핑 추출 (속성 및 default 지원)
fn parse_es_systems_cores(
    xml: &str,
    core_to_systems: &mut HashMap<String, HashSet<String>>,
    default_systems_by_core: &mut HashMap<String, Vec<String>>,
) {
    let mut in_system = false;
    let mut current_system = String::new();

    for line in xml.lines() {
        let line = line.trim();
        if line.contains("<system>") {
            in_system = true;
            current_system.clear();
        } else if line.contains("</system>") {
            in_system = false;
            current_system.clear();
        }

        if in_system {
            if current_system.is_empty() {
                if let Some(start_tag) = line.find("<name") {
                    if let Some(close_bracket) = line[start_tag..].find('>') {
                        let content_start = start_tag + close_bracket + 1;
                        if let Some(end_tag) = line[content_start..].find("</name>") {
                            current_system = line[content_start..content_start + end_tag].trim().to_lowercase();
                        }
                    }
                }
            }

            if !current_system.is_empty() {
                if let Some(start_tag) = line.find("<core") {
                    if let Some(close_bracket) = line[start_tag..].find('>') {
                        let tag_str = &line[start_tag..start_tag + close_bracket];
                        let is_default = tag_str.contains("default=\"true\"") || tag_str.contains("default='true'");

                        let content_start = start_tag + close_bracket + 1;
                        if let Some(end_tag) = line[content_start..].find("</core>") {
                            let core_id = line[content_start..content_start + end_tag].trim().to_lowercase();
                            if !core_id.is_empty() {
                                core_to_systems.entry(core_id.clone())
                                    .or_default()
                                    .insert(current_system.clone());

                                if is_default {
                                    let list = default_systems_by_core.entry(core_id).or_default();
                                    if !list.contains(&current_system) {
                                        list.push(current_system.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// info 파일이 없더라도 널리 쓰이는 주요 레트로 코어를 식별할 수 있는 내장 메타데이터
struct BuiltinCoreInfo {
    pub display_name: &'static str,
    pub core_name: &'static str,
    pub system_name: &'static str,
    pub supported_extensions: &'static [&'static str],
    pub supported_systems: &'static [&'static str],
    pub authors: Option<&'static str>,
    pub license: Option<&'static str>,
}

fn get_builtin_core_info(id: &str) -> Option<BuiltinCoreInfo> {
    let clean = id.trim_end_matches("_libretro");
    match clean {
        // Nintendo - Game Boy Advance
        "mgba" => Some(BuiltinCoreInfo {
            display_name: "Game Boy Advance (mGBA)",
            core_name: "mGBA",
            system_name: "Nintendo - Game Boy Advance",
            supported_extensions: &["gba", "gb", "gbc"],
            supported_systems: &["gba", "gb", "gbc"],
            authors: Some("Vicki Pfau"),
            license: Some("MPL 2.0"),
        }),
        "gpsp" => Some(BuiltinCoreInfo {
            display_name: "Game Boy Advance (gpSP)",
            core_name: "gpSP",
            system_name: "Nintendo - Game Boy Advance",
            supported_extensions: &["gba", "bin", "zip"],
            supported_systems: &["gba"],
            authors: Some("Exophase"),
            license: Some("GPLv2"),
        }),
        "vbam" => Some(BuiltinCoreInfo {
            display_name: "Game Boy Advance (VBA-M)",
            core_name: "VBA-M",
            system_name: "Nintendo - Game Boy Advance",
            supported_extensions: &["gba", "gb", "gbc"],
            supported_systems: &["gba", "gb", "gbc"],
            authors: Some("VBA-M Team"),
            license: Some("GPLv2"),
        }),

        // Nintendo - Game Boy / Color
        "gambatte" => Some(BuiltinCoreInfo {
            display_name: "Game Boy / Color (Gambatte)",
            core_name: "Gambatte",
            system_name: "Nintendo - Game Boy / Color",
            supported_extensions: &["gb", "gbc", "dmg", "zip"],
            supported_systems: &["gb", "gbc"],
            authors: Some("Sinamas"),
            license: Some("GPLv2"),
        }),
        "gearboy" => Some(BuiltinCoreInfo {
            display_name: "Game Boy / Color (Gearboy)",
            core_name: "Gearboy",
            system_name: "Nintendo - Game Boy / Color",
            supported_extensions: &["gb", "gbc", "dmg"],
            supported_systems: &["gb", "gbc"],
            authors: Some("Ignacio Sanchez"),
            license: Some("GPLv3"),
        }),
        "sameboy" => Some(BuiltinCoreInfo {
            display_name: "Game Boy / Color (SameBoy)",
            core_name: "SameBoy",
            system_name: "Nintendo - Game Boy / Color",
            supported_extensions: &["gb", "gbc", "dmg", "zip"],
            supported_systems: &["gb", "gbc"],
            authors: Some("Lior Halphon"),
            license: Some("Expat"),
        }),

        // Nintendo - Super NES
        "snes9x" => Some(BuiltinCoreInfo {
            display_name: "Super Nintendo (Snes9x)",
            core_name: "Snes9x",
            system_name: "Nintendo - Super NES",
            supported_extensions: &["smc", "sfc", "fig", "zip", "7z"],
            supported_systems: &["snes", "sfc"],
            authors: Some("Snes9x Team"),
            license: Some("Non-commercial"),
        }),
        "snes9x2010" | "snes9x2005" | "snes9x2002" => Some(BuiltinCoreInfo {
            display_name: "Super Nintendo (Snes9x 2010)",
            core_name: "Snes9x 2010",
            system_name: "Nintendo - Super NES",
            supported_extensions: &["smc", "sfc", "zip"],
            supported_systems: &["snes", "sfc"],
            authors: Some("Snes9x Team"),
            license: Some("Non-commercial"),
        }),
        "bsnes" | "bsnes_hd" => Some(BuiltinCoreInfo {
            display_name: "Super Nintendo (bsnes)",
            core_name: "bsnes",
            system_name: "Nintendo - Super NES",
            supported_extensions: &["smc", "sfc", "bml"],
            supported_systems: &["snes", "sfc"],
            authors: Some("Near"),
            license: Some("GPLv3"),
        }),
        "mesen-s" => Some(BuiltinCoreInfo {
            display_name: "Super Nintendo (Mesen-S)",
            core_name: "Mesen-S",
            system_name: "Nintendo - Super NES",
            supported_extensions: &["sfc", "smc", "fig", "zip"],
            supported_systems: &["snes", "sfc"],
            authors: Some("Sour"),
            license: Some("GPLv3"),
        }),

        // Nintendo - NES / Famicom
        "fceumm" => Some(BuiltinCoreInfo {
            display_name: "NES / Famicom (FCEUmm)",
            core_name: "FCEUmm",
            system_name: "Nintendo - NES / Famicom",
            supported_extensions: &["nes", "fds", "unf", "unif", "zip"],
            supported_systems: &["nes", "fc", "fds"],
            authors: Some("FCEU Team"),
            license: Some("GPLv2"),
        }),
        "nestopia" => Some(BuiltinCoreInfo {
            display_name: "NES / Famicom (Nestopia UE)",
            core_name: "Nestopia",
            system_name: "Nintendo - NES / Famicom",
            supported_extensions: &["nes", "fds", "unf", "zip"],
            supported_systems: &["nes", "fc", "fds"],
            authors: Some("Martin Freij"),
            license: Some("GPLv2"),
        }),
        "mesen" => Some(BuiltinCoreInfo {
            display_name: "NES / Famicom (Mesen)",
            core_name: "Mesen",
            system_name: "Nintendo - NES / Famicom",
            supported_extensions: &["nes", "fds", "unf", "zip"],
            supported_systems: &["nes", "fc", "fds"],
            authors: Some("Sour"),
            license: Some("GPLv3"),
        }),
        "quicknes" => Some(BuiltinCoreInfo {
            display_name: "NES / Famicom (QuickNES)",
            core_name: "QuickNES",
            system_name: "Nintendo - NES / Famicom",
            supported_extensions: &["nes", "zip"],
            supported_systems: &["nes", "fc"],
            authors: Some("Blargg, Paul Kodchar"),
            license: Some("GPLv2"),
        }),

        // Nintendo - Nintendo 64
        "mupen64plus_next" | "mupen64plus" => Some(BuiltinCoreInfo {
            display_name: "Nintendo 64 (Mupen64Plus-Next)",
            core_name: "Mupen64Plus-Next",
            system_name: "Nintendo - Nintendo 64",
            supported_extensions: &["n64", "v64", "z64", "zip", "7z"],
            supported_systems: &["n64"],
            authors: Some("Mupen64Plus Team"),
            license: Some("GPLv3"),
        }),
        "parallel_n64" => Some(BuiltinCoreInfo {
            display_name: "Nintendo 64 (ParaLLEl N64)",
            core_name: "ParaLLEl N64",
            system_name: "Nintendo - Nintendo 64",
            supported_extensions: &["n64", "v64", "z64", "zip"],
            supported_systems: &["n64"],
            authors: Some("Hacktarux"),
            license: Some("GPLv2"),
        }),

        // Nintendo - Nintendo DS / 3DS
        "melonds" | "melonds_ds" => Some(BuiltinCoreInfo {
            display_name: "Nintendo DS (melonDS)",
            core_name: "melonDS",
            system_name: "Nintendo - Nintendo DS",
            supported_extensions: &["nds", "zip", "7z"],
            supported_systems: &["nds"],
            authors: Some("Arisotura"),
            license: Some("GPLv3"),
        }),
        "desmume" | "desmume2015" => Some(BuiltinCoreInfo {
            display_name: "Nintendo DS (DeSmuME)",
            core_name: "DeSmuME",
            system_name: "Nintendo - Nintendo DS",
            supported_extensions: &["nds", "bin"],
            supported_systems: &["nds"],
            authors: Some("YopSolo"),
            license: Some("GPLv2"),
        }),
        "citra" | "citra_canary" => Some(BuiltinCoreInfo {
            display_name: "Nintendo 3DS (Citra)",
            core_name: "Citra",
            system_name: "Nintendo - Nintendo 3DS",
            supported_extensions: &["3ds", "3dsx", "elf", "cci", "cxi"],
            supported_systems: &["3ds"],
            authors: Some("Citra Team"),
            license: Some("GPLv2+"),
        }),
        "pokemini" => Some(BuiltinCoreInfo {
            display_name: "Pokemon Mini (PokeMini)",
            core_name: "PokeMini",
            system_name: "Nintendo - Pokemon Mini",
            supported_extensions: &["min", "zip"],
            supported_systems: &["pokemini"],
            authors: Some("JustBurn"),
            license: Some("GPLv3"),
        }),

        // Sony - PlayStation
        "pcsx_rearmed" => Some(BuiltinCoreInfo {
            display_name: "PlayStation (PCSX ReARMed)",
            core_name: "PCSX ReARMed",
            system_name: "Sony - PlayStation",
            supported_extensions: &["chd", "cue", "pbp", "bin", "iso", "img"],
            supported_systems: &["psx", "ps1"],
            authors: Some("PCSX Team, Notaz"),
            license: Some("GPLv2"),
        }),
        "duckstation" | "swanstation" => Some(BuiltinCoreInfo {
            display_name: "PlayStation (DuckStation)",
            core_name: "DuckStation",
            system_name: "Sony - PlayStation",
            supported_extensions: &["chd", "cue", "iso", "pbp", "m3u"],
            supported_systems: &["psx", "ps1"],
            authors: Some("Stenzek"),
            license: Some("GPLv3"),
        }),
        "mednafen_psx" | "beetle_psx" | "mednafen_psx_hw" | "beetle_psx_hw" => Some(BuiltinCoreInfo {
            display_name: "PlayStation (Beetle PSX)",
            core_name: "Beetle PSX",
            system_name: "Sony - PlayStation",
            supported_extensions: &["chd", "cue", "toc", "m3u", "ccd"],
            supported_systems: &["psx", "ps1"],
            authors: Some("Mednafen Team"),
            license: Some("GPLv2"),
        }),
        "ppsspp" => Some(BuiltinCoreInfo {
            display_name: "PSP (PPSSPP)",
            core_name: "PPSSPP",
            system_name: "Sony - PlayStation Portable",
            supported_extensions: &["iso", "cso", "chd", "pbp", "elf"],
            supported_systems: &["psp"],
            authors: Some("Henrik Rydgård"),
            license: Some("GPLv2+"),
        }),

        // Sega
        "genesis_plus_gx" | "genesis_plus_gx_wide" => Some(BuiltinCoreInfo {
            display_name: "Genesis / Mega Drive (Genesis Plus GX)",
            core_name: "Genesis Plus GX",
            system_name: "Sega - Mega Drive / Genesis",
            supported_extensions: &["md", "smd", "gen", "bin", "cue", "chd", "sms", "gg"],
            supported_systems: &["megadrive", "genesis", "mastersystem", "gamegear", "segacd"],
            authors: Some("Charles MacDonald, Eke-Eke"),
            license: Some("Non-commercial"),
        }),
        "picodrive" => Some(BuiltinCoreInfo {
            display_name: "Genesis / 32X / Sega CD (PicoDrive)",
            core_name: "PicoDrive",
            system_name: "Sega - 32X / Mega Drive / CD",
            supported_extensions: &["bin", "gen", "smd", "md", "32x", "cue", "chd", "iso"],
            supported_systems: &["megadrive", "genesis", "sega32x", "segacd"],
            authors: Some("Notaz"),
            license: Some("MAME"),
        }),
        "gearsystem" => Some(BuiltinCoreInfo {
            display_name: "Master System / Game Gear (Gearsystem)",
            core_name: "Gearsystem",
            system_name: "Sega - Master System / Game Gear",
            supported_extensions: &["sms", "gg", "sg", "bin", "zip", "7z"],
            supported_systems: &["mastersystem", "gamegear", "sg-1000", "sg1000"],
            authors: Some("Ignacio Sanchez"),
            license: Some("GPLv3"),
        }),
        "flycast" => Some(BuiltinCoreInfo {
            display_name: "Dreamcast / NAOMI (Flycast)",
            core_name: "Flycast",
            system_name: "Sega - Dreamcast / NAOMI",
            supported_extensions: &["chd", "cdi", "gdi", "cue", "zip", "7z"],
            supported_systems: &["dreamcast", "naomi", "atomiswave"],
            authors: Some("flyinghead"),
            license: Some("GPLv2"),
        }),
        "yabasanshiro" | "beetle_saturn" | "mednafen_saturn" | "kronos" => Some(BuiltinCoreInfo {
            display_name: "Sega Saturn (Beetle / YabaSanshiro / Kronos)",
            core_name: "Saturn Core",
            system_name: "Sega - Saturn",
            supported_extensions: &["chd", "cue", "iso", "m3u"],
            supported_systems: &["saturn"],
            authors: Some("Mednafen Team, FCare"),
            license: Some("GPLv2"),
        }),

        // NEC - PC Engine / TurboGrafx-16
        "beetle_pce_fast" | "mednafen_pce_fast" | "beetle_pce" | "mednafen_pce" | "pce_fast" => Some(BuiltinCoreInfo {
            display_name: "PC Engine / TurboGrafx-16 (Beetle PCE Fast)",
            core_name: "Beetle PCE Fast",
            system_name: "NEC - PC Engine / TurboGrafx-16",
            supported_extensions: &["pce", "cue", "ccd", "chd", "toc", "bin", "iso"],
            supported_systems: &["pce", "pcengine", "tg16", "pcecd", "tg-cd"],
            authors: Some("Mednafen Team"),
            license: Some("GPLv2"),
        }),
        "beetle_supergrafx" | "mednafen_supergrafx" => Some(BuiltinCoreInfo {
            display_name: "PC Engine SuperGrafx (Beetle SuperGrafx)",
            core_name: "Beetle SuperGrafx",
            system_name: "NEC - PC Engine SuperGrafx",
            supported_extensions: &["sgx", "pce", "cue", "chd"],
            supported_systems: &["supergrafx", "sgx", "pce"],
            authors: Some("Mednafen Team"),
            license: Some("GPLv2"),
        }),
        "beetle_pcfx" | "mednafen_pcfx" => Some(BuiltinCoreInfo {
            display_name: "PC-FX (Beetle PC-FX)",
            core_name: "Beetle PC-FX",
            system_name: "NEC - PC-FX",
            supported_extensions: &["cue", "ccd", "toc", "chd", "bin"],
            supported_systems: &["pcfx"],
            authors: Some("Mednafen Team"),
            license: Some("GPLv2"),
        }),

        // SNK & Arcade
        "fbneo" => Some(BuiltinCoreInfo {
            display_name: "Arcade (FinalBurn Neo)",
            core_name: "FBNeo",
            system_name: "Arcade - FinalBurn Neo",
            supported_extensions: &["zip", "7z"],
            supported_systems: &["fbneo", "fba", "arcade", "neogeo"],
            authors: Some("FBNeo Team"),
            license: Some("Non-commercial"),
        }),
        "fbalpha2012" | "fbalpha" => Some(BuiltinCoreInfo {
            display_name: "Arcade (FB Alpha 2012)",
            core_name: "FB Alpha 2012",
            system_name: "Arcade - FB Alpha",
            supported_extensions: &["zip", "7z"],
            supported_systems: &["fba", "arcade", "neogeo"],
            authors: Some("FB Alpha Team"),
            license: Some("Non-commercial"),
        }),
        "fbalpha2012_cps1" | "fbalpha2012_cps2" | "fbalpha2012_cps3" => Some(BuiltinCoreInfo {
            display_name: "Capcom CPS 1/2/3 (FB Alpha)",
            core_name: "FB Alpha CPS",
            system_name: "Arcade - Capcom CPS",
            supported_extensions: &["zip", "7z"],
            supported_systems: &["cps1", "cps2", "cps3", "arcade"],
            authors: Some("FB Alpha Team"),
            license: Some("Non-commercial"),
        }),
        "mame2003_plus" | "mame2003" | "mame2010" | "mame2015" | "mame2016" | "mame" => Some(BuiltinCoreInfo {
            display_name: "Arcade (MAME)",
            core_name: "MAME",
            system_name: "Arcade - MAME",
            supported_extensions: &["zip", "chd", "7z"],
            supported_systems: &["mame", "arcade"],
            authors: Some("MAME Team"),
            license: Some("MAME License / GPL"),
        }),
        "beetle_ngp" | "mednafen_ngp" | "race" => Some(BuiltinCoreInfo {
            display_name: "Neo Geo Pocket / Color (Beetle NeoPop)",
            core_name: "Beetle NeoPop",
            system_name: "SNK - Neo Geo Pocket / Color",
            supported_extensions: &["ngp", "ngc", "zip", "7z"],
            supported_systems: &["ngp", "ngpc", "neogeopocket"],
            authors: Some("Mednafen Team"),
            license: Some("GPLv2"),
        }),
        "neocd" | "geolith" => Some(BuiltinCoreInfo {
            display_name: "Neo Geo / CD (NeoCD)",
            core_name: "NeoCD",
            system_name: "SNK - Neo Geo CD",
            supported_extensions: &["cue", "chd", "iso", "zip", "neo"],
            supported_systems: &["neogeocd", "neocd", "neogeo"],
            authors: Some("Fabrice Martinez"),
            license: Some("GPLv3"),
        }),

        // Bandai - WonderSwan
        "beetle_wswan" | "mednafen_wswan" => Some(BuiltinCoreInfo {
            display_name: "WonderSwan / Color (Beetle Cygne)",
            core_name: "Beetle Cygne",
            system_name: "Bandai - WonderSwan / Color",
            supported_extensions: &["ws", "wsc", "zip", "7z"],
            supported_systems: &["wswan", "wsc", "wonderswan", "wonderswancolor"],
            authors: Some("Mednafen Team, Dox"),
            license: Some("GPLv2"),
        }),

        // Atari
        "stella" | "stella2014" => Some(BuiltinCoreInfo {
            display_name: "Atari 2600 (Stella)",
            core_name: "Stella",
            system_name: "Atari - 2600",
            supported_extensions: &["a26", "bin", "zip", "7z"],
            supported_systems: &["atari2600", "a2600", "atari"],
            authors: Some("Stephen Anthony"),
            license: Some("GPLv2"),
        }),
        "prosystem" => Some(BuiltinCoreInfo {
            display_name: "Atari 7800 (ProSystem)",
            core_name: "ProSystem",
            system_name: "Atari - 7800",
            supported_extensions: &["a78", "bin", "zip"],
            supported_systems: &["atari7800", "a7800"],
            authors: Some("Greg Stanton"),
            license: Some("GPLv2"),
        }),
        "handy" | "beetle_lynx" | "mednafen_lynx" => Some(BuiltinCoreInfo {
            display_name: "Atari Lynx (Handy)",
            core_name: "Handy",
            system_name: "Atari - Lynx",
            supported_extensions: &["lnx", "o", "zip"],
            supported_systems: &["atarilynx", "lynx"],
            authors: Some("K. Wilkins"),
            license: Some("Zlib"),
        }),
        "hatari" => Some(BuiltinCoreInfo {
            display_name: "Atari ST (Hatari)",
            core_name: "Hatari",
            system_name: "Atari - ST",
            supported_extensions: &["st", "msa", "stx", "dim", "ipf", "zip"],
            supported_systems: &["atarist", "atarios"],
            authors: Some("Hatari Team"),
            license: Some("GPLv2"),
        }),
        "atari800" => Some(BuiltinCoreInfo {
            display_name: "Atari 800 / 5200 (Atari800)",
            core_name: "Atari800",
            system_name: "Atari - 800 / 5200",
            supported_extensions: &["atr", "xfd", "atx", "cdb", "bas", "bin", "a52", "xex", "cas"],
            supported_systems: &["atari800", "atari5200"],
            authors: Some("Atari800 Development Team"),
            license: Some("GPLv2"),
        }),

        // PC & Other Retro Consoles
        "dosbox_pure" | "dosbox_svn" => Some(BuiltinCoreInfo {
            display_name: "DOS (DOSBox-Pure)",
            core_name: "DOSBox-Pure",
            system_name: "PC - DOS",
            supported_extensions: &["zip", "dosz", "exe", "com", "bat", "iso", "cue"],
            supported_systems: &["dos", "pc"],
            authors: Some("Bernhard Schelling"),
            license: Some("GPLv2"),
        }),
        "scummvm" => Some(BuiltinCoreInfo {
            display_name: "Adventure (ScummVM)",
            core_name: "ScummVM",
            system_name: "PC - ScummVM",
            supported_extensions: &["scummvm", "svm", "zip"],
            supported_systems: &["scummvm"],
            authors: Some("ScummVM Team"),
            license: Some("GPLv2+"),
        }),
        "fmsx" => Some(BuiltinCoreInfo {
            display_name: "MSX / MSX2 (fMSX)",
            core_name: "fMSX",
            system_name: "Microsoft / ASCII - MSX / MSX2",
            supported_extensions: &["rom", "mx1", "mx2", "dsk", "cas", "zip"],
            supported_systems: &["msx", "msx1", "msx2"],
            authors: Some("Marat Fayzullin"),
            license: Some("Non-commercial"),
        }),
        "bluemsx" => Some(BuiltinCoreInfo {
            display_name: "MSX / ColecoVision (blueMSX)",
            core_name: "blueMSX",
            system_name: "Microsoft / ASCII - MSX / MSX2",
            supported_extensions: &["rom", "mx1", "mx2", "dsk", "col", "sg", "sc", "cas", "zip"],
            supported_systems: &["msx", "msx1", "msx2", "colecovision", "sg1000"],
            authors: Some("Daniel Vik"),
            license: Some("GPLv2"),
        }),
        "gearcoleco" => Some(BuiltinCoreInfo {
            display_name: "ColecoVision (Gearcoleco)",
            core_name: "Gearcoleco",
            system_name: "Coleco - ColecoVision",
            supported_extensions: &["col", "bin", "rom", "zip", "7z"],
            supported_systems: &["colecovision", "coleco"],
            authors: Some("Ignacio Sanchez"),
            license: Some("GPLv3"),
        }),
        "vice_x64" | "vice_x64sc" | "vice_x128" | "vice_xvic" => Some(BuiltinCoreInfo {
            display_name: "Commodore 64 (VICE)",
            core_name: "VICE x64",
            system_name: "Commodore - C64",
            supported_extensions: &["d64", "d71", "d81", "t64", "tap", "prg", "p00", "crt", "zip"],
            supported_systems: &["c64", "commodore64", "vic20"],
            authors: Some("VICE Team"),
            license: Some("GPLv2"),
        }),
        "puae" | "puae2021" => Some(BuiltinCoreInfo {
            display_name: "Amiga (PUAE)",
            core_name: "PUAE",
            system_name: "Commodore - Amiga",
            supported_extensions: &["adf", "dms", "fdi", "ipf", "hd_img", "hdf", "lha", "cue", "chd", "iso", "m3u", "zip"],
            supported_systems: &["amiga", "amiga500", "amiga1200", "amigacd32"],
            authors: Some("Mustafa 'GnoStiC' TUFAN, Toni Wilen"),
            license: Some("GPLv2"),
        }),
        "fuse" => Some(BuiltinCoreInfo {
            display_name: "ZX Spectrum (Fuse)",
            core_name: "Fuse",
            system_name: "Sinclair - ZX Spectrum",
            supported_extensions: &["tzx", "tap", "z80", "szx", "sna", "dsk", "trd", "scl", "zip"],
            supported_systems: &["zxspectrum"],
            authors: Some("Philip Kendall"),
            license: Some("GPLv3"),
        }),
        "opera" | "4do" => Some(BuiltinCoreInfo {
            display_name: "3DO (Opera)",
            core_name: "Opera",
            system_name: "The 3DO Company - 3DO",
            supported_extensions: &["iso", "bin", "chd", "cue"],
            supported_systems: &["3do"],
            authors: Some("Johnny 'Thunder' F., Trapexoid"),
            license: Some("GPLv2"),
        }),
        "vecx" => Some(BuiltinCoreInfo {
            display_name: "Vectrex (vecx)",
            core_name: "vecx",
            system_name: "GCE - Vectrex",
            supported_extensions: &["vec", "bin", "zip"],
            supported_systems: &["vectrex"],
            authors: Some("Valavan Manohararajah"),
            license: Some("GPLv3"),
        }),
        "gw" => Some(BuiltinCoreInfo {
            display_name: "Game & Watch (GW)",
            core_name: "GW",
            system_name: "Nintendo - Game & Watch",
            supported_extensions: &["mgw", "zip"],
            supported_systems: &["gameandwatch", "gw"],
            authors: Some("Andre Leiradella"),
            license: Some("Zlib"),
        }),
        "tic80" => Some(BuiltinCoreInfo {
            display_name: "TIC-80 (TIC-80)",
            core_name: "TIC-80",
            system_name: "TIC-80",
            supported_extensions: &["tic", "zip"],
            supported_systems: &["tic80"],
            authors: Some("Nesbox"),
            license: Some("MIT"),
        }),
        "retro8" | "fake08" => Some(BuiltinCoreInfo {
            display_name: "PICO-8 (Fake-08 / Retro8)",
            core_name: "Pico-8",
            system_name: "Lexaloffle - PICO-8",
            supported_extensions: &["p8", "png"],
            supported_systems: &["pico8"],
            authors: Some("jtothebell"),
            license: Some("MIT"),
        }),
        "prboom" => Some(BuiltinCoreInfo {
            display_name: "Doom Engine (PrBoom)",
            core_name: "PrBoom",
            system_name: "PC - Doom",
            supported_extensions: &["wad", "iwad", "pwad"],
            supported_systems: &["doom", "ports"],
            authors: Some("Florian Schulze, Colin Phipps"),
            license: Some("GPLv2"),
        }),
        "nxengine" => Some(BuiltinCoreInfo {
            display_name: "Cave Story (NXEngine)",
            core_name: "NXEngine",
            system_name: "PC - Cave Story",
            supported_extensions: &["exe"],
            supported_systems: &["cavestory", "ports"],
            authors: Some("CaitSith2"),
            license: Some("GPLv3"),
        }),
        _ => None,
    }
}
