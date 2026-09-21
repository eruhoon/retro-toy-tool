use std::collections::HashSet;
use std::time::Instant;
use base64::Engine;
use tauri::Emitter;
use crate::models::{
    ConnectionTestResult, GameItem, RomUploadProgress, RomUploadResult, ScrapedGame,
    ScreenScraperAccountStatus, ScreenScraperCredentials, StorageLocation, SystemPlatform,
};
use crate::rom_scanner::{load_system_games, save_system_games, scan_systems};
use crate::scraper::{
    clean_query, download_image, download_video, enrich_media_sizes, sanitize_scraped_game,
    search_dlsite, search_dlsite_eng, search_dlsite_kor, search_rawg, search_screenscraper,
    search_steam, search_wikipedia, test_screenscraper_account,
};
use crate::ssh_client::RemoteSession;



pub async fn detect_storage_locations(
    session: &RemoteSession,
    candidate_path: Option<&str>,
) -> Vec<StorageLocation> {
    let mut storages = Vec::new();
    let mut seen_paths = HashSet::new();

    let mut add_storage = |path: String, label: String, storage_type: &str| {
        let clean = path.trim_end_matches('/').to_string();
        if !seen_paths.contains(&clean) {
            seen_paths.insert(clean.clone());
            let id = format!("{}_{}", storage_type, storages.len() + 1);
            storages.push(StorageLocation {
                id,
                label,
                path: clean,
                storage_type: storage_type.to_string(),
            });
        }
    };

    // 1. Check custom candidate if provided
    if let Some(cand) = candidate_path {
        let cand = cand.trim().trim_end_matches('/');
        if !cand.is_empty() && session.path_exists(cand).await {
            let is_ext = cand.contains("mount") || cand.contains("media") || cand.contains("sdcard");
            let label = if is_ext {
                format!("지정 외장 경로 ({})", cand)
            } else {
                format!("지정 경로 ({})", cand)
            };
            add_storage(cand.to_string(), label, if is_ext { "external" } else { "internal" });
        }
    }

    // 2. Discover external storages:
    // A) /userdata/mount/...
    if let Ok(mount_items) = session.list_dir("/userdata/mount").await {
        for m in mount_items {
            if m.is_dir {
                let p1 = format!("/userdata/mount/{}/roms", m.name);
                let p2 = format!("/userdata/mount/{}", m.name);
                if session.path_exists(&p1).await {
                    add_storage(p1, format!("외장 SD카드 (TF2: {})", m.name), "external");
                } else if session.path_exists(&format!("{}/gba", p2)).await || session.path_exists(&format!("{}/snes", p2)).await {
                    add_storage(p2, format!("외장 SD카드 (TF2: {})", m.name), "external");
                } else if m.name.to_lowercase() == "roms" {
                    add_storage(p2, "외장 SD카드 (TF2 roms)".to_string(), "external");
                }
            }
        }
    }
    if session.path_exists("/userdata/mount/roms").await {
        add_storage("/userdata/mount/roms".to_string(), "외장 SD카드 (TF2 /userdata/mount/roms)".to_string(), "external");
    }

    // B) /media/...
    if let Ok(media_items) = session.list_dir("/media").await {
        for m in media_items {
            if m.is_dir {
                let p1 = format!("/media/{}/roms", m.name);
                let p2 = format!("/media/{}", m.name);
                if session.path_exists(&p1).await {
                    add_storage(p1, format!("외장 SD카드 (TF2: {})", m.name), "external");
                } else if session.path_exists(&format!("{}/gba", p2)).await || session.path_exists(&format!("{}/snes", p2)).await {
                    add_storage(p2, format!("외장 SD카드 (TF2: {})", m.name), "external");
                } else if m.name.to_lowercase() == "roms" {
                    add_storage(p2, "외장 SD카드 (TF2 roms)".to_string(), "external");
                }
            }
        }
    }

    // C) /storage/sdcard/roms (ROCKNIX / JELOS)
    if session.path_exists("/storage/sdcard/roms").await {
        add_storage("/storage/sdcard/roms".to_string(), "외장 SD카드 (TF2 /storage/sdcard)".to_string(), "external");
    } else if session.path_exists("/storage/sdcard").await {
        add_storage("/storage/sdcard".to_string(), "외장 SD카드 (TF2 /storage/sdcard)".to_string(), "external");
    }

    // 3. Discover internal storages:
    if session.path_exists("/userdata/roms").await {
        add_storage("/userdata/roms".to_string(), "내장 메모리 (TF1 /userdata)".to_string(), "internal");
    }
    if session.path_exists("/storage/roms").await {
        add_storage("/storage/roms".to_string(), "내장 메모리 (TF1 /storage)".to_string(), "internal");
    }
    if session.path_exists("/roms").await {
        add_storage("/roms".to_string(), "내장 메모리 (/roms)".to_string(), "internal");
    }
    if session.path_exists("/home/pi/RetroPie/roms").await {
        add_storage("/home/pi/RetroPie/roms".to_string(), "RetroPie 내장 메모리".to_string(), "internal");
    }

    storages
}

