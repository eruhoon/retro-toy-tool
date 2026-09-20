use std::collections::{HashMap, HashSet};
use crate::models::{GameItem, GameStatus, SystemPlatform};
use crate::ssh_client::RemoteSession;
use crate::gamelist_xml::{parse_gamelist_xml, serialize_gamelist_xml, normalize_rel_path, GameListXml, GameXml};

// Common platform friendly names
pub fn get_platform_display_name(id: &str) -> String {
    match id.to_lowercase().as_str() {
        "snes" | "sfc" | "superfamicom" => "Super Nintendo (SFC/SNES)",
        "nes" | "fc" | "famicom" => "Nintendo Famicom (FC/NES)",
        "gba" | "gbadvance" | "gameboyadvance" => "Game Boy Advance",
        "gbc" | "gbcolor" | "gameboycolor" => "Game Boy Color",
        "gb" | "gameboy" => "Game Boy",
        "megadrive" | "genesis" | "md" => "Sega Genesis / Mega Drive",
        "mastersystem" | "ms" => "Sega Master System",
        "gamegear" | "gg" => "Sega Game Gear",
        "segacd" => "Sega CD / Mega CD",
        "sega32x" => "Sega 32X",
        "psx" | "ps1" | "ps" => "Sony PlayStation",
        "psp" => "Sony PlayStation Portable",
        "nds" => "Nintendo DS",
        "n64" => "Nintendo 64",
        "arcade" | "mame" | "fbneo" => "Arcade (MAME/FBNeo)",
        "cps1" => "Capcom Play System I",
        "cps2" => "Capcom Play System II",
        "cps3" => "Capcom Play System III",
        "neogeo" => "SNK Neo Geo",
        "pce" | "pcengine" => "PC Engine / TurboGrafx-16",
        "pcecd" => "PC Engine CD",
        "wonderswan" | "ws" => "WonderSwan",
        "wonderswancolor" | "wsc" => "WonderSwan Color",
        "dreamcast" | "dc" => "Sega Dreamcast",
        "saturn" => "Sega Saturn",
        "atarivc" | "atrap" | "at2600" => "Atari 2600",
        "at7800" => "Atari 7800",
        "lynx" => "Atari Lynx",
        "ports" | "port" => "Ports",
        "scummvm" => "ScummVM",
        "dos" | "pc" => "MS-DOS / PC",
        "pico-8" | "pico8" => "PICO-8",
        _ => "",
    }.to_string()
}

// Check if a directory entry is a media/system folder rather than a game file
fn is_media_or_system_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    matches!(
        lower.as_str(),
        "images" | "media" | "videos" | "marquees" | "thumbnails" |
        "covers" | "screenshots" | "wheels" | "titles" | "fanart" |
        "manuals" | "gamelist.xml" | "gamelist.xml.bak" | ".ds_store" | "thumbs.db" |
        "bios" | "savestates" | "saves" | "themes" | "theme" |
        "downloads" | "download" | "backups" | "backup" | "cheats" |
        "music" | "sound" | "sounds" | "overlays" | "shaders" | "system" |
        "records" | "recordings" | "logs" | "decorations" | "extra" |
        "configs" | "config" | "tools" | "package" | "packages" |
        "retroarch" | "kodi" | "splash" | "lost+found"
    )
}

// Check if a directory itself is a game bundle (e.g. game.scummvm, game.pc, game.dos)
fn is_folder_game_bundle(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".scummvm") ||
    lower.ends_with(".pc") ||
    lower.ends_with(".dos") ||
    lower.ends_with(".squashfs")
}

// Check if a file should be ignored from being treated as a ROM (e.g. txt, nfo, logs, manuals, scripts, media)
fn is_ignored_rom_file(system_id: &str, name: &str) -> bool {
    let lower = name.to_lowercase();
    if lower.starts_with('.') || is_media_or_system_name(&lower) {
        return true;
    }

    let sys_lower = system_id.to_lowercase();

    // Special handling for Ports (PortMaster / ES-DE):
    // In ports, .sh launcher scripts (and .squashfs) ARE the game executables!
    if sys_lower == "ports" || sys_lower == "port" {
        if lower.ends_with(".sh") || lower.ends_with(".squashfs") {
            return false; // Valid port game launcher
        }
        return true; // Ignore auxiliary data files, text files, and configs in ports root
    }

    // PICO-8: .p8 and .p8.png are carts!
    if (sys_lower == "pico-8" || sys_lower == "pico8") && lower.ends_with(".p8.png") {
        return false;
    }

    // DOS / PC: .bat, .com, .exe are launchers!
    if (sys_lower == "dos" || sys_lower == "pc") && (lower.ends_with(".bat") || lower.ends_with(".exe") || lower.ends_with(".com")) {
        return false;
    }

    // RetroArch / emulator save states (.state, .state1, .state.auto, etc.)
    if let Some(pos) = lower.rfind(".state") {
        let suffix = &lower[pos + 6..];
        if suffix.is_empty() || suffix == ".auto" || suffix == ".bak" || suffix.chars().all(|c| c.is_ascii_digit()) {
            return true;
        }
    }

    // General console ignored extensions (documents, configs, media, save files)
    const IGNORED_EXTS: &[&str] = &[
        ".txt", ".nfo", ".md", ".log", ".pdf", ".doc", ".docx", ".rtf",
        ".xml", ".bak", ".json", ".yaml", ".yml", ".ini", ".cfg", ".conf",
        ".png", ".jpg", ".jpeg", ".gif", ".webp", ".mp4", ".mkv", ".avi",
        ".db", ".sh", ".bat", ".dat",
        ".srm", ".sav", ".dsv", ".rtc", ".cht", ".mcr", ".mpk", ".nvram", ".bsv",
        ".ips", ".bps", ".ups", ".opt", ".rmp"
    ];

    for ext in IGNORED_EXTS {
        if lower.ends_with(ext) {
            return true;
        }
    }

    false
}

