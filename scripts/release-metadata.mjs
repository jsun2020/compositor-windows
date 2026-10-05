import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

export function validateVersions(packageVersion, tauriVersion, cargoVersion, refType, refName) {
  if (!/^\d+\.\d+\.\d+$/.test(packageVersion)) throw new Error('A stable semantic version is required');
  if (packageVersion !== tauriVersion || packageVersion !== cargoVersion) throw new Error('Package, Tauri and Cargo versions must match');
  const tag = `v${packageVersion}`;
  if (refType === 'tag' ? refName !== tag : refName !== 'main') throw new Error('Release only from main or its matching version tag');
  return { version: packageVersion, tag };
}

export function projectVersion(root = process.cwd()) {
  const pkg = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8'));
  const tauri = JSON.parse(fs.readFileSync(path.join(root, 'src-tauri/tauri.conf.json'), 'utf8'));
  const workspace = fs.readFileSync(path.join(root, 'Cargo.toml'), 'utf8').match(/\[workspace\.package\]([\s\S]*?)(?:\n\[|$)/)?.[1];
  const cargo = workspace?.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
  return { pkg: pkg.version, tauri: tauri.version, cargo };
}

export async function releaseExists(response) {
  if (response.status === 404) return false;
  if (!response.ok) throw new Error(`Release lookup failed: HTTP ${response.status}`);
  const release = await response.json();
  if (release.draft) throw new Error('An existing draft must be inspected; it will not be overwritten');
  return true;
}

async function main() {
  const versions = projectVersion();
  const { version, tag } = validateVersions(versions.pkg, versions.tauri, versions.cargo, process.env.GITHUB_REF_TYPE, process.env.GITHUB_REF_NAME);
  const repository = process.env.GITHUB_REPOSITORY;
  if (!/^[\w.-]+\/[\w.-]+$/.test(repository ?? '')) throw new Error('Invalid repository');
  const response = await fetch(`${process.env.GITHUB_API_URL ?? 'https://api.github.com'}/repos/${repository}/releases/tags/${tag}`, {
    headers: { Accept: 'application/vnd.github+json', Authorization: `Bearer ${process.env.GH_TOKEN}`, 'X-GitHub-Api-Version': '2022-11-28' },
    signal: AbortSignal.timeout(30_000),
  });
  const exists = await releaseExists(response);
  if (!exists) {
    if (!fs.existsSync(`docs/releases/${version}.md`)) throw new Error('Versioned release notes are required');
    const lookup = spawnSync('git', ['rev-parse', '--verify', '--quiet', `refs/tags/${tag}^{commit}`], { encoding: 'utf8' });
    if (lookup.error || ![0, 1].includes(lookup.status)) throw new Error('Tag identity lookup failed');
    const local = lookup.status === 0 ? lookup.stdout.trim() : '';
    if (local && local !== process.env.GITHUB_SHA) throw new Error('Existing tag points to a different source commit');
  }
  fs.appendFileSync(process.env.GITHUB_OUTPUT, `publish=${!exists}\nversion=${version}\ntag=${tag}\n`);
  console.log(exists ? `${tag} is already published; retained unchanged.` : `${tag} requires tests and a fresh portable build.`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main().catch(error => { console.error(error.message); process.exitCode = 1; });