#[tauri::command]
pub async fn test_connection(
    host: String,
    port: u16,
    username: String,
    password: String,
    candidate_path: Option<String>,
) -> Result<ConnectionTestResult, String> {
    let session = RemoteSession::connect(&host, port, &username, &password).await?;
    let storages = detect_storage_locations(&session, candidate_path.as_deref()).await;

    // Prefer external storage if found, otherwise use first storage
    let preferred = storages.iter().find(|s| s.storage_type == "external")
        .or_else(|| storages.first());

    let detected_path = preferred.map(|s| s.path.clone());
    let os_name = preferred.map(|s| s.label.clone());

    Ok(ConnectionTestResult {
        success: true,
        message: "연결 성공".to_string(),
        detected_roms_path: detected_path,
        os_name,
        storages,
    })
}

#[tauri::command]
pub async fn detect_storages_cmd(
    host: String,
    port: u16,
    username: String,
    password: String,
    candidate_path: Option<String>,
) -> Result<Vec<StorageLocation>, String> {
    let session = RemoteSession::connect(&host, port, &username, &password).await?;
    Ok(detect_storage_locations(&session, candidate_path.as_deref()).await)
}

#[tauri::command]
pub async fn get_systems(
    host: String,
    port: u16,
    username: String,
    password: String,
    roms_path: String,
) -> Result<Vec<SystemPlatform>, String> {
    let session = RemoteSession::connect(&host, port, &username, &password).await?;
    scan_systems(&session, &roms_path).await
}

#[tauri::command]
pub async fn get_system_games(
    host: String,
    port: u16,
    username: String,
    password: String,
    roms_path: String,
    system_id: String,
) -> Result<Vec<GameItem>, String> {
    let session = RemoteSession::connect(&host, port, &username, &password).await?;
    load_system_games(&session, &roms_path, &system_id).await
}

#[tauri::command]
pub async fn save_system_games_cmd(
    host: String,
    port: u16,
    username: String,
    password: String,
    roms_path: String,
    system_id: String,
    games: Vec<GameItem>,
) -> Result<(), String> {
    let session = RemoteSession::connect(&host, port, &username, &password).await?;
    save_system_games(&session, &roms_path, &system_id, games).await
}

#[tauri::command]
pub async fn upload_game_image_cmd(
    host: String,
    port: u16,
    username: String,
    password: String,
    roms_path: String,
    system_id: String,
    filename: String,
    image_base64: String,
) -> Result<String, String> {
    let session = RemoteSession::connect(&host, port, &username, &password).await?;
    
    // Strip data URI prefix if present (e.g. "data:image/png;base64,")
    let raw_base64 = if let Some(idx) = image_base64.find(";base64,") {
        &image_base64[idx + 8..]
    } else {
        &image_base64
    };

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(raw_base64)
        .map_err(|e| format!("Base64 이미지 디코딩 실패: {}", e))?;

    let clean_filename = filename
        .strip_prefix("./")
        .unwrap_or(&filename)
        .trim_start_matches('/');

    let pure_filename = if let Some(pos) = clean_filename.rfind(['/', '\\']) {
        &clean_filename[pos + 1..]
    } else {
        clean_filename
    };

    let images_dir = format!("{}/{}/images", roms_path.trim_end_matches('/'), system_id);
    let remote_file_path = format!("{}/{}", images_dir, pure_filename);

    if let Some(pos) = remote_file_path.rfind('/') {
        let parent_dir = &remote_file_path[..pos];
        session.create_dir_all(parent_dir).await?;
    }

    session.write_file_bytes(&remote_file_path, &bytes).await?;

    // Return relative path for gamelist.xml
    Ok(format!("./images/{}", pure_filename))
}

