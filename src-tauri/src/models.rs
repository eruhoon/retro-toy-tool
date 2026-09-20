use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceProfile {
    pub id: String,
    pub name: String,
    pub device_type: String, // "knulli", "rocknix", "batocera", "retropie", "custom"
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub roms_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemPlatform {
    pub id: String,          // e.g. "snes", "gba", "psx"
    pub name: String,        // display friendly name
    pub rom_count: usize,    // total files in rom folder
    pub metadata_count: usize, // entries in gamelist.xml
    pub unregistered_count: usize, // files not in gamelist.xml
    pub missing_image_count: usize, // items without image
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GameStatus {
    Registered,   // Exists in both gamelist.xml and filesystem
    Unregistered, // Exists in filesystem, but NOT in gamelist.xml
    Missing,      // Exists in gamelist.xml, but file is missing from filesystem
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameItem {
    pub path: String,              // e.g. "./Super Mario World (USA).zip"
    pub filename: String,          // e.g. "Super Mario World (USA).zip"
    pub name: String,
    pub desc: String,
    pub image: Option<String>,     // relative or absolute path to image
    pub thumbnail: Option<String>,
    pub video: Option<String>,
    pub rating: Option<f32>,
    pub releasedate: Option<String>, // format: YYYYMMDDT000000
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub genre: Option<String>,
    pub players: Option<String>,
    pub favorite: bool,
    pub hidden: bool,
    pub status: GameStatus,
    pub file_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorageLocation {
    pub id: String,              // e.g. "external_share", "internal"
    pub label: String,           // e.g. "외장 SD카드 (TF2)", "내장 메모리 (TF1)"
    pub path: String,            // e.g. "/media/SHARE/roms", "/userdata/roms"
    pub storage_type: String,    // "external" | "internal"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTestResult {
    pub success: bool,
    pub message: String,
    pub detected_roms_path: Option<String>,
    pub os_name: Option<String>,
    pub storages: Vec<StorageLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapedGame {
    pub id: String,
    pub source: String, // "screenscraper", "wikipedia", "rawg", "openretro"
    pub name: String,
    pub desc: String,
    pub cover_url: Option<String>,
    pub cover_size: Option<u64>,
    pub video_url: Option<String>,
    pub video_size: Option<u64>,
    pub releasedate: Option<String>,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub genre: Option<String>,
    pub rating: Option<f32>,
    pub players: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenScraperCredentials {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenScraperAccountStatus {
    pub valid: bool,
    pub message: String,
    pub user_id: Option<String>,
    pub requests_today: Option<u32>,
    pub max_requests_per_day: Option<u32>,
}

