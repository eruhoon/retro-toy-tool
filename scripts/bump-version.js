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
  const match = clean.match(/^(\d+)\.(\d+)\.(\d+)(?:-(.+))?$/);
  if (!match) {
    throw new Error(`Invalid SemVer format: "${ver}"`);
  }
  return {
    major: parseInt(match[1], 10),
    minor: parseInt(match[2], 10),
    patch: parseInt(match[3], 10),
    prerelease: match[4] || ''
  };
}

function bumpVersion(type, current) {
  const sem = parseSemVer(current);
  switch (type) {
    case 'major':
      return `${sem.major + 1}.0.0`;
    case 'minor':
      return `${sem.major}.${sem.minor + 1}.0`;
    case 'patch':
      return `${sem.major}.${sem.minor}.${sem.patch + 1}`;
    default:
      // If a specific version string was passed directly (e.g. 0.2.0)
      if (/^\d+\.\d+\.\d+/.test(type)) {
        return type.replace(/^v/, '');
      }
      throw new Error(`Unknown bump type or version format: "${type}". Use 'major', 'minor', 'patch', or a explicit 'X.Y.Z'.`);
  }
}

function updatePackageJson(newVersion) {
  const content = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
  content.version = newVersion;
  fs.writeFileSync(packageJsonPath, JSON.stringify(content, null, 2) + '\n', 'utf8');
}

function updateTauriConf(newVersion) {
  const content = JSON.parse(fs.readFileSync(tauriConfPath, 'utf8'));
  content.version = newVersion;
  fs.writeFileSync(tauriConfPath, JSON.stringify(content, null, 2) + '\n', 'utf8');
}

function updateCargoToml(newVersion) {
  const content = fs.readFileSync(cargoTomlPath, 'utf8');
  // Match version = "0.1.0" under [package]
  const updated = content.replace(
    /(\[package\][\s\S]*?version\s*=\s*")[^"]+(")/,
    `$1${newVersion}$2`
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