#[tauri::command]
pub async fn upload_game_video_cmd(
    host: String,
    port: u16,
    username: String,
    password: String,
    roms_path: String,
    system_id: String,
    remote_filename: String,
    base64_video_data: String,
) -> Result<String, String> {
    let session = RemoteSession::connect(&host, port, &username, &password).await?;

    let clean_b64 = if let Some(pos) = base64_video_data.find(',') {
        &base64_video_data[pos + 1..]
    } else {
        &base64_video_data
    };

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(clean_b64)
        .map_err(|e| format!("Base64 비디오 디코딩 실패: {}", e))?;

    let clean_filename = remote_filename.trim().replace('\\', "/");
    let pure_filename = if let Some(pos) = clean_filename.rfind(['/', '\\']) {
        &clean_filename[pos + 1..]
    } else {
        clean_filename.as_str()
    };

    let videos_dir = format!("{}/{}/videos", roms_path.trim_end_matches('/'), system_id);
    let remote_file_path = format!("{}/{}", videos_dir, pure_filename);

    if let Some(pos) = remote_file_path.rfind('/') {
        let parent_dir = &remote_file_path[..pos];
        session.create_dir_all(parent_dir).await?;
    }

    session.write_file_bytes(&remote_file_path, &bytes).await?;

    // Return relative path for gamelist.xml
    Ok(format!("./videos/{}", pure_filename))
}

#[tauri::command]
pub async fn fetch_remote_image_cmd(
    host: String,
    port: u16,
    username: String,
    password: String,
    roms_path: String,
    system_id: String,
    image_rel_or_abs_path: String,
) -> Result<String, String> {
    let session = RemoteSession::connect(&host, port, &username, &password).await?;

    let clean_path = image_rel_or_abs_path.trim().replace('\\', "/");
    let base_full_path = if clean_path.starts_with('/') {
        clean_path.clone()
    } else {
        let mut stripped = clean_path.strip_prefix("./").unwrap_or(&clean_path);
        let sys_prefix = format!("{}/", system_id);
        if stripped.starts_with(&sys_prefix) {
            stripped = &stripped[sys_prefix.len()..];
        }
        format!("{}/{}/{}", roms_path.trim_end_matches('/'), system_id, stripped)
    };

    let bytes = match session.read_file_bytes(&base_full_path).await {
        Ok(b) => b,
        Err(err) => {
            let clean_name = clean_path.rsplit('/').next().unwrap_or(&clean_path);
            let fallback_paths = vec![
                format!("{}/{}/images/{}", roms_path.trim_end_matches('/'), system_id, clean_name),
                format!("{}/{}/media/images/{}", roms_path.trim_end_matches('/'), system_id, clean_name),
                format!("{}/{}/covers/{}", roms_path.trim_end_matches('/'), system_id, clean_name),
            ];

            let mut found = None;
            for fb in fallback_paths {
                if fb != base_full_path {
                    if let Ok(b) = session.read_file_bytes(&fb).await {
                        found = Some(b);
                        break;
                    }
                }
            }

            match found {
                Some(b) => b,
                None => return Err(format!("이미지 파일을 읽을 수 없습니다: {}", err)),
            }
        }
    };

    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);

    let mime = if clean_path.ends_with(".png") {
        "image/png"
    } else if clean_path.ends_with(".jpg") || clean_path.ends_with(".jpeg") {
        "image/jpeg"
    } else if clean_path.ends_with(".webp") {
        "image/webp"
    } else {
        "image/png"
    };

    Ok(format!("data:{};base64,{}", mime, b64))
}