#[derive(Debug, Clone)]
pub struct ScannedRomFile {
    pub rel_path: String, // e.g. "Celeste.sh" or "sub/Celeste.sh"
    pub filename: String, // e.g. "Celeste.sh"
    pub size: u64,
}

/// Filters out companion CD track data files (.bin, .img) when their descriptor (.cue, .ccd, .m3u, .gdi) is present in the same directory.
fn filter_companion_track_files(roms: Vec<ScannedRomFile>) -> Vec<ScannedRomFile> {
    // Group rom files by directory (relative path directory)
    let mut dir_groups: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, rom) in roms.iter().enumerate() {
        let dir = rom.rel_path.rfind('/').map(|i| &rom.rel_path[..i]).unwrap_or("").to_string();
        dir_groups.entry(dir).or_default().push(idx);
    }

    let mut remove_indices = HashSet::new();

    for (_dir, indices) in dir_groups {
        // Collect cue/descriptor stems in this directory
        let mut descriptor_stems: Vec<String> = Vec::new();

        for &i in &indices {
            let fname = &roms[i].filename;
            let lower = fname.to_lowercase();
            if lower.ends_with(".cue") || lower.ends_with(".ccd") || lower.ends_with(".m3u") || lower.ends_with(".gdi") {
                let stem = if let Some(dot_pos) = fname.rfind('.') {
                    &fname[..dot_pos]
                } else {
                    fname
                };
                descriptor_stems.push(stem.to_lowercase());
            }
        }

        if descriptor_stems.is_empty() {
            continue;
        }

        // For each file in this directory, check if it is a companion .bin / .img track file
        for &i in &indices {
            let fname = &roms[i].filename;
            let lower = fname.to_lowercase();

            if lower.ends_with(".bin") || lower.ends_with(".img") {
                let bin_stem = if let Some(dot_pos) = lower.rfind('.') {
                    &lower[..dot_pos]
                } else {
                    &lower
                };

                let mut is_companion = false;

                // 1. Exact stem match: e.g. "Dragon Knight 4.cue" vs "Dragon Knight 4.bin"
                // 2. Track prefix match: e.g. "Dragon Knight 4 (Track 1).bin"
                for desc_stem in &descriptor_stems {
                    if bin_stem == desc_stem || bin_stem.starts_with(desc_stem) {
                        is_companion = true;
                        break;
                    }
                }

                // 3. Generic track naming if descriptor exists in folder (e.g. "track01.bin", "track 1.bin")
                if !is_companion {
                    if bin_stem.starts_with("track")
                        || bin_stem.contains("track ")
                        || bin_stem.contains("track_")
                        || bin_stem.contains("track-")
                        || bin_stem.contains("(track")
                    {
                        is_companion = true;
                    }
                }

                if is_companion {
                    remove_indices.insert(i);
                }
            }
        }
    }

    roms.into_iter()
        .enumerate()
        .filter(|(i, _)| !remove_indices.contains(i))
        .map(|(_, rom)| rom)
        .collect()
}

fn is_video_file_ext(ext: &str) -> bool {
    matches!(ext, "mp4" | "webm" | "mkv" | "avi")
}

fn is_image_file_ext(ext: &str) -> bool {
    matches!(ext, "png" | "jpg" | "jpeg" | "webp")
}

fn clean_media_stem(filename: &str) -> String {
    let lower = filename.to_lowercase();
    let stem = if let Some(pos) = lower.rfind('.') {
        &lower[..pos]
    } else {
        &lower
    };

    let clean = stem
        .strip_suffix("-video")
        .or_else(|| stem.strip_suffix("_video"))
        .or_else(|| stem.strip_suffix("-snap"))
        .or_else(|| stem.strip_suffix("_snap"))
        .or_else(|| stem.strip_suffix("-image"))
        .or_else(|| stem.strip_suffix("_image"))
        .or_else(|| stem.strip_suffix("-thumb"))
        .or_else(|| stem.strip_suffix("_thumb"))
        .or_else(|| stem.strip_suffix("-boxart"))
        .or_else(|| stem.strip_suffix("_boxart"))
        .or_else(|| stem.strip_suffix("-cover"))
        .or_else(|| stem.strip_suffix("_cover"))
        .unwrap_or(stem);

    clean.trim().to_string()
}

