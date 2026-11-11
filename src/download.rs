use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    io::{BufRead, BufReader},
    path::PathBuf,
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use url::Url;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum Content {
    #[default]
    VideoAudio,
    Video,
    Audio,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum VideoCodec {
    #[default]
    Auto,
    H264,
    Vp9,
    Av1,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum AudioCodec {
    #[default]
    Auto,
    Mp3,
    Aac,
    Opus,
}
impl VideoCodec {
    pub(crate) fn ffmpeg(self) -> (&'static str, &'static str) {
        match self {
            Self::Auto => ("", "copy"),
            Self::H264 => ("h264", "libx264"),
            Self::Vp9 => ("vp9", "libvpx-vp9"),
            Self::Av1 => ("av1", "libsvtav1"),
        }
    }
}
impl AudioCodec {
    pub(crate) fn ffmpeg(self) -> (&'static str, &'static str) {
        match self {
            Self::Auto => ("", "copy"),
            Self::Mp3 => ("mp3", "libmp3lame"),
            Self::Aac => ("aac", "aac"),
            Self::Opus => ("opus", "libopus"),
        }
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FormatOptions {
    pub content: Content,
    pub max_height: Option<u32>,
    pub filename: String,
    pub video_codec: VideoCodec,
    pub audio_codec: AudioCodec,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Request {
    pub url: String,
    pub output: PathBuf,
    pub range: Option<(u64, u64)>,
    pub live_from_start: bool,
    pub is_live: bool,
    pub retries: u8,
    pub cookies: Option<PathBuf>,
    pub format: FormatOptions,
}
impl Request {
    pub fn validate(&self) -> Result<(), String> {
        let url = Url::parse(&self.url).map_err(|_| "請輸入完整的 YouTube 或 Twitch 網址")?;
        let host = url.host_str().unwrap_or_default();
        let youtube = host == "youtu.be" || host == "youtube.com" || host.ends_with(".youtube.com");
        let twitch = host == "twitch.tv" || host.ends_with(".twitch.tv");
        if !matches!(url.scheme(), "http" | "https")
            || !(youtube || twitch)
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err("僅接受 YouTube／Twitch 的 HTTP(S) 網址".into());
        }
        if self.output.as_os_str().is_empty() {
            return Err("請選擇輸出資料夾".into());
        }
        if let Some((start, end)) = self.range
            && start >= end
        {
            return Err("結束時間必須大於開始時間".into());
        }
        if self.live_from_start && !youtube {
            return Err("從直播開頭下載目前僅提供 YouTube（需來源支援 DVR）".into());
        }
        if self.retries > 10 {
            return Err("重試次數必須介於 0–10".into());
        }
        if self.cookies.as_ref().is_some_and(|p| !p.is_file()) {
            return Err("Cookie 檔案不存在".into());
        }
        if self.format.filename.len() > 160
            || self.format.filename.contains(['/', '\\'])
            || matches!(self.format.filename.trim(), "." | "..")
        {
            return Err("檔案名稱不可包含路徑，且需少於 160 bytes".into());
        }
        let stem = self
            .format
            .filename
            .trim()
            .split('.')
            .next()
            .unwrap_or_default()
            .trim_end()
            .to_ascii_uppercase();
        if matches!(
            stem.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
        ) || stem
            .strip_prefix("COM")
            .or_else(|| stem.strip_prefix("LPT"))
            .is_some_and(|suffix| {
                matches!(
                    suffix,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
        {
            return Err("檔案名稱不可使用 Windows 保留的裝置名稱".into());
        }
        if self.format.max_height.is_some_and(|h| h == 0 || h > 16384) {
            return Err("影片品質超出支援範圍".into());
        }
        Ok(())
    }
}
pub fn parse_time(value: &str) -> Result<u64, String> {
    let parts: Vec<_> = value.trim().split(':').collect();
    if parts.is_empty() || parts.len() > 3 {
        return Err("時間格式為秒數、MM:SS 或 HH:MM:SS".into());
    }
    let mut total = 0u64;
    for (i, part) in parts.iter().enumerate() {
        let n: u64 = part.parse().map_err(|_| "時間必須是非負整數")?;
        if i > 0 && n >= 60 {
            return Err("分／秒欄位必須小於 60".into());
        }
        total = total
            .checked_mul(60)
            .and_then(|v| v.checked_add(n))
            .ok_or("時間過大")?;
    }
    Ok(total)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Phase {
    Pending,
    Queued,
    Paused,
    Starting,
    Downloading,
    Processing,
    Retrying,
    Completed,
    Failed,
    Cancelled,
    Stopped,
}
impl Phase {
    pub fn terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Cancelled | Self::Stopped
        )
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::Pending => "待開始",
            Self::Queued => "等待下載",
            Self::Paused => "已暫停",
            Self::Starting => "讀取來源",
            Self::Downloading => "下載中",
            Self::Processing => "合併／處理中",
            Self::Retrying => "等待重試",
            Self::Completed => "下載完成",
            Self::Failed => "下載失敗",
            Self::Cancelled => "已取消",
            Self::Stopped => "已停止，請確認輸出",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Snapshot {
    pub id: u128,
    pub url: String,
    pub title: String,
    pub phase: Phase,
    pub attempt: u8,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub speed: Option<f64>,
    pub eta: Option<u64>,
    pub files: Vec<PathBuf>,
    pub error: Option<String>,
    pub directory: PathBuf,
}
pub struct Job {
    pub request: Request,
    pub state: Mutex<Snapshot>,
    cancel: AtomicBool,
    stop: AtomicBool,
    paused: AtomicBool,
}
impl Job {
    pub fn snapshot(&self) -> Snapshot {
        self.state.lock().unwrap().clone()
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
        let mut s = self.state.lock().unwrap();
        if matches!(s.phase, Phase::Pending | Phase::Queued | Phase::Paused) {
            s.phase = Phase::Cancelled;
        }
    }
    pub fn set_title(&self, title: String) {
        self.state.lock().unwrap().title = title;
    }
    pub fn pause(&self) {
        if self.request.is_live {
            return;
        }
        self.paused.store(true, Ordering::Relaxed);
        let mut s = self.state.lock().unwrap();
        if s.phase == Phase::Queued {
            s.phase = Phase::Paused;
        }
    }
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}
#[derive(Clone)]
pub struct Tools {
    pub ytdlp: PathBuf,
    pub ffmpeg: PathBuf,
}
impl Default for Tools {
    fn default() -> Self {
        Self {
            ytdlp: std::env::var_os("FLOWMOOR_YTDLP")
                .unwrap_or_else(|| "yt-dlp".into())
                .into(),
            ffmpeg: std::env::var_os("FLOWMOOR_FFMPEG")
                .unwrap_or_else(|| "ffmpeg".into())
                .into(),
        }
    }
}
pub struct Manager {
    tx: Option<mpsc::Sender<()>>,
    pending: Arc<Mutex<VecDeque<Arc<Job>>>>,
    workers: Vec<thread::JoinHandle<()>>,
    pub jobs: Vec<Arc<Job>>,
    next_id: u128,
}
impl Manager {
    pub fn new(tools: Tools) -> Self {
        Self::with_parallelism(tools, 3)
    }
    pub fn with_parallelism(tools: Tools, count: usize) -> Self {
        let (tx, rx) = mpsc::channel::<()>();
        let receiver = Arc::new(Mutex::new(rx));
        let pending = Arc::new(Mutex::new(VecDeque::<Arc<Job>>::new()));
        let workers = (0..count.clamp(1, 4))
            .map(|_| {
                let queue = pending.clone();
                let receiver = receiver.clone();
                let tools = tools.clone();
                thread::spawn(move || {
                    loop {
                        let message = receiver.lock().unwrap().recv();
                        if message.is_err() {
                            break;
                        }
                        loop {
                            let job = {
                                let mut queue = queue.lock().unwrap();
                                queue
                                    .iter()
                                    .position(|j| !j.paused.load(Ordering::Relaxed))
                                    .and_then(|i| queue.remove(i))
                            };
                            let Some(job) = job else {
                                break;
                            };
                            {
                                let mut s = job.state.lock().unwrap();
                                if s.phase != Phase::Queued {
                                    continue;
                                }
                                s.phase = Phase::Starting;
                            }
                            run_job(&job, &tools);
                            let mut queue = queue.lock().unwrap();
                            if job.snapshot().phase == Phase::Paused
                                && !queue.iter().any(|j| Arc::ptr_eq(j, &job))
                            {
                                queue.push_back(job);
                            }
                        }
                    }
                })
            })
            .collect();
        Self {
            tx: Some(tx),
            pending,
            workers,
            jobs: Vec::new(),
            next_id: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
        }
    }
    pub fn enqueue(&mut self, request: Request) -> Result<Arc<Job>, String> {
        self.insert(request, true)
    }
    pub fn enqueue_pending(&mut self, request: Request) -> Result<Arc<Job>, String> {
        self.insert(request, false)
    }
    fn insert(&mut self, mut request: Request, start_now: bool) -> Result<Arc<Job>, String> {
        request.validate()?;
        if crate::channel::ChannelSource::parse(&request.url).is_some_and(|s| !s.youtube) {
            request.is_live = true;
        }
        self.next_id += 1;
        let directory = request.output.join(format!("flowmoor-{}", self.next_id));
        let job = Arc::new(Job {
            state: Mutex::new(Snapshot {
                id: self.next_id,
                url: request.url.clone(),
                title: request.url.clone(),
                phase: if start_now {
                    Phase::Queued
                } else {
                    Phase::Pending
                },
                attempt: 0,
                downloaded: 0,
                total: None,
                speed: None,
                eta: None,
                files: vec![],
                error: None,
                directory,
            }),
            request,
            cancel: AtomicBool::new(false),
            stop: AtomicBool::new(false),
            paused: AtomicBool::new(!start_now),
        });
        self.pending.lock().unwrap().push_back(job.clone());
        self.tx
            .as_ref()
            .unwrap()
            .send(())
            .map_err(|_| "下載服務已停止")?;
        self.jobs.push(job.clone());
        Ok(job)
    }
    pub fn resume(&self, job: &Arc<Job>) {
        let mut queue = self.pending.lock().unwrap();
        let mut s = job.state.lock().unwrap();
        if !matches!(s.phase, Phase::Pending | Phase::Paused) {
            return;
        }
        job.paused.store(false, Ordering::Relaxed);
        s.phase = Phase::Queued;
        if !queue.iter().any(|j| Arc::ptr_eq(j, job)) {
            queue.push_back(job.clone());
        }
        let _ = self.tx.as_ref().unwrap().send(());
    }
    pub fn move_queued(&mut self, id: u128, direction: isize) {
        let mut queue = self.pending.lock().unwrap();
        queue.retain(|j| !j.snapshot().phase.terminal());
        if !queue.iter().any(|j| j.snapshot().id == id)
            && let Some(job) = self.jobs.iter().find(|j| {
                let s = j.snapshot();
                s.id == id && s.phase == Phase::Paused
            })
        {
            queue.push_back(job.clone());
        }
        if let Some(index) = queue.iter().position(|j| j.snapshot().id == id) {
            let target = if direction == 0 {
                0
            } else {
                index.saturating_add_signed(direction).min(queue.len() - 1)
            };
            let job = queue.remove(index).unwrap();
            queue.insert(target, job);
            let ids: Vec<_> = queue.iter().map(|j| j.snapshot().id).collect();
            let positions: Vec<_> = self
                .jobs
                .iter()
                .enumerate()
                .filter(|(_, j)| ids.contains(&j.snapshot().id))
                .map(|(i, _)| i)
                .collect();
            for (i, job) in positions.into_iter().zip(queue.iter()) {
                self.jobs[i] = job.clone();
            }
        }
    }
}
impl Drop for Manager {
    fn drop(&mut self) {
        for job in &self.jobs {
            if !job.snapshot().phase.terminal() {
                job.cancel();
            }
        }
        self.tx.take();
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

pub fn arguments(
    request: &Request,
    directory: &std::path::Path,
    tools: &Tools,
) -> Vec<std::ffi::OsString> {
    let mut args: Vec<std::ffi::OsString> = [
        "--ignore-config",
        "--no-playlist",
        "--no-colors",
        "--no-quiet",
        "--downloader-args",
        "ffmpeg:-progress pipe:2 -nostats",
        "--newline",
        "--no-simulate",
        "--continue",
        "--no-overwrites",
        "--retries",
        "0",
        "--fragment-retries",
        "0",
        "--abort-on-unavailable-fragments",
        "--socket-timeout",
        "20",
        "--progress",
        "--progress-template",
        "download:FLOWMOOR_PROGRESS %(progress)j",
        "--print",
        "before_dl:FLOWMOOR_TITLE %(title)j",
        "--print",
        "after_move:FLOWMOOR_FILE %(filepath)j",
        "--merge-output-format",
        "mkv",
        "--paths",
    ]
    .iter()
    .map(Into::into)
    .collect();
    args.push(directory.into());
    let cap = request
        .format
        .max_height
        .map(|h| format!("[height<={h}]"))
        .unwrap_or_default();
    let format = match request.format.content {
        Content::VideoAudio => format!("bv*{cap}+ba/b{cap}"),
        Content::Video => format!("bv{cap}/b{cap}"),
        Content::Audio => "ba/b".into(),
    };
    let preferred = match request.format.video_codec {
        VideoCodec::Auto => None,
        VideoCodec::H264 => Some("h264"),
        VideoCodec::Vp9 => Some("vp9"),
        VideoCodec::Av1 => Some("av1"),
    };
    if let Some(codec) = preferred {
        args.extend(["--format-sort".into(), format!("vcodec:{codec}").into()]);
    }
    args.extend([
        "--format".into(),
        format.into(),
        "--windows-filenames".into(),
        "--output".into(),
    ]);
    args.push(if request.format.filename.trim().is_empty() {
        "%(title).150B [%(id)s].%(ext)s".into()
    } else {
        format!(
            "{} [%(id)s].%(ext)s",
            request.format.filename.trim().replace('%', "%%")
        )
        .into()
    });

    if tools.ffmpeg != std::path::Path::new("ffmpeg") {
        args.push("--ffmpeg-location".into());
        args.push(tools.ffmpeg.clone().into_os_string());
    }
    if let Some((start, end)) = request.range {
        args.extend([
            "--download-sections".into(),
            format!("*{start}-{end}").into(),
            "--force-keyframes-at-cuts".into(),
        ]);
    }
    if request.live_from_start {
        args.push("--live-from-start".into());
    }
    if let Some(cookies) = &request.cookies {
        args.push("--cookies".into());
        args.push(cookies.clone().into_os_string());
    }
    args.push("--".into());
    args.push(request.url.clone().into());
    args
}

#[derive(Deserialize)]
struct Progress {
    status: Option<String>,
    downloaded_bytes: Option<u64>,
    total_bytes: Option<u64>,
    total_bytes_estimate: Option<u64>,
    speed: Option<f64>,
    eta: Option<u64>,
}
fn apply_line(state: &mut Snapshot, line: &str) {
    if let Some(json) = line.strip_prefix("FLOWMOOR_PROGRESS ") {
        if let Ok(p) = serde_json::from_str::<Progress>(json) {
            state.phase = if p.status.as_deref() == Some("finished") {
                Phase::Processing
            } else {
                Phase::Downloading
            };
            state.downloaded = p.downloaded_bytes.unwrap_or(0);
            state.total = p.total_bytes.or(p.total_bytes_estimate).filter(|n| *n > 0);
            state.speed = p.speed.filter(|n| n.is_finite() && *n >= 0.0);
            state.eta = p.eta;
        }
    } else if let Some(json) = line.strip_prefix("FLOWMOOR_TITLE ") {
        if let Ok(title) = serde_json::from_str::<String>(json) {
            state.title = title;
        }
    } else if let Some(json) = line.strip_prefix("FLOWMOOR_FILE ")
        && let Ok(path) = serde_json::from_str::<String>(json)
    {
        state.files.push(path.into());
    }
}
fn permanent_error(error: &str) -> bool {
    let e = error.to_lowercase();
    [
        "unsupported url",
        "private video",
        "video unavailable",
        "has been removed",
        "sign in",
        "login required",
        "not currently live",
        "no space left",
        "permission denied",
        "requested format is not available",
        "http error 404",
        "ffmpeg not found",
        "unknown encoder",
        "沒有可回溯的 dvr",
    ]
    .iter()
    .any(|s| e.contains(s))
}
fn execute(
    job: &Arc<Job>,
    mut cmd: Command,
    processing: bool,
) -> (std::io::Result<std::process::ExitStatus>, String, String) {
    let mut child = match crate::process::spawn(&mut cmd) {
        Ok(child) => child,
        Err(e) => {
            return (
                Err(e),
                "無法啟動下載／編碼工具，請檢查 yt-dlp 與 FFmpeg。".into(),
                String::new(),
            );
        }
    };
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let output_job = job.clone();
    let output = thread::spawn(move || {
        let mut tail = VecDeque::new();
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            apply_line(&mut output_job.state.lock().unwrap(), &line);
            if tail.len() == 30 {
                tail.pop_front();
            }
            tail.push_back(line);
        }
        tail.into_iter().collect::<Vec<_>>().join("\n")
    });
    let progress_job = job.clone();
    let errors = thread::spawn(move || {
        let started = std::time::Instant::now();
        let mut tail = VecDeque::new();
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if let Some(bytes) = line
                .strip_prefix("total_size=")
                .and_then(|v| v.parse::<u64>().ok())
            {
                let mut s = progress_job.state.lock().unwrap();
                s.phase = if processing {
                    Phase::Processing
                } else {
                    Phase::Downloading
                };
                s.downloaded = bytes;
                s.total = None;
                s.speed = Some(bytes as f64 / started.elapsed().as_secs_f64().max(0.001));
                continue;
            }
            if [
                "frame=",
                "fps=",
                "stream_",
                "bitrate=",
                "out_time",
                "dup_frames=",
                "drop_frames=",
                "speed=",
                "progress=",
            ]
            .iter()
            .any(|prefix| line.starts_with(prefix))
            {
                continue;
            }
            if tail.len() == 30 {
                tail.pop_front();
            }
            tail.push_back(line);
        }
        tail.into_iter().collect::<Vec<_>>().join("\n")
    });
    let mut stop_at = None;
    let mut control_error = None;
    let result = loop {
        let timed_out =
            stop_at.is_some_and(|t: std::time::Instant| t.elapsed() > Duration::from_secs(15));
        let force =
            job.cancel.load(Ordering::Relaxed) || job.paused.load(Ordering::Relaxed) || timed_out;
        if timed_out {
            control_error = Some("停止下載逾時，已強制終止；請檢查保留的檔案".into());
        }
        let stop = job.stop.load(Ordering::Relaxed) && stop_at.is_none();
        if force || stop {
            if stop && !force {
                stop_at = Some(std::time::Instant::now());
            }
            if let Err(error) = crate::process::signal(&mut child, !force) {
                control_error = Some(format!("無法停止下載程序：{error}"));
                if let Err(error) = crate::process::signal(&mut child, false) {
                    control_error = Some(format!("無法終止下載程序：{error}"));
                }
            }
        }
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) => thread::sleep(Duration::from_millis(100)),
            Err(e) => {
                let _ = crate::process::signal(&mut child, false);
                let _ = child.wait();
                break Err(e);
            }
        }
    };
    drop(child);
    let stdout = output.join().unwrap_or_default();
    let error = errors
        .join()
        .unwrap_or_else(|_| "無法讀取下載工具錯誤".into());
    if let Some(control_error) = control_error {
        (
            Err(std::io::Error::other(control_error.clone())),
            control_error,
            stdout,
        )
    } else {
        (result, error, stdout)
    }
}

fn run_job(job: &Arc<Job>, tools: &Tools) {
    let directory = job.snapshot().directory;
    if let Err(error) = std::fs::create_dir_all(&directory) {
        let mut s = job.state.lock().unwrap();
        s.phase = Phase::Failed;
        s.error = Some(error.to_string());
        return;
    }
    for attempt in 0..=job.request.retries {
        if job.paused.load(Ordering::Relaxed)
            && !job.cancel.load(Ordering::Relaxed)
            && !job.stop.load(Ordering::Relaxed)
        {
            job.state.lock().unwrap().phase = Phase::Paused;
            return;
        }
        if job.cancel.load(Ordering::Relaxed) || job.stop.load(Ordering::Relaxed) {
            job.state.lock().unwrap().phase = Phase::Cancelled;
            return;
        }
        {
            let mut s = job.state.lock().unwrap();
            s.phase = Phase::Starting;
            s.attempt = attempt + 1;
            s.error = None;
            s.speed = None;
            s.eta = None;
            s.total = None;
            s.downloaded = 0;
            s.files.clear();
        }
        let mut direct_file = None;
        let result = if let Some(source) =
            crate::channel::ChannelSource::parse(&job.request.url).filter(|s| !s.youtube)
        {
            crate::twitch::live(&source, job.request.cookies.as_deref())
                .and_then(|info| {
                    job.set_title(info.title);
                    let stream = info.twitch_stream.ok_or("Twitch 沒有可抓取的串流")?;
                    let name = if job.request.format.filename.trim().is_empty() {
                        "twitch-live"
                    } else {
                        job.request.format.filename.trim()
                    };
                    let name: String = name
                        .chars()
                        .map(|c| {
                            if c.is_control() || "<>:\"|?*".contains(c) {
                                '_'
                            } else {
                                c
                            }
                        })
                        .collect();
                    let file = directory.join(if attempt == 0 {
                        format!("{name}.mkv")
                    } else {
                        format!("{name}.retry-{attempt}.mkv")
                    });
                    let snapshot = if job.request.range.is_some() && stream.dvr.is_some() {
                        let url = stream.selected_url(
                            job.request.format.max_height,
                            job.request.format.content == Content::Audio,
                        )?;
                        let text = String::from_utf8(crate::twitch::fetch_media(&url)?)
                            .map_err(|_| "Twitch DVR 清單編碼錯誤")?;
                        let text = crate::twitch::freeze_playlist(
                            &text,
                            &url::Url::parse(&url).map_err(|e| e.to_string())?,
                        )?;
                        let path = file.with_extension("range.m3u8");
                        std::fs::write(&path, text).map_err(|e| e.to_string())?;
                        Some(path)
                    } else {
                        None
                    };
                    let outcome = stream
                        .command(&job.request, tools, &file, snapshot.as_deref())
                        .map(|cmd| execute(job, cmd, false));
                    if let Some(snapshot) = snapshot {
                        let _ = std::fs::remove_file(snapshot);
                    }
                    let (status, error, _) = outcome?;
                    if file.metadata().is_ok_and(|m| m.len() > 0) {
                        job.state.lock().unwrap().files = vec![file.clone()];
                    }
                    direct_file = Some(file);
                    Ok((status, error))
                })
                .unwrap_or_else(|error| (Err(std::io::Error::other("Twitch 來源解析失敗")), error))
        } else {
            let mut cmd = crate::process::command(&tools.ytdlp);
            cmd.args(arguments(&job.request, &directory, tools));
            let (status, error, _) = execute(job, cmd, false);
            (status, error)
        };
        let (result, error) = result;
        if let Some(file) = &direct_file
            && file.metadata().is_ok_and(|m| m.len() > 0)
        {
            let mut state = job.state.lock().unwrap();
            state.files = vec![file.clone()];
            state.downloaded = file.metadata().map_or(state.downloaded, |m| m.len());
        }
        job.state
            .lock()
            .unwrap()
            .files
            .retain(|p| p.metadata().is_ok_and(|m| m.is_file() && m.len() > 0));
        let cancelled = job.cancel.load(Ordering::Relaxed);
        let stopped = job.stop.load(Ordering::Relaxed);
        if stopped && result.is_err() {
            let mut state = job.state.lock().unwrap();
            state.phase = Phase::Failed;
            state.error = Some(error);
            return;
        }
        if job.paused.load(Ordering::Relaxed) && !cancelled && !stopped {
            job.state.lock().unwrap().phase = Phase::Paused;
            return;
        }
        if cancelled {
            job.state.lock().unwrap().phase = Phase::Cancelled;
            return;
        }
        if stopped && job.snapshot().files.is_empty() {
            job.state.lock().unwrap().phase = Phase::Stopped;
            return;
        }
        if stopped {
            job.stop.store(false, Ordering::Relaxed);
        }
        let mut s = job.state.lock().unwrap();
        if (stopped || result.as_ref().is_ok_and(|status| status.success())) && !s.files.is_empty()
        {
            drop(s);
            let f = &job.request.format;
            // Video-only also strips muxed audio when Twitch has no separate video stream.
            if f.video_codec != VideoCodec::Auto
                || f.audio_codec != AudioCodec::Auto
                || f.content != Content::VideoAudio
            {
                let sources = job.snapshot().files;
                for source in sources {
                    let extension = if f.content == Content::Audio {
                        match f.audio_codec {
                            AudioCodec::Mp3 => "mp3",
                            AudioCodec::Aac => "m4a",
                            AudioCodec::Opus => "opus",
                            AudioCodec::Auto => "mka",
                        }
                    } else {
                        "mkv"
                    };
                    let target = source.with_file_name(format!(
                        "{}.encoded.{extension}",
                        source.file_stem().unwrap_or_default().to_string_lossy()
                    ));
                    #[derive(Deserialize)]
                    struct Stream {
                        codec_type: String,
                        codec_name: String,
                    }
                    #[derive(Deserialize)]
                    struct Streams {
                        streams: Vec<Stream>,
                    }
                    let mut probe =
                        crate::process::command(tools.ffmpeg.with_file_name(if cfg!(windows) {
                            "ffprobe.exe"
                        } else {
                            "ffprobe"
                        }));
                    probe
                        .args([
                            "-v",
                            "error",
                            "-show_entries",
                            "stream=codec_type,codec_name",
                            "-of",
                            "json",
                        ])
                        .arg(&source);
                    // Probe through the same managed process so pause/cancel/stop also reap ffprobe.
                    let (_, _, metadata) = execute(job, probe, true);
                    let streams = serde_json::from_str::<Streams>(&metadata)
                        .map(|v| v.streams)
                        .unwrap_or_default();
                    let same_codec = |kind, codec| {
                        streams
                            .iter()
                            .find(|s| s.codec_type == kind)
                            .is_some_and(|s| s.codec_name == codec)
                    };
                    let (video_name, video_encoder) = f.video_codec.ffmpeg();
                    let (audio_name, audio_encoder) = f.audio_codec.ffmpeg();
                    let mut cmd = crate::process::command(&tools.ffmpeg);
                    cmd.args([
                        "-nostdin",
                        "-y",
                        "-hide_banner",
                        "-loglevel",
                        "error",
                        "-progress",
                        "pipe:2",
                        "-i",
                    ])
                    .arg(&source);
                    if f.content != Content::Audio {
                        cmd.args([
                            "-map",
                            "0:v:0",
                            "-c:v",
                            if same_codec("video", video_name) {
                                "copy"
                            } else {
                                video_encoder
                            },
                        ]);
                    }
                    if f.content != Content::Video {
                        cmd.args([
                            "-map",
                            "0:a:0",
                            "-c:a",
                            if same_codec("audio", audio_name) {
                                "copy"
                            } else {
                                audio_encoder
                            },
                        ]);
                    }
                    cmd.arg(&target);
                    {
                        let mut s = job.state.lock().unwrap();
                        s.phase = Phase::Processing;
                        s.speed = None;
                        s.eta = None;
                    }
                    let (result, error, _) = execute(job, cmd, true);
                    let mut s = job.state.lock().unwrap();
                    if job.paused.load(Ordering::Relaxed) {
                        s.phase = Phase::Paused;
                        return;
                    }
                    if job.cancel.load(Ordering::Relaxed) || job.stop.load(Ordering::Relaxed) {
                        s.phase = if job.cancel.load(Ordering::Relaxed) {
                            Phase::Cancelled
                        } else {
                            Phase::Stopped
                        };
                        return;
                    }
                    if !result.as_ref().is_ok_and(|s| s.success())
                        || !target.metadata().is_ok_and(|m| m.len() > 0)
                    {
                        s.phase = Phase::Failed;
                        s.error = Some(format!("編碼失敗，來源檔已保留：{error} {result:?}"));
                        return;
                    }
                    s.files = vec![target];
                    let _ = std::fs::remove_file(source);
                }
            }
            let mut s = job.state.lock().unwrap();
            s.phase = if stopped {
                Phase::Stopped
            } else {
                Phase::Completed
            };
            s.downloaded = s
                .files
                .iter()
                .filter_map(|p| p.metadata().ok())
                .map(|m| m.len())
                .sum();
            s.total = Some(s.downloaded);
            s.speed = None;
            s.eta = None;
            return;
        }
        let error = if error.is_empty() {
            format!("下載未產生已確認的輸出檔案（程序結果：{result:?}）")
        } else {
            error
        };
        s.error = Some(error.clone());
        if attempt == job.request.retries
            || permanent_error(&error)
            || result.as_ref().is_err_and(|e| {
                matches!(
                    e.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::PermissionDenied
                )
            })
        {
            s.phase = Phase::Failed;
            return;
        }
        s.phase = Phase::Retrying;
        drop(s);
        let delay = 2u64.pow((attempt as u32).min(5));
        for _ in 0..delay * 10 {
            if job.cancel.load(Ordering::Relaxed)
                || job.stop.load(Ordering::Relaxed)
                || job.paused.load(Ordering::Relaxed)
            {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(url: &str) -> Request {
        Request {
            url: url.into(),
            output: "/tmp/out".into(),
            range: None,
            live_from_start: false,
            is_live: false,
            retries: 2,
            cookies: None,
            format: FormatOptions::default(),
        }
    }
    #[test]
    fn validates_sources_and_range() {
        for url in [
            "https://youtube.com/watch?v=test",
            "https://youtu.be/test",
            "https://www.twitch.tv/videos/123",
            "https://twitch.tv/channel",
        ] {
            assert!(request(url).validate().is_ok());
        }
        for url in [
            "file:///etc/passwd",
            "https://youtube.com.evil.test/a",
            "https://user@twitch.tv/a",
            "--exec=bad",
        ] {
            assert!(request(url).validate().is_err());
        }
        let mut r = request("https://twitch.tv/a");
        r.range = Some((5, 5));
        assert!(r.validate().is_err());
        r.range = None;
        r.live_from_start = true;
        assert!(r.validate().is_err());
    }
    #[test]
    fn times_are_checked() {
        assert_eq!(parse_time("1:02:03"), Ok(3723));
        assert_eq!(parse_time("90"), Ok(90));
        for bad in ["", "-1", "1:60", "1.5", "9999999999999999999999999"] {
            assert!(parse_time(bad).is_err());
        }
    }
    #[test]
    fn filenames_reject_windows_devices_on_every_platform() {
        let mut r = request("https://twitch.tv/channel");
        for name in [
            "NUL", " con ", "PRN.txt", "AUX", "COM1", "lpt9.mkv", "COM¹", "LPT²", "CONIN$",
            "CONOUT$",
        ] {
            r.format.filename = name.into();
            assert!(r.validate().is_err(), "accepted {name}");
        }
        for name in ["", "concert", "null", "COM10", "影片 01"] {
            r.format.filename = name.into();
            assert!(r.validate().is_ok(), "rejected {name}");
        }
    }
    #[test]
    fn progress_does_not_mark_completed_before_merge() {
        let manager = Manager::new(Tools::default());
        let mut s = Snapshot {
            id: 1,
            url: "".into(),
            title: "".into(),
            phase: Phase::Starting,
            attempt: 1,
            downloaded: 0,
            total: None,
            speed: None,
            eta: None,
            files: vec![],
            error: None,
            directory: "/tmp".into(),
        };
        apply_line(
            &mut s,
            r#"FLOWMOOR_PROGRESS {"status":"downloading","downloaded_bytes":123,"total_bytes":null,"speed":null,"eta":null}"#,
        );
        assert_eq!(s.total, None);
        assert_eq!(s.downloaded, 123);
        apply_line(
            &mut s,
            r#"FLOWMOOR_PROGRESS {"status":"finished","downloaded_bytes":123}"#,
        );
        assert_eq!(s.phase, Phase::Processing);
        drop(manager);
    }
    #[test]
    fn arguments_keep_paths_and_urls_separate() {
        let mut r = request("https://youtu.be/abc");
        r.range = Some((10, 20));
        let args = arguments(&r, std::path::Path::new("/tmp/a b"), &Tools::default());
        assert!(args.contains(&"*10-20".into()));
        assert!(args.contains(&"/tmp/a b".into()));
        assert_eq!(args.last().unwrap(), "https://youtu.be/abc");
    }
    #[test]
    fn format_options_are_applied_without_template_injection() {
        let mut r = request("https://youtu.be/abc");
        r.format.max_height = Some(720);
        r.format.filename = "100% clip".into();
        let args = arguments(&r, std::path::Path::new("/tmp"), &Tools::default());
        assert!(args.contains(&"bv*[height<=720]+ba/b[height<=720]".into()));
        assert!(args.contains(&"100%% clip [%(id)s].%(ext)s".into()));
        r.format.content = Content::Audio;
        assert!(
            arguments(&r, std::path::Path::new("/tmp"), &Tools::default()).contains(&"ba/b".into())
        );
        r.format.filename = "../escape".into();
        assert!(r.validate().is_err());
    }
}