#[tauri::command]
pub async fn fetch_remote_video_cmd(
    host: String,
    port: u16,
    username: String,
    password: String,
    roms_path: String,
    system_id: String,
    video_rel_or_abs_path: String,
) -> Result<String, String> {
    let session = RemoteSession::connect(&host, port, &username, &password).await?;

    let clean_path = video_rel_or_abs_path.trim().replace('\\', "/");
    let base_full_path = if clean_path.starts_with('/') {
        clean_path.clone()
    } else {
        let mut stripped = clean_path.strip_prefix("./").unwrap_or(&clean_path);
        let sys_prefix = format!("{}/", system_id);
        if stripped.starts_with(&sys_prefix) {
            stripped = &stripped[sys_prefix.len()..];
        }
        format!("{}/{}/{}", roms_path.trim_end_matches('/'), system_id, stripped)
    };

    let bytes = match session.read_file_bytes(&base_full_path).await {
        Ok(b) => b,
        Err(err) => {
            let clean_name = clean_path.rsplit('/').next().unwrap_or(&clean_path);
            let fallback_paths = vec![
                format!("{}/{}/videos/{}", roms_path.trim_end_matches('/'), system_id, clean_name),
                format!("{}/{}/media/videos/{}", roms_path.trim_end_matches('/'), system_id, clean_name),
                format!("{}/{}/snap/{}", roms_path.trim_end_matches('/'), system_id, clean_name),
                format!("{}/{}/snaps/{}", roms_path.trim_end_matches('/'), system_id, clean_name),
            ];

            let mut found = None;
            for fb in fallback_paths {
                if fb != base_full_path {
                    if let Ok(b) = session.read_file_bytes(&fb).await {
                        found = Some(b);
                        break;
                    }
                }
            }

            match found {
                Some(b) => b,
                None => return Err(format!("비디오 파일을 읽을 수 없습니다: {}", err)),
            }
        }
    };

    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);

    let mime = if clean_path.ends_with(".webm") {
        "video/webm"
    } else {
        "video/mp4"
    };

    Ok(format!("data:{};base64,{}", mime, b64))
}

