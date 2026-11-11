#![cfg(unix)]
use flowmoor_rust::download::{Manager, Phase, Request, Tools};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
static FIXTURE_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new(script: &str) -> Self {
        let p = std::env::temp_dir().join(format!(
            "flowmoor-test-{}-{}-{}",
            FIXTURE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&p).unwrap();
        fs::write(p.join("yt-dlp"), format!("#!/bin/sh\n{script}")).unwrap();
        fs::set_permissions(p.join("yt-dlp"), fs::Permissions::from_mode(0o755)).unwrap();
        Self(p)
    }
    fn manager(&self) -> Manager {
        Manager::with_parallelism(
            Tools {
                ytdlp: self.0.join("yt-dlp"),
                ffmpeg: "ffmpeg".into(),
            },
            1,
        )
    }
    fn request(&self) -> Request {
        Request {
            url: "https://youtu.be/test".into(),
            output: self.0.clone(),
            range: None,
            live_from_start: false,
            is_live: false,
            retries: 2,
            cookies: None,
            format: Default::default(),
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn wait(job: &flowmoor_rust::download::Job) -> flowmoor_rust::download::Snapshot {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        let s = job.snapshot();
        if s.phase.terminal() {
            return s;
        }
        assert!(Instant::now() < deadline, "worker timeout: {s:?}");
        std::thread::sleep(Duration::from_millis(30));
    }
}
#[test]
fn retries_transient_and_requires_real_output() {
    let f = Fixture::new(
        r#"while [ "$1" != "--paths" ]; do shift; done
shift
out="$1"
if [ ! -f "$out/attempt" ]; then touch "$out/attempt"; echo 'HTTP Error 503: unavailable' >&2; exit 1; fi
printf media > "$out/video.mkv"
printf 'FLOWMOOR_PROGRESS {"status":"finished","downloaded_bytes":100}\n'
printf 'FLOWMOOR_FILE "%s/video.mkv"\n' "$out"
"#,
    );
    let mut m = f.manager();
    let j = m.enqueue(f.request()).unwrap();
    let s = wait(&j);
    assert_eq!(s.phase, Phase::Completed);
    assert_eq!(s.attempt, 2);
    assert_eq!(s.files.len(), 1);
    assert_eq!(s.downloaded, 5);
    assert_eq!(s.total, Some(5));
    assert!(s.speed.is_none() && s.eta.is_none());
}
#[test]
fn permanent_error_does_not_retry() {
    for message in ["ERROR: Private video. Sign in", "Unknown encoder libsvtav1"] {
        let f = Fixture::new(&format!("echo '{message}' >&2; exit 1"));
        let mut m = f.manager();
        let j = m.enqueue(f.request()).unwrap();
        let s = wait(&j);
        assert_eq!(s.phase, Phase::Failed);
        assert_eq!(s.attempt, 1);
    }
}
#[test]
fn zero_exit_without_output_is_not_success() {
    for script in [
        "exit 0",
        r#"while [ "$1" != "--paths" ]; do shift; done
shift
touch "$1/video.mkv"
printf 'FLOWMOOR_FILE "%s/video.mkv"\n' "$1""#,
    ] {
        let f = Fixture::new(script);
        let mut m = f.manager();
        let mut r = f.request();
        r.retries = 0;
        let s = wait(&m.enqueue(r).unwrap());
        assert_eq!(s.phase, Phase::Failed);
        assert!(s.files.is_empty());
    }
}

#[test]
fn cancellation_kills_children_and_queued_job() {
    let f = Fixture::new(
        r#"sleep 60 &
echo $! > "$(dirname "$0")/pid"
wait
"#,
    );
    let mut m = f.manager();
    let a = m.enqueue(f.request()).unwrap();
    let b = m.enqueue(f.request()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !f.0.join("pid").exists() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(30));
    }
    b.cancel();
    a.cancel();
    assert_eq!(wait(&a).phase, Phase::Cancelled);
    assert_eq!(wait(&b).phase, Phase::Cancelled);
    let pid: i32 = fs::read_to_string(f.0.join("pid"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
}
#[test]
fn stop_is_distinct_from_completed() {
    let f = Fixture::new("trap 'exit 0' INT\nwhile :; do sleep 1; done");
    let mut m = f.manager();
    let j = m.enqueue(f.request()).unwrap();
    std::thread::sleep(Duration::from_millis(200));
    j.stop();
    assert_eq!(wait(&j).phase, Phase::Stopped);
}
#[test]
fn dropping_manager_reaps_active_job() {
    let f = Fixture::new("sleep 60");
    let mut m = f.manager();
    let j = m.enqueue(f.request()).unwrap();
    std::thread::sleep(Duration::from_millis(200));
    drop(m);
    assert_eq!(j.snapshot().phase, Phase::Cancelled);
}

#[test]
fn ffmpeg_reports_live_bytes_before_completion() {
    let f = Fixture::new("echo 'total_size=123456' >&2\nsleep 60");
    let mut m = f.manager();
    let j = m.enqueue(f.request()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let s = j.snapshot();
        if s.downloaded == 123456 {
            assert_eq!(s.phase, Phase::Downloading);
            assert_eq!(s.total, None);
            assert!(s.speed.unwrap() > 0.0);
            break;
        }
        assert!(Instant::now() < deadline, "FFmpeg progress missing: {s:?}");
        std::thread::sleep(Duration::from_millis(30));
    }
    j.cancel();
    assert_eq!(wait(&j).phase, Phase::Cancelled);
}

#[test]
fn metadata_uses_the_same_validated_source_and_decodes_formats() {
    let f = Fixture::new(
        r#"
for arg in "$@"; do
 case "$arg" in --write-thumbnail|--convert-thumbnails|--no-simulate) exit 42;; esac
done
printf 'call\n' >> "$(dirname "$0")/calls"
printf '%s\n' '{"title":"Example","uploader":"Channel","duration":12,"is_live":false,"thumbnail":"https://i.ytimg.com/example.jpg","formats":[{"height":720,"vcodec":"avc1"}]}'
"#,
    );
    let tools = Tools {
        ytdlp: f.0.join("yt-dlp"),
        ffmpeg: "ffmpeg".into(),
    };
    let info = flowmoor_rust::metadata::inspect(&f.request(), &tools).unwrap();
    assert_eq!(info.title, "Example");
    assert_eq!(info.formats[0].height, Some(720));
    assert!(!info.is_live);
    assert!(info.thumbnail.is_none());
    assert_eq!(
        info.thumbnail_url.as_deref(),
        Some("https://i.ytimg.com/example.jpg")
    );
    flowmoor_rust::metadata::inspect(&f.request(), &tools).unwrap();
    assert_eq!(
        fs::read_to_string(f.0.join("calls"))
            .unwrap()
            .lines()
            .count(),
        1
    );
    let mut cookie_request = f.request();
    let first_cookie = f.0.join("cookies-1.txt");
    fs::write(&first_cookie, "first").unwrap();
    cookie_request.cookies = Some(first_cookie);
    flowmoor_rust::metadata::inspect(&cookie_request, &tools).unwrap();
    let second_cookie = f.0.join("cookies-2.txt");
    fs::write(&second_cookie, "second").unwrap();
    cookie_request.cookies = Some(second_cookie);
    flowmoor_rust::metadata::inspect(&cookie_request, &tools).unwrap();
    assert_eq!(
        fs::read_to_string(f.0.join("calls"))
            .unwrap()
            .lines()
            .count(),
        3
    );
    assert!(flowmoor_rust::metadata::fetch_thumbnail("file:///etc/passwd").is_err());
    let mut request = f.request();
    request.url = "file:///etc/passwd".into();
    assert!(flowmoor_rust::metadata::inspect(&request, &tools).is_err());
}

#[test]
fn encoding_failure_keeps_source_and_never_reports_completed() {
    use flowmoor_rust::download::AudioCodec;
    let f = Fixture::new(
        r#"while [ "$1" != "--paths" ]; do shift; done
shift
printf 'source' > "$1/video.mkv"
printf 'FLOWMOOR_FILE "%s/video.mkv"\n' "$1""#,
    );
    let ffmpeg = f.0.join("ffmpeg");
    fs::write(
        &ffmpeg,
        "#!/bin/sh\necho 'encoder unavailable' >&2\nexit 1\n",
    )
    .unwrap();
    fs::set_permissions(&ffmpeg, fs::Permissions::from_mode(0o755)).unwrap();
    let mut manager = Manager::new(Tools {
        ytdlp: f.0.join("yt-dlp"),
        ffmpeg,
    });
    let mut r = f.request();
    r.format.audio_codec = AudioCodec::Mp3;
    let job = manager.enqueue(r).unwrap();
    let s = wait(&job);
    assert_eq!(s.phase, Phase::Failed);
    assert!(s.error.unwrap().contains("encoder unavailable"));
    assert_eq!(fs::read_to_string(&s.files[0]).unwrap(), "source");
}

#[test]
fn paused_download_resumes_in_same_directory_and_queue_can_be_reordered() {
    let f = Fixture::new(
        r#"while [ "$1" != "--paths" ]; do shift; done
shift
out="$1"
touch "$out/started"
while [ ! -f "$out/release" ]; do sleep 0.03; done
echo 'media' > "$out/video.mkv"
printf 'FLOWMOOR_FILE "%s/video.mkv"\n' "$out""#,
    );
    let mut manager = f.manager();
    let a = manager.enqueue(f.request()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !a.snapshot().directory.join("started").exists() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
    }
    let b = manager.enqueue(f.request()).unwrap();
    let c = manager.enqueue(f.request()).unwrap();
    b.pause();
    manager.move_queued(c.snapshot().id, 0);
    a.pause();
    while a.snapshot().phase != Phase::Paused {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
    }
    while !c.snapshot().directory.join("started").exists() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(b.snapshot().phase, Phase::Paused);
    let d = manager.enqueue(f.request()).unwrap();
    manager.move_queued(a.snapshot().id, 0);
    let directory = a.snapshot().directory;
    fs::write(directory.join("release"), "").unwrap();
    manager.resume(&a);
    fs::write(c.snapshot().directory.join("release"), "").unwrap();
    assert_eq!(wait(&c).phase, Phase::Completed);
    assert_eq!(wait(&a).phase, Phase::Completed);
    while !d.snapshot().directory.exists() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
    }
    fs::write(d.snapshot().directory.join("release"), "").unwrap();
    assert_eq!(wait(&d).phase, Phase::Completed);
    assert_eq!(a.snapshot().directory, directory);
    b.cancel();
    assert_eq!(b.snapshot().phase, Phase::Cancelled);
}

#[test]
fn independent_live_outputs_start_concurrently() {
    let f = Fixture::new(
        r#"while [ "$1" != "--paths" ]; do shift; done
shift
touch "$1/started"
sleep 60"#,
    );
    let mut manager = Manager::new(Tools {
        ytdlp: f.0.join("yt-dlp"),
        ffmpeg: "ffmpeg".into(),
    });
    let jobs: Vec<_> = (0..3)
        .map(|_| manager.enqueue(f.request()).unwrap())
        .collect();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !jobs
        .iter()
        .all(|j| j.snapshot().directory.join("started").exists())
    {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
    }
    for job in &jobs {
        job.cancel();
    }
    for job in &jobs {
        assert_eq!(wait(job).phase, Phase::Cancelled);
    }
}

#[test]
fn stopped_live_capture_still_finishes_requested_encoding() {
    use flowmoor_rust::download::AudioCodec;
    let f = Fixture::new(
        r#"while [ "$1" != "--paths" ]; do shift; done
shift
out="$1"
echo source > "$out/source.mkv"
finish() { printf 'FLOWMOOR_FILE "%s/source.mkv"\n' "$out"; exit 0; }
trap finish INT
touch "$out/started"
while :; do sleep 1; done"#,
    );
    let ffmpeg = f.0.join("ffmpeg");
    fs::write(
        &ffmpeg,
        "#!/bin/sh\nfor output; do :; done\necho encoded > \"$output\"\n",
    )
    .unwrap();
    fs::set_permissions(&ffmpeg, fs::Permissions::from_mode(0o755)).unwrap();
    let mut manager = Manager::new(Tools {
        ytdlp: f.0.join("yt-dlp"),
        ffmpeg,
    });
    let mut r = f.request();
    r.is_live = true;
    r.format.audio_codec = AudioCodec::Mp3;
    let job = manager.enqueue(r).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !job.snapshot().directory.join("started").exists() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
    }
    job.pause();
    assert_ne!(job.snapshot().phase, Phase::Paused);
    job.stop();
    let s = wait(&job);
    assert_eq!(s.phase, Phase::Stopped);
    assert_eq!(s.files.len(), 1);
    assert!(s.files[0].to_string_lossy().ends_with(".encoded.mkv"));
    assert_eq!(fs::read_to_string(&s.files[0]).unwrap(), "encoded\n");
}

