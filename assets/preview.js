(() => {
  if (window.top !== window) return;
  let playbackError = null;
  window.flowmoorPreview = {
    toggle() {
      const twitch = document.querySelector('[data-a-target="player-play-pause-button"]');
      if (twitch) { twitch.click(); return; }
      const v = document.querySelector('video');
      const p = document.getElementById('movie_player');
      if (p?.playVideo) {
        if (p.getPlayerState() === 1) p.pauseVideo(); else p.playVideo();
      } else if (v) {
        if (v.paused) { playbackError = null; if (v.ended) v.currentTime = 0; v.play().catch(error => { playbackError = error.message || '無法開始播放，請重新取得來源資訊。'; }); } else v.pause();
      } else {
        document.querySelector('.ytp-large-play-button, [data-a-target="player-play-pause-button"]')?.click();
      }
    },
    seek(time) {
      if (document.querySelector('.ad-showing')) return;
      const p = document.getElementById('movie_player');
      const v = document.querySelector('video');
      if (p?.seekTo) p.seekTo(time, true); else if (v) v.currentTime = time;
    }
  };
  let previous = '';
  let initialVolume = false;
  let contentTime = 0;
  let contentDuration = 0;
  setInterval(() => {
    const v = document.querySelector('video');
    const providerError = playbackError || document.querySelector('.ytp-error-content-wrap')?.textContent?.trim() || (v?.error ? '無法播放串流，請重新取得來源資訊。' : null);
    const loaded = !!v || !!document.getElementById('movie_player')?.playVideo;
    if (!loaded && !providerError) return;
    if (v && !initialVolume) { v.volume = 1; initialVolume = true; }
    const ad = !!document.querySelector('.ad-showing');
    if (!ad && v) { contentTime = v.currentTime; contentDuration = Number.isFinite(v.duration) ? v.duration : 0; }
    const state = JSON.stringify({
      time: contentTime,
      duration: contentDuration,
      playing: !!v && !v.paused && !v.ended,
      loaded,
      ready: !!v && v.readyState >= 2 && !ad,
      muted: v?.muted ?? false,
      volume: v?.volume ?? 1,
      error: providerError || (v?.error ? `播放器錯誤 ${v.error.code}` : null),
    });
    if (state !== previous) { previous = state; window.ipc.postMessage(state); }
  }, 250);
})();
