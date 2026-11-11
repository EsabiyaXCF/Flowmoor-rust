#!/usr/bin/env node
const { spawn } = require('node:child_process');
const { existsSync, mkdirSync, writeFileSync } = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const root = path.resolve(__dirname, '../..');
async function main() {
  const argv = process.argv.slice(2);
  const url = argv.shift();
  if (!url) throw new Error('Usage: node tests/scripts/test-flowmoor-complete-download.cjs URL [--start TIME --end TIME] [--live-from-start] [--cookies FILE]');
  const output = path.join(root, 'test-output');
  mkdirSync(output, { recursive: true });
  const reportPath = path.join(output, `report-${Date.now()}-${process.pid}.json`);
  const env = { ...process.env };
  if (!env.FLOWMOOR_YTDLP) env.FLOWMOOR_YTDLP = (process.platform === 'win32' ? ['.tools/yt-dlp.exe'] : ['.tools/yt-dlp-macos/yt-dlp', '.tools/yt-dlp']).map(p => path.join(root,p)).find(existsSync) || 'yt-dlp';
  const report = { url, options: argv, startedAt: new Date().toISOString(), snapshots: [] };
  const child = spawn(path.join(root, `target/debug/flowmoor-download${process.platform === 'win32' ? '.exe' : ''}`), [url, output, '--retries', '0', '--cancel-on-stdin', ...argv], { env, stdio: ['pipe', 'pipe', 'inherit'] });
  // Let the CLI reap its process tree on every platform instead of killing only its root.
  const timeout = setTimeout(() => child.stdin.end(), 120_000);
  let pending = '';
  child.stdout.setEncoding('utf8').on('data', chunk => {
    pending += chunk;
    let end;
    while ((end = pending.indexOf('\n')) >= 0) {
      const line = pending.slice(0, end); pending = pending.slice(end + 1);
      if (line) { const snapshot = JSON.parse(line); report.snapshots.push(snapshot); console.log(`${snapshot.phase} ${snapshot.downloaded} bytes`); }
    }
  });
  const exit = await new Promise((resolve, reject) => { child.once('error', reject); child.once('exit', resolve); }).finally(() => clearTimeout(timeout));
  report.exitCode = exit;
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
    const option = key => argv.includes(key) ? argv[argv.indexOf(key) + 1] : null;
    const content = option('--content') || 'video+audio';
    const video = media.streams.find(s => s.codec_type === 'video');
    const audio = media.streams.find(s => s.codec_type === 'audio');
    assert.equal(!!video, content !== 'audio-only', 'unexpected video stream');
    assert.equal(!!audio, content !== 'video-only', 'unexpected audio stream');
    if (video && option('--height')) assert(video.height <= Number(option('--height')), 'height exceeds requested cap');
    if (video && option('--video-codec') && option('--video-codec') !== 'auto') assert.equal(video.codec_name, option('--video-codec'));
    if (audio && option('--audio-codec') && option('--audio-codec') !== 'auto') assert.equal(audio.codec_name, option('--audio-codec'));
    report.media = media;
    writeFileSync(reportPath, JSON.stringify(report, null, 2));
    assert(Number(media.format.duration) > 0, 'empty output');
    const time = value => value.split(':').reduce((total, part) => total * 60 + Number(part), 0);
    if (argv.includes('--start') && argv.includes('--end')) {
      const expected = time(argv[argv.indexOf('--end') + 1]) - time(argv[argv.indexOf('--start') + 1]);
      assert(Math.abs(Number(media.format.duration) - expected) < 0.25, 'unexpected time-range duration');
      if (video && audio) assert(Math.abs(Number(video.start_time || 0) - Number(audio.start_time || 0)) < 0.25, 'video/audio start times diverge');
    }
    console.log(`Verified ${file}: ${media.format.duration}s`);
  }
}
main().catch(error => { console.error(error.message); process.exitCode = 1; });