#[tauri::command]
pub async fn search_game_metadata_cmd(
    query: String,
    system_id: Option<String>,
    screenscraper_creds: Option<ScreenScraperCredentials>,
    source: Option<String>,
) -> Result<Vec<ScrapedGame>, String> {
    let clean = clean_query(&query);
    let search_term = if clean.trim().is_empty() { query.trim() } else { clean.as_str() };

    let src = source.as_deref().unwrap_or("auto");
    let mut all_results = Vec::new();

    // 1. ScreenScraper search if auto or screenscraper
    if src == "auto" || src == "screenscraper" {
        if let Ok(ss_results) = search_screenscraper(search_term, system_id.as_deref(), screenscraper_creds.as_ref()).await {
            all_results.extend(ss_results);
        }
    }

    // 2. Steam Store search if auto or steam
    if src == "auto" || src == "steam" {
        if let Ok(steam_results) = search_steam(search_term).await {
            all_results.extend(steam_results);
        }
    }

    // 3. DLsite search (runs Japanese, Korean, and English searches simultaneously)
    if src == "auto" || src == "dlsite" || src == "dlsite_kor" || src == "dlsite_eng" {
        let (dlsite_res, dlsite_kor_res, dlsite_eng_res) = tokio::join!(
            search_dlsite(search_term),
            search_dlsite_kor(search_term),
            search_dlsite_eng(search_term)
        );

        let jp_items = dlsite_res.unwrap_or_default();
        let kor_items = dlsite_kor_res.unwrap_or_default();
        let eng_items = dlsite_eng_res.unwrap_or_default();

        let extract_pid = |id: &str| -> String {
            id.replace("dlsite_kor_", "").replace("dlsite_eng_", "").replace("dlsite_", "")
        };

        let mut used_jp_ids = std::collections::HashSet::new();
        let mut used_eng_ids = std::collections::HashSet::new();

        // 1. First process all found Korean items (highest priority for Korean users)
        for mut kor_item in kor_items {
            let pid = extract_pid(&kor_item.id);
            let matching_jp = jp_items.iter().find(|j| extract_pid(&j.id) == pid).cloned();
            let matching_eng = eng_items.iter().find(|e| extract_pid(&e.id) == pid).cloned();

            let best_cover = kor_item.cover_url.clone()
                .or_else(|| matching_jp.as_ref().and_then(|j| j.cover_url.clone()))
                .or_else(|| matching_eng.as_ref().and_then(|e| e.cover_url.clone()));

            kor_item.cover_url = best_cover.clone();
            if !all_results.iter().any(|r| r.id == kor_item.id) {
                all_results.push(kor_item);
            }

            // Matching JP (placed right after Korean)
            if let Some(mut jp_item) = matching_jp {
                if !all_results.iter().any(|r| r.id == jp_item.id) {
                    if jp_item.cover_url.is_none() {
                        jp_item.cover_url = best_cover.clone();
                    }
                    all_results.push(jp_item.clone());
                    used_jp_ids.insert(jp_item.id);
                }
            }

            // Matching ENG (placed right after Japanese)
            if let Some(mut eng_item) = matching_eng {
                if !all_results.iter().any(|r| r.id == eng_item.id) {
                    if eng_item.cover_url.is_none() {
                        eng_item.cover_url = best_cover.clone();
                    }
                    all_results.push(eng_item.clone());
                    used_eng_ids.insert(eng_item.id);
                }
            }
        }

        // 2. Process JP items that didn't have a KOR version
        for mut jp_item in jp_items {
            if used_jp_ids.contains(&jp_item.id) || all_results.iter().any(|r| r.id == jp_item.id) {
                continue;
            }
            let pid = extract_pid(&jp_item.id);
            let matching_eng = eng_items.iter().find(|e| extract_pid(&e.id) == pid).cloned();

            let best_cover = jp_item.cover_url.clone()
                .or_else(|| matching_eng.as_ref().and_then(|e| e.cover_url.clone()));

            jp_item.cover_url = best_cover.clone();
            all_results.push(jp_item);

            if let Some(mut eng_item) = matching_eng {
                if !all_results.iter().any(|r| r.id == eng_item.id) {
                    if eng_item.cover_url.is_none() {
                        eng_item.cover_url = best_cover.clone();
                    }
                    all_results.push(eng_item.clone());
                    used_eng_ids.insert(eng_item.id);
                }
            }
        }

        // 3. Process any remaining standalone ENG items
        for mut eng_item in eng_items {
            if !used_eng_ids.contains(&eng_item.id) && !all_results.iter().any(|r| r.id == eng_item.id) {
                let pid = extract_pid(&eng_item.id);
                if eng_item.cover_url.is_none() {
                    if let Some(existing) = all_results.iter().find(|r| extract_pid(&r.id) == pid) {
                        eng_item.cover_url = existing.cover_url.clone();
                    }
                }
                all_results.push(eng_item);
            }
        }
    }

    // 4. Wikipedia search (Korean & English summary) if auto or wikipedia
    if src == "auto" || src == "wikipedia" {
        if let Ok(wiki_results) = search_wikipedia(search_term).await {
            all_results.extend(wiki_results);
        }
    }

    // 5. RAWG search if auto or rawg
    if src == "auto" || src == "rawg" {
        if let Ok(rawg_results) = search_rawg(search_term).await {
            all_results.extend(rawg_results);
        }
    }

    // Concurrent HEAD requests to resolve image and video file sizes
    enrich_media_sizes(&mut all_results).await;

    let all_results: Vec<ScrapedGame> = all_results.into_iter().map(sanitize_scraped_game).collect();

    Ok(all_results)
}

#[tauri::command]
pub async fn test_screenscraper_account_cmd(
    username: String,
    password: String,
) -> Result<ScreenScraperAccountStatus, String> {
    let creds = ScreenScraperCredentials { username, password };
    test_screenscraper_account(&creds).await
}

