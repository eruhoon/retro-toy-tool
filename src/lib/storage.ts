import type { DeviceProfile, ScraperSettings } from './types';

const STORAGE_KEY = 'toy_manager_profiles';
const ACTIVE_DEVICE_KEY = 'toy_manager_active_device_id';

export function loadProfiles(): DeviceProfile[] {
  if (typeof window === 'undefined') return [];
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    return JSON.parse(raw);
  } catch (e) {
    console.error('프로필 로드 실패:', e);
    return [];
  }
}

export function saveProfiles(profiles: DeviceProfile[]): void {
  if (typeof window === 'undefined') return;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(profiles));
  } catch (e) {
    console.error('프로필 저장 실패:', e);
  }
}

export function loadActiveDeviceId(): string | null {
  if (typeof window === 'undefined') return null;
  return localStorage.getItem(ACTIVE_DEVICE_KEY);
}

export function saveActiveDeviceId(id: string | null): void {
  if (typeof window === 'undefined') return;
  if (id) {
    localStorage.setItem(ACTIVE_DEVICE_KEY, id);
  } else {
    localStorage.removeItem(ACTIVE_DEVICE_KEY);
  }
}

const SCRAPER_SETTINGS_KEY = 'toy_manager_scraper_settings';


export const DEFAULT_SCRAPER_SETTINGS: ScraperSettings = {
  screenscraper_username: '',
  screenscraper_password: '',
  default_source: 'auto'
};

export function loadScraperSettings(): ScraperSettings {
  if (typeof window === 'undefined') return DEFAULT_SCRAPER_SETTINGS;
  try {
    const raw = localStorage.getItem(SCRAPER_SETTINGS_KEY);
    if (!raw) return DEFAULT_SCRAPER_SETTINGS;
    return { ...DEFAULT_SCRAPER_SETTINGS, ...JSON.parse(raw) };
  } catch (e) {
    console.error('스크래퍼 설정 로드 실패:', e);
    return DEFAULT_SCRAPER_SETTINGS;
  }
}

export function saveScraperSettings(settings: ScraperSettings): void {
  if (typeof window === 'undefined') return;
  try {
    localStorage.setItem(SCRAPER_SETTINGS_KEY, JSON.stringify(settings));
  } catch (e) {
    console.error('스크래퍼 설정 저장 실패:', e);
  }
}