// Scans files for a system platform. Traverses up to 2 levels of subfolders (e.g. psx/Final Fantasy VII/CD1.chd)
async fn collect_system_roms(
    session: &RemoteSession,
    sys_path: &str,
    system_id: &str,
) -> (Vec<ScannedRomFile>, bool, HashMap<String, String>, HashMap<String, String>) {
    let clean_sys_path = sys_path.trim_end_matches('/');

    let mut detected_images: HashMap<String, String> = HashMap::new();
    let mut detected_videos: HashMap<String, String> = HashMap::new();

    // 1. Fast SSH exec: lists files and game bundles directly on console (0 packet desync, instant)
    let find_cmd = if system_id.eq_ignore_ascii_case("ports") || system_id.eq_ignore_ascii_case("port") {
        format!(
            "sh -c 'TARGET=\"{}\"; [ -f \"$TARGET/gamelist.xml\" ] && echo \"__GAMELIST_XML_FOUND__\"; find \"$TARGET\" -maxdepth 2 -type f \\( -name \"*.sh\" -o -name \"*.squashfs\" \\) ! -name \".*\" ! -path \"*/[pP]ort[mM]aster/*\" 2>/dev/null'",
            clean_sys_path
        )
    } else {
        format!(
            "sh -c 'TARGET=\"{}\"; [ -f \"$TARGET/gamelist.xml\" ] && echo \"__GAMELIST_XML_FOUND__\"; find \"$TARGET\" -maxdepth 3 -type f 2>/dev/null; find \"$TARGET\" -maxdepth 2 -type d \\( -name \"*.scummvm\" -o -name \"*.pc\" -o -name \"*.dos\" -o -name \"*.squashfs\" \\) 2>/dev/null'",
            clean_sys_path
        )
    };

    if let Ok(output) = session.exec_command(&find_cmd).await {
        let lines: Vec<&str> = output.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
        if !lines.is_empty() {
            let mut results = Vec::new();
            let mut has_gamelist = false;

            for line in lines {
                if line == "__GAMELIST_XML_FOUND__" {
                    has_gamelist = true;
                    continue;
                }

                if let Some(rel) = line.strip_prefix(clean_sys_path) {
                    let rel_path = rel.trim_start_matches('/').to_string();
                    if rel_path.is_empty() {
                        continue;
                    }
                    let filename = rel_path.rsplit('/').next().unwrap_or(&rel_path).to_string();

                    if filename.eq_ignore_ascii_case("gamelist.xml") {
                        has_gamelist = true;
                        continue;
                    }

                    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
                    if is_video_file_ext(&ext) {
                        let stem = clean_media_stem(&filename);
                        let clean_stem = clean_rom_name(&stem).to_lowercase();
                        let target_rel = format!("./{}", rel_path);
                        detected_videos.entry(stem).or_insert_with(|| target_rel.clone());
                        if !clean_stem.is_empty() {
                            detected_videos.entry(clean_stem).or_insert(target_rel);
                        }
                        continue;
                    } else if is_image_file_ext(&ext) {
                        let stem = clean_media_stem(&filename);
                        let clean_stem = clean_rom_name(&stem).to_lowercase();
                        let target_rel = format!("./{}", rel_path);
                        detected_images.entry(stem).or_insert_with(|| target_rel.clone());
                        if !clean_stem.is_empty() {
                            detected_images.entry(clean_stem).or_insert(target_rel);
                        }
                        continue;
                    }

                    if !is_ignored_rom_file(system_id, &filename) {
                        results.push(ScannedRomFile {
                            rel_path,
                            filename,
                            size: 0,
                        });
                    }
                }
            }

            return (filter_companion_track_files(results), has_gamelist, detected_images, detected_videos);
        }
    }

    // 2. Fallback to SFTP traversal if SSH exec was not available
    let mut results = Vec::new();
    let entries = match session.list_dir(sys_path).await {
        Ok(e) => e,
        Err(err) => {
            eprintln!("[collect_system_roms] list_dir error for {}: {}", sys_path, err);
            return (results, false, detected_images, detected_videos);
        }
    };

    let mut has_gamelist = false;
    for item in &entries {
        if !item.is_dir && item.name.eq_ignore_ascii_case("gamelist.xml") {
            has_gamelist = true;
        }

        if item.is_dir {
            let lower_dir = item.name.to_lowercase();
            // Skip media folders, hidden directories, and PortMaster tool directory
            if is_media_or_system_name(&lower_dir) || lower_dir.starts_with('.') || lower_dir == "portmaster" {
                continue;
            }

            // If the folder itself is a game bundle (e.g. .scummvm, .pc, .dos, .squashfs)
            if is_folder_game_bundle(&lower_dir) {
                results.push(ScannedRomFile {
                    rel_path: item.name.clone(),
                    filename: item.name.clone(),
                    size: item.size,
                });
                continue;
            }

            // Level 1 Subfolder
            let sub_path = format!("{}/{}", sys_path.trim_end_matches('/'), item.name);
            let sub_entries = session.list_dir(&sub_path).await.unwrap_or_default();
            for sub_item in sub_entries {
                if sub_item.is_dir {
                    let sub_lower = sub_item.name.to_lowercase();
                    if !is_media_or_system_name(&sub_lower) && !sub_lower.starts_with('.') {
                        if is_folder_game_bundle(&sub_lower) {
                            results.push(ScannedRomFile {
                                rel_path: format!("{}/{}", item.name, sub_item.name),
                                filename: sub_item.name.clone(),
                                size: sub_item.size,
                            });
                            continue;
                        }

                        // Level 2 Subfolder
                        let deep_path = format!("{}/{}", sub_path, sub_item.name);
                        let deep_entries = session.list_dir(&deep_path).await.unwrap_or_default();
                        for deep_item in deep_entries {
                            if !deep_item.is_dir && !is_ignored_rom_file(system_id, &deep_item.name) {
                                results.push(ScannedRomFile {
                                    rel_path: format!("{}/{}/{}", item.name, sub_item.name, deep_item.name),
                                    filename: deep_item.name.clone(),
                                    size: deep_item.size,
                                });
                            }
                        }
                    }
                } else if !is_ignored_rom_file(system_id, &sub_item.name) {
                    results.push(ScannedRomFile {
                        rel_path: format!("{}/{}", item.name, sub_item.name),
                        filename: sub_item.name.clone(),
                        size: sub_item.size,
                    });
                }
            }
        } else {
            if !is_ignored_rom_file(system_id, &item.name) {
                results.push(ScannedRomFile {
                    rel_path: item.name.clone(),
                    filename: item.name.clone(),
                    size: item.size,
                });
            }
        }
    }

    (filter_companion_track_files(results), has_gamelist, detected_images, detected_videos)
}