#[tauri::command]
pub async fn download_and_upload_scraped_image_cmd(
    host: String,
    port: u16,
    username: String,
    password: String,
    roms_path: String,
    system_id: String,
    rom_filename: String,
    image_url: String,
) -> Result<String, String> {
    // 1. Download image bytes
    let (image_bytes, ext) = download_image(&image_url).await?;

    // 2. Connect via SSH
    let session = RemoteSession::connect(&host, port, &username, &password).await?;

    // 3. Prepare target filename: e.g. "Game (USA)-image.png"
    let clean_rom_filename = rom_filename
        .strip_prefix("./")
        .unwrap_or(&rom_filename)
        .trim_start_matches('/');

    let pure_filename = if let Some(pos) = clean_rom_filename.rfind(['/', '\\']) {
        &clean_rom_filename[pos + 1..]
    } else {
        clean_rom_filename
    };

    let base_name = pure_filename
        .rfind('.')
        .map(|pos| &pure_filename[..pos])
        .unwrap_or(pure_filename);
    let target_filename = format!("{}-image.{}", base_name.trim(), ext);

    let images_dir = format!("{}/{}/images", roms_path.trim_end_matches('/'), system_id);
    let remote_file_path = format!("{}/{}", images_dir, target_filename);

    if let Some(pos) = remote_file_path.rfind('/') {
        let parent_dir = &remote_file_path[..pos];
        session.create_dir_all(parent_dir).await?;
    }

    session.write_file_bytes(&remote_file_path, &image_bytes).await?;

    // Return relative path for gamelist.xml
    Ok(format!("./images/{}", target_filename))
}

#[tauri::command]
pub async fn download_and_upload_scraped_video_cmd(
    host: String,
    port: u16,
    username: String,
    password: String,
    roms_path: String,
    system_id: String,
    rom_filename: String,
    video_url: String,
) -> Result<String, String> {
    // 1. Download video bytes
    let (video_bytes, ext) = download_video(&video_url).await?;

    // 2. Connect via SSH
    let session = RemoteSession::connect(&host, port, &username, &password).await?;

    // 3. Prepare target filename: e.g. "Game (USA)-video.mp4"
    let clean_rom_filename = rom_filename
        .strip_prefix("./")
        .unwrap_or(&rom_filename)
        .trim_start_matches('/');

    let pure_filename = if let Some(pos) = clean_rom_filename.rfind(['/', '\\']) {
        &clean_rom_filename[pos + 1..]
    } else {
        clean_rom_filename
    };

    let base_name = pure_filename
        .rfind('.')
        .map(|pos| &pure_filename[..pos])
        .unwrap_or(pure_filename);
    let target_filename = format!("{}-video.{}", base_name.trim(), ext);

    let videos_dir = format!("{}/{}/videos", roms_path.trim_end_matches('/'), system_id);
    let remote_file_path = format!("{}/{}", videos_dir, target_filename);

    if let Some(pos) = remote_file_path.rfind('/') {
        let parent_dir = &remote_file_path[..pos];
        session.create_dir_all(parent_dir).await?;
    }

    session.write_file_bytes(&remote_file_path, &video_bytes).await?;

    // Return relative path for gamelist.xml
    Ok(format!("./videos/{}", target_filename))
}

struct RomToUpload {
    local_path: std::path::PathBuf,
    rel_name: String,
    size: u64,
}

fn collect_upload_files(
    path: &std::path::Path,
    prefix: &str,
    out: &mut Vec<RomToUpload>,
) {
    if path.is_file() {
        if let Ok(meta) = path.metadata() {
            let fname = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            let rel = if prefix.is_empty() {
                fname
            } else {
                format!("{}/{}", prefix, fname)
            };
            out.push(RomToUpload {
                local_path: path.to_path_buf(),
                rel_name: rel,
                size: meta.len(),
            });
        }
    } else if path.is_dir() {
        let dir_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let new_prefix = if prefix.is_empty() {
            dir_name
        } else {
            format!("{}/{}", prefix, dir_name)
        };

        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                collect_upload_files(&entry.path(), &new_prefix, out);
            }
        }
    }
}

