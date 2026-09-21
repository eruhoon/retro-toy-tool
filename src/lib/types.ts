export interface DeviceProfile {
  id: string;
  name: string;
  device_type: 'knulli' | 'rocknix' | 'batocera' | 'retropie' | 'custom';
  host: string;
  port: number;
  username: string;
  password: string;
  roms_path: string;
}

export interface SystemPlatform {
  id: string;
  name: string;
  rom_count: number;
  metadata_count: number;
  unregistered_count: number;
  missing_image_count: number;
}

export type GameStatus = 'registered' | 'unregistered' | 'missing';

export interface GameItem {
  path: string;
  filename: string;
  name: string;
  desc: string;
  image?: string | null;
  thumbnail?: string | null;
  video?: string | null;
  rating?: number | null;
  releasedate?: string | null;
  developer?: string | null;
  publisher?: string | null;
  genre?: string | null;
  players?: string | null;
  favorite: boolean;
  hidden: boolean;
  status: GameStatus;
  file_size: number;
}

export interface StorageLocation {
  id: string;
  label: string;
  path: string;
  storage_type: 'internal' | 'external';
}

export interface ConnectionTestResult {
  success: boolean;
  message: string;
  detected_roms_path?: string | null;
  os_name?: string | null;
  storages?: StorageLocation[];
}

export interface DevicePreset {
  id: 'knulli' | 'rocknix' | 'batocera' | 'retropie' | 'custom';
  name: string;
  defaultPort: number;
  defaultUser: string;
  defaultPass: string;
  defaultRomsPath: string;
  description: string;
}

export const DEVICE_PRESETS: DevicePreset[] = [
  {
    id: 'knulli',
    name: 'Knulli (ES-DE)',
    defaultPort: 22,
    defaultUser: 'root',
    defaultPass: 'linux',
    defaultRomsPath: '/userdata/roms',
    description: 'Anbernic RG35XX, RG40XX, TrimUI 등 ES-DE 탑재 기기'
  },
  {
    id: 'rocknix',
    name: 'ROCKNIX (ES-DE)',
    defaultPort: 22,
    defaultUser: 'root',
    defaultPass: 'rocknix',
    defaultRomsPath: '/storage/roms',
    description: 'Powkiddy RGB30, RK3566, x86 등 ES-DE 탑재 기기'
  },
  {
    id: 'batocera',
    name: 'Batocera Linux',
    defaultPort: 22,
    defaultUser: 'root',
    defaultPass: 'linux',
    defaultRomsPath: '/userdata/roms',
    description: 'PC, 라즈베리 파이, 핸드헬드 기기'
  },
  {
    id: 'retropie',
    name: 'RetroPie',
    defaultPort: 22,
    defaultUser: 'pi',
    defaultPass: 'raspberry',
    defaultRomsPath: '/home/pi/RetroPie/roms',
    description: 'Raspberry Pi 기반 EmulationStation'
  },
  {
    id: 'custom',
    name: '기타 / 사용자 정의',
    defaultPort: 22,
    defaultUser: 'root',
    defaultPass: '',
    defaultRomsPath: '/roms',
    description: '직접 ROM 경로와 계정을 설정'
  }
];

export interface ScrapedGame {
  id: string;
  source: string;
  name: string;
  desc: string;
  cover_url?: string | null;
  cover_size?: number | null;
  video_url?: string | null;
  video_size?: number | null;
  releasedate?: string | null;
  developer?: string | null;
  publisher?: string | null;
  genre?: string | null;
  rating?: number | null;
  players?: string | null;
}

export interface ScreenScraperAccountStatus {
  valid: boolean;
  message: string;
  user_id?: string | null;
  requests_today?: number | null;
  max_requests_per_day?: number | null;
}

export interface ScraperSettings {
  screenscraper_username: string;
  screenscraper_password: string;
  default_source: 'auto' | 'screenscraper' | 'steam' | 'dlsite' | 'wikipedia' | 'rawg';
}

export interface RomUploadProgressPayload {
  file_name: string;
  file_index: number;
  total_files: number;
  current_bytes: number;
  total_bytes: number;
  overall_current_bytes: number;
  overall_total_bytes: number;
  bytes_per_sec: number;
  is_finished: boolean;
}

export interface RomUploadResult {
  success_count: number;
  failed_files: string[];
  message: string;
}


