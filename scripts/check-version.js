import fs from 'node:fs';
import path from 'node:path';
import { execSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '..');

const packageJsonPath = path.join(rootDir, 'package.json');

function parseSemVer(ver) {
  if (!ver) return null;
  const clean = ver.trim().replace(/^v/, '');
  const match = clean.match(/^(\d+)\.(\d+)\.(\d+)(?:-(.+))?$/);
  if (!match) return null;
  return {
    raw: clean,
    major: parseInt(match[1], 10),
    minor: parseInt(match[2], 10),
    patch: parseInt(match[3], 10),
  };
}

function getPreviousVersion() {
  // 1. Try finding the latest semver git tag
  try {
    const tagsOutput = execSync('git tag --sort=-v:refname', { encoding: 'utf8', cwd: rootDir }).trim();
    const tags = tagsOutput.split('\n').map(t => t.trim()).filter(Boolean);
    for (const tag of tags) {
      const parsed = parseSemVer(tag);
      if (parsed) {
        return parsed;
      }
    }
  } catch (err) {
    // git tags might fail if shallow clone or no tags
  }

  // 2. Try git show HEAD~1:package.json
  try {
    const prevPkgStr = execSync('git show HEAD~1:package.json', { encoding: 'utf8', cwd: rootDir });
    const prevPkg = JSON.parse(prevPkgStr);
    const parsed = parseSemVer(prevPkg.version);
    if (parsed) {
      return parsed;
    }
  } catch (err) {
    // No previous commit (first commit)
  }

  // 3. Fallback to 0.0.0
  return { raw: '0.0.0', major: 0, minor: 0, patch: 0 };
}

function tagExists(tagName) {
  try {
    const out = execSync(`git tag -l "${tagName}"`, { encoding: 'utf8', cwd: rootDir }).trim();
    return out === tagName;
  } catch {
    return false;
  }
}

function setGitHubOutput(key, value) {
  const outputFile = process.env.GITHUB_OUTPUT;
  if (outputFile) {
    fs.appendFileSync(outputFile, `${key}=${value}\n`, 'utf8');
  }
  console.log(`[OUTPUT] ${key}=${value}`);
}

function main() {
  const pkg = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
  const current = parseSemVer(pkg.version);

  if (!current) {
    console.error(`Invalid package.json version: "${pkg.version}"`);
    process.exit(1);
  }

  const prev = getPreviousVersion();
  const tagName = `v${current.raw}`;
  const isAlreadyTagged = tagExists(tagName);

  console.log(`Current version: ${current.raw}`);
  console.log(`Previous base version: ${prev.raw}`);
  console.log(`Tag ${tagName} already exists: ${isAlreadyTagged}`);

  const isMajorBump = current.major > prev.major;
  const isMinorBump = current.major === prev.major && current.minor > prev.minor;
  const isPatchOnly = current.major === prev.major && current.minor === prev.minor && current.patch > prev.patch;

  let shouldRelease = false;
  let reason = '';

  if (isAlreadyTagged) {
    shouldRelease = false;
    reason = `Tag ${tagName} already exists. Skipping release to prevent duplicate.`;
  } else if (isMajorBump) {
    shouldRelease = true;
    reason = `Major version bump detected (${prev.raw} -> ${current.raw}). Triggering release!`;
  } else if (isMinorBump) {
    shouldRelease = true;
    reason = `Minor version bump detected (${prev.raw} -> ${current.raw}). Triggering release!`;
  } else if (isPatchOnly) {
    shouldRelease = false;
    reason = `Patch version bump only (${prev.raw} -> ${current.raw}). Minor-or-above required. Skipping release.`;
  } else {
    shouldRelease = false;
    reason = `No version increase detected (${prev.raw} vs ${current.raw}). Skipping release.`;
  }

  console.log(`Result: ${reason}`);

  setGitHubOutput('should_release', shouldRelease ? 'true' : 'false');
  setGitHubOutput('version', current.raw);
  setGitHubOutput('tag_name', tagName);
  setGitHubOutput('reason', reason);
}

main();
