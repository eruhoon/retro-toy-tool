import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '..');

const packageJsonPath = path.join(rootDir, 'package.json');
const tauriConfPath = path.join(rootDir, 'src-tauri', 'tauri.conf.json');
const cargoTomlPath = path.join(rootDir, 'src-tauri', 'Cargo.toml');

function readPackageJson() {
  return JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
}

function parseSemVer(ver) {
  const clean = ver.replace(/^v/, '');
  const match = clean.match(/^(\d+)\.(\d+)\.(\d+)(?:\.(\d+))?(?:-(.+))?$/);
  if (!match) {
    throw new Error(`Invalid SemVer format: "${ver}"`);
  }
  return {
    major: parseInt(match[1], 10),
    minor: parseInt(match[2], 10),
    patch: parseInt(match[3], 10),
    revision: match[4] !== undefined ? parseInt(match[4], 10) : 0,
    prerelease: match[5] || ''
  };
}

function bumpVersion(type, current) {
  const sem = parseSemVer(current);
  switch (type) {
    case 'major':
      return `${sem.major + 1}.0.0.0`;
    case 'minor':
      return `${sem.major}.${sem.minor + 1}.0.0`;
    case 'patch':
      return `${sem.major}.${sem.minor}.${sem.patch + 1}.0`;
    case 'revision':
      return `${sem.major}.${sem.minor}.${sem.patch}.${sem.revision + 1}`;
    default:
      // If a specific version string was passed directly (e.g. 0.2.1.0 or 0.2.1)
      if (/^\d+\.\d+\.\d+/.test(type)) {
        const parts = type.replace(/^v/, '').split('.');
        while (parts.length < 4) {
          parts.push('0');
        }
        return parts.slice(0, 4).join('.');
      }
      throw new Error(`Unknown bump type or version format: "${type}". Use 'major', 'minor', 'patch', 'revision', or an explicit 'X.Y.Z.R'.`);
  }
}

function updatePackageJson(newVersion) {
  const content = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
  content.version = newVersion;
  fs.writeFileSync(packageJsonPath, JSON.stringify(content, null, 2) + '\n', 'utf8');
}

function updateTauriConf(newVersion) {
  const content = JSON.parse(fs.readFileSync(tauriConfPath, 'utf8'));
  // Tauri v2 strictly enforces 3-digit SemVer (X.Y.Z) in tauri.conf.json
  const semParts = newVersion.replace(/^v/, '').split('.').slice(0, 3).join('.');
  content.version = semParts;
  fs.writeFileSync(tauriConfPath, JSON.stringify(content, null, 2) + '\n', 'utf8');
}

function updateCargoToml(newVersion) {
  const content = fs.readFileSync(cargoTomlPath, 'utf8');
  // Cargo.toml only supports 3-digit semver (X.Y.Z)
  const semParts = newVersion.replace(/^v/, '').split('.').slice(0, 3).join('.');
  const updated = content.replace(
    /(\[package\][\s\S]*?version\s*=\s*")[^"]+(")/,
    `$1${semParts}$2`
  );
  fs.writeFileSync(cargoTomlPath, updated, 'utf8');
}

function main() {
  const target = process.argv[2];
  if (!target) {
    console.error('Usage: node scripts/bump-version.js <major|minor|patch|X.Y.Z>');
    process.exit(1);
  }

  const pkg = readPackageJson();
  const currentVersion = pkg.version;
  const newVersion = bumpVersion(target, currentVersion);

  console.log(`Bumping version: ${currentVersion} -> ${newVersion}`);

  updatePackageJson(newVersion);
  updateTauriConf(newVersion);
  updateCargoToml(newVersion);

  console.log('✅ Successfully updated:');
  console.log(` - ${packageJsonPath}`);
  console.log(` - ${tauriConfPath}`);
  console.log(` - ${cargoTomlPath}`);
}

main();