#[tauri::command]
pub async fn upload_rom_files(
    app: tauri::AppHandle,
    host: String,
    port: u16,
    username: String,
    password: String,
    roms_path: String,
    system_id: String,
    local_paths: Vec<String>,
) -> Result<RomUploadResult, String> {
    if local_paths.is_empty() {
        return Ok(RomUploadResult {
            success_count: 0,
            failed_files: vec![],
            message: "업로드할 파일이 지정되지 않았습니다.".to_string(),
        });
    }

    // 1. Gather all files to upload (including directories)
    let mut files_to_upload = Vec::new();
    for p in &local_paths {
        let path = std::path::Path::new(p);
        collect_upload_files(path, "", &mut files_to_upload);
    }

    if files_to_upload.is_empty() {
        return Ok(RomUploadResult {
            success_count: 0,
            failed_files: vec![],
            message: "전송 가능한 파일이 없습니다.".to_string(),
        });
    }

    let total_files_count = files_to_upload.len();
    let overall_total_bytes: u64 = files_to_upload.iter().map(|f| f.size).sum();

    // 2. Connect to remote session
    let session = RemoteSession::connect(&host, port, &username, &password).await?;

    let base_remote_dir = format!("{}/{}", roms_path.trim_end_matches('/'), system_id.trim_matches('/'));
    session.create_dir_all(&base_remote_dir).await?;

    let mut overall_current_bytes: u64 = 0;
    let mut success_count = 0;
    let mut failed_files = Vec::new();

    let start_time = Instant::now();

    for (idx, file) in files_to_upload.iter().enumerate() {
        let remote_dest = format!("{}/{}", base_remote_dir, file.rel_name.replace('\\', "/"));
        let mut file_written: u64 = 0;
        let mut last_emit = Instant::now();

        // Initial progress emit for this file
        let _ = app.emit(
            "rom_upload_progress",
            RomUploadProgress {
                file_name: file.rel_name.clone(),
                file_index: idx + 1,
                total_files: total_files_count,
                current_bytes: 0,
                total_bytes: file.size,
                overall_current_bytes,
                overall_total_bytes,
                bytes_per_sec: 0.0,
                is_finished: false,
            },
        );

        let res = session
            .write_file_from_local(&file.local_path, &remote_dest, |written| {
                file_written = written;
                let cur_overall = overall_current_bytes + written;
                let elapsed = start_time.elapsed().as_secs_f64();
                let bps = if elapsed > 0.05 {
                    cur_overall as f64 / elapsed
                } else {
                    0.0
                };

                if last_emit.elapsed() >= std::time::Duration::from_millis(100) || written == file.size {
                    last_emit = Instant::now();
                    let _ = app.emit(
                        "rom_upload_progress",
                        RomUploadProgress {
                            file_name: file.rel_name.clone(),
                            file_index: idx + 1,
                            total_files: total_files_count,
                            current_bytes: written,
                            total_bytes: file.size,
                            overall_current_bytes: cur_overall,
                            overall_total_bytes,
                            bytes_per_sec: bps,
                            is_finished: false,
                        },
                    );
                }
            })
            .await;

        match res {
            Ok(bytes) => {
                overall_current_bytes += bytes;
                success_count += 1;
            }
            Err(e) => {
                failed_files.push(format!("{}: {}", file.rel_name, e));
            }
        }
    }

    // Final finish event
    let elapsed = start_time.elapsed().as_secs_f64();
    let final_bps = if elapsed > 0.0 {
        overall_current_bytes as f64 / elapsed
    } else {
        0.0
    };

    let _ = app.emit(
        "rom_upload_progress",
        RomUploadProgress {
            file_name: "완료".to_string(),
            file_index: total_files_count,
            total_files: total_files_count,
            current_bytes: 0,
            total_bytes: 0,
            overall_current_bytes,
            overall_total_bytes,
            bytes_per_sec: final_bps,
            is_finished: true,
        },
    );

    let message = if failed_files.is_empty() {
        format!("{}개 파일 전송 완료", success_count)
    } else {
        format!("{}개 전송 성공, {}개 실패", success_count, failed_files.len())
    };

    Ok(RomUploadResult {
        success_count,
        failed_files,
        message,
    })
}