pub async fn scan_systems(session: &RemoteSession, roms_path: &str) -> Result<Vec<SystemPlatform>, String> {
    let clean_roms_path = roms_path.trim_end_matches('/');

    // 1. Fast SSH scan for all systems at once (0.3s instead of 500+ SFTP packets)
    let survey_cmd = format!(
        "sh -c 'BASE=\"{}\"; for d in \"$BASE\"/*; do [ -d \"$d\" ] || continue; s=$(basename \"$d\"); case \"$s\" in images|media|videos|marquees|thumbnails|covers|screenshots|wheels|titles|fanart|manuals|bios|savestates|saves|themes|theme|downloads|download|backups|backup|cheats|music|sound|sounds|overlays|shaders|system|records|recordings|logs|decorations|extra|configs|config|tools|package|packages|retroarch|kodi|splash|lost+found|.*) continue ;; esac; has_xml=0; [ -f \"$d/gamelist.xml\" ] && has_xml=1; if [ \"$s\" = \"ports\" ] || [ \"$s\" = \"port\" ]; then cnt=$(find \"$d\" -maxdepth 2 -type f \\( -name \"*.sh\" -o -name \"*.squashfs\" \\) ! -name \".*\" ! -path \"*/[pP]ort[mM]aster/*\" 2>/dev/null | wc -l); else has_cue=$(find \"$d\" -maxdepth 3 -type f \\( -name \"*.cue\" -o -name \"*.CUE\" -o -name \"*.m3u\" -o -name \"*.M3U\" -o -name \"*.gdi\" -o -name \"*.GDI\" -o -name \"*.ccd\" -o -name \"*.CCD\" \\) 2>/dev/null | head -n 1); if [ -n \"$has_cue\" ]; then cnt=$(find \"$d\" -maxdepth 3 -type f ! -name \".*\" ! -name \"*.xml*\" ! -name \"*.bak\" ! -name \"*.json\" ! -name \"*.yaml\" ! -name \"*.yml\" ! -name \"*.txt\" ! -name \"*.nfo\" ! -name \"*.md\" ! -name \"*.log\" ! -name \"*.pdf\" ! -name \"*.doc*\" ! -name \"*.rtf\" ! -name \"*.ini\" ! -name \"*.cfg\" ! -name \"*.conf\" ! -name \"*.db\" ! -name \"*[tT]humbs.db*\" ! -name \"*desktop.ini*\" ! -name \"*.png\" ! -name \"*.PNG\" ! -name \"*.jpg\" ! -name \"*.JPG\" ! -name \"*.jpeg\" ! -name \"*.JPEG\" ! -name \"*.gif\" ! -name \"*.GIF\" ! -name \"*.webp\" ! -name \"*.mp4\" ! -name \"*.MP4\" ! -name \"*.mkv\" ! -name \"*.avi\" ! -name \"*.dat\" ! -name \"*.sh\" ! -name \"*.bat\" ! -name \"*.srm\" ! -name \"*.SRM\" ! -name \"*.sav*\" ! -name \"*.SAV*\" ! -name \"*.state*\" ! -name \"*.dsv\" ! -name \"*.DSV\" ! -name \"*.rtc*\" ! -name \"*.RTC*\" ! -name \"*.cht*\" ! -name \"*.CHT*\" ! -name \"*.ips*\" ! -name \"*.IPS*\" ! -name \"*.bps*\" ! -name \"*.BPS*\" ! -name \"*.ups*\" ! -name \"*.opt*\" ! -name \"*.rmp*\" ! -name \"*.nvram*\" ! -name \"*.mcr*\" ! -name \"*.mpk*\" ! -name \"*.bin\" ! -name \"*.BIN\" ! -name \"*.img\" ! -name \"*.IMG\" ! -path \"*/images/*\" ! -path \"*/media/*\" ! -path \"*/videos/*\" ! -path \"*/thumbnails/*\" ! -path \"*/covers/*\" ! -path \"*/screenshots/*\" ! -path \"*/wheels/*\" ! -path \"*/titles/*\" ! -path \"*/fanart/*\" ! -path \"*/manuals/*\" ! -path \"*/portmaster/*\" ! -path \"*/saves/*\" ! -path \"*/save/*\" ! -path \"*/states/*\" ! -path \"*/savestates/*\" ! -path \"*/cheats/*\" ! -path \"*/backup/*\" ! -path \"*/backups/*\" 2>/dev/null | wc -l); else cnt=$(find \"$d\" -maxdepth 3 -type f ! -name \".*\" ! -name \"*.xml*\" ! -name \"*.bak\" ! -name \"*.json\" ! -name \"*.yaml\" ! -name \"*.yml\" ! -name \"*.txt\" ! -name \"*.nfo\" ! -name \"*.md\" ! -name \"*.log\" ! -name \"*.pdf\" ! -name \"*.doc*\" ! -name \"*.rtf\" ! -name \"*.ini\" ! -name \"*.cfg\" ! -name \"*.conf\" ! -name \"*.db\" ! -name \"*[tT]humbs.db*\" ! -name \"*desktop.ini*\" ! -name \"*.png\" ! -name \"*.PNG\" ! -name \"*.jpg\" ! -name \"*.JPG\" ! -name \"*.jpeg\" ! -name \"*.JPEG\" ! -name \"*.gif\" ! -name \"*.GIF\" ! -name \"*.webp\" ! -name \"*.mp4\" ! -name \"*.MP4\" ! -name \"*.mkv\" ! -name \"*.avi\" ! -name \"*.dat\" ! -name \"*.sh\" ! -name \"*.bat\" ! -name \"*.srm\" ! -name \"*.SRM\" ! -name \"*.sav*\" ! -name \"*.SAV*\" ! -name \"*.state*\" ! -name \"*.dsv\" ! -name \"*.DSV\" ! -name \"*.rtc*\" ! -name \"*.RTC*\" ! -name \"*.cht*\" ! -name \"*.CHT*\" ! -name \"*.ips*\" ! -name \"*.IPS*\" ! -name \"*.bps*\" ! -name \"*.BPS*\" ! -name \"*.ups*\" ! -name \"*.opt*\" ! -name \"*.rmp*\" ! -name \"*.nvram*\" ! -name \"*.mcr*\" ! -name \"*.mpk*\" ! -path \"*/images/*\" ! -path \"*/media/*\" ! -path \"*/videos/*\" ! -path \"*/thumbnails/*\" ! -path \"*/covers/*\" ! -path \"*/screenshots/*\" ! -path \"*/wheels/*\" ! -path \"*/titles/*\" ! -path \"*/fanart/*\" ! -path \"*/manuals/*\" ! -path \"*/portmaster/*\" ! -path \"*/saves/*\" ! -path \"*/save/*\" ! -path \"*/states/*\" ! -path \"*/savestates/*\" ! -path \"*/cheats/*\" ! -path \"*/backup/*\" ! -path \"*/backups/*\" 2>/dev/null | wc -l); fi; fi; echo \"$s|$cnt|$has_xml\"; done'",
        clean_roms_path
    );

    if let Ok(output) = session.exec_command(&survey_cmd).await {
        let lines: Vec<&str> = output.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
        if !lines.is_empty() && lines.iter().any(|l| l.contains('|')) {
            let mut systems = Vec::new();
            for line in lines {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 3 {
                    let system_id = parts[0].trim().to_string();
                    let mut rom_count: usize = parts[1].trim().parse().unwrap_or(0);
                    let has_xml = parts[2].trim() == "1";

                    let mut metadata_count = 0;
                    let mut missing_image_count = 0;
                    let mut unregistered_count = rom_count;

                    // If has_xml, parse gamelist.xml if present
                    if has_xml {
                        let xml_path = format!("{}/{}/gamelist.xml", clean_roms_path, system_id);
                        if let Ok(xml_content) = session.read_file_string(&xml_path).await {
                            if let Ok(gamelist) = parse_gamelist_xml(&xml_content) {
                                metadata_count = gamelist.games.len();
                                for g in &gamelist.games {
                                    if g.image.is_none() || g.image.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
                                        missing_image_count += 1;
                                    }
                                }
                                if rom_count < metadata_count {
                                    rom_count = metadata_count;
                                }
                                unregistered_count = rom_count.saturating_sub(metadata_count);
                            }
                        }
                    }

                    let display_name = get_platform_display_name(&system_id);
                    let name = if display_name.is_empty() {
                        system_id.to_uppercase()
                    } else {
                        format!("{} ({})", display_name, system_id)
                    };

                    systems.push(SystemPlatform {
                        id: system_id,
                        name,
                        rom_count,
                        metadata_count,
                        unregistered_count,
                        missing_image_count,
                    });
                }
            }

            if !systems.is_empty() {
                systems.sort_by(|a, b| a.id.to_lowercase().cmp(&b.id.to_lowercase()));
                return Ok(systems);
            }
        }
    }

    // 2. SFTP fallback scan if SSH exec failed
    let root_items = session.list_dir(clean_roms_path).await?;
    let mut systems = Vec::new();

    for item in root_items {
        if is_media_or_system_name(&item.name) || item.name.starts_with('.') {
            continue;
        }

        if !item.is_dir {
            continue;
        }

        let system_id = item.name.clone();
        let sys_path = format!("{}/{}", clean_roms_path, system_id);

        // Collect rom files (including ports subdirectories) and check if gamelist.xml exists
        let (rom_files, has_gamelist, detected_images, _detected_videos) = collect_system_roms(session, &sys_path, &system_id).await;
        let rom_count = rom_files.len();

        // Check gamelist.xml only if it exists in the folder
        let mut metadata_count = 0;
        let mut missing_image_count = 0;
        let mut unregistered_count = rom_count;

        if has_gamelist {
            let xml_path = format!("{}/gamelist.xml", sys_path);
            if let Ok(xml_content) = session.read_file_string(&xml_path).await {
                if let Ok(gamelist) = parse_gamelist_xml(&xml_content) {
                    metadata_count = gamelist.games.len();
                    
                    let mut xml_paths = HashSet::new();
                    for g in &gamelist.games {
                        let norm = normalize_rel_path(&g.path, &system_id);
                        xml_paths.insert(norm);
                        let has_img = g.image.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);
                        if !has_img {
                            let g_filename = g.path.rsplit('/').next().unwrap_or(&g.path);
                            let stem = g_filename.rsplit('.').next().unwrap_or(g_filename).to_lowercase();
                            let clean = clean_rom_name(g_filename).to_lowercase();
                            if !detected_images.contains_key(&stem) && !detected_images.contains_key(&clean) {
                                missing_image_count += 1;
                            }
                        }
                    }

                    // Calculate unregistered roms
                    let mut registered_count = 0;
                    for rf in &rom_files {
                        let norm = normalize_rel_path(&rf.rel_path, &system_id);
                        if xml_paths.contains(&norm) {
                            registered_count += 1;
                        }
                    }
                    unregistered_count = rom_count.saturating_sub(registered_count);
                }
            }
        }

        let display_name = get_platform_display_name(&system_id);
        let name = if display_name.is_empty() {
            system_id.to_uppercase()
        } else {
            format!("{} ({})", display_name, system_id)
        };

        systems.push(SystemPlatform {
            id: system_id,
            name,
            rom_count,
            metadata_count,
            unregistered_count,
            missing_image_count,
        });
    }

    systems.sort_by(|a, b| a.id.to_lowercase().cmp(&b.id.to_lowercase()));
    Ok(systems)
}