#[test]
fn channel_pages_skip_unavailable_entries_without_losing_cursor() {
    let f = Fixture::new(
        r#"printf '%s\n' '{"title":"Channel - Videos","channel":"Channel","entries":[{"id":"a","url":"https://youtu.be/aaaaaaaaaaa","title":"A","duration":12},null,{"id":"b","url":"https://youtu.be/bbbbbbbbbbb","title":"B","live_status":"is_live"}]}'"#,
    );
    let tools = Tools {
        ytdlp: f.0.join("yt-dlp"),
        ffmpeg: "ffmpeg".into(),
    };
    let mut request = f.request();
    request.url = "https://www.youtube.com/@example".into();
    let page = flowmoor_rust::channel::inspect(&request, &tools, 7).unwrap();
    assert_eq!(page.title, "Channel");
    assert_eq!(page.entries.len(), 2);
    assert_eq!(page.next, 10);
    assert!(!page.has_more);
    assert_eq!(page.entries[1].live_status.as_deref(), Some("is_live"));
}

#[test]
fn pending_jobs_wait_for_explicit_start_without_blocking_immediate_jobs() {
    let f = Fixture::new(
        r#"while [ "$1" != "--paths" ]; do shift; done
shift
out="$1"
printf media > "$out/video.mkv"
printf 'FLOWMOOR_FILE "%s/video.mkv"\n' "$out"
"#,
    );
    let mut m = f.manager();
    let pending = m.enqueue_pending(f.request()).unwrap();
    let immediate = m.enqueue(f.request()).unwrap();
    assert_eq!(wait(&immediate).phase, Phase::Completed);
    assert_eq!(pending.snapshot().phase, Phase::Pending);
    assert_eq!(pending.snapshot().attempt, 0);
    m.resume(&pending);
    assert_eq!(wait(&pending).phase, Phase::Completed);
    let cancelled = m.enqueue_pending(f.request()).unwrap();
    cancelled.cancel();
    assert_eq!(cancelled.snapshot().phase, Phase::Cancelled);
}

