use reqwest::Client;
use serde_json::Value;
use crate::models::{ScrapedGame, ScreenScraperAccountStatus, ScreenScraperCredentials};


// Default softname and dev keys for ScreenScraper API
const SS_SOFTNAME: &str = "skyscraper";
const SS_DEVID: &str = "muldjord";
const SS_DEVPASSWORD: &str = "uWu5VRc9QDVMPpD8";

// RAWG public demo key for game discovery
const RAWG_API_KEY: &str = "c542e67aec3a4340908f9de9e86038af";

/// Clean ROM filename / query by stripping brackets, extensions, and tags
pub fn clean_query(raw: &str) -> String {
    let trimmed = raw.trim();

    // If query is an exact DLsite product code (e.g. RJ123456, VJ01234567, BJ123456), return it directly
    let upper = trimmed.to_uppercase();
    if (upper.starts_with("RJ") || upper.starts_with("VJ") || upper.starts_with("BJ"))
        && upper.len() >= 8 && upper.len() <= 10
        && upper[2..].chars().all(|c| c.is_ascii_digit())
    {
        return upper;
    }

    // 1. Remove directory prefix (e.g. "RJ139385/game.sh" -> "game.sh")
    let raw_file = if let Some(pos) = raw.rfind(['/', '\\']) {
        &raw[pos + 1..]
    } else {
        raw
    };

    let mut s = raw_file.to_string();

    // 2. Strip any file extension (.zip, .sh, .squashfs, etc.)
    if let Some(pos) = s.rfind('.') {
        if pos > 0 && s.len() - pos <= 9 {
            s.truncate(pos);
        }
    }

    // Remove text inside parentheses (e.g. (USA), (Japan), (En,Fr,De), (Rev 1))
    while let Some(start) = s.find('(') {
        if let Some(end) = s[start..].find(')') {
            s.replace_range(start..start + end + 1, " ");
        } else {
            break;
        }
    }

    // Remove text inside square brackets (e.g. [!], [b1], [T+Kor])
    while let Some(start) = s.find('[') {
        if let Some(end) = s[start..].find(']') {
            s.replace_range(start..start + end + 1, " ");
        } else {
            break;
        }
    }

    // Replace underscores and dashes with spaces
    s = s.replace('_', " ").replace('-', " ");

    // Collapse multiple whitespace
    let cleaned: Vec<&str> = s.split_whitespace().collect();
    cleaned.join(" ")
}

/// Map ES-DE system ID to ScreenScraper system ID
fn get_screenscraper_system_id(sys: &str) -> Option<u32> {
    match sys.to_lowercase().as_str() {
        "snes" | "sfc" => Some(4),
        "nes" | "fc" => Some(3),
        "gba" => Some(12),
        "gb" => Some(9),
        "gbc" => Some(10),
        "megadrive" | "genesis" | "md" => Some(1),
        "psx" | "ps1" | "playstation" => Some(57),
        "n64" => Some(14),
        "mastersystem" | "sms" => Some(2),
        "gamegear" | "gg" => Some(21),
        "neogeo" => Some(142),
        "arcade" | "mame" | "fbneo" => Some(75),
        "nds" => Some(15),
        "psp" => Some(61),
        "pce" | "pcengine" | "tg16" => Some(31),
        "wonderswan" | "ws" => Some(45),
        "wonderswancolor" | "wsc" => Some(46),
        "dreamcast" | "dc" => Some(23),
        "saturn" => Some(22),
        "atari2600" => Some(26),
        "atari7800" => Some(28),
        _ => None,
    }
}

/// Check and test ScreenScraper user credentials
pub async fn test_screenscraper_account(creds: &ScreenScraperCredentials) -> Result<ScreenScraperAccountStatus, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .build()
        .map_err(|e| format!("HTTP 클라이언트 생성 실패: {}", e))?;

    let url = "https://www.screenscraper.fr/api2/ssuserInfos.php";
    let resp = client
        .get(url)
        .query(&[
            ("devid", SS_DEVID),
            ("devpassword", SS_DEVPASSWORD),
            ("softname", SS_SOFTNAME),
            ("output", "json"),
            ("ssid", &creds.username),
            ("sspassword", &creds.password),
        ])
        .send()
        .await
        .map_err(|e| format!("ScreenScraper 서버 연결 실패: {}", e))?;

    let status_code = resp.status();
    let text = resp.text().await.map_err(|e| format!("응답 본문 읽기 실패: {}", e))?;

    if !status_code.is_success() {
        return Ok(ScreenScraperAccountStatus {
            valid: false,
            message: format!("인증 실패 (HTTP 상태 코드 {}): {}", status_code, text.trim()),
            user_id: None,
            requests_today: None,
            max_requests_per_day: None,
        });
    }

    if text.contains("Erreur de login") {
        return Ok(ScreenScraperAccountStatus {
            valid: false,
            message: "아이디 또는 비밀번호가 올바르지 않습니다. (ScreenScraper 계정 정보를 확인해주세요)".to_string(),
            user_id: None,
            requests_today: None,
            max_requests_per_day: None,
        });
    }

    let json: Value = serde_json::from_str(&text).map_err(|_| {
        format!("응답 파싱 실패: {}", text.trim())
    })?;

    let ssuser = &json["response"]["ssuser"];

    if ssuser.is_null() || ssuser["id"].is_null() {
        let err_msg = json["response"]["message"].as_str().unwrap_or("계정 정보를 확인할 수 없습니다.");
        return Ok(ScreenScraperAccountStatus {
            valid: false,
            message: err_msg.to_string(),
            user_id: None,
            requests_today: None,
            max_requests_per_day: None,
        });
    }

    let user_id = ssuser["id"].as_str().map(|s| s.to_string());
    let req_today = ssuser["requeststoday"].as_str().and_then(|s| s.parse::<u32>().ok())
        .or_else(|| ssuser["requeststoday"].as_u64().map(|n| n as u32));
    let max_req = ssuser["maxrequestsperday"].as_str().and_then(|s| s.parse::<u32>().ok())
        .or_else(|| ssuser["maxrequestsperday"].as_u64().map(|n| n as u32));

    Ok(ScreenScraperAccountStatus {
        valid: true,
        message: format!("인증 성공! (오늘 요청: {} / {})", req_today.unwrap_or(0), max_req.unwrap_or(50000)),
        user_id,
        requests_today: req_today,
        max_requests_per_day: max_req,
    })
}


async fn fetch_screenscraper_medias(
    client: &Client,
    game_id: &str,
    creds: Option<&ScreenScraperCredentials>,
) -> (Option<String>, Option<u64>, Option<String>, Option<u64>) {
    let mut req = client.get("https://www.screenscraper.fr/api2/jeuInfos.php")
        .query(&[
            ("devid", SS_DEVID),
            ("devpassword", SS_DEVPASSWORD),
            ("softname", SS_SOFTNAME),
            ("output", "json"),
            ("gameid", game_id),
        ]);

    if let Some(c) = creds {
        if !c.username.is_empty() && !c.password.is_empty() {
            req = req.query(&[("ssid", &c.username), ("sspassword", &c.password)]);
        }
    }

    let mut cover_url = None;
    let mut cover_size = None;
    let mut video_url = None;
    let mut video_size = None;

    if let Ok(resp) = req.send().await {
        if resp.status().is_success() {
            if let Ok(json) = resp.json::<Value>().await {
                if let Some(medias) = json["response"]["jeu"]["medias"].as_array() {
                    for m in medias {
                        let m_type = m["type"].as_str().unwrap_or("");
                        let url = m["url"].as_str().unwrap_or("");
                        let size = m["size"].as_str().and_then(|s| s.parse::<u64>().ok())
                            .or_else(|| m["size"].as_u64());

                        // Prefer 2D box / box-2D or box-3D
                        if (m_type == "box-2D" || m_type == "box-3D" || m_type == "box") && !url.is_empty() && cover_url.is_none() {
                            cover_url = Some(url.to_string());
                            cover_size = size;
                        }
                        // Prefer normalized video (optimized for ES-DE / frontends), fallback to video
                        if m_type == "video-normalized" && !url.is_empty() {
                            video_url = Some(url.to_string());
                            video_size = size;
                        } else if m_type == "video" && !url.is_empty() && video_url.is_none() {
                            video_url = Some(url.to_string());
                            video_size = size;
                        }
                    }
                }
            }
        }
    }

    (cover_url, cover_size, video_url, video_size)
}

