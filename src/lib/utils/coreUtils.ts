import type { InstalledCore } from '../types';

/**
 * 주어진 에뮬레이터 코어가 현재 선택된 콘솔 플랫폼(기종 ID)과 일치하는지 여부를 검사합니다.
 */
export function isCoreMatchingSystem(core: InstalledCore, selectedSystemId: string | null): boolean {
  if (!selectedSystemId) return true;
  const sysId = selectedSystemId.toLowerCase().trim();

  // 1. es_systems.cfg 매핑 확인
  if (core.supported_systems.map((s) => s.toLowerCase()).includes(sysId)) {
    return true;
  }

  // 2. core id, core_name, file_name, system_name 확인
  const cid = core.id.toLowerCase();
  const cname = core.core_name.toLowerCase();
  const sname = core.system_name.toLowerCase();

  // 3. 기종별 정밀 매핑 휴리스틱
  switch (sysId) {
    case 'gba':
      return cid.includes('mgba') || cid.includes('gpsp') || cid.includes('vbam');
    case 'gb':
    case 'gbc':
      return cid.includes('gambatte') || cid.includes('mgba') || cid.includes('gearboy') || cid.includes('sameboy');
    case 'snes':
    case 'sfc':
      return cid.includes('snes') || cid.includes('bsnes') || cid.includes('mesen-s');
    case 'nes':
    case 'fc':
    case 'fds':
      return cid.includes('fceumm') || cid.includes('nestopia') || cid.includes('mesen') || cid.includes('quicknes');
    case 'n64':
      return cid.includes('mupen64') || cid.includes('parallel');
    case 'nds':
      return cid.includes('melonds') || cid.includes('desmume');
    case '3ds':
      return cid.includes('citra');
    case 'psx':
    case 'ps1':
      return (
        cid.includes('pcsx') ||
        cid.includes('duckstation') ||
        cid.includes('swanstation') ||
        cid.includes('beetle_psx') ||
        cid.includes('mednafen_psx')
      );
    case 'psp':
      return cid.includes('ppsspp');
    case 'megadrive':
    case 'genesis':
    case 'md':
    case 'segacd':
    case 'sega32x':
      return cid.includes('genesis_plus') || cid.includes('picodrive');
    case 'mastersystem':
    case 'gamegear':
    case 'sms':
    case 'gg':
    case 'sg1000':
    case 'sg-1000':
      return cid.includes('gearsystem') || cid.includes('genesis_plus') || cid.includes('picodrive');
    case 'saturn':
      return (
        cid.includes('yabasanshiro') ||
        cid.includes('beetle_saturn') ||
        cid.includes('kronos') ||
        cid.includes('mednafen_saturn')
      );
    case 'dreamcast':
    case 'naomi':
    case 'atomiswave':
      return cid.includes('flycast');
    case 'pce':
    case 'pcengine':
    case 'pcecd':
    case 'tg16':
    case 'tg-cd':
      return cid.includes('pce') || cid.includes('supergrafx');
    case 'supergrafx':
    case 'sgx':
      return cid.includes('supergrafx') || cid.includes('pce');
    case 'pcfx':
      return cid.includes('pcfx');
    case 'wswan':
    case 'wsc':
    case 'wonderswan':
    case 'wonderswancolor':
      return cid.includes('wswan');
    case 'ngp':
    case 'ngpc':
    case 'neogeopocket':
      return cid.includes('ngp') || cid.includes('race');
    case 'neogeo':
    case 'neogeocd':
    case 'neocd':
      return (
        cid.includes('neocd') ||
        cid.includes('geolith') ||
        cid.includes('fbneo') ||
        cid.includes('fba')
      );
    case 'fbneo':
    case 'fba':
      return cid.includes('fbneo') || cid.includes('fba');
    case 'mame':
    case 'arcade':
      return cid.includes('mame') || cid.includes('fbneo') || cid.includes('fba');
    case 'atari2600':
    case 'a2600':
      return cid.includes('stella');
    case 'atari7800':
    case 'a7800':
      return cid.includes('prosystem');
    case 'atarilynx':
    case 'lynx':
      return cid.includes('handy') || cid.includes('lynx');
    case 'atarist':
      return cid.includes('hatari');
    case 'msx':
    case 'msx1':
    case 'msx2':
      return cid.includes('fmsx') || cid.includes('bluemsx');
    case 'colecovision':
    case 'coleco':
      return cid.includes('gearcoleco') || cid.includes('bluemsx');
    case 'c64':
    case 'commodore64':
    case 'vic20':
      return cid.includes('vice');
    case 'amiga':
    case 'amiga500':
    case 'amiga1200':
    case 'amigacd32':
      return cid.includes('puae');
    case 'zxspectrum':
      return cid.includes('fuse');
    case 'dos':
    case 'pc':
      return cid.includes('dosbox');
    case 'scummvm':
      return cid.includes('scummvm');
    case '3do':
      return cid.includes('opera') || cid.includes('4do');
    case 'pokemini':
      return cid.includes('pokemini');
    case 'pico8':
      return cid.includes('retro8') || cid.includes('fake08') || cid.includes('fake-08');
    case 'tic80':
      return cid.includes('tic80');
    default:
      break;
  }

  // 시스템 이름이나 코어 이름에 sysId가 포함되어 있는지 확인
  if (sname.includes(sysId) || cname.includes(sysId)) return true;

  return false;
}

/**
 * 에뮬레이터 코어의 시스템 대분류 카테고리를 판별합니다.
 */
export function getSystemCategory(core: InstalledCore): 'nintendo' | 'sony' | 'sega' | 'arcade' | 'other' {
  const text = `${core.system_name} ${core.display_name} ${core.supported_systems.join(' ')}`.toLowerCase();
  if (
    text.includes('nintendo') ||
    text.includes('game boy') ||
    text.includes('gba') ||
    text.includes('snes') ||
    text.includes('nes') ||
    text.includes('n64') ||
    text.includes('ds') ||
    text.includes('famicom') ||
    text.includes('pokemini')
  ) {
    return 'nintendo';
  }
  if (
    text.includes('playstation') ||
    text.includes('sony') ||
    text.includes('psx') ||
    text.includes('psp') ||
    text.includes('ps1') ||
    text.includes('ps2')
  ) {
    return 'sony';
  }
  if (
    text.includes('sega') ||
    text.includes('genesis') ||
    text.includes('megadrive') ||
    text.includes('saturn') ||
    text.includes('dreamcast') ||
    text.includes('game gear') ||
    text.includes('master system')
  ) {
    return 'sega';
  }
  if (
    text.includes('arcade') ||
    text.includes('mame') ||
    text.includes('fbneo') ||
    text.includes('neogeo') ||
    text.includes('fba') ||
    text.includes('capcom')
  ) {
    return 'arcade';
  }
  return 'other';
}