pub async fn load_system_games(
    session: &RemoteSession,
    roms_path: &str,
    system_id: &str,
) -> Result<Vec<GameItem>, String> {
    let sys_path = format!("{}/{}", roms_path.trim_end_matches('/'), system_id);
    let (rom_files, has_gamelist, detected_images, detected_videos) = collect_system_roms(session, &sys_path, system_id).await;

    // Helper to find auto-detected media (image or video) by filename or clean rom name
    let find_auto_media = |fname: &str, map: &HashMap<String, String>| -> Option<String> {
        let lower = fname.to_lowercase();
        let stem = if let Some(pos) = lower.rfind('.') {
            lower[..pos].to_string()
        } else {
            lower.clone()
        };
        let clean = clean_rom_name(fname).to_lowercase();
        map.get(&stem).or_else(|| map.get(&clean)).cloned()
    };

    // Map normalized rel_path -> ScannedRomFile
    let mut rel_map: HashMap<String, ScannedRomFile> = HashMap::new();
    for f in &rom_files {
        let norm_rel = normalize_rel_path(&f.rel_path, system_id);
        rel_map.insert(norm_rel, f.clone());
    }

    let gamelist = if has_gamelist {
        let xml_path = format!("{}/gamelist.xml", sys_path);
        match session.read_file_string(&xml_path).await {
            Ok(content) => parse_gamelist_xml(&content).unwrap_or_default(),
            Err(_) => GameListXml::default(),
        }
    } else {
        GameListXml::default()
    };

    let mut games: Vec<GameItem> = Vec::new();
    let mut matched_rel_paths: HashSet<String> = HashSet::new();
    let mut seen_game_paths: HashSet<String> = HashSet::new();

    // 1. Process entries in gamelist.xml
    for g in gamelist.games {
        let clean_path = normalize_rel_path(&g.path, system_id);
        if clean_path.is_empty() {
            continue;
        }

        // Deduplicate exact duplicate paths in gamelist.xml
        if !seen_game_paths.insert(clean_path.clone()) {
            continue;
        }

        let rel_path = if let Some(stripped) = g.path.strip_prefix("./") {
            stripped.to_string()
        } else {
            g.path.clone()
        };
        let filename = rel_path.rsplit('/').next().unwrap_or(&rel_path).to_string();

        // Skip non-rom files even if present in gamelist.xml (e.g. readme.txt)
        if is_ignored_rom_file(system_id, &filename) {
            continue;
        }

        let matched_rom = rel_map.get(&clean_path);

        let (status, file_size) = if let Some(file) = matched_rom {
            matched_rel_paths.insert(file.rel_path.clone());
            (GameStatus::Registered, file.size)
        } else {
            (GameStatus::Missing, 0)
        };

        let default_name = clean_rom_name(&filename);
        let name = g.name.unwrap_or(default_name);

        let mut image = g.image;
        if image.is_none() || image.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
            image = find_auto_media(&filename, &detected_images);
        }

        let mut video = g.video;
        if video.is_none() || video.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
            video = find_auto_media(&filename, &detected_videos);
        }

        games.push(GameItem {
            path: g.path,
            filename,
            name,
            desc: g.desc.unwrap_or_default(),
            image,
            thumbnail: g.thumbnail,
            video,
            rating: g.rating,
            releasedate: g.releasedate,
            developer: g.developer,
            publisher: g.publisher,
            genre: g.genre,
            players: g.players,
            favorite: g.favorite.unwrap_or(false),
            hidden: g.hidden.unwrap_or(false),
            status,
            file_size,
        });
    }

    // 2. Add files that are in filesystem but NOT in gamelist.xml
    for file in rom_files {
        let clean_file_rel = normalize_rel_path(&file.rel_path, system_id);
        if !matched_rel_paths.contains(&file.rel_path) && !seen_game_paths.contains(&clean_file_rel) {
            seen_game_paths.insert(clean_file_rel);
            let default_name = clean_rom_name(&file.filename);
            let image = find_auto_media(&file.filename, &detected_images);
            let video = find_auto_media(&file.filename, &detected_videos);

            games.push(GameItem {
                path: format!("./{}", file.rel_path),
                filename: file.filename.clone(),
                name: default_name,
                desc: String::new(),
                image,
                thumbnail: None,
                video,
                rating: None,
                releasedate: None,
                developer: None,
                publisher: None,
                genre: None,
                players: None,
                favorite: false,
                hidden: false,
                status: GameStatus::Unregistered,
                file_size: file.size,
            });
        }
    }

    // Sort by name
    games.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(games)
}