/// Search games on ScreenScraper
pub async fn search_screenscraper(
    query: &str,
    system_id: Option<&str>,
    creds: Option<&ScreenScraperCredentials>,
) -> Result<Vec<ScrapedGame>, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .build()
        .map_err(|e| e.to_string())?;

    let mut req = client.get("https://www.screenscraper.fr/api2/jeuRecherche.php")
        .query(&[
            ("devid", SS_DEVID),
            ("devpassword", SS_DEVPASSWORD),
            ("softname", SS_SOFTNAME),
            ("output", "json"),
            ("recherche", query),
        ]);

    if let Some(c) = creds {
        if !c.username.is_empty() && !c.password.is_empty() {
            req = req.query(&[("ssid", &c.username), ("sspassword", &c.password)]);
        }
    }

    if let Some(sys) = system_id {
        if let Some(ss_id) = get_screenscraper_system_id(sys) {
            req = req.query(&[("systemeid", &ss_id.to_string())]);
        }
    }

    let resp = req.send().await.map_err(|e| format!("ScreenScraper 요청 실패: {}", e))?;
    if !resp.status().is_success() {
        return Ok(Vec::new());
    }

    let json: Value = resp.json().await.map_err(|e| format!("JSON 파싱 실패: {}", e))?;
    let mut results = Vec::new();

    let games = match &json["response"]["jeux"] {
        Value::Array(arr) => arr.clone(),
        Value::Object(_) => vec![json["response"]["jeux"].clone()],
        _ => Vec::new(),
    };

    // Parallel fetch real medias (boxart and video) from jeuInfos.php for candidates
    let games_to_process: Vec<Value> = games.into_iter().take(6).collect();
    let mut join_set = tokio::task::JoinSet::new();

    for (idx, g) in games_to_process.into_iter().enumerate() {
        let client_clone = client.clone();
        let creds_clone = creds.cloned();
        let id = g["id"].as_str().or_else(|| g["id"].as_u64().map(|_| "ss_id")).unwrap_or("0").to_string();

        join_set.spawn(async move {
            let (c_url, c_size, v_url, v_size) =
                fetch_screenscraper_medias(&client_clone, &id, creds_clone.as_ref()).await;
            (idx, g, c_url, c_size, v_url, v_size)
        });
    }

    let mut enriched = Vec::new();
    while let Some(res) = join_set.join_next().await {
        if let Ok(item) = res {
            enriched.push(item);
        }
    }
    enriched.sort_by_key(|(idx, ..)| *idx);

    for (_, g, cover_url, cover_size, video_url, video_size) in enriched {
        let id = g["id"].as_str().or_else(|| g["id"].as_u64().map(|_| "ss_id")).unwrap_or("0").to_string();

        // ScreenScraper names (prefer Korean, then US, then default)
        let mut name = String::new();
        if let Some(n_arr) = g["noms"].as_array() {
            for n in n_arr {
                let region = n["region"].as_str().unwrap_or("");
                let text = n["text"].as_str().unwrap_or("");
                if region == "kr" && !text.is_empty() {
                    name = text.to_string();
                    break;
                } else if (region == "us" || region == "wor") && name.is_empty() {
                    name = text.to_string();
                }
            }
            if name.is_empty() && !n_arr.is_empty() {
                name = n_arr[0]["text"].as_str().unwrap_or("").to_string();
            }
        }
        if name.is_empty() {
            name = g["nom"].as_str().unwrap_or("알 수 없는 게임").to_string();
        }

        // Synopsis / Description (prefer Korean, then English)
        let mut desc = String::new();
        if let Some(s_arr) = g["synopsis"].as_array() {
            for s in s_arr {
                let lang = s["langue"].as_str().unwrap_or("");
                let text = s["text"].as_str().unwrap_or("");
                if lang == "kr" && !text.is_empty() {
                    desc = text.to_string();
                    break;
                } else if lang == "en" && desc.is_empty() {
                    desc = text.to_string();
                }
            }
            if desc.is_empty() && !s_arr.is_empty() {
                desc = s_arr[0]["text"].as_str().unwrap_or("").to_string();
            }
        }

        // Release date
        let mut releasedate = None;
        if let Some(dates) = g["dates"].as_array() {
            for d in dates {
                let region = d["region"].as_str().unwrap_or("");
                let text = d["text"].as_str().unwrap_or("");
                if (region == "kr" || region == "us" || region == "wor") && !text.is_empty() {
                    let clean_digits: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
                    if clean_digits.len() >= 8 {
                        releasedate = Some(format!("{}T000000", &clean_digits[..8]));
                        break;
                    } else if clean_digits.len() >= 4 {
                        releasedate = Some(format!("{}0101T000000", &clean_digits[..4]));
                    }
                }
            }
        }

        // Developer / Publisher
        let developer = g["developpeur"]["text"].as_str().map(|s| s.to_string());
        let publisher = g["editeur"]["text"].as_str().map(|s| s.to_string());

        // Genre (prefer Korean, then English)
        let mut genre = None;
        if let Some(genres) = g["genres"].as_array() {
            for gen in genres {
                if let Some(noms) = gen["noms"].as_array() {
                    for n in noms {
                        let lang = n["langue"].as_str().unwrap_or("");
                        let text = n["text"].as_str().unwrap_or("");
                        if lang == "kr" && !text.is_empty() {
                            genre = Some(text.to_string());
                            break;
                        } else if lang == "en" && genre.is_none() {
                            genre = Some(text.to_string());
                        }
                    }
                }
            }
        }

        // Rating
        let rating = g["note"].as_str().and_then(|s| s.parse::<f32>().ok()).map(|r| r / 20.0) // 0~20 scale -> 0~1.0
            .or_else(|| g["note"].as_f64().map(|r| (r as f32) / 20.0));

        // Players
        let players = g["joueurs"]["text"].as_str().map(|s| s.to_string());

        results.push(ScrapedGame {
            id: format!("ss_{}", id),
            source: "ScreenScraper".to_string(),
            name,
            desc,
            cover_url,
            cover_size,
            video_url,
            video_size,
            releasedate,
            developer,
            publisher,
            genre,
            rating,
            players,
        });
    }

    Ok(results)
}

/// Search games on RAWG Video Games Database
pub async fn search_rawg(query: &str) -> Result<Vec<ScrapedGame>, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!(
        "https://api.rawg.io/api/games?search={}&page_size=8&key={}",
        urlencoding::encode(query),
        RAWG_API_KEY
    );

    let resp = client.get(&url).send().await.map_err(|e| format!("RAWG 요청 실패: {}", e))?;
    if !resp.status().is_success() {
        return Ok(Vec::new());
    }

    let json: Value = resp.json().await.map_err(|e| format!("RAWG 파싱 실패: {}", e))?;
    let mut results = Vec::new();

    if let Some(arr) = json["results"].as_array() {
        for g in arr {
            let id = g["id"].as_i64().unwrap_or(0);
            let name = g["name"].as_str().unwrap_or("").to_string();
            let cover_url = g["background_image"].as_str().map(|s| s.to_string());

            let released = g["released"].as_str().map(|s| {
                let clean: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
                if clean.len() >= 8 {
                    format!("{}T000000", &clean[..8])
                } else {
                    s.to_string()
                }
            });

            let rating = g["rating"].as_f64().map(|r| (r as f32) / 5.0); // 0~5 to 0~1.0

            let genre = g["genres"].as_array().and_then(|arr| {
                let names: Vec<&str> = arr.iter().filter_map(|x| x["name"].as_str()).collect();
                if names.is_empty() { None } else { Some(names.join(", ")) }
            });

            let video_url = g["clip"]["clip"].as_str().map(|s| s.to_string());

            results.push(ScrapedGame {
                id: format!("rawg_{}", id),
                source: "RAWG".to_string(),
                name,
                desc: String::new(), // Brief details will be enriched or presented
                cover_url,
                cover_size: None,
                video_url,
                video_size: None,
                releasedate: released,
                developer: None,
                publisher: None,
                genre,
                rating,
                players: None,
            });
        }
    }

    Ok(results)
}

