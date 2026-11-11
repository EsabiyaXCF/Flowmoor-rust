use flowmoor_rust::download::{Manager, Phase, Request, Tools};
use std::{
    fs,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

fn until(mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while !ready() {
        assert!(Instant::now() < deadline, "process test timed out");
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn main() {
    flowmoor_rust::handle_process_signal();
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args
        .first()
        .is_some_and(|arg| arg == "--fixture-descendant")
    {
        loop {
            fs::write(&args[1], format!("{:?}", std::time::SystemTime::now())).unwrap();
            std::thread::sleep(Duration::from_millis(25));
        }
    }
    if let Some(index) = args.iter().position(|arg| arg == "--paths") {
        let output = Path::new(&args[index + 1]);
        let url = args.last().unwrap();
        let mode = url.split("case=").nth(1).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        ctrlc::set_handler(move || stopping.store(true, Ordering::Relaxed)).unwrap();
        if mode == "ffmpeg" {
            let file = output.join("real-live.mkv");
            let mut ffmpeg =
                std::process::Command::new(std::env::var_os("FLOWMOOR_TEST_FFMPEG").unwrap())
                    .args([
                        "-nostdin",
                        "-y",
                        "-hide_banner",
                        "-loglevel",
                        "error",
                        "-re",
                        "-f",
                        "lavfi",
                        "-i",
                        "anullsrc=r=8000:cl=mono",
                        "-c:a",
                        "pcm_s16le",
                        "-flush_packets",
                        "1",
                    ])
                    .arg(&file)
                    .spawn()
                    .unwrap();
            until(|| {
                assert!(
                    ffmpeg.try_wait().unwrap().is_none(),
                    "FFmpeg exited before stop"
                );
                file.metadata().is_ok_and(|m| m.len() > 1000)
            });
            println!("FLOWMOOR_PROGRESS {{\"status\":\"downloading\",\"downloaded_bytes\":7}}");
            until(|| stop.load(Ordering::Relaxed));
            until(|| ffmpeg.try_wait().unwrap().is_some());
            ffmpeg.wait().unwrap();
            println!("FLOWMOOR_FILE {}", serde_json::to_string(&file).unwrap());
            return;
        }
        fs::write(output.join("argv.json"), serde_json::to_vec(&args).unwrap()).unwrap();
        if mode == "retry" && !output.join("attempt").exists() {
            fs::write(output.join("attempt"), b"first").unwrap();
            eprintln!("HTTP Error 503: unavailable");
            std::process::exit(1);
        }
        if mode == "empty" {
            return;
        }
        let mut descendant = if mode == "tree" {
            Some(
                std::process::Command::new(std::env::current_exe().unwrap())
                    .arg("--fixture-descendant")
                    .arg(output.join("heartbeat"))
                    .spawn()
                    .unwrap(),
            )
        } else {
            None
        };
        fs::write(output.join("started"), b"ready").unwrap();
        println!("FLOWMOOR_PROGRESS {{\"status\":\"downloading\",\"downloaded_bytes\":7}}");
        if matches!(mode, "live" | "tree" | "pause") {
            until(|| stop.load(Ordering::Relaxed) || output.join("release").exists());
        }
        if let Some(child) = &mut descendant {
            let _ = child.kill();
            let _ = child.wait();
        }
        let file = output.join("影片 complete.mkv");
        fs::write(&file, b"finalized media").unwrap();
        println!("FLOWMOOR_FILE {}", serde_json::to_string(&file).unwrap());
        return;
    }

    let output =
        std::env::temp_dir().join(format!("flowmoor platform 空間-{}", std::process::id()));
    fs::create_dir_all(&output).unwrap();
    let request = |mode: &str| Request {
        url: format!("https://youtu.be/fixture?case={mode}"),
        output: output.clone(),
        range: None,
        live_from_start: false,
        is_live: mode == "live",
        retries: 1,
        cookies: None,
        format: Default::default(),
    };
    let tools = Tools {
        ytdlp: std::env::current_exe().unwrap(),
        ffmpeg: "ffmpeg".into(),
    };
    let mut manager = Manager::new(tools);

    // Exercise real argv transport, including Windows quoting and a Unicode output path.
    let mut quoted = request("retry");
    quoted.format.filename = "quoted \"title\" 空間".into();
    let job = manager.enqueue(quoted).unwrap();
    until(|| job.snapshot().phase.terminal());
    let done = job.snapshot();
    assert_eq!(done.phase, Phase::Completed, "{done:?}");
    assert_eq!(done.attempt, 2);
    assert_eq!(fs::read(&done.files[0]).unwrap(), b"finalized media");
    let argv: Vec<String> =
        serde_json::from_slice(&fs::read(done.directory.join("argv.json")).unwrap()).unwrap();
    assert!(
        argv.iter()
            .any(|arg| arg == "quoted \"title\" 空間 [%(id)s].%(ext)s")
    );

    let empty = manager.enqueue(request("empty")).unwrap();
    until(|| empty.snapshot().phase.terminal());
    assert_eq!(empty.snapshot().phase, Phase::Failed);

    let a = manager.enqueue(request("live")).unwrap();
    let b = manager.enqueue(request("live")).unwrap();
    until(|| {
        [a.clone(), b.clone()]
            .iter()
            .all(|job| job.snapshot().downloaded == 7)
    });
    a.stop();
    until(|| a.snapshot().phase.terminal());
    assert_eq!(a.snapshot().phase, Phase::Stopped, "{:?}", a.snapshot());
    assert_eq!(
        fs::read(&a.snapshot().files[0]).unwrap(),
        b"finalized media"
    );
    assert!(
        !b.snapshot().phase.terminal(),
        "stopping one job interrupted another"
    );
    b.cancel();
    until(|| b.snapshot().phase.terminal());
    assert_eq!(b.snapshot().phase, Phase::Cancelled);

    let paused = manager.enqueue(request("pause")).unwrap();
    until(|| paused.snapshot().downloaded == 7);
    let directory = paused.snapshot().directory;
    paused.pause();
    until(|| paused.snapshot().phase == Phase::Paused);
    fs::write(directory.join("release"), b"resume").unwrap();
    manager.resume(&paused);
    until(|| paused.snapshot().phase.terminal());
    assert_eq!(paused.snapshot().phase, Phase::Completed);
    assert_eq!(paused.snapshot().directory, directory);

    let tree = manager.enqueue(request("tree")).unwrap();
    let heartbeat = tree.snapshot().directory.join("heartbeat");
    until(|| heartbeat.exists());
    tree.cancel();
    until(|| tree.snapshot().phase.terminal());
    std::thread::sleep(Duration::from_millis(150));
    let before = fs::read(&heartbeat).unwrap();
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(
        fs::read(&heartbeat).unwrap(),
        before,
        "descendant survived cancellation"
    );

    if let Some(ffmpeg) = std::env::var_os("FLOWMOOR_TEST_FFMPEG") {
        let live = manager.enqueue(request("ffmpeg")).unwrap();
        until(|| live.snapshot().downloaded == 7 || live.snapshot().phase.terminal());
        assert!(!live.snapshot().phase.terminal(), "{:?}", live.snapshot());
        live.stop();
        until(|| live.snapshot().phase.terminal());
        assert_eq!(
            live.snapshot().phase,
            Phase::Stopped,
            "{:?}",
            live.snapshot()
        );
        let probe = std::path::PathBuf::from(ffmpeg).with_file_name(if cfg!(windows) {
            "ffprobe.exe"
        } else {
            "ffprobe"
        });
        let media = std::process::Command::new(probe)
            .args([
                "-v",
                "error",
                "-show_format",
                "-show_streams",
                "-of",
                "json",
            ])
            .arg(&live.snapshot().files[0])
            .output()
            .unwrap();
        assert!(media.status.success());
        let media: serde_json::Value = serde_json::from_slice(&media.stdout).unwrap();
        assert!(
            media["format"]["duration"]
                .as_str()
                .unwrap()
                .parse::<f64>()
                .unwrap()
                > 0.0
        );
        assert_eq!(media["streams"][0]["codec_name"], "pcm_s16le");
        println!("Real FFmpeg: Ctrl+C finalized a playable recording");
    }

    let active = manager.enqueue(request("live")).unwrap();
    until(|| active.snapshot().downloaded == 7);
    drop(manager);
    assert_eq!(active.snapshot().phase, Phase::Cancelled);

    // The acceptance script cancels through stdin; every stdout line must remain JSON.
    {
        use std::io::{BufRead, BufReader};
        use std::process::Stdio;
        let mut cli = std::process::Command::new(env!("CARGO_BIN_EXE_flowmoor-download"))
            .arg("https://youtu.be/fixture?case=tree")
            .arg(&output)
            .arg("--cancel-on-stdin")
            .env("FLOWMOOR_YTDLP", std::env::current_exe().unwrap())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut lines = BufReader::new(cli.stdout.take().unwrap()).lines();
        let directory = loop {
            let line = lines.next().expect("CLI exited before progress").unwrap();
            let snapshot: flowmoor_rust::download::Snapshot = serde_json::from_str(&line).unwrap();
            if snapshot.downloaded == 7 {
                break snapshot.directory;
            }
            assert!(!snapshot.phase.terminal(), "{snapshot:?}");
        };
        until(|| directory.join("heartbeat").exists());
        drop(cli.stdin.take());
        let mut last = None;
        for line in lines {
            last = Some(
                serde_json::from_str::<flowmoor_rust::download::Snapshot>(&line.unwrap()).unwrap(),
            );
        }
        assert_eq!(cli.wait().unwrap().code(), Some(1));
        assert_eq!(last.unwrap().phase, Phase::Cancelled);
        let before = fs::read(directory.join("heartbeat")).unwrap();
        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(fs::read(directory.join("heartbeat")).unwrap(), before);
    }
    fs::remove_dir_all(output).unwrap();
    println!(
        "Platform processes: argv, retry, empty output, isolated graceful stop, pause/resume, tree cancellation and shutdown passed"
    );
}
