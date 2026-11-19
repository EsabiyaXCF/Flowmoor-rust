#!/usr/bin/env node
'use strict';

const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const { createHash } = require('node:crypto');
const { parseArgs } = require('node:util');
const root = path.resolve(__dirname, '../..');
const url = 'https://youtu.be/njX2bu-_Vw4';

function buildMatrix(referenceOnly = false, allQualities = false) {
  const cases = [];
  for (const auto of referenceOnly ? [false] : [false, true]) {
    for (const height of [1080, 2160]) {
      for (const video of auto ? ['h264', 'vp9', 'av1', 'auto'] : ['h264', 'vp9', 'av1']) {
        for (const audio of auto ? ['aac', 'opus', 'mp3', 'auto'] : ['aac', 'opus', 'mp3']) {
          if (auto && video !== 'auto' && audio !== 'auto') continue;
          for (const content of ['video+audio', 'video-only', 'audio-only']) {
            for (const ranged of [false, true]) {
              cases.push({ id: String(cases.length + 1).padStart(3, '0'), height, video, audio, content, ranged, reference: !auto });
            }
          }
        }
      }
    }
  }
  if (allQualities) {
    for (const height of [null, 1440, 720, 608, 480, 360, 240, 144]) {
      for (const video of ['h264', 'vp9', 'av1', 'auto']) {
        for (const audio of ['aac', 'opus', 'mp3', 'auto']) {
          for (const content of ['video+audio', 'video-only', 'audio-only']) {
            for (const ranged of [false, true]) {
              cases.push({ id: String(cases.length + 1).padStart(3, '0'), height, video, audio, content, ranged, reference: false });
            }
          }
        }
      }
    }
  }
  return cases;
}