/// Search games on Wikipedia (Korean & English) for localized descriptions
pub async fn search_wikipedia(query: &str) -> Result<Vec<ScrapedGame>, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| e.to_string())?;

    // 1. Try Korean Wikipedia search
    let search_url = format!(
        "https://ko.wikipedia.org/w/api.php?action=query&list=search&srsearch={}&format=json&utf8=1",
        urlencoding::encode(query)
    );

    let mut results = Vec::new();

    if let Ok(resp) = client.get(&search_url).send().await {
        if resp.status().is_success() {
            if let Ok(json) = resp.json::<Value>().await {
                if let Some(items) = json["query"]["search"].as_array() {
                    for item in items.iter().take(3) {
                        let title = item["title"].as_str().unwrap_or("");
                        if title.is_empty() { continue; }

                        // Fetch summary
                        let summary_url = format!(
                            "https://ko.wikipedia.org/api/rest_v1/page/summary/{}",
                            urlencoding::encode(title)
                        );
                        if let Ok(sum_resp) = client.get(&summary_url).send().await {
                            if sum_resp.status().is_success() {
                                if let Ok(sum_json) = sum_resp.json::<Value>().await {
                                    let display_title = sum_json["title"].as_str().unwrap_or(title).to_string();
                                    let desc = sum_json["extract"].as_str().unwrap_or("").to_string();
                                    let thumb = sum_json["thumbnail"]["source"].as_str().map(|s| s.to_string());

                                    results.push(ScrapedGame {
                                        id: format!("wiki_ko_{}", title),
                                        source: "한국어 위키백과".to_string(),
                                        name: display_title,
                                        desc,
                                        cover_url: thumb,
                                        cover_size: None,
                                        video_url: None,
                                        video_size: None,
                                        releasedate: None,
                                        developer: None,
                                        publisher: None,
                                        genre: None,
                                        rating: None,
                                        players: None,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(results)
}

/// Search Steam Store games and fetch detailed metadata + 600x900 vertical boxarts
pub async fn search_steam(query: &str) -> Result<Vec<ScrapedGame>, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let search_url = format!(
        "https://store.steampowered.com/api/storesearch/?term={}&l=korean&cc=KR",
        urlencoding::encode(query)
    );

    let resp = client.get(&search_url).send().await.map_err(|e| format!("Steam 검색 요청 실패: {}", e))?;
    if !resp.status().is_success() {
        return Ok(Vec::new());
    }

    let json: Value = resp.json().await.map_err(|e| format!("Steam 응답 파싱 실패: {}", e))?;
    let mut results = Vec::new();

    if let Some(items) = json["items"].as_array() {
        for item in items.iter().take(4) {
            let appid = match item["id"].as_i64() {
                Some(id) => id,
                None => continue,
            };

            let app_name = item["name"].as_str().unwrap_or("").to_string();

            // Fetch app details
            let detail_url = format!(
                "https://store.steampowered.com/api/appdetails?appids={}&l=korean&cc=KR",
                appid
            );

            let mut desc = String::new();
            let mut releasedate = None;
            let mut developer = None;
            let mut publisher = None;
            let mut genre = None;
            let mut rating = None;
            let mut video_url = None;

            if let Ok(det_resp) = client.get(&detail_url).send().await {
                if det_resp.status().is_success() {
                    if let Ok(det_json) = det_resp.json::<Value>().await {
                        let app_data = &det_json[appid.to_string()]["data"];
                        if !app_data.is_null() {
                            if let Some(d) = app_data["short_description"].as_str() {
                                desc = d.replace("&quot;", "\"").replace("&amp;", "&").replace("&#39;", "'");
                            }

                            if let Some(r_date) = app_data["release_date"]["date"].as_str() {
                                let digits: String = r_date.chars().filter(|c| c.is_ascii_digit()).collect();
                                if digits.len() >= 8 {
                                    releasedate = Some(format!("{}T000000", &digits[..8]));
                                } else if digits.len() >= 4 {
                                    releasedate = Some(format!("{}0101T000000", &digits[..4]));
                                }
                            }

                            if let Some(devs) = app_data["developers"].as_array() {
                                if let Some(d) = devs.first().and_then(|v| v.as_str()) {
                                    developer = Some(d.to_string());
                                }
                            }

                            if let Some(pubs) = app_data["publishers"].as_array() {
                                if let Some(p) = pubs.first().and_then(|v| v.as_str()) {
                                    publisher = Some(p.to_string());
                                }
                            }

                            if let Some(genres) = app_data["genres"].as_array() {
                                let names: Vec<&str> = genres.iter().filter_map(|g| g["description"].as_str()).collect();
                                if !names.is_empty() {
                                    genre = Some(names.join(", "));
                                }
                            }

                            if let Some(score) = app_data["metacritic"]["score"].as_i64() {
                                rating = Some((score as f32) / 100.0);
                            }

                            if let Some(movies) = app_data["movies"].as_array() {
                                for mov in movies {
                                    if let Some(mp4) = mov["mp4"]["480"].as_str().or_else(|| mov["mp4"]["max"].as_str()) {
                                        if !mp4.is_empty() {
                                            video_url = Some(mp4.to_string());
                                            break;
                                        }
                                    } else if let Some(webm) = mov["webm"]["480"].as_str().or_else(|| mov["webm"]["max"].as_str()) {
                                        if !webm.is_empty() {
                                            video_url = Some(webm.to_string());
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // High quality vertical 600x900 cover
            let cover_url = format!(
                "https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/{}/library_600x900_2x.jpg",
                appid
            );

            results.push(ScrapedGame {
                id: format!("steam_{}", appid),
                source: "Steam".to_string(),
                name: app_name,
                desc,
                cover_url: Some(cover_url),
                cover_size: None,
                video_url,
                video_size: None,
                releasedate,
                developer,
                publisher,
                genre,
                rating,
                players: Some("1".to_string()),
            });
        }
    }

    Ok(results)
}

/// Cleans raw HTML snippet into clean plain text for game descriptions
pub fn clean_html_description(raw: &str) -> String {
    let s = raw
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("</p>", "\n\n")
        .replace("</li>", "\n")
        .replace("</div>", "\n")
        .replace("</tr>", "\n");

    let mut in_tag = false;
    let mut clean = String::with_capacity(s.len());
    for c in s.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            clean.push(c);
        }
    }

    let unescaped = clean
        .replace("&quot;", "\"")
        .replace("&#039;", "'")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ");

    let mut lines = Vec::new();
    let mut empty_count = 0;
    for line in unescaped.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            empty_count += 1;
            if empty_count <= 1 && !lines.is_empty() {
                lines.push("");
            }
        } else {
            empty_count = 0;
            lines.push(trimmed);
        }
    }

    let result = lines.join("\n").trim().to_string();
    if result.chars().count() > 1500 {
        let truncated: String = result.chars().take(1500).collect();
        format!("{}...", truncated)
    } else {
        result
    }
}

/// Extracts game description from DLsite product page HTML
pub fn extract_dlsite_description(html: &str) -> Option<String> {
    // 1. Try work_parts_area (the main creator description body)
    if let Some(pos) = html.find("class=\"work_parts_area\">") {
        let after = &html[pos + 24..];
        if let Some(end_pos) = after.find("</div>") {
            let chunk = &after[..end_pos];
            let cleaned = clean_html_description(chunk);
            if !cleaned.is_empty() {
                return Some(cleaned);
            }
        }
    }

    // 2. Try itemprop="description" container
    if let Some(pos) = html.find("itemprop=\"description\"") {
        let after = &html[pos..];
        if let Some(start_tag) = after.find('>') {
            let content_after = &after[start_tag + 1..];
            let limit = content_after.len().min(3000);
            let chunk = &content_after[..limit];
            let cleaned = clean_html_description(chunk);
            if !cleaned.is_empty() {
                return Some(cleaned);
            }
        }
    }

    // 3. Fallback to meta name="description" or property="og:description"
    for tag in &["property=\"og:description\" content=\"", "name=\"description\" content=\""] {
        if let Some(pos) = html.find(tag) {
            let after = &html[pos + tag.len()..];
            if let Some(end_pos) = after.find('"') {
                let meta_desc = &after[..end_pos];
                // Strip boilerplate DLsite suffix (e.g. ."DLsite... or .&quot;DLsite... or  - DLsite)
                let clean_meta = if let Some(cut) = meta_desc.find(".\"DLsite") {
                    &meta_desc[..cut + 1]
                } else if let Some(cut) = meta_desc.find(".&quot;DLsite") {
                    &meta_desc[..cut + 1]
                } else if let Some(cut) = meta_desc.find(" - DLsite") {
                    &meta_desc[..cut]
                } else {
                    meta_desc
                };
                let cleaned = clean_html_description(clean_meta);
                if !cleaned.is_empty() {
                    return Some(cleaned);
                }
            }
        }
    }

    None
}

/// Helper to extract DLsite cover image URL from HTML snippet (looks for modpub/images, _img_main, _img_sam, etc.)
pub fn extract_dlsite_image_from_html(html_snippet: &str) -> Option<String> {
    for pattern in ["_img_main.jpg", "_img_main.webp", "_img_sam.jpg", "_img_sam.webp", "_img_main.png"] {
        if let Some(img_pos) = html_snippet.find(pattern) {
            let start = if img_pos > 250 { img_pos - 250 } else { 0 };
            let slice = &html_snippet[start..img_pos + pattern.len()];
            if let Some(http_pos) = slice.find("//img.dlsite") {
                return Some(format!("https:{}", &slice[http_pos..]));
            }
        }
    }
    if let Some(pos) = html_snippet.find("//img.dlsite") {
        let slice = &html_snippet[pos..];
        if let Some(end) = slice.find(['"', '\'', ' ', '>', '<']) {
            let url = &slice[..end];
            if url.contains("/modpub/images") || url.contains("/images2/") || url.contains("_img_") {
                return Some(if url.starts_with("//") {
                    format!("https:{}", url)
                } else {
                    url.to_string()
                });
            }
        }
    }
    None
}

/// Helper to fetch DLsite product page HTML and extract metadata (title, desc, maker, genre, image)
async fn fetch_dlsite_work_page(
    client: &Client,
    pid: &str,
) -> (Option<String>, Option<String>, Option<String>, Option<String>, Option<String>) {
    let work_pages = [
        format!("https://www.dlsite.com/home/work/=/product_id/{}.html", pid),
        format!("https://www.dlsite.com/maniax/work/=/product_id/{}.html", pid),
    ];

    for wp in &work_pages {
        if let Ok(resp) = client.get(wp).send().await {
            if resp.status().is_success() {
                if let Ok(html) = resp.text().await {
                    let mut title = None;
                    if let Some(w_pos) = html.find("id=\"work_name\">") {
                        let after = &html[w_pos + 15..];
                        if let Some(w_end) = after.find('<') {
                            let t = after[..w_end].trim();
                            if !t.is_empty() {
                                title = Some(t.to_string());
                            }
                        }
                    }

                    let mut maker = None;
                    if let Some(m_pos) = html.find("class=\"maker_name\"") {
                        let after = &html[m_pos..];
                        if let Some(a_pos) = after.find("<a ") {
                            let after_a = &after[a_pos..];
                            if let Some(tag_end) = after_a.find('>') {
                                let content_part = &after_a[tag_end + 1..];
                                if let Some(close_a) = content_part.find("</a>") {
                                    let clean_m = clean_html_description(&content_part[..close_a]);
                                    if !clean_m.is_empty() {
                                        maker = Some(clean_m);
                                    }
                                }
                            }
                        }
                    }

                    let desc = extract_dlsite_description(&html);

                    let mut genre = None;
                    if let Some(g_pos) = html.find("class=\"work_category") {
                        let after = &html[g_pos..];
                        if let Some(a_start) = after.find("<a ") {
                            let after_a = &after[a_start..];
                            if let Some(tag_end) = after_a.find('>') {
                                let content_part = &after_a[tag_end + 1..];
                                if let Some(close_a) = content_part.find("</a>") {
                                    let clean_g = clean_html_description(&content_part[..close_a]);
                                    if !clean_g.is_empty() {
                                        genre = Some(clean_g);
                                    }
                                }
                            }
                        }
                    }

                    let mut image = None;
                    if let Some(og_pos) = html.find("property=\"og:image\"") {
                        let chunk = &html[og_pos..og_pos + 300.min(html.len() - og_pos)];
                        if let Some(c_pos) = chunk.find("content=\"") {
                            let start = c_pos + 9;
                            if let Some(end) = chunk[start..].find('"') {
                                let img = chunk[start..start + end].trim();
                                if !img.is_empty() {
                                    image = Some(img.to_string());
                                }
                            }
                        }
                    }
                    if image.is_none() {
                        image = extract_dlsite_image_from_html(&html);
                    }

                    return (title, desc, maker, genre, image);
                }
            }
        }
    }

    (None, None, None, None, None)
}

/// Search DLsite games by RJ/VJ code or title keyword
pub async fn search_dlsite(query: &str) -> Result<Vec<ScrapedGame>, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();

    // 1. Check if query contains an RJ/VJ product code (e.g. RJ123456, RJ01234567, VJ123456)
    let uppercase_q = query.to_uppercase();
    let mut detected_rj = None;
    for token in uppercase_q.split(|c: char| !c.is_alphanumeric()) {
        if (token.starts_with("RJ") || token.starts_with("VJ") || token.starts_with("BJ")) && token.len() >= 6 && token[2..].chars().all(|c| c.is_ascii_digit()) {
            detected_rj = Some(token.to_string());
            break;
        }
    }

    if let Some(rj_code) = detected_rj {
        let endpoints = [
            format!("https://www.dlsite.com/home/product/info/ajax?product_id={}", rj_code),
            format!("https://www.dlsite.com/maniax/product/info/ajax?product_id={}", rj_code),
        ];

        for ep in &endpoints {
            if let Ok(resp) = client.get(ep).send().await {
                if resp.status().is_success() {
                    if let Ok(json) = resp.json::<Value>().await {
                        if let Some(work) = json.get(&rj_code) {
                            let mut name = work["work_name"].as_str().unwrap_or(&rj_code).to_string();
                            let mut image = work["work_image"].as_str().map(|s| s.to_string());
                            if let Some(ref mut img) = image {
                                if img.starts_with("//") {
                                    *img = format!("https:{}", img);
                                }
                            }

                            let regist_date = work["regist_date"].as_str().map(|s| {
                                let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
                                if digits.len() >= 8 {
                                    format!("{}T000000", &digits[..8])
                                } else {
                                    s.to_string()
                                }
                            });

                            let mut maker = work["maker_id"].as_str().map(|s| s.to_string());
                            let mut genre = work["work_type"].as_str().map(|s| s.to_string());
                            let rating = work["rate_average_2dp"].as_f64().map(|r| (r as f32) / 5.0);

                            let mut desc = format!("[{}] DLsite 등록 작품", rj_code);
                            let (wp_title, wp_desc, wp_maker, wp_genre, wp_img) = fetch_dlsite_work_page(&client, &rj_code).await;
                            if let Some(t) = wp_title {
                                if !t.is_empty() {
                                    name = t;
                                }
                            }
                            if let Some(d) = wp_desc {
                                desc = d;
                            }
                            if let Some(m) = wp_maker {
                                maker = Some(m);
                            }
                            if let Some(g) = wp_genre {
                                genre = Some(g);
                            }
                            if image.is_none() {
                                image = wp_img;
                            }

                            results.push(ScrapedGame {
                                id: format!("dlsite_{}", rj_code),
                                source: "DLsite".to_string(),
                                name,
                                desc,
                                cover_url: image,
                                cover_size: None,
                                video_url: None,
                                video_size: None,
                                releasedate: regist_date,
                                developer: maker,
                                publisher: Some("DLsite".to_string()),
                                genre,
                                rating,
                                players: Some("1".to_string()),
                            });
                            return Ok(results);
                        }
                    }
                }
            }
        }
    }

    // 2. Keyword search on DLsite HTML (home & maniax)
    let search_urls = [
        format!("https://www.dlsite.com/home/fsr/=/keyword/{}", urlencoding::encode(query)),
        format!("https://www.dlsite.com/maniax/fsr/=/keyword/{}", urlencoding::encode(query)),
    ];

    for s_url in &search_urls {
        if let Ok(resp) = client.get(s_url).send().await {
            if resp.status().is_success() {
                if let Ok(html) = resp.text().await {
                    let mut start_idx = 0;
                    while let Some(pid_pos) = html[start_idx..].find("data-product_id=\"") {
                        let actual_pos = start_idx + pid_pos + 17;
                        if let Some(end_pid) = html[actual_pos..].find('"') {
                            let pid = &html[actual_pos..actual_pos + end_pid];
                            start_idx = actual_pos + end_pid;

                            if results.iter().any(|r| r.id == format!("dlsite_{}", pid)) {
                                continue;
                            }

                            let chunk_start = actual_pos.saturating_sub(600);
                            let chunk_end = (start_idx + 1500).min(html.len());
                            let chunk = &html[chunk_start..chunk_end];

                            let title = if let Some(t_pos) = chunk.find("class=\"work_name\"") {
                                let after = &chunk[t_pos..];
                                if let Some(a_start) = after.find("title=\"") {
                                    let t_val_start = a_start + 7;
                                    if let Some(t_val_end) = after[t_val_start..].find('"') {
                                        after[t_val_start..t_val_start + t_val_end].to_string()
                                    } else { pid.to_string() }
                                } else { pid.to_string() }
                            } else { pid.to_string() };

                            let mut img_url = extract_dlsite_image_from_html(chunk);

                            let mut maker = if let Some(m_pos) = chunk.find("class=\"maker_name\"") {
                                let after = &chunk[m_pos..];
                                if let Some(tag_end) = after.find('>') {
                                    let text_part = &after[tag_end + 1..];
                                    if let Some(close_tag) = text_part.find('<') {
                                        let clean_m = text_part[..close_tag].trim();
                                        if !clean_m.is_empty() { Some(clean_m.to_string()) } else { None }
                                    } else { None }
                                } else { None }
                            } else { None };

                            let mut desc = format!("[{}] DLsite 검색 결과", pid);
                            let mut genre = Some("동인 게임".to_string());
                            if results.len() < 3 {
                                let (_t, wp_desc, wp_maker, wp_genre, wp_img) = fetch_dlsite_work_page(&client, pid).await;
                                if let Some(d) = wp_desc { desc = d; }
                                if let Some(m) = wp_maker { maker = Some(m); }
                                if let Some(g) = wp_genre { genre = Some(g); }
                                if img_url.is_none() { img_url = wp_img; }
                            }

                            results.push(ScrapedGame {
                                id: format!("dlsite_{}", pid),
                                source: "DLsite".to_string(),
                                name: title,
                                desc,
                                cover_url: img_url,
                                cover_size: None,
                                video_url: None,
                                video_size: None,
                                releasedate: None,
                                developer: maker,
                                publisher: Some("DLsite".to_string()),
                                genre,
                                rating: None,
                                players: Some("1".to_string()),
                            });

                            if results.len() >= 6 {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    if !results.is_empty() {
                        break;
                    }
                }
            }
        }
    }

    Ok(results)
}

/// Search DLsite games in English (localized title, circle, and categories)
pub async fn search_dlsite_eng(query: &str) -> Result<Vec<ScrapedGame>, String> {
    let mut default_headers = reqwest::header::HeaderMap::new();
    default_headers.insert(
        reqwest::header::COOKIE,
        reqwest::header::HeaderValue::from_static("locale=en_US; adultchecked=1"),
    );

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .default_headers(default_headers)
        .build()
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();

    // 1. Check if query contains an RJ/VJ product code
    let uppercase_q = query.to_uppercase();
    let mut detected_rj = None;
    for token in uppercase_q.split(|c: char| !c.is_alphanumeric()) {
        if (token.starts_with("RJ") || token.starts_with("VJ") || token.starts_with("BJ")) && token.len() >= 6 && token[2..].chars().all(|c| c.is_ascii_digit()) {
            detected_rj = Some(token.to_string());
            break;
        }
    }

    if let Some(rj_code) = detected_rj {
        // Try English search for this RJ code first
        let eng_search_urls = [
            format!("https://www.dlsite.com/home/fsr/=/language/en/keyword/{}", rj_code),
            format!("https://www.dlsite.com/maniax/fsr/=/language/en/keyword/{}", rj_code),
        ];

        for s_url in &eng_search_urls {
            if let Ok(resp) = client.get(s_url).send().await {
                if resp.status().is_success() {
                    if let Ok(html) = resp.text().await {
                        if let Some(pid_pos) = html.find("data-product_id=\"") {
                            let actual_pos = pid_pos + 17;
                            if let Some(end_pid) = html[actual_pos..].find('"') {
                                let pid = &html[actual_pos..actual_pos + end_pid];
                                let chunk_start = actual_pos.saturating_sub(600);
                                let chunk_end = (actual_pos + end_pid + 1500).min(html.len());
                                let chunk = &html[chunk_start..chunk_end];

                                let title = if let Some(t_pos) = chunk.find("class=\"work_name\"") {
                                    let after = &chunk[t_pos..];
                                    if let Some(a_start) = after.find("title=\"") {
                                        let t_val_start = a_start + 7;
                                        if let Some(t_val_end) = after[t_val_start..].find('"') {
                                            after[t_val_start..t_val_start + t_val_end].to_string()
                                        } else { pid.to_string() }
                                    } else { pid.to_string() }
                                } else { pid.to_string() };

                                let mut img_url = extract_dlsite_image_from_html(chunk);

                                let maker = if let Some(m_pos) = chunk.find("class=\"maker_name\"") {
                                    let after = &chunk[m_pos..];
                                    if let Some(tag_end) = after.find('>') {
                                        let text_part = &after[tag_end + 1..];
                                        if let Some(close_tag) = text_part.find('<') {
                                            let clean_m = text_part[..close_tag].trim();
                                            if !clean_m.is_empty() { Some(clean_m.to_string()) } else { None }
                                        } else { None }
                                    } else { None }
                                } else { None };

                                let genre = if let Some(g_pos) = chunk.find("class=\"work_category") {
                                    let after = &chunk[g_pos..];
                                    if let Some(a_start) = after.find("<a ") {
                                        let after_a = &after[a_start..];
                                        if let Some(tag_end) = after_a.find('>') {
                                            let text_part = &after_a[tag_end + 1..];
                                            if let Some(close_tag) = text_part.find('<') {
                                                let clean_g = text_part[..close_tag].trim();
                                                if !clean_g.is_empty() { Some(clean_g.to_string()) } else { None }
                                            } else { None }
                                        } else { None }
                                    } else { None }
                                } else { None };

                                let (_wp_title, wp_desc, wp_maker, wp_genre, wp_img) = fetch_dlsite_work_page(&client, pid).await;
                                let desc = wp_desc.unwrap_or_else(|| format!("[{}] DLsite (ENG)", pid));
                                let maker = maker.or(wp_maker);
                                let genre = genre.or(wp_genre);
                                if img_url.is_none() {
                                    img_url = wp_img;
                                }

                                // Try to fetch release date and image from AJAX endpoints (both home and maniax)
                                let mut releasedate = None;
                                let endpoints = [
                                    format!("https://www.dlsite.com/home/product/info/ajax?product_id={}", pid),
                                    format!("https://www.dlsite.com/maniax/product/info/ajax?product_id={}", pid),
                                ];
                                for ep in &endpoints {
                                    if let Ok(resp) = client.get(ep).send().await {
                                        if resp.status().is_success() {
                                            if let Ok(json) = resp.json::<Value>().await {
                                                if let Some(work) = json.get(pid) {
                                                    if releasedate.is_none() {
                                                        releasedate = work["regist_date"].as_str().map(|s| {
                                                            let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
                                                            if digits.len() >= 8 {
                                                                format!("{}T000000", &digits[..8])
                                                            } else {
                                                                s.to_string()
                                                            }
                                                        });
                                                    }
                                                    if img_url.is_none() {
                                                        if let Some(w_img) = work["work_image"].as_str() {
                                                            let full_img = if w_img.starts_with("//") {
                                                                format!("https:{}", w_img)
                                                            } else {
                                                                w_img.to_string()
                                                            };
                                                            img_url = Some(full_img);
                                                        }
                                                    }
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }

                                results.push(ScrapedGame {
                                    id: format!("dlsite_eng_{}", pid),
                                    source: "DLsite (ENG)".to_string(),
                                    name: title,
                                    desc,
                                    cover_url: img_url,
                                    cover_size: None,
                                    video_url: None,
                                    video_size: None,
                                    releasedate,
                                    developer: maker,
                                    publisher: Some("DLsite".to_string()),
                                    genre,
                                    rating: None,
                                    players: Some("1".to_string()),
                                });
                                return Ok(results);
                            }
                        }
                    }
                }
            }
        }

        // Check AJAX endpoint
        let endpoints = [
            format!("https://www.dlsite.com/home/product/info/ajax?product_id={}", rj_code),
            format!("https://www.dlsite.com/maniax/product/info/ajax?product_id={}", rj_code),
        ];

        for ep in &endpoints {
            if let Ok(resp) = client.get(ep).send().await {
                if resp.status().is_success() {
                    if let Ok(json) = resp.json::<Value>().await {
                        if let Some(work) = json.get(&rj_code) {
                            let mut title = work["work_name"].as_str().unwrap_or(&rj_code).to_string();
                            let mut image = work["work_image"].as_str().map(|s| s.to_string());
                            if let Some(ref mut img) = image {
                                if img.starts_with("//") {
                                    *img = format!("https:{}", img);
                                }
                            }

                            let regist_date = work["regist_date"].as_str().map(|s| {
                                let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
                                if digits.len() >= 8 {
                                    format!("{}T000000", &digits[..8])
                                } else {
                                    s.to_string()
                                }
                            });

                            let mut maker = work["maker_id"].as_str().map(|s| s.to_string());
                            let mut genre = work["work_type"].as_str().map(|s| s.to_string());
                            let rating = work["rate_average_2dp"].as_f64().map(|r| (r as f32) / 5.0);

                            // Also try product page in English to get localized title & description if available
                            let (wp_title, wp_desc, wp_maker, wp_genre, wp_img) = fetch_dlsite_work_page(&client, &rj_code).await;
                            if let Some(t) = wp_title {
                                if !t.is_empty() {
                                    title = t;
                                }
                            }
                            let desc = wp_desc.unwrap_or_else(|| format!("[{}] DLsite (ENG)", rj_code));
                            if let Some(m) = wp_maker {
                                maker = Some(m);
                            }
                            if let Some(g) = wp_genre {
                                genre = Some(g);
                            }
                            if image.is_none() {
                                image = wp_img;
                            }

                            results.push(ScrapedGame {
                                id: format!("dlsite_eng_{}", rj_code),
                                source: "DLsite (ENG)".to_string(),
                                name: title,
                                desc,
                                cover_url: image,
                                cover_size: None,
                                video_url: None,
                                video_size: None,
                                releasedate: regist_date,
                                developer: maker,
                                publisher: Some("DLsite".to_string()),
                                genre,
                                rating,
                                players: Some("1".to_string()),
                            });
                            return Ok(results);
                        }
                    }
                }
            }
        }
    }

    // 2. Keyword search (first try language=en, then fallback to general search with locale=en_US)
    let search_urls = [
        format!("https://www.dlsite.com/home/fsr/=/language/en/keyword/{}", urlencoding::encode(query)),
        format!("https://www.dlsite.com/maniax/fsr/=/language/en/keyword/{}", urlencoding::encode(query)),
        format!("https://www.dlsite.com/home/fsr/=/keyword/{}", urlencoding::encode(query)),
        format!("https://www.dlsite.com/maniax/fsr/=/keyword/{}", urlencoding::encode(query)),
    ];

    for s_url in &search_urls {
        if let Ok(resp) = client.get(s_url).send().await {
            if resp.status().is_success() {
                if let Ok(html) = resp.text().await {
                    let mut start_idx = 0;
                    while let Some(pid_pos) = html[start_idx..].find("data-product_id=\"") {
                        let actual_pos = start_idx + pid_pos + 17;
                        if let Some(end_pid) = html[actual_pos..].find('"') {
                            let pid = &html[actual_pos..actual_pos + end_pid];
                            start_idx = actual_pos + end_pid;

                            if results.iter().any(|r| r.id == format!("dlsite_eng_{}", pid)) {
                                continue;
                            }

                            let chunk_end = (start_idx + 1500).min(html.len());
                            let chunk = &html[start_idx..chunk_end];

                            let title = if let Some(t_pos) = chunk.find("class=\"work_name\"") {
                                let after = &chunk[t_pos..];
                                if let Some(a_start) = after.find("title=\"") {
                                    let t_val_start = a_start + 7;
                                    if let Some(t_val_end) = after[t_val_start..].find('"') {
                                        after[t_val_start..t_val_start + t_val_end].to_string()
                                    } else { pid.to_string() }
                                } else { pid.to_string() }
                            } else { pid.to_string() };

                            let mut img_url = extract_dlsite_image_from_html(chunk);

                            let mut maker = if let Some(m_pos) = chunk.find("class=\"maker_name\"") {
                                let after = &chunk[m_pos..];
                                if let Some(tag_end) = after.find('>') {
                                    let text_part = &after[tag_end + 1..];
                                    if let Some(close_tag) = text_part.find('<') {
                                        let clean_m = text_part[..close_tag].trim();
                                        if !clean_m.is_empty() { Some(clean_m.to_string()) } else { None }
                                    } else { None }
                                } else { None }
                            } else { None };

                            let mut genre = if let Some(g_pos) = chunk.find("class=\"work_category") {
                                let after = &chunk[g_pos..];
                                if let Some(a_start) = after.find("<a ") {
                                    let after_a = &after[a_start..];
                                    if let Some(tag_end) = after_a.find('>') {
                                        let text_part = &after_a[tag_end + 1..];
                                        if let Some(close_tag) = text_part.find('<') {
                                            let clean_g = text_part[..close_tag].trim();
                                            if !clean_g.is_empty() { Some(clean_g.to_string()) } else { None }
                                        } else { None }
                                    } else { None }
                                } else { None }
                            } else { None };

                            let mut desc = format!("[{}] DLsite (ENG)", pid);
                            if results.len() < 3 {
                                let (_t, wp_desc, wp_maker, wp_genre, wp_img) = fetch_dlsite_work_page(&client, pid).await;
                                if let Some(d) = wp_desc { desc = d; }
                                if let Some(m) = wp_maker { maker = Some(m); }
                                if let Some(g) = wp_genre { genre = Some(g); }
                                if img_url.is_none() { img_url = wp_img; }
                            }

                            if img_url.is_none() {
                                let endpoints = [
                                    format!("https://www.dlsite.com/home/product/info/ajax?product_id={}", pid),
                                    format!("https://www.dlsite.com/maniax/product/info/ajax?product_id={}", pid),
                                ];
                                for ep in &endpoints {
                                    if let Ok(resp) = client.get(ep).send().await {
                                        if resp.status().is_success() {
                                            if let Ok(json) = resp.json::<Value>().await {
                                                if let Some(work) = json.get(pid) {
                                                    if let Some(w_img) = work["work_image"].as_str() {
                                                        let full_img = if w_img.starts_with("//") {
                                                            format!("https:{}", w_img)
                                                        } else {
                                                            w_img.to_string()
                                                        };
                                                        img_url = Some(full_img);
                                                    }
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            results.push(ScrapedGame {
                                id: format!("dlsite_eng_{}", pid),
                                source: "DLsite (ENG)".to_string(),
                                name: title,
                                desc,
                                cover_url: img_url,
                                cover_size: None,
                                video_url: None,
                                video_size: None,
                                releasedate: None,
                                developer: maker,
                                publisher: Some("DLsite".to_string()),
                                genre,
                                rating: None,
                                players: Some("1".to_string()),
                            });

                            if results.len() >= 6 {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    if !results.is_empty() {
                        break;
                    }
                }
            }
        }
    }

    Ok(results)
}

/// Search DLsite games in Korean (localized Korean title, circle, categories, and description)
pub async fn search_dlsite_kor(query: &str) -> Result<Vec<ScrapedGame>, String> {
    let mut default_headers = reqwest::header::HeaderMap::new();
    default_headers.insert(
        reqwest::header::COOKIE,
        reqwest::header::HeaderValue::from_static("locale=ko_KR; adultchecked=1"),
    );

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .default_headers(default_headers)
        .build()
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();

    // 1. Check if query contains an RJ/VJ/BJ product code
    let uppercase_q = query.to_uppercase();
    let mut detected_rj = None;
    for token in uppercase_q.split(|c: char| !c.is_alphanumeric()) {
        if (token.starts_with("RJ") || token.starts_with("VJ") || token.starts_with("BJ"))
            && token.len() >= 6
            && token[2..].chars().all(|c| c.is_ascii_digit())
        {
            detected_rj = Some(token.to_string());
            break;
        }
    }

    if let Some(rj_code) = detected_rj {
        // Try Korean-filtered search URLs for this product code first
        let kor_search_urls = [
            format!("https://www.dlsite.com/home/fsr/=/language/ko/keyword/{}", rj_code),
            format!("https://www.dlsite.com/maniax/fsr/=/language/ko/keyword/{}", rj_code),
            format!("https://www.dlsite.com/home/fsr/=/options/KO_KR/keyword/{}", rj_code),
            format!("https://www.dlsite.com/maniax/fsr/=/options/KO_KR/keyword/{}", rj_code),
        ];

        for s_url in &kor_search_urls {
            if let Ok(resp) = client.get(s_url).send().await {
                if resp.status().is_success() {
                    if let Ok(html) = resp.text().await {
                        if let Some(pid_pos) = html.find("data-product_id=\"") {
                            let actual_pos = pid_pos + 17;
                            if let Some(end_pid) = html[actual_pos..].find('"') {
                                let pid = &html[actual_pos..actual_pos + end_pid];
                                let chunk_start = actual_pos.saturating_sub(600);
                                let chunk_end = (actual_pos + end_pid + 1500).min(html.len());
                                let chunk = &html[chunk_start..chunk_end];

                                let title = if let Some(t_pos) = chunk.find("class=\"work_name\"") {
                                    let after = &chunk[t_pos..];
                                    if let Some(a_start) = after.find("title=\"") {
                                        let t_val_start = a_start + 7;
                                        if let Some(t_val_end) = after[t_val_start..].find('"') {
                                            after[t_val_start..t_val_start + t_val_end].to_string()
                                        } else { pid.to_string() }
                                    } else { pid.to_string() }
                                } else { pid.to_string() };

                                let mut img_url = extract_dlsite_image_from_html(chunk);

                                let maker = if let Some(m_pos) = chunk.find("class=\"maker_name\"") {
                                    let after = &chunk[m_pos..];
                                    if let Some(tag_end) = after.find('>') {
                                        let text_part = &after[tag_end + 1..];
                                        if let Some(close_tag) = text_part.find('<') {
                                            let clean_m = text_part[..close_tag].trim();
                                            if !clean_m.is_empty() { Some(clean_m.to_string()) } else { None }
                                        } else { None }
                                    } else { None }
                                } else { None };

                                let genre = if let Some(g_pos) = chunk.find("class=\"work_category") {
                                    let after = &chunk[g_pos..];
                                    if let Some(a_start) = after.find("<a ") {
                                        let after_a = &after[a_start..];
                                        if let Some(tag_end) = after_a.find('>') {
                                            let text_part = &after_a[tag_end + 1..];
                                            if let Some(close_tag) = text_part.find('<') {
                                                let clean_g = text_part[..close_tag].trim();
                                                if !clean_g.is_empty() { Some(clean_g.to_string()) } else { None }
                                            } else { None }
                                        } else { None }
                                    } else { None }
                                } else { None };

                                let (_wp_title, wp_desc, wp_maker, wp_genre, wp_img) = fetch_dlsite_work_page(&client, pid).await;
                                let desc = wp_desc.unwrap_or_else(|| format!("[{}] DLsite (한국어)", pid));
                                let maker = maker.or(wp_maker);
                                let genre = genre.or(wp_genre);
                                if img_url.is_none() {
                                    img_url = wp_img;
                                }

                                let mut releasedate = None;
                                let endpoints = [
                                    format!("https://www.dlsite.com/home/product/info/ajax?product_id={}", pid),
                                    format!("https://www.dlsite.com/maniax/product/info/ajax?product_id={}", pid),
                                ];
                                for ep in &endpoints {
                                    if let Ok(resp) = client.get(ep).send().await {
                                        if resp.status().is_success() {
                                            if let Ok(json) = resp.json::<Value>().await {
                                                if let Some(work) = json.get(pid) {
                                                    if releasedate.is_none() {
                                                        releasedate = work["regist_date"].as_str().map(|s| {
                                                            let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
                                                            if digits.len() >= 8 {
                                                                format!("{}T000000", &digits[..8])
                                                            } else {
                                                                s.to_string()
                                                            }
                                                        });
                                                    }
                                                    if img_url.is_none() {
                                                        if let Some(w_img) = work["work_image"].as_str() {
                                                            let full_img = if w_img.starts_with("//") {
                                                                format!("https:{}", w_img)
                                                            } else {
                                                                w_img.to_string()
                                                            };
                                                            img_url = Some(full_img);
                                                        }
                                                    }
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }

                                results.push(ScrapedGame {
                                    id: format!("dlsite_kor_{}", pid),
                                    source: "DLsite (KOR)".to_string(),
                                    name: title,
                                    desc,
                                    cover_url: img_url,
                                    cover_size: None,
                                    video_url: None,
                                    video_size: None,
                                    releasedate,
                                    developer: maker,
                                    publisher: Some("DLsite".to_string()),
                                    genre,
                                    rating: None,
                                    players: Some("1".to_string()),
                                });
                                return Ok(results);
                            }
                        }
                    }
                }
            }
        }

        // Direct product page / AJAX check in Korean locale
        let endpoints = [
            format!("https://www.dlsite.com/home/product/info/ajax?product_id={}", rj_code),
            format!("https://www.dlsite.com/maniax/product/info/ajax?product_id={}", rj_code),
        ];

        for ep in &endpoints {
            if let Ok(resp) = client.get(ep).send().await {
                if resp.status().is_success() {
                    if let Ok(json) = resp.json::<Value>().await {
                        if let Some(work) = json.get(&rj_code) {
                            let mut title = work["work_name"].as_str().unwrap_or(&rj_code).to_string();
                            let mut image = work["work_image"].as_str().map(|s| s.to_string());
                            if let Some(ref mut img) = image {
                                if img.starts_with("//") {
                                    *img = format!("https:{}", img);
                                }
                            }

                            let regist_date = work["regist_date"].as_str().map(|s| {
                                let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
                                if digits.len() >= 8 {
                                    format!("{}T000000", &digits[..8])
                                } else {
                                    s.to_string()
                                }
                            });

                            let mut maker = work["maker_id"].as_str().map(|s| s.to_string());
                            let mut genre = work["work_type"].as_str().map(|s| s.to_string());
                            let rating = work["rate_average_2dp"].as_f64().map(|r| (r as f32) / 5.0);

                            // Fetch work page with Korean client to obtain Korean description, title & category
                            let (wp_title, wp_desc, wp_maker, wp_genre, wp_img) = fetch_dlsite_work_page(&client, &rj_code).await;
                            if let Some(t) = wp_title {
                                if !t.is_empty() {
                                    title = t;
                                }
                            }
                            let desc = wp_desc.unwrap_or_else(|| format!("[{}] DLsite (한국어)", rj_code));
                            if let Some(m) = wp_maker {
                                maker = Some(m);
                            }
                            if let Some(g) = wp_genre {
                                genre = Some(g);
                            }
                            if image.is_none() {
                                image = wp_img;
                            }

                            results.push(ScrapedGame {
                                id: format!("dlsite_kor_{}", rj_code),
                                source: "DLsite (KOR)".to_string(),
                                name: title,
                                desc,
                                cover_url: image,
                                cover_size: None,
                                video_url: None,
                                video_size: None,
                                releasedate: regist_date,
                                developer: maker,
                                publisher: Some("DLsite".to_string()),
                                genre,
                                rating,
                                players: Some("1".to_string()),
                            });
                            return Ok(results);
                        }
                    }
                }
            }
        }
    }

    // 2. Keyword search in Korean locale (home & maniax)
    let search_urls = [
        format!("https://www.dlsite.com/home/fsr/=/language/ko/keyword/{}", urlencoding::encode(query)),
        format!("https://www.dlsite.com/maniax/fsr/=/language/ko/keyword/{}", urlencoding::encode(query)),
        format!("https://www.dlsite.com/home/fsr/=/options/KO_KR/keyword/{}", urlencoding::encode(query)),
        format!("https://www.dlsite.com/maniax/fsr/=/options/KO_KR/keyword/{}", urlencoding::encode(query)),
        format!("https://www.dlsite.com/home/fsr/=/keyword/{}", urlencoding::encode(query)),
        format!("https://www.dlsite.com/maniax/fsr/=/keyword/{}", urlencoding::encode(query)),
    ];

    for s_url in &search_urls {
        if let Ok(resp) = client.get(s_url).send().await {
            if resp.status().is_success() {
                if let Ok(html) = resp.text().await {
                    let mut start_idx = 0;
                    while let Some(pid_pos) = html[start_idx..].find("data-product_id=\"") {
                        let actual_pos = start_idx + pid_pos + 17;
                        if let Some(end_pid) = html[actual_pos..].find('"') {
                            let pid = &html[actual_pos..actual_pos + end_pid];
                            start_idx = actual_pos + end_pid;

                            if results.iter().any(|r| r.id == format!("dlsite_kor_{}", pid)) {
                                continue;
                            }

                            let chunk_end = (start_idx + 1500).min(html.len());
                            let chunk = &html[start_idx..chunk_end];

                            let title = if let Some(t_pos) = chunk.find("class=\"work_name\"") {
                                let after = &chunk[t_pos..];
                                if let Some(a_start) = after.find("title=\"") {
                                    let t_val_start = a_start + 7;
                                    if let Some(t_val_end) = after[t_val_start..].find('"') {
                                        after[t_val_start..t_val_start + t_val_end].to_string()
                                    } else { pid.to_string() }
                                } else { pid.to_string() }
                            } else { pid.to_string() };

                            let mut img_url = extract_dlsite_image_from_html(chunk);

                            let mut maker = if let Some(m_pos) = chunk.find("class=\"maker_name\"") {
                                let after = &chunk[m_pos..];
                                if let Some(tag_end) = after.find('>') {
                                    let text_part = &after[tag_end + 1..];
                                    if let Some(close_tag) = text_part.find('<') {
                                        let clean_m = text_part[..close_tag].trim();
                                        if !clean_m.is_empty() { Some(clean_m.to_string()) } else { None }
                                    } else { None }
                                } else { None }
                            } else { None };

                            let mut genre = if let Some(g_pos) = chunk.find("class=\"work_category") {
                                let after = &chunk[g_pos..];
                                if let Some(a_start) = after.find("<a ") {
                                    let after_a = &after[a_start..];
                                    if let Some(tag_end) = after_a.find('>') {
                                        let text_part = &after_a[tag_end + 1..];
                                        if let Some(close_tag) = text_part.find('<') {
                                            let clean_g = text_part[..close_tag].trim();
                                            if !clean_g.is_empty() { Some(clean_g.to_string()) } else { None }
                                        } else { None }
                                    } else { None }
                                } else { None }
                            } else { None };

                            let mut desc = format!("[{}] DLsite (한국어)", pid);
                            if results.len() < 3 {
                                let (_t, wp_desc, wp_maker, wp_genre, wp_img) = fetch_dlsite_work_page(&client, pid).await;
                                if let Some(d) = wp_desc { desc = d; }
                                if let Some(m) = wp_maker { maker = Some(m); }
                                if let Some(g) = wp_genre { genre = Some(g); }
                                if img_url.is_none() { img_url = wp_img; }
                            }

                            if img_url.is_none() {
                                let endpoints = [
                                    format!("https://www.dlsite.com/home/product/info/ajax?product_id={}", pid),
                                    format!("https://www.dlsite.com/maniax/product/info/ajax?product_id={}", pid),
                                ];
                                for ep in &endpoints {
                                    if let Ok(resp) = client.get(ep).send().await {
                                        if resp.status().is_success() {
                                            if let Ok(json) = resp.json::<Value>().await {
                                                if let Some(work) = json.get(pid) {
                                                    if let Some(w_img) = work["work_image"].as_str() {
                                                        let full_img = if w_img.starts_with("//") {
                                                            format!("https:{}", w_img)
                                                        } else {
                                                            w_img.to_string()
                                                        };
                                                        img_url = Some(full_img);
                                                    }
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            results.push(ScrapedGame {
                                id: format!("dlsite_kor_{}", pid),
                                source: "DLsite (KOR)".to_string(),
                                name: title,
                                desc,
                                cover_url: img_url,
                                cover_size: None,
                                video_url: None,
                                video_size: None,
                                releasedate: None,
                                developer: maker,
                                publisher: Some("DLsite".to_string()),
                                genre,
                                rating: None,
                                players: Some("1".to_string()),
                            });

                            if results.len() >= 6 {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    if !results.is_empty() {
                        break;
                    }
                }
            }
        }
    }

    Ok(results)
}


/// Download image bytes from URL
pub async fn download_image(url: &str) -> Result<(Vec<u8>, String), String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("HTTP 클라이언트 생성 실패: {}", e))?;

    let resp = client.get(url).send().await.map_err(|e| format!("이미지 다운로드 실패: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("이미지 다운로드 오류 (HTTP {})", resp.status()));
    }

    // Determine extension from content-type or url
    let content_type = resp.headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let ext = if content_type.contains("image/jpeg") || url.ends_with(".jpg") || url.ends_with(".jpeg") {
        "jpg"
    } else if content_type.contains("image/webp") || url.ends_with(".webp") {
        "webp"
    } else {
        "png"
    };

    let bytes = resp.bytes().await.map_err(|e| format!("바이트 읽기 실패: {}", e))?;
    Ok((bytes.to_vec(), ext.to_string()))
}

/// Download video bytes from URL with 35s timeout
pub async fn download_video(url: &str) -> Result<(Vec<u8>, String), String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(35))
        .build()
        .map_err(|e| format!("HTTP 클라이언트 생성 실패: {}", e))?;

    let resp = client.get(url).send().await.map_err(|e| format!("비디오 다운로드 실패: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("비디오 다운로드 오류 (HTTP {})", resp.status()));
    }

    let content_type = resp.headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let ext = if content_type.contains("video/webm") || url.ends_with(".webm") {
        "webm"
    } else {
        "mp4"
    };

    let bytes = resp.bytes().await.map_err(|e| format!("비디오 바이트 읽기 실패: {}", e))?;
    Ok((bytes.to_vec(), ext.to_string()))
}

/// Helper to query HTTP Content-Length (file size) using HEAD request with short 3s timeout
pub async fn fetch_content_length(url: &str) -> Option<u64> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .ok()?;

    if let Ok(resp) = client.head(url).send().await {
        if resp.status().is_success() {
            if let Some(len) = resp.content_length() {
                if len > 0 {
                    return Some(len);
                }
            }
        }
    }
    None
}

/// Concurrently query cover_size and video_size for all scraped games using tokio JoinSet
pub async fn enrich_media_sizes(games: &mut [ScrapedGame]) {
    let mut set = tokio::task::JoinSet::new();

    for (idx, game) in games.iter().enumerate() {
        let cover_url = game.cover_url.clone();
        let video_url = game.video_url.clone();

        set.spawn(async move {
            let c_size = if let Some(ref u) = cover_url {
                fetch_content_length(u).await
            } else {
                None
            };
            let v_size = if let Some(ref u) = video_url {
                fetch_content_length(u).await
            } else {
                None
            };
            (idx, c_size, v_size)
        });
    }

    while let Some(res) = set.join_next().await {
        if let Ok((idx, c_size, v_size)) = res {
            if idx < games.len() {
                games[idx].cover_size = c_size;
                games[idx].video_size = v_size;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_html_description() {
        let raw = "<p>First line.<br>Second line &amp; &quot;quote&quot;.</p><p>Third line.</p>";
        let cleaned = clean_html_description(raw);
        assert_eq!(cleaned, "First line.\nSecond line & \"quote\".\n\nThird line.");
    }

    #[test]
    fn test_extract_dlsite_description_work_parts() {
        let html = r#"
        <div class="header">Game Info</div>
        <div itemprop="description" class="work_parts_container">
            <div class="work_parts type_text">
                <div class="work_parts_area">
                    <p>An indie adventure game.<br />Made with RPG Maker.<br />Features multiple branching endings.</p>
                </div>
            </div>
        </div>
        "#;
        let desc = extract_dlsite_description(html);
        assert!(desc.is_some());
        assert_eq!(
            desc.unwrap(),
            "An indie adventure game.\nMade with RPG Maker.\nFeatures multiple branching endings."
        );
    }

    #[test]
    fn test_extract_dlsite_description_meta_fallback() {
        let html = r#"
        <meta property="og:description" content="A thrilling dungeon crawler game.&quot;DLsite Doujin R18&quot; is a download shop...">
        "#;
        let desc = extract_dlsite_description(html);
        assert!(desc.is_some());
        assert_eq!(desc.unwrap(), "A thrilling dungeon crawler game.");
    }

    #[tokio::test]
    async fn test_enrich_media_sizes_empty_and_dummy() {
        let mut games = vec![
            ScrapedGame {
                id: "test1".to_string(),
                source: "test".to_string(),
                name: "Test Game".to_string(),
                desc: "".to_string(),
                cover_url: None,
                cover_size: None,
                video_url: None,
                video_size: None,
                releasedate: None,
                developer: None,
                publisher: None,
                genre: None,
                rating: None,
                players: None,
            }
        ];
        enrich_media_sizes(&mut games).await;
        assert_eq!(games[0].cover_size, None);
        assert_eq!(games[0].video_size, None);
    }
}
