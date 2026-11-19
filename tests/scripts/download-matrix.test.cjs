const test = require('node:test');
const assert = require('node:assert/strict');
const { buildMatrix } = require('./test-flowmoor-complete-download-matrix.cjs');
const { validateMedia } = require('./test-flowmoor-complete-download.cjs');

test('original 108 cases are unchanged; auto adds 84 unique combinations', () => {
  const reference = buildMatrix(true), all = buildMatrix();
  assert.equal(reference.length, 108); assert.equal(all.length, 192);
  assert.deepEqual(all.slice(0, 108), reference);
  assert.equal(new Set(all.map(c => [c.height, c.video, c.audio, c.content, c.ranged].join('|'))).size, 192);
  assert.equal(reference[107].height, 2160); assert.equal(reference[107].audio, 'mp3');
  const full = buildMatrix(false, true);
  assert.equal(full.length, 960);
  assert.deepEqual(full.slice(0, 192), all);
  assert.equal(new Set(full.map(c => [c.height, c.video, c.audio, c.content, c.ranged].join('|'))).size, 960);
  assert.equal(new Set(full.map(c => c.height)).size, 10);
});

test('validation rejects downgraded quality, wrong codecs, missing streams and incomplete ranges', () => {
  const args = ['--height', '2160', '--content', 'video+audio', '--video-codec', 'h264', '--audio-codec', 'aac', '--start', '30', '--end', '45'];
  const media = { streams: [{ codec_type: 'video', codec_name: 'h264', height: 2160, start_time: '0' }, { codec_type: 'audio', codec_name: 'aac', start_time: '0.02' }], format: { duration: '15.02' } };
  assert.deepEqual(validateMedia(media, args, 126, true), []);
  const downgraded = structuredClone(media); downgraded.streams[0].height = 1080;
  assert.match(validateMedia(downgraded, args, 126, true).join(), /height/);
  const wrong = structuredClone(media); wrong.streams[1].codec_name = 'opus'; wrong.streams[0].start_time = '1'; wrong.format.duration = '14';
  assert.equal(validateMedia(wrong, args, 126, true).length, 3);
  assert.match(validateMedia({ ...media, streams: media.streams.slice(0, 1) }, args, 126, true).join(), /audio stream/);
  assert.deepEqual(validateMedia({ streams: [{ codec_type: 'audio', codec_name: 'mp3' }], format: { duration: '126.1' } }, ['--content', 'audio-only', '--audio-codec', 'mp3'], 126, true), []);
});

test('a Matroska file renamed to MP4 does not satisfy an MP4 request', () => {
  const args = ['--content', 'video-only', '--container', 'mp4'];
  const media = { streams: [{ codec_type: 'video', codec_name: 'h264' }], format: { filename: 'clip.mp4', duration: '5', format_name: 'matroska,webm' } };
  assert.match(validateMedia(media, args).join(), /container/);
  media.format.format_name = 'mov,mp4,m4a,3gp,3g2,mj2';
  assert.deepEqual(validateMedia(media, args), []);
});
