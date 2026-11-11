use crate::download::{Request, Tools};
use serde::Deserialize;
use std::{
    path::PathBuf,
    sync::Mutex,
    time::{Instant, SystemTime},
};
#[derive(Clone, Debug, Deserialize)]
pub struct MediaInfo {
    pub title: String,
    pub webpage_url: Option<String>,
    #[serde(default)]
    pub formats: Vec<MediaFormat>,
    pub uploader: Option<String>,
    pub channel_url: Option<String>,
    pub uploader_url: Option<String>,
    pub uploader_id: Option<String>,
    #[serde(rename = "uploader_avatar")]
    pub avatar_url: Option<String>,
    #[serde(skip)]
    pub avatar: Option<PathBuf>,
    pub view_count: Option<u64>,
    pub duration: Option<f64>,
    #[serde(default)]
    pub is_live: bool,
    #[serde(skip)]
    pub thumbnail: Option<PathBuf>,
    #[serde(rename = "thumbnail")]
    pub thumbnail_url: Option<String>,
    #[serde(skip)]
    pub twitch_stream: Option<crate::twitch::LiveStream>,
}
impl MediaInfo {
    pub fn seekable(&self) -> bool {
        !self.is_live || self.twitch_stream.as_ref().is_some_and(|s| s.dvr.is_some())
    }
}
#[derive(Clone, Debug, Deserialize)]
pub struct MediaFormat {
    pub vcodec: Option<String>,
    pub height: Option<u32>,
}

type CacheKey = (String, PathBuf, Option<(PathBuf, Option<SystemTime>)>);
static CACHE: Mutex<Option<(CacheKey, Instant, MediaInfo)>> = Mutex::new(None);

pub fn fetch_thumbnail(source: &str) -> Result<PathBuf, String> {
    let url = url::Url::parse(source).map_err(|e| e.to_string())?;
    if !matches!(url.scheme(), "https" | "http") {
        return Err("縮圖必須使用 HTTP(S)".into());
    }
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut hash);
    let path = std::env::temp_dir().join(format!("flowmoor-thumbnail-{:x}.jpg", hash.finish()));
    if path
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|at| at.elapsed().ok())
        .is_some_and(|age| age < std::time::Duration::from_secs(60))
    {
        return Ok(path);
    }
    let pending = path.with_extension(format!(
        "{}-{:?}.part",
        std::process::id(),
        std::thread::current().id()
    ));
    let result = crate::process::command("curl")
        .args([
            "--fail",
            "--silent",
            "--location",
            "--proto",
            "=https,http",
            "--proto-redir",
            "=https,http",
            "--connect-timeout",
            "5",
            "--max-time",
            "10",
            "--output",
        ])
        .arg(&pending)
        .arg("--")
        .arg(source)
        .output()
        .map_err(|e| e.to_string())?;
    if !result.status.success() {
        let _ = std::fs::remove_file(&pending);
        return Err("縮圖暫時無法載入".into());
    }
    std::fs::rename(pending, &path).map_err(|e| e.to_string())?;
    Ok(path)
}

