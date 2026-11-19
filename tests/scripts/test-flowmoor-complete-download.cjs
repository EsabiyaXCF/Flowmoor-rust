#!/usr/bin/env node
const { spawn } = require('node:child_process');
const { existsSync, mkdirSync, writeFileSync } = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const root = path.resolve(__dirname, '../..');
function validateMedia(media, argv, expectedDuration, exactHeight) {
  const option = key => argv.includes(key) ? argv[argv.indexOf(key) + 1] : null;
  const content = option('--content') || 'video+audio';
  const video = media.streams.filter(s => s.codec_type === 'video');
  const audio = media.streams.filter(s => s.codec_type === 'audio');
  const errors = [];
  const container = option('--container');
  if (content !== 'audio-only' && (container === 'mp4' || container === 'mkv')) {
    const format = container === 'mp4' ? 'mp4' : 'matroska';
    if (!media.format.format_name.split(',').includes(format) || path.extname(media.format.filename).toLowerCase() !== '.' + container) {
      errors.push(`container: requested ${container}, got ${media.format.format_name} (${media.format.filename})`);
    }
  }
  if (video.length !== (content === 'audio-only' ? 0 : 1)) errors.push('unexpected video stream count');
  if (audio.length !== (content === 'video-only' ? 0 : 1)) errors.push('unexpected audio stream count');
  if (video[0] && option('--height')) {
    const height = Number(option('--height'));
    if (exactHeight ? video[0].height !== height : video[0].height > height) errors.push(`height: requested ${height}, got ${video[0].height}`);
  }
  for (const [stream, flag] of [[video[0], '--video-codec'], [audio[0], '--audio-codec']]) {
    const codec = option(flag);
    if (stream && codec && codec !== 'auto' && stream.codec_name !== codec) errors.push(`${flag}: requested ${codec}, got ${stream.codec_name}`);
  }
  const duration = Number(media.format.duration);
  if (!(duration > 0)) errors.push('empty output');
  const time = value => value.split(':').reduce((total, part) => total * 60 + Number(part), 0);
  const ranged = argv.includes('--start') && argv.includes('--end');
  const expected = ranged ? time(option('--end')) - time(option('--start')) : expectedDuration;
  const tolerance = ranged ? 0.25 : 3;
  if (expected != null && Math.abs(duration - expected) >= tolerance) errors.push(`duration: expected ${expected}, got ${duration}`);
  if (video[0] && audio[0] && Math.abs(Number(video[0].start_time || 0) - Number(audio[0].start_time || 0)) >= 0.25) errors.push('video/audio start times diverge');
  return errors;
}
async function main() {
  const argv = process.argv.slice(2);
  const url = argv.shift();
  if (!url) throw new Error('Usage: node tests/scripts/test-flowmoor-complete-download.cjs URL [--start TIME --end TIME] [--container auto|mp4|mkv] [--live-from-start] [--cookies FILE]');
  let output = path.join(root, 'test-output');
  let reportPath;
  let timeoutMs = 120_000;
  let expectedDuration = null;
  let exactHeight = false;
  for (let i = 0; i < argv.length;) {
    const flag = argv[i];
    if (flag === '--exact-height') { exactHeight = true; argv.splice(i, 1); }
    else if (['--output', '--report', '--timeout-ms', '--expected-duration'].includes(flag)) {
      const value = argv[i + 1];
      assert(value, `missing ${flag}`);
      if (flag === '--output') output = path.resolve(value);
      if (flag === '--report') reportPath = path.resolve(value);
      if (flag === '--timeout-ms') { timeoutMs = Number(value); assert(Number.isInteger(timeoutMs) && timeoutMs > 0, 'invalid timeout'); }
      if (flag === '--expected-duration') { expectedDuration = Number(value); assert(Number.isFinite(expectedDuration) && expectedDuration > 0, 'invalid duration'); }
      argv.splice(i, 2);
    } else i += 1;
  }
  mkdirSync(output, { recursive: true });
  reportPath ||= path.join(output, `report-${Date.now()}-${process.pid}.json`);
  mkdirSync(path.dirname(reportPath), { recursive: true });
  const env = { ...process.env };
  if (!env.FLOWMOOR_YTDLP) env.FLOWMOOR_YTDLP = (process.platform === 'win32' ? ['.tools/yt-dlp.exe'] : ['.tools/yt-dlp-macos/yt-dlp', '.tools/yt-dlp']).map(p => path.join(root,p)).find(existsSync) || 'yt-dlp';
  const report = { url, options: argv, expectedDuration, exactHeight, startedAt: new Date().toISOString(), snapshots: [], errors: [] };
  writeFileSync(reportPath, JSON.stringify(report, null, 2));
  const child = spawn(path.join(root, `target/debug/flowmoor-download${process.platform === 'win32' ? '.exe' : ''}`), [url, output, '--retries', '0', '--cancel-on-stdin', ...argv], { env, stdio: ['pipe', 'pipe', 'inherit'] });
  const cancel = () => { report.interrupted = true; child.stdin.end(); };
  process.on('SIGINT', cancel); process.on('SIGTERM', cancel);
  // Let the CLI reap its process tree on every platform instead of killing only its root.
  const timeout = setTimeout(() => { report.timedOut = true; child.stdin.end(); }, timeoutMs);
  let pending = '';
  child.stdout.setEncoding('utf8').on('data', chunk => {
    pending += chunk;
    let end;
    while ((end = pending.indexOf('\n')) >= 0) {
      const line = pending.slice(0, end); pending = pending.slice(end + 1);
      if (line) { const snapshot = JSON.parse(line); report.snapshots.push(snapshot); console.log(`${snapshot.phase} ${snapshot.downloaded} bytes`); }
    }
  });
  const exit = await new Promise((resolve, reject) => { child.once('error', reject); child.once('exit', resolve); }).finally(() => {
    clearTimeout(timeout); process.removeListener('SIGINT', cancel); process.removeListener('SIGTERM', cancel);
  });
  report.exitCode = exit;
  report.elapsedMs = Date.now() - Date.parse(report.startedAt);
  writeFileSync(reportPath, JSON.stringify(report, null, 2));
  assert.equal(exit, 0, `download failed; see ${reportPath}`);
  const last = report.snapshots.at(-1);
  assert.equal(last.phase, 'Completed');
  assert(last.files.length > 0);
  for (const file of last.files) {
    let json = '';
    const probe = spawn(process.env.FLOWMOOR_FFPROBE || 'ffprobe', ['-v', 'error', '-show_format', '-show_streams', '-of', 'json', file]);
    probe.stdout.on('data', chunk => json += chunk);
    const code = await new Promise((resolve, reject) => { probe.once('error', reject); probe.once('exit', resolve); });
    assert.equal(code, 0, 'ffprobe failed');
    const media = JSON.parse(json);
    report.media = media;
    report.errors.push(...validateMedia(media, argv, expectedDuration, exactHeight));
    writeFileSync(reportPath, JSON.stringify(report, null, 2));
    console.log(`Verified ${file}: ${media.format.duration}s`);
  }
  assert.equal(report.errors.length, 0, report.errors.join('; '));
}
if (require.main === module) main().catch(error => { console.error(error.message); process.exitCode = 1; });
module.exports = { validateMedia };
