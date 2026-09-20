import { invoke } from '@tauri-apps/api/core';
import type {
  ConnectionTestResult,
  DeviceProfile,
  GameItem,
  ScrapedGame,
  ScreenScraperAccountStatus,
  StorageLocation,
  SystemPlatform,
} from './types';


export async function testConnection(
  host: string,
  port: number,
  username: string,
  password: string,
  candidatePath?: string
): Promise<ConnectionTestResult> {
  return await invoke<ConnectionTestResult>('test_connection', {
    host,
    port,
    username,
    password,
    candidatePath: candidatePath || null,
  });
}

export async function detectStorages(
  device: DeviceProfile,
  candidatePath?: string
): Promise<StorageLocation[]> {
  return await invoke<StorageLocation[]>('detect_storages_cmd', {
    host: device.host,
    port: device.port,
    username: device.username,
    password: device.password,
    candidatePath: candidatePath || device.roms_path || null,
  });
}

export async function getSystems(
  device: DeviceProfile,
  romsPathOverride?: string
): Promise<SystemPlatform[]> {
  return await invoke<SystemPlatform[]>('get_systems', {
    host: device.host,
    port: device.port,
    username: device.username,
    password: device.password,
    romsPath: romsPathOverride || device.roms_path,
  });
}

export async function getSystemGames(
  device: DeviceProfile,
  systemId: string,
  romsPathOverride?: string
): Promise<GameItem[]> {
  return await invoke<GameItem[]>('get_system_games', {
    host: device.host,
    port: device.port,
    username: device.username,
    password: device.password,
    romsPath: romsPathOverride || device.roms_path,
    systemId,
  });
}

export async function saveSystemGames(
  device: DeviceProfile,
  systemId: string,
  games: GameItem[],
  romsPathOverride?: string
): Promise<void> {
  return await invoke<void>('save_system_games_cmd', {
    host: device.host,
    port: device.port,
    username: device.username,
    password: device.password,
    romsPath: romsPathOverride || device.roms_path,
    systemId,
    games,
  });
}

export async function uploadGameImage(
  device: DeviceProfile,
  systemId: string,
  filename: string,
  imageBase64: string,
  romsPathOverride?: string
): Promise<string> {
  return await invoke<string>('upload_game_image_cmd', {
    host: device.host,
    port: device.port,
    username: device.username,
    password: device.password,
    romsPath: romsPathOverride || device.roms_path,
    systemId,
    filename,
    imageBase64,
  });
}

export async function uploadGameVideo(
  device: DeviceProfile,
  systemId: string,
  remoteFilename: string,
  videoBase64: string,
  romsPathOverride?: string
): Promise<string> {
  return await invoke<string>('upload_game_video_cmd', {
    host: device.host,
    port: device.port,
    username: device.username,
    password: device.password,
    romsPath: romsPathOverride || device.roms_path,
    systemId,
    remoteFilename,
    base64VideoData: videoBase64,
  });
}

export async function fetchRemoteImage(
  device: DeviceProfile,
  systemId: string,
  imageRelOrAbsPath: string,
  romsPathOverride?: string
): Promise<string> {
  return await invoke<string>('fetch_remote_image_cmd', {
    host: device.host,
    port: device.port,
    username: device.username,
    password: device.password,
    romsPath: romsPathOverride || device.roms_path,
    systemId,
    imageRelOrAbsPath,
  });
}

export async function fetchRemoteVideo(
  device: DeviceProfile,
  systemId: string,
  videoRelOrAbsPath: string,
  romsPathOverride?: string
): Promise<string> {
  return await invoke<string>('fetch_remote_video_cmd', {
    host: device.host,
    port: device.port,
    username: device.username,
    password: device.password,
    romsPath: romsPathOverride || device.roms_path,
    systemId,
    videoRelOrAbsPath,
  });
}

export async function searchGameMetadata(
  query: string,
  systemId?: string,
  scraperSettings?: { screenscraper_username: string; screenscraper_password: string },
  source?: string
): Promise<ScrapedGame[]> {
  const creds = (scraperSettings?.screenscraper_username && scraperSettings?.screenscraper_password)
    ? { username: scraperSettings.screenscraper_username, password: scraperSettings.screenscraper_password }
    : null;

  return await invoke<ScrapedGame[]>('search_game_metadata_cmd', {
    query,
    systemId: systemId || null,
    screenscraperCreds: creds,
    source: source || 'auto',
  });
}

export async function testScreenscraperAccount(
  username: string,
  password: string
): Promise<ScreenScraperAccountStatus> {
  return await invoke<ScreenScraperAccountStatus>('test_screenscraper_account_cmd', {
    username,
    password,
  });
}

export async function downloadAndUploadScrapedImage(
  device: DeviceProfile,
  systemId: string,
  romFilename: string,
  imageUrl: string,
  romsPathOverride?: string
): Promise<string> {
  return await invoke<string>('download_and_upload_scraped_image_cmd', {
    host: device.host,
    port: device.port,
    username: device.username,
    password: device.password,
    romsPath: romsPathOverride || device.roms_path,
    systemId,
    romFilename,
    imageUrl,
  });
}

export async function downloadAndUploadScrapedVideo(
  device: DeviceProfile,
  systemId: string,
  romFilename: string,
  videoUrl: string,
  romsPathOverride?: string
): Promise<string> {
  return await invoke<string>('download_and_upload_scraped_video_cmd', {
    host: device.host,
    port: device.port,
    username: device.username,
    password: device.password,
    romsPath: romsPathOverride || device.roms_path,
    systemId,
    romFilename,
    videoUrl,
  });
}