pub async fn save_system_games(
    session: &RemoteSession,
    roms_path: &str,
    system_id: &str,
    games: Vec<GameItem>,
) -> Result<(), String> {
    let sys_path = format!("{}/{}", roms_path.trim_end_matches('/'), system_id);
    let xml_path = format!("{}/gamelist.xml", sys_path);
    let bak_path = format!("{}/gamelist.xml.bak", sys_path);

    // If original gamelist.xml exists, backup first
    if let Ok(existing) = session.read_file_bytes(&xml_path).await {
        let _ = session.write_file_bytes(&bak_path, &existing).await;
    }

    let mut xml_games = Vec::new();
    for g in games {
        // Exclude missing ROMs if desired or keep them; usually keep registered and new
        let path = if !g.path.starts_with("./") && !g.path.starts_with('/') {
            format!("./{}", g.path)
        } else {
            g.path
        };

        xml_games.push(GameXml {
            path,
            name: Some(g.name),
            desc: if g.desc.is_empty() { None } else { Some(g.desc) },
            image: g.image,
            thumbnail: g.thumbnail,
            video: g.video,
            rating: g.rating,
            releasedate: g.releasedate,
            developer: g.developer,
            publisher: g.publisher,
            genre: g.genre,
            players: g.players,
            favorite: if g.favorite { Some(true) } else { None },
            hidden: if g.hidden { Some(true) } else { None },
        });
    }

    let gamelist = GameListXml { games: xml_games };
    let xml_str = serialize_gamelist_xml(&gamelist)?;

    session.write_file_bytes(&xml_path, xml_str.as_bytes()).await?;
    Ok(())
}