#[test]
fn matching_codecs_are_remuxed_without_reencoding() {
    use flowmoor_rust::download::{AudioCodec, VideoCodec};
    let f = Fixture::new(
        r#"while [ "$1" != "--paths" ]; do shift; done
shift
printf source > "$1/video.mkv"
printf 'FLOWMOOR_FILE "%s/video.mkv"\n' "$1""#,
    );
    fs::write(f.0.join("ffprobe"), "#!/bin/sh\nprintf '%s' '{\"streams\":[{\"codec_type\":\"video\",\"codec_name\":\"h264\"},{\"codec_type\":\"audio\",\"codec_name\":\"aac\"}]}'\n").unwrap();
    fs::write(f.0.join("ffmpeg"), "#!/bin/sh\nprintf '%s\n' \"$@\" > \"$(dirname \"$0\")/args\"\nfor output; do :; done\nprintf encoded > \"$output\"\n").unwrap();
    for tool in ["ffprobe", "ffmpeg"] {
        fs::set_permissions(f.0.join(tool), fs::Permissions::from_mode(0o755)).unwrap();
    }
    let mut m = Manager::new(Tools {
        ytdlp: f.0.join("yt-dlp"),
        ffmpeg: f.0.join("ffmpeg"),
    });
    let mut r = f.request();
    r.format.video_codec = VideoCodec::H264;
    r.format.audio_codec = AudioCodec::Aac;
    let s = wait(&m.enqueue(r).unwrap());
    assert_eq!(s.phase, Phase::Completed);
    let args = fs::read_to_string(f.0.join("args")).unwrap();
    assert!(args.contains("-c:v\ncopy\n"));
    assert!(args.contains("-c:a\ncopy\n"));
    assert_eq!(s.downloaded, 7);
}

#[test]
fn missing_executable_fails_without_retrying() {
    let f = Fixture::new("exit 0");
    let mut m = Manager::new(Tools {
        ytdlp: f.0.join("missing-tool"),
        ffmpeg: "ffmpeg".into(),
    });
    let s = wait(&m.enqueue(f.request()).unwrap());
    assert_eq!(s.phase, Phase::Failed);
    assert_eq!(s.attempt, 1);
}