pub fn inspect(request: &Request, tools: &Tools) -> Result<MediaInfo, String> {
    request.validate()?;
    let key = (
        request.url.clone(),
        tools.ytdlp.clone(),
        request.cookies.as_ref().map(|p| {
            (
                p.clone(),
                std::fs::metadata(p).and_then(|m| m.modified()).ok(),
            )
        }),
    );
    if let Some((previous, at, info)) = CACHE.lock().map_err(|e| e.to_string())?.as_ref()
        && previous == &key
        && at.elapsed().as_secs() < if info.is_live { 5 } else { 180 }
    {
        return Ok(info.clone());
    }
    if let Some(source) = crate::channel::ChannelSource::parse(&request.url)
        && !source.youtube
    {
        let info = crate::twitch::live(&source, request.cookies.as_deref())?;
        *CACHE.lock().map_err(|e| e.to_string())? = Some((key, Instant::now(), info.clone()));
        return Ok(info);
    }
    let mut command = crate::process::command(&tools.ytdlp);
    command.args([
        "--ignore-config",
        "--no-playlist",
        "--skip-download",
        "--no-warnings",
        "--force-ipv4",
        "--socket-timeout",
        "15",
        "--retries",
        "0",
        "--print",
        "%(.{title,webpage_url,uploader,channel_url,uploader_url,uploader_id,uploader_avatar,view_count,duration,is_live,formats,thumbnail})j",
    ]);
    if let Some(cookies) = &request.cookies {
        command.arg("--cookies").arg(cookies);
    }
    let output = command
        .arg("--")
        .arg(&request.url)
        .output()
        .map_err(|e| format!("無法取得影片資訊：{e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    let info: MediaInfo =
        serde_json::from_slice(&output.stdout).map_err(|e| format!("影片資訊格式錯誤：{e}"))?;
    *CACHE.lock().map_err(|e| e.to_string())? = Some((key, Instant::now(), info.clone()));
    Ok(info)
}

pub fn channel_avatar(info: &MediaInfo, tools: &Tools) -> Result<PathBuf, String> {
    if let Some(url) = &info.avatar_url {
        return fetch_thumbnail(url);
    }
    let twitch_channel = info
        .webpage_url
        .as_deref()
        .filter(|u| u.contains("twitch.tv"))
        .and(info.uploader_id.as_ref())
        .map(|login| format!("https://www.twitch.tv/{login}"));
    let channel = info
        .channel_url
        .as_ref()
        .or(info.uploader_url.as_ref())
        .or(twitch_channel.as_ref())
        .ok_or("來源未提供頻道")?;
    if channel.contains("twitch.tv") {
        return crate::twitch::avatar(channel).and_then(|url| fetch_thumbnail(&url));
    }
    #[derive(Deserialize)]
    struct Image {
        url: String,
        width: Option<u32>,
        height: Option<u32>,
    }
    #[derive(Deserialize)]
    struct Profile {
        #[serde(default)]
        thumbnails: Vec<Image>,
    }
    let output = crate::process::command(&tools.ytdlp)
        .args([
            "--ignore-config",
            "--flat-playlist",
            "--playlist-items",
            "0",
            "--skip-download",
            "--dump-single-json",
            "--no-warnings",
            "--socket-timeout",
            "10",
            "--retries",
            "0",
            "--",
            channel,
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("頻道頭像暫時無法取得".into());
    }
    let profile: Profile = serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
    let image = profile
        .thumbnails
        .iter()
        .filter(|i| i.width == i.height)
        .max_by_key(|i| i.width.unwrap_or(0))
        .ok_or("頻道未提供頭像")?;
    fetch_thumbnail(&image.url)
}

pub fn preview_url(source: &str) -> Result<String, String> {
    let url = url::Url::parse(source).map_err(|_| "無效影片網址")?;
    let host = url.host_str().unwrap_or_default();
    let parts: Vec<_> = url.path_segments().map(|s| s.collect()).unwrap_or_default();
    if host == "youtu.be" || host == "youtube.com" || host.ends_with(".youtube.com") {
        let id = if host == "youtu.be" {
            parts.first().map(|v| v.to_string())
        } else if matches!(parts.first().copied(), Some("embed" | "shorts" | "live")) {
            parts.get(1).map(|v| v.to_string())
        } else {
            url.query_pairs()
                .find(|(k, _)| k == "v")
                .map(|(_, v)| v.into_owned())
        };
        let id = id
            .filter(|s| {
                s.len() == 11
                    && s.bytes()
                        .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
            })
            .ok_or("請提供影片或直播網址")?;
        return Ok(format!(
            "https://www.youtube.com/embed/{id}?autoplay=0&controls=0&playsinline=1&rel=0"
        ));
    }
    if host == "twitch.tv" || host.ends_with(".twitch.tv") {
        let parameter = if parts.first() == Some(&"videos") {
            let id = parts
                .get(1)
                .filter(|s| !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit()))
                .ok_or("無效 Twitch VOD 網址")?;
            format!("video=v{id}")
        } else {
            let channel = parts
                .first()
                .filter(|s| {
                    !s.is_empty() && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
                })
                .ok_or("無效 Twitch 頻道網址")?;
            format!("channel={channel}")
        };
        return Ok(format!(
            "https://player.twitch.tv/?{parameter}&parent=localhost&parent=127.0.0.1&autoplay=false"
        ));
    }
    Err("僅支援 YouTube／Twitch 預覽".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn thumbnails_expire_without_exposing_partial_downloads() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/thumbnail.jpg", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            for body in ["first", "second"] {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(3)))
                    .unwrap();
                let mut request = [0; 1024];
                let _ = stream.read(&mut request).unwrap();
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
            }
        });
        let path = fetch_thumbnail(&url).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"first");
        assert_eq!(fetch_thumbnail(&url).unwrap(), path);
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_times(
                std::fs::FileTimes::new()
                    .set_modified(SystemTime::now() - std::time::Duration::from_secs(61)),
            )
            .unwrap();
        assert_eq!(fetch_thumbnail(&url).unwrap(), path);
        assert_eq!(std::fs::read(&path).unwrap(), b"second");
        server.join().unwrap();
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn preview_sources_remain_on_the_platform_player() {
        for source in [
            "https://youtu.be/jNQXAC9IVRw",
            "https://www.youtube.com/watch?v=jNQXAC9IVRw",
            "https://youtube.com/live/jNQXAC9IVRw",
        ] {
            assert!(
                preview_url(source)
                    .unwrap()
                    .starts_with("https://www.youtube.com/embed/jNQXAC9IVRw?")
            );
        }
        assert!(
            preview_url("https://twitch.tv/videos/123")
                .unwrap()
                .contains("video=v123")
        );
        assert!(
            preview_url("https://twitch.tv/example")
                .unwrap()
                .contains("channel=example")
        );
        assert!(preview_url("https://youtube.com.evil/watch?v=jNQXAC9IVRw").is_err());
        assert!(preview_url("https://youtube.com/playlist?list=123").is_err());
    }
}