fn clean_rom_name(filename: &str) -> String {
    let pure = if let Some(idx) = filename.rfind(['/', '\\']) {
        &filename[idx + 1..]
    } else {
        filename
    };
    let name = if let Some(idx) = pure.rfind('.') {
        &pure[..idx]
    } else {
        pure
    };
    name.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_ignored_rom_file() {
        // General console tests (e.g. SNES, GBA)
        assert!(is_ignored_rom_file("snes", "readme.txt"));
        assert!(is_ignored_rom_file("snes", "cheats.TXT"));
        assert!(is_ignored_rom_file("snes", "info.nfo"));
        assert!(is_ignored_rom_file("snes", "manual.pdf"));
        assert!(is_ignored_rom_file("snes", "script.sh"));
        assert!(is_ignored_rom_file("snes", "gamelist.xml"));
        assert!(is_ignored_rom_file("snes", ".ds_store"));
        assert!(is_ignored_rom_file("snes", "cover.png"));

        // Save files & save states must be ignored!
        assert!(is_ignored_rom_file("gba", "Pocket Monsters - Emerald (J-K).srm"));
        assert!(is_ignored_rom_file("gba", "Pokemon Fire Red.sav"));
        assert!(is_ignored_rom_file("gba", "Pokemon Fire Red.state"));
        assert!(is_ignored_rom_file("gba", "Pokemon Fire Red.state1"));
        assert!(is_ignored_rom_file("gba", "Pokemon Fire Red.state.auto"));
        assert!(is_ignored_rom_file("nds", "Pokemon Platinum.dsv"));

        // Valid ROM files
        assert!(!is_ignored_rom_file("snes", "Super Mario World (USA).zip"));
        assert!(!is_ignored_rom_file("gba", "Pokemon Fire Red.gba"));
        assert!(!is_ignored_rom_file("snes", "Chrono Trigger.sfc"));
        assert!(!is_ignored_rom_file("psx", "Tekken 3.chd"));
        assert!(!is_ignored_rom_file("arcade", "Metal Slug.bin"));
        assert!(!is_ignored_rom_file("psx", "Sonic.cue"));
        assert!(!is_ignored_rom_file("psx", "Final Fantasy VII.iso"));

        // Ports specific tests:
        // .sh and .squashfs ARE valid ROMs for ports!
        assert!(!is_ignored_rom_file("ports", "Celeste.sh"));
        assert!(!is_ignored_rom_file("ports", "Balatro.sh"));
        assert!(!is_ignored_rom_file("ports", "ShovelKnight.squashfs"));
        // Aux/data files in ports root should still be ignored
        assert!(is_ignored_rom_file("ports", "readme.txt"));
        assert!(is_ignored_rom_file("ports", "config.json"));
        assert!(is_ignored_rom_file("ports", "cover.png"));
    }

    #[test]
    fn test_filter_companion_track_files() {
        let files = vec![
            ScannedRomFile {
                rel_path: "Dragon Knight 4.cue".to_string(),
                filename: "Dragon Knight 4.cue".to_string(),
                size: 100,
            },
            ScannedRomFile {
                rel_path: "Dragon Knight 4.bin".to_string(),
                filename: "Dragon Knight 4.bin".to_string(),
                size: 500000000,
            },
            ScannedRomFile {
                rel_path: "Sonic The Hedgehog.bin".to_string(),
                filename: "Sonic The Hedgehog.bin".to_string(),
                size: 1000000,
            },
            ScannedRomFile {
                rel_path: "sub/Final Fantasy VII (Disc 1).cue".to_string(),
                filename: "Final Fantasy VII (Disc 1).cue".to_string(),
                size: 200,
            },
            ScannedRomFile {
                rel_path: "sub/Final Fantasy VII (Disc 1) (Track 1).bin".to_string(),
                filename: "Final Fantasy VII (Disc 1) (Track 1).bin".to_string(),
                size: 400000000,
            },
        ];

        let filtered = filter_companion_track_files(files);
        assert_eq!(filtered.len(), 3);
        assert!(filtered.iter().any(|f| f.filename == "Dragon Knight 4.cue"));
        assert!(filtered.iter().any(|f| f.filename == "Sonic The Hedgehog.bin"));
        assert!(filtered.iter().any(|f| f.filename == "Final Fantasy VII (Disc 1).cue"));
        assert!(!filtered.iter().any(|f| f.filename == "Dragon Knight 4.bin"));
        assert!(!filtered.iter().any(|f| f.filename == "Final Fantasy VII (Disc 1) (Track 1).bin"));
    }
}