async function main() {
  const { values } = parseArgs({ options: {
    list: { type: 'boolean' }, 'reference-only': { type: 'boolean' }, 'all-qualities': { type: 'boolean' },
    output: { type: 'string' }, resume: { type: 'string' },
    case: { type: 'string' }, 'from-case': { type: 'string', default: '1' }, 'to-case': { type: 'string' },
    jobs: { type: 'string', default: '2' }, 'case-timeout-ms': { type: 'string', default: '7200000' },
    'keep-success-files': { type: 'boolean' },
  } });
  assert(!(values['reference-only'] && values['all-qualities']), 'choose reference-only or all-qualities');
  const cases = buildMatrix(values['reference-only'], values['all-qualities']);
  const from = Number(values.case || values['from-case']);
  const to = Number(values.case || values['to-case'] || cases.length);
  const jobs = Number(values.jobs);
  const timeoutMs = Number(values['case-timeout-ms']);
  assert(Number.isInteger(from) && from >= 1 && Number.isInteger(to) && from <= to && to <= cases.length, 'invalid case range');
  assert(Number.isInteger(jobs) && jobs >= 1 && jobs <= 4, 'jobs must be 1–4');
  assert(Number.isInteger(timeoutMs) && timeoutMs >= 1000, 'invalid case timeout');
  const selected = cases.slice(from - 1, to);
  if (values.list) { for (const c of selected) console.log(JSON.stringify(c)); return; }
  const output = values.resume ? path.dirname(path.resolve(values.resume)) : path.resolve(values.output || path.join(root, 'test-output', `matrix-${new Date().toISOString().replace(/[:.]/g, '-')}`));
  const reportPath = values.resume ? path.resolve(values.resume) : path.join(output, 'matrix-report.json');
  fs.mkdirSync(output, { recursive: true });
  const binary = path.join(root, 'target/debug', `flowmoor-download${process.platform === 'win32' ? '.exe' : ''}`);
  const fingerprint = createHash('sha256').update(fs.readFileSync(binary)).update(fs.readFileSync(__filename)).update(fs.readFileSync(path.join(__dirname, 'test-flowmoor-complete-download.cjs'))).digest('hex');
  const report = values.resume ? JSON.parse(fs.readFileSync(reportPath, 'utf8')) : {
    schema: 1, url, startedAt: new Date().toISOString(), output, referenceCases: 108, cases, results: [],
    checks: ['Completed', 'single attempt', 'exact height', 'stream counts', 'codecs', 'full duration ±3s / range duration ±0.25s', 'AV start difference <0.25s'],
    limits: [values['all-qualities'] ? 'all current qualities of the fixed benchmark source, plus best' : '1080p/2160p as in original Flowmoor', 'default container only; no YouTube tuning options', 'no independent network-speed enforcement'],
  };
  assert.equal(report.schema, 1);
  assert.equal(report.url, url);
  assert.deepEqual(report.cases, cases, 'resume with the same matrix selection');
  let stopping = false;
  const children = new Set();
  const stop = () => { stopping = true; for (const child of children) child.kill('SIGINT'); };
  process.on('SIGINT', stop);
  process.on('SIGTERM', stop);
  const save = () => {
    const results = report.results.filter(r => r.fingerprint === fingerprint && selected.some(c => c.id === r.id));
    report.updatedAt = new Date().toISOString();
    report.fingerprint = fingerprint;
    report.summary = {
      selected: selected.length, passed: results.filter(r => r.status === 'passed').length,
      failed: results.filter(r => r.status === 'failed').length,
      pending: selected.length - results.length,
      failures: results.filter(r => r.status === 'failed').map(r => r.id),
    };
    fs.writeFileSync(`${reportPath}.tmp`, JSON.stringify(report, null, 2));
    fs.renameSync(`${reportPath}.tmp`, reportPath);
  };
  save();
  console.log(`Matrix ${selected.length} cases (${cases.length} total), ${jobs} workers: ${reportPath}`);
  let cursor = 0;
  async function worker() {
    while (!stopping && cursor < selected.length) {
      const c = selected[cursor++];
      if (report.results.some(r => r.id === c.id && r.fingerprint === fingerprint && r.status === 'passed')) continue;
      const directory = path.join(output, `case-${c.id}`);
      const mediaDirectory = path.join(directory, 'media');
      fs.mkdirSync(directory, { recursive: true });
      const childReportPath = path.join(directory, 'download-report.json');
      const logPath = path.join(directory, 'download.log');
      const args = [path.join(__dirname, 'test-flowmoor-complete-download.cjs'), url,
        '--video-codec', c.video, '--audio-codec', c.audio, '--content', c.content,
        '--filename', `矩陣 case-${c.id}`, '--output', mediaDirectory, '--report', childReportPath,
        '--expected-duration', '126', '--exact-height', '--timeout-ms', String(timeoutMs)];
      if (c.height != null) args.push('--height', String(c.height));
      if (c.ranged) args.push('--start', '30', '--end', '45');
      const label = `${c.id} ${c.height == null ? 'best' : `${c.height}p`} ${c.video}/${c.audio} ${c.content} ${c.ranged ? '30–45s' : 'full'}`;
      console.log(`START ${label}`);
      const log = fs.createWriteStream(logPath);
      const started = Date.now();
      const child = spawn(process.execPath, args, { cwd: root, stdio: ['ignore', 'pipe', 'pipe'], env: process.env });
      children.add(child);
      child.stdout.pipe(log, { end: false }); child.stderr.pipe(log, { end: false });
      let spawnError;
      const exitCode = await new Promise(resolve => {
        child.once('error', error => { spawnError = error.message; resolve(null); });
        child.once('close', resolve);
      });
      children.delete(child);
      await new Promise(resolve => log.end(resolve));
      const childReport = fs.existsSync(childReportPath) ? JSON.parse(fs.readFileSync(childReportPath, 'utf8')) : null;
      const last = childReport?.snapshots.at(-1);
      const errors = [...(childReport?.errors || [])];
      if (spawnError) errors.push(spawnError);
      if (exitCode !== 0) errors.push(`exit ${exitCode}`);
      if (last?.phase !== 'Completed') errors.push(`phase ${last?.phase || 'missing'}: ${last?.error || ''}`);
      if (last?.attempt !== 1) errors.push(`attempt ${last?.attempt || 'missing'}`);
      if (!childReport?.media) errors.push('missing FFprobe result');
      if (c.height == null && c.content !== 'audio-only' && childReport?.media?.streams.find(s => s.codec_type === 'video')?.height !== 2160) errors.push('best quality did not preserve 2160p');
      const status = errors.length ? 'failed' : 'passed';
      const result = { ...c, fingerprint, status, elapsedMs: Date.now() - started, errors, childReportPath, logPath,
        media: childReport?.media, files: last?.files || [], command: [process.execPath, ...args] };
      report.results = report.results.filter(r => r.id !== c.id);
      report.results.push(result);
      if (status === 'passed' && !values['keep-success-files']) fs.rmSync(mediaDirectory, { recursive: true, force: true });
      save();
      console.log(`${status.toUpperCase()} ${label} (${(result.elapsedMs / 1000).toFixed(1)}s) ${errors.join('; ')}`);
    }
  }
  await Promise.all(Array.from({ length: jobs }, worker));
  process.removeListener('SIGINT', stop); process.removeListener('SIGTERM', stop);
  save();
  console.log(JSON.stringify(report.summary));
  console.log(reportPath);
  if (report.summary.failed || report.summary.pending) process.exitCode = 1;
}

if (require.main === module) main().catch(error => { console.error(error); process.exitCode = 1; });
module.exports = { buildMatrix };
