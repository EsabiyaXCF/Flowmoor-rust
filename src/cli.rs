use flowmoor_rust::download::{
    AudioCodec, Content, Manager, Request, Tools, VideoCodec, parse_time,
};
use std::{path::PathBuf, time::Duration};
fn main() {
    flowmoor_rust::handle_process_signal();
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let Some(url) = args.next() else {
        return Err("用法：flowmoor-download URL OUTPUT [--start HH:MM:SS --end HH:MM:SS] [--live-from-start] [--retries 3] [--cookies FILE] [--content video+audio|video-only|audio-only] [--height 720] [--video-codec auto|h264|vp9|av1] [--audio-codec auto|mp3|aac|opus] [--filename NAME]".into());
    };
    let output = PathBuf::from(args.next().ok_or("缺少輸出資料夾")?);
    let mut r = Request {
        url,
        output,
        range: None,
        live_from_start: false,
        is_live: false,
        retries: 3,
        cookies: None,
        format: Default::default(),
    };
    let (mut start, mut end) = (None, None);
    let mut cancel_on_stdin = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--cancel-on-stdin" => cancel_on_stdin = true,
            "--start" => start = Some(parse_time(&args.next().ok_or("缺少開始時間")?)?),
            "--end" => end = Some(parse_time(&args.next().ok_or("缺少結束時間")?)?),
            "--live-from-start" => r.live_from_start = true,
            "--retries" => {
                r.retries = args
                    .next()
                    .ok_or("缺少次數")?
                    .parse()
                    .map_err(|_| "無效重試次數")?
            }
            "--content" => {
                r.format.content = match args.next().ok_or("缺少內容類型")?.as_str() {
                    "video+audio" => Content::VideoAudio,
                    "video-only" => Content::Video,
                    "audio-only" => Content::Audio,
                    _ => return Err("無效內容類型".into()),
                }
            }
            "--video-codec" => {
                r.format.video_codec = match args.next().ok_or("缺少視訊編碼")?.as_str() {
                    "auto" => VideoCodec::Auto,
                    "h264" => VideoCodec::H264,
                    "vp9" => VideoCodec::Vp9,
                    "av1" => VideoCodec::Av1,
                    _ => return Err("無效視訊編碼".into()),
                }
            }
            "--audio-codec" => {
                r.format.audio_codec = match args.next().ok_or("缺少音訊編碼")?.as_str() {
                    "auto" => AudioCodec::Auto,
                    "mp3" => AudioCodec::Mp3,
                    "aac" => AudioCodec::Aac,
                    "opus" => AudioCodec::Opus,
                    _ => return Err("無效音訊編碼".into()),
                }
            }
            "--height" => {
                r.format.max_height = Some(
                    args.next()
                        .ok_or("缺少高度")?
                        .parse()
                        .map_err(|_| "無效影片高度")?,
                )
            }
            "--filename" => r.format.filename = args.next().ok_or("缺少檔案名稱")?,
            "--cookies" => r.cookies = Some(args.next().ok_or("缺少 Cookie 檔案")?.into()),
            _ => return Err(format!("未知選項：{arg}")),
        }
    }
    r.range = match (start, end) {
        (None, None) => None,
        (Some(a), Some(b)) => Some((a, b)),
        _ => return Err("時間範圍必須同時指定開始與結束".into()),
    };
    let mut manager = Manager::new(Tools::default());
    let job = manager.enqueue(r)?;
    let cancel = job.clone();
    ctrlc::set_handler(move || cancel.cancel()).map_err(|e| e.to_string())?;
    if cancel_on_stdin {
        let cancel = job.clone();
        std::thread::spawn(move || {
            use std::io::Read;
            let _ = std::io::stdin().read(&mut [0]);
            cancel.cancel();
        });
    }
    loop {
        let s = job.snapshot();
        println!("{}", serde_json::to_string(&s).map_err(|e| e.to_string())?);
        if s.phase.terminal() {
            return if s.phase == flowmoor_rust::download::Phase::Completed {
                Ok(())
            } else {
                Err(s.error.unwrap_or_else(|| s.phase.label().into()))
            };
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}
