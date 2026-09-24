use std::collections::{HashMap, HashSet};
use crate::models::InstalledCore;
use crate::ssh_client::RemoteSession;

pub async fn fetch_installed_cores(session: &RemoteSession) -> Result<Vec<InstalledCore>, String> {
    // 1. 광범위한 다중 OS 코어 및 info 파일 동적 탐색 쉘 스크립트
    let scan_cmd = r#"sh -c '
    # 코어 파일 검색 후보 디렉토리
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
        /mnt/SDCARD/RetroArch/cores
        /mnt/SDCARD/.retroarch/cores
    "

    # info 파일 검색 후보 디렉토리
    CANDIDATE_INFO_DIRS="
        /usr/share/libretro/info
        /storage/.config/retroarch/cores
        /userdata/system/configs/retroarch/cores
        /home/ark/.config/retroarch/cores
        /opt/retropie/configs/all/retroarch/cores
    "

    # 1. 존재하는 info 디렉토리 찾기
    INFO_DIR=""
    for idir in $CANDIDATE_INFO_DIRS; do
        if [ -d "$idir" ]; then
            INFO_DIR="$idir"
            break
        fi
    done

    # 2. 존재하는 코어 디렉토리들에서 .so 파일 수집
    found_cores=""
    collect_from_dir() {
        dir="$1"
        [ -d "$dir" ] || return 0
        for f in $(find "$dir" -maxdepth 2 -name "*libretro*.so" -o -name "*.so" 2>/dev/null); do
            [ -f "$f" ] || continue
            fname=$(basename "$f")
            
            case " $found_cores " in
                *" $fname "*) continue ;;
            esac
            found_cores="$found_cores $fname"

            base=${fname%_libretro.so}
            base=${base%.so}

            dname=""
            sname=""
            cname=""
            exts=""
            auth=""
            lic=""

            # info 파일 확인
            inf=""
            if [ -n "$INFO_DIR" ]; then
                for cand in "${base}_libretro.info" "${fname%.so}.info" "${base}.info"; do
                    if [ -f "$INFO_DIR/$cand" ]; then
                        inf="$INFO_DIR/$cand"
                        break
                    fi
                done
            fi

            if [ -n "$inf" ] && [ -f "$inf" ]; then
                dname=$(grep -E "^display_name[[:space:]]*=" "$inf" 2>/dev/null | head -n 1 | cut -d"=" -f2- | tr -d "\"'\r" | sed -e "s/^[[:space:]]*//" -e "s/[[:space:]]*$//")
                sname=$(grep -E "^systemname[[:space:]]*=" "$inf" 2>/dev/null | head -n 1 | cut -d"=" -f2- | tr -d "\"'\r" | sed -e "s/^[[:space:]]*//" -e "s/[[:space:]]*$//")
                cname=$(grep -E "^corename[[:space:]]*=" "$inf" 2>/dev/null | head -n 1 | cut -d"=" -f2- | tr -d "\"'\r" | sed -e "s/^[[:space:]]*//" -e "s/[[:space:]]*$//")
                exts=$(grep -E "^supported_extensions[[:space:]]*=" "$inf" 2>/dev/null | head -n 1 | cut -d"=" -f2- | tr -d "\"'\r" | sed -e "s/^[[:space:]]*//" -e "s/[[:space:]]*$//")
                auth=$(grep -E "^authors[[:space:]]*=" "$inf" 2>/dev/null | head -n 1 | cut -d"=" -f2- | tr -d "\"'\r" | sed -e "s/^[[:space:]]*//" -e "s/[[:space:]]*$//")
                lic=$(grep -E "^license[[:space:]]*=" "$inf" 2>/dev/null | head -n 1 | cut -d"=" -f2- | tr -d "\"'\r" | sed -e "s/^[[:space:]]*//" -e "s/[[:space:]]*$//")
            fi

            echo "CORE|$base|$fname|$dname|$sname|$cname|$exts|$auth|$lic"
        done
    }

    for cdir in $CANDIDATE_CORE_DIRS; do
        collect_from_dir "$cdir"
    done

    # 만약 위 후보에서 하나도 못 찾았다면 /usr, /storage, /userdata, /opt 대상 심층 탐색
    if [ -z "$found_cores" ]; then
        for deep_root in /usr/lib /storage /userdata /opt /home; do
            [ -d "$deep_root" ] || continue
            for f in $(find "$deep_root" -maxdepth 4 -name "*_libretro.so" 2>/dev/null); do
                [ -f "$f" ] || continue
                fname=$(basename "$f")
                base=${fname%_libretro.so}
                base=${base%.so}
                echo "CORE|$base|$fname||||||"
            done
        done
    fi
    '"#;

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

    // 3. es_systems.cfg XML 파싱
    let mut core_to_systems: HashMap<String, HashSet<String>> = HashMap::new();
    let es_cfg_paths = [
        "/usr/share/emulationstation/es_systems.cfg",
        "/etc/emulationstation/es_systems.cfg",
        "/userdata/system/configs/emulationstation/es_systems.cfg",
        "/storage/.config/emulationstation/es_systems.cfg",
        "/etc/emulationstation/es_systems.xml",
        "/opt/retropie/configs/all/emulationstation/es_systems.cfg",
    ];

    for path in es_cfg_paths {
        if let Ok(xml_content) = session.read_file_string(path).await {
            parse_es_systems_cores(&xml_content, &mut core_to_systems);
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
        if !line.starts_with("CORE|") {
            continue;
        }
        let parts: Vec<&str> = line.split('|').collect();
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
                .split(|c| c == '|' || c == ',')
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

/// es_systems.cfg XML에서 <system>의 <name>과 <core> 매핑 추출
fn parse_es_systems_cores(xml: &str, map: &mut HashMap<String, HashSet<String>>) {
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
            if line.contains("<name>") && current_system.is_empty() {
                if let Some(start) = line.find("<name>") {
                    if let Some(end) = line.find("</name>") {
                        current_system = line[start + 6..end].trim().to_lowercase();
                    }
                }
            }

            if line.contains("<core>") && !current_system.is_empty() {
                if let Some(start) = line.find("<core>") {
                    if let Some(end) = line.find("</core>") {
                        let core_id = line[start + 6..end].trim().to_lowercase();
                        if !core_id.is_empty() {
                            map.entry(core_id)
                                .or_default()
                                .insert(current_system.clone());
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
        // Nintendo
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
        "snes9x" => Some(BuiltinCoreInfo {
            display_name: "Super Nintendo (Snes9x)",
            core_name: "Snes9x",
            system_name: "Nintendo - Super NES",
            supported_extensions: &["smc", "sfc", "fig", "zip", "7z"],
            supported_systems: &["snes", "sfc"],
            authors: Some("Snes9x Team"),
            license: Some("Non-commercial"),
        }),
        "snes9x2010" | "snes9x2005" => Some(BuiltinCoreInfo {
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
        "melonds" => Some(BuiltinCoreInfo {
            display_name: "Nintendo DS (melonDS)",
            core_name: "melonDS",
            system_name: "Nintendo - Nintendo DS",
            supported_extensions: &["nds", "zip", "7z"],
            supported_systems: &["nds"],
            authors: Some("Arisotura"),
            license: Some("GPLv3"),
        }),
        "desmume" => Some(BuiltinCoreInfo {
            display_name: "Nintendo DS (DeSmuME)",
            core_name: "DeSmuME",
            system_name: "Nintendo - Nintendo DS",
            supported_extensions: &["nds", "bin"],
            supported_systems: &["nds"],
            authors: Some("YopSolo"),
            license: Some("GPLv2"),
        }),

        // Sony
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
        "mednafen_psx" | "beetle_psx" => Some(BuiltinCoreInfo {
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
        "genesis_plus_gx" => Some(BuiltinCoreInfo {
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
        "flycast" => Some(BuiltinCoreInfo {
            display_name: "Dreamcast / NAOMI (Flycast)",
            core_name: "Flycast",
            system_name: "Sega - Dreamcast / NAOMI",
            supported_extensions: &["chd", "cdi", "gdi", "cue", "zip", "7z"],
            supported_systems: &["dreamcast", "naomi", "atomiswave"],
            authors: Some("flyinghead"),
            license: Some("GPLv2"),
        }),
        "yabasanshiro" | "beetle_saturn" | "mednafen_saturn" => Some(BuiltinCoreInfo {
            display_name: "Sega Saturn (Beetle / YabaSanshiro)",
            core_name: "Saturn Core",
            system_name: "Sega - Saturn",
            supported_extensions: &["chd", "cue", "iso", "m3u"],
            supported_systems: &["saturn"],
            authors: Some("Mednafen Team"),
            license: Some("GPLv2"),
        }),

        // Arcade
        "fbneo" => Some(BuiltinCoreInfo {
            display_name: "Arcade (FinalBurn Neo)",
            core_name: "FBNeo",
            system_name: "Arcade - FinalBurn Neo",
            supported_extensions: &["zip", "7z"],
            supported_systems: &["fbneo", "fba", "arcade", "neogeo"],
            authors: Some("FBNeo Team"),
            license: Some("Non-commercial"),
        }),
        "fbalpha2012" => Some(BuiltinCoreInfo {
            display_name: "Arcade (FB Alpha 2012)",
            core_name: "FB Alpha 2012",
            system_name: "Arcade - FB Alpha",
            supported_extensions: &["zip", "7z"],
            supported_systems: &["fba", "arcade", "neogeo"],
            authors: Some("FB Alpha Team"),
            license: Some("Non-commercial"),
        }),
        "mame2003_plus" | "mame2003" | "mame2010" | "mame" => Some(BuiltinCoreInfo {
            display_name: "Arcade (MAME)",
            core_name: "MAME",
            system_name: "Arcade - MAME",
            supported_extensions: &["zip", "chd", "7z"],
            supported_systems: &["mame", "arcade"],
            authors: Some("MAME Team"),
            license: Some("MAME License / GPL"),
        }),

        // PC & Others
        "dosbox_pure" => Some(BuiltinCoreInfo {
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
