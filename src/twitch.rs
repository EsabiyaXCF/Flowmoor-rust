use crate::{
    channel::{self, ChannelSource, Entry},
    download::{AudioCodec, Content, Request, VideoCodec},
    metadata::{MediaFormat, MediaInfo},
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::json;
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
const CLIENT: &str = "kimne78kx3ncx6brgo4mv6wki5h1ko";
#[derive(Clone, Debug)]
pub struct LiveSummary {
    pub id: String,
    pub login: String,
    pub title: String,
    pub started: u64,
}
impl LiveSummary {
    pub fn elapsed(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .saturating_sub(self.started)
    }
}
#[derive(Clone, Debug)]
pub struct Variant {
    pub url: String,
    pub height: u32,
    pub audio: bool,
    pub name: String,
}
#[derive(Clone, Debug)]
pub struct LiveStream {
    pub variants: Vec<Variant>,
    pub dvr: Option<String>,
    pub duration: f64,
    pub preview_playlist: Option<String>,
}
#[derive(Deserialize)]
struct Envelope<T> {
    data: Option<T>,
    #[serde(default)]
    errors: Vec<GqlError>,
}
#[derive(Deserialize)]
struct GqlError {
    message: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct User {
    display_name: String,
    #[serde(rename = "profileImageURL")]
    avatar: Option<String>,
    stream: Option<Stream>,
    archive: Option<Connection>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Stream {
    id: String,
    title: String,
    created_at: String,
}
#[derive(Deserialize)]
struct Connection {
    edges: Vec<Edge>,
    #[serde(rename = "pageInfo")]
    page_info: PageInfo,
}
#[derive(Deserialize)]
struct PageInfo {
    #[serde(rename = "hasNextPage")]
    has_more: bool,
}
#[derive(Deserialize)]
struct Edge {
    node: Vod,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Vod {
    id: String,
    title: String,
    length_seconds: u64,
    view_count: u64,
    created_at: String,
    #[serde(rename = "previewThumbnailURL")]
    thumbnail: Option<String>,
}
#[derive(Deserialize)]
struct ChannelData {
    user: Option<User>,
}
#[derive(Deserialize)]
struct LiveData {
    user: Option<User>,
    #[serde(rename = "streamPlaybackAccessToken")]
    token: Option<Token>,
}
#[derive(Deserialize)]
struct Token {
    value: String,
    signature: String,
}
fn http(
    url: &str,
    body: Option<serde_json::Value>,
    cookies: Option<&Path>,
) -> Result<Vec<u8>, String> {
    let mut cmd = crate::process::command("curl");
    cmd.args([
        "--fail",
        "--silent",
        "--show-error",
        "--location",
        "--proto",
        "=https",
        "--proto-redir",
        "=https",
        "--connect-timeout",
        "5",
        "--max-time",
        "12",
    ]);
    let mut headers = format!(
        "Client-ID: {CLIENT}\nContent-Type: application/json\nReferer: https://www.twitch.tv/\n"
    );
    if let Some(path) = cookies {
        let contents =
            std::fs::read_to_string(path).map_err(|e| format!("無法讀取 Twitch Cookie：{e}"))?;
        for line in contents.lines() {
            let fields: Vec<_> = line.trim_start_matches("#HttpOnly_").split('\t').collect();
            if fields.len() == 7
                && matches!(fields[0], ".twitch.tv" | "twitch.tv" | "www.twitch.tv")
                && fields[5] == "auth-token"
            {
                let token = fields[6];
                if !token.is_empty() && token.bytes().all(|c| c.is_ascii_alphanumeric()) {
                    headers.push_str(&format!("Authorization: OAuth {token}\n"));
                } else {
                    return Err("Twitch auth-token 格式錯誤".into());
                }
            }
        }
    }
    cmd.args(["--header", "@-"])
        .arg("--url")
        .arg(url)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(body) = body {
        cmd.arg("--data-binary").arg(body.to_string());
    }
    let mut child = cmd.spawn().map_err(|e| format!("無法連線 Twitch：{e}"))?;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(headers.as_bytes())
        .map_err(|e| e.to_string())?;
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "Twitch 來源請求失敗：{}",
            String::from_utf8_lossy(&output.stderr).replace(url, "Twitch")
        ));
    }
    Ok(output.stdout)
}
pub fn fetch_media(url: &str) -> Result<Vec<u8>, String> {
    http(url, None, None)
}
fn gql<T: DeserializeOwned>(
    query: &str,
    variables: serde_json::Value,
    cookies: Option<&Path>,
) -> Result<T, String> {
    decode(&http(
        "https://gql.twitch.tv/gql",
        Some(json!({"query":query,"variables":variables})),
        cookies,
    )?)
}
fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    let response: Envelope<T> =
        serde_json::from_slice(bytes).map_err(|e| format!("Twitch 回應格式錯誤：{e}"))?;
    if !response.errors.is_empty() {
        return Err(response
            .errors
            .into_iter()
            .map(|e| e.message)
            .collect::<Vec<_>>()
            .join("；"));
    }
    response.data.ok_or_else(|| "Twitch 沒有回傳資料".into())
}
fn summary(login: &str, stream: Stream) -> Result<LiveSummary, String> {
    let started = time::OffsetDateTime::parse(
        &stream.created_at,
        &time::format_description::well_known::Rfc3339,
    )
    .map_err(|_| "Twitch 開播時間格式錯誤")?
    .unix_timestamp();
    if started <= 0 || !stream.id.bytes().all(|c| c.is_ascii_digit()) {
        return Err("Twitch 直播識別資料錯誤".into());
    }
    Ok(LiveSummary {
        id: stream.id,
        login: login.into(),
        title: stream.title,
        started: started as u64,
    })
}
pub fn avatar(channel: &str) -> Result<String, String> {
    let source = ChannelSource::parse(channel).ok_or("無效 Twitch 頻道")?;
    let data: ChannelData = gql(
        "query($login:String!){user(login:$login){displayName profileImageURL(width:70)}}",
        json!({"login":source.base.rsplit('/').next()}),
        None,
    )?;
    data.user
        .and_then(|u| u.avatar)
        .ok_or("頻道未提供頭像".into())
}
pub fn channel(
    source: &ChannelSource,
    start: usize,
    cookies: Option<&Path>,
) -> Result<channel::Page, String> {
    let login = source.base.rsplit('/').next().unwrap_or_default();
    // Twitch rejects public cursor requests; Flowmoor expands the first window instead.
    let limit = start.saturating_sub(1) + 30;
    let data: ChannelData = gql(
        "query ChannelVods($login:String!,$limit:Int!){user(login:$login){displayName profileImageURL(width:70) stream{id title createdAt} archive:videos(first:$limit,type:ARCHIVE,sort:TIME){edges{node{id title lengthSeconds viewCount createdAt previewThumbnailURL(width:320,height:180)}} pageInfo{hasNextPage}}}}",
        json!({"login":login,"limit":limit}),
        cookies,
    )?;
    let user = data.user.ok_or("找不到 Twitch 頻道")?;
    let live = user.stream.map(|s| summary(login, s)).transpose()?;
    if let Some(live) = live.clone() {
        std::thread::spawn(move || {
            dvr(&live);
        });
    }
    let archive = user.archive.ok_or("Twitch 未回傳 VOD 清單")?;
    let count = archive.edges.len();
    let entries = archive
        .edges
        .into_iter()
        .skip(start.saturating_sub(1))
        .map(|e| Entry {
            id: e.node.id.clone(),
            url: format!("https://www.twitch.tv/videos/{}", e.node.id),
            title: e.node.title,
            duration: Some(e.node.length_seconds as f64),
            live_status: None,
            thumbnail: e.node.thumbnail,
            thumbnails: Vec::new(),
            image: None,
            view_count: Some(e.node.view_count),
            upload_date: Some(e.node.created_at),
        })
        .collect();
    Ok(channel::Page {
        title: user.display_name,
        entries,
        next: count + 1,
        has_more: archive.page_info.has_more && count >= limit,
        live,
        avatar_url: user.avatar,
        avatar: None,
    })
}
type DvrCache = Option<(String, Instant, Arc<OnceLock<Option<String>>>)>;
static DVR: Mutex<DvrCache> = Mutex::new(None);
fn dvr(live: &LiveSummary) -> Option<String> {
    let key = format!("{}_{}_{}", live.login, live.id, live.started);
    let cell = {
        let mut cache = DVR.lock().unwrap();
        if let Some((previous, at, cell)) = &*cache
            && *previous == key
            && at.elapsed() < Duration::from_secs(600)
        {
            cell.clone()
        } else {
            let cell = Arc::new(OnceLock::new());
            *cache = Some((key, Instant::now(), cell.clone()));
            cell
        }
    };
    cell.get_or_init(|| {
        let hosts = ["dgeft87wbj63p", "d2e2de1etea730", "d2vjef5jvl6bfs", "d1ymi26ma8va5x", "dqrpb9wgowsf5", "ds0h3roq6wcgc", "d2nvs31859zcd8", "d2aba1wr3818hz", "d3c27h4odz752x", "d1m7jfoe9zdc1j", "d3vd9lfkzbru3h", "d1mhjrowxxagfy", "ddacn6pr5v0tl", "d3aqoihi2n8ty8"];
        for offsets in [0i64, 61, -61, 120, -120, 1, -1, 2, -2, 30, -30].chunks(2) {
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::scope(|scope| {
                for offset in offsets {
                    let timestamp = live.started as i64 + offset;
                    if timestamp <= 0 { continue; }
                    let hash = sha1_smol::Sha1::from(format!("{}_{}_{timestamp}", live.login, live.id)).digest().to_string();
                    for host in hosts {
                        let url = format!("https://{host}.cloudfront.net/{}_{}_{}_{timestamp}/chunked/index-dvr.m3u8", &hash[..20], live.login, live.id);
                        let tx = tx.clone();
                        scope.spawn(move || {
                            let mut cmd = crate::process::command("curl");
                            cmd.args(["--head", "--fail", "--silent", "--output", if cfg!(windows) { "NUL" } else { "/dev/null" }, "--max-time", "1.5", "--url"]).arg(&url);
                            if cmd.status().is_ok_and(|s| s.success()) { let _ = tx.send(url); }
                        });
                    }
                }
            });
            if let Ok(url) = rx.try_recv() { return Some(url); }
        }
        None
    }).clone()
}

pub fn live(source: &ChannelSource, cookies: Option<&Path>) -> Result<MediaInfo, String> {
    let login = source.base.rsplit('/').next().unwrap_or_default();
    let data: LiveData = gql(
        "query($login:String!){user(login:$login){displayName profileImageURL(width:70) stream{id title createdAt}} streamPlaybackAccessToken(channelName:$login,params:{platform:\"web\",playerBackend:\"mediaplayer\",playerType:\"site\"}){value signature}}",
        json!({"login":login}),
        cookies,
    )?;
    let user = data.user.ok_or("找不到 Twitch 頻道")?;
    let summary = summary(login, user.stream.ok_or("此 Twitch 頻道目前沒有直播")?)?;
    let token = data
        .token
        .ok_or("無法取得 Twitch 播放授權；請檢查 Cookie 或頻道存取權限")?;
    let mut url = url::Url::parse(&format!(
        "https://usher.ttvnw.net/api/channel/hls/{login}.m3u8"
    ))
    .unwrap();
    url.query_pairs_mut()
        .append_pair("allow_source", "true")
        .append_pair("allow_audio_only", "true")
        .append_pair("supported_codecs", "avc1")
        .append_pair("token", &token.value)
        .append_pair("sig", &token.signature);
    let master = http(url.as_str(), None, None)?;
    let variants = manifest(&String::from_utf8_lossy(&master), &url)?;
    let dvr = dvr(&summary);
    let (duration, preview_playlist) = if let Some(dvr) = &dvr {
        let text =
            String::from_utf8(http(dvr, None, None)?).map_err(|_| "Twitch DVR 清單編碼錯誤")?;
        (
            playlist_duration(&text),
            Some(freeze_playlist(
                &text,
                &url::Url::parse(dvr).map_err(|e| e.to_string())?,
            )?),
        )
    } else {
        let best = variants
            .iter()
            .filter(|v| !v.audio)
            .max_by_key(|v| v.height)
            .ok_or("Twitch 沒有可下載的影片來源")?;
        (
            playlist_duration(&String::from_utf8_lossy(&http(&best.url, None, None)?)),
            None,
        )
    };
    if !duration.is_finite() || duration <= 0. {
        return Err("Twitch 串流尚未提供可下載的片段，請稍後重新取得".into());
    }
    let formats = variants
        .iter()
        .map(|v| MediaFormat {
            height: (!v.audio).then_some(v.height),
            vcodec: Some(if v.audio { "none" } else { "h264" }.into()),
        })
        .collect();
    Ok(MediaInfo {
        title: summary.title,
        webpage_url: Some(source.base.clone()),
        formats,
        uploader: Some(user.display_name),
        channel_url: Some(source.base.clone()),
        uploader_url: None,
        uploader_id: Some(login.into()),
        avatar_url: user.avatar,
        avatar: None,
        view_count: None,
        duration: Some(duration),
        is_live: true,
        thumbnail: None,
        thumbnail_url: Some(format!(
            "https://static-cdn.jtvnw.net/previews-ttv/live_user_{login}-640x360.jpg"
        )),
        twitch_stream: Some(LiveStream {
            variants,
            dvr,
            duration,
            preview_playlist,
        }),
    })
}
pub(crate) fn freeze_playlist(text: &str, base: &url::Url) -> Result<String, String> {
    let resolve = |value: &str| -> Result<String, String> {
        let url = base.join(value).map_err(|e| e.to_string())?;
        if url.scheme() != "https" {
            return Err("Twitch DVR 片段必須使用 HTTPS".into());
        }
        Ok(url.into())
    };
    let mut result = String::new();
    for line in text.lines() {
        if line.starts_with("#EXT-X-PLAYLIST-TYPE:")
            || line == "#EXT-X-ENDLIST"
            || line.starts_with("#EXT-X-START:")
        {
            continue;
        }
        let mut line = line.to_string();
        if !line.starts_with('#') && !line.trim().is_empty() {
            line = resolve(line.trim())?;
        } else if let Some(start) = line.find("URI=\"") {
            let start = start + 5;
            if let Some(end) = line[start..].find('"') {
                let end = start + end;
                let url = resolve(&line[start..end])?;
                line.replace_range(start..end, &url);
            }
        }
        result.push_str(&line);
        result.push('\n');
    }
    result.push_str("#EXT-X-PLAYLIST-TYPE:VOD\n#EXT-X-ENDLIST\n");
    Ok(result)
}
fn playlist_duration(text: &str) -> f64 {
    text.lines()
        .filter_map(|l| l.strip_prefix("#EXTINF:"))
        .filter_map(|v| v.split(',').next()?.parse::<f64>().ok())
        .filter(|v| v.is_finite() && *v > 0.)
        .sum()
}
fn manifest(text: &str, base: &url::Url) -> Result<Vec<Variant>, String> {
    let mut result = Vec::new();
    let mut attributes = None;
    for line in text.lines().map(str::trim) {
        if let Some(value) = line.strip_prefix("#EXT-X-STREAM-INF:") {
            attributes = Some(value);
        } else if !line.is_empty()
            && !line.starts_with('#')
            && let Some(attrs) = attributes.take()
        {
            let url = base.join(line).map_err(|_| "Twitch 播放清單網址錯誤")?;
            if url.scheme() != "https" {
                return Err("Twitch 串流必須使用 HTTPS".into());
            }
            let height = attrs
                .split(',')
                .find_map(|a| a.strip_prefix("RESOLUTION="))
                .and_then(|r| r.split_once('x'))
                .and_then(|(_, h)| h.parse().ok())
                .unwrap_or(0);
            result.push(Variant {
                url: url.into(),
                height,
                audio: height == 0,
                name: attrs
                    .split(',')
                    .find_map(|a| a.strip_prefix("VIDEO="))
                    .map(|v| v.trim_matches('"').to_string())
                    .unwrap_or_else(|| format!("{height}p30")),
            });
        }
    }
    if result.is_empty() {
        return Err("Twitch 沒有回傳可用畫質".into());
    }
    Ok(result)
}
impl LiveStream {
    pub fn selected_url(&self, height: Option<u32>, audio: bool) -> Result<String, String> {
        let best = self
            .variants
            .iter()
            .filter(|v| v.audio == audio && height.is_none_or(|h| v.height <= h))
            .max_by_key(|v| v.height)
            .or_else(|| {
                self.variants
                    .iter()
                    .filter(|v| !v.audio && (audio || height.is_none_or(|h| v.height <= h)))
                    .max_by_key(|v| v.height)
            })
            .ok_or("Twitch 沒有符合指定畫質的來源")?;
        if let Some(dvr) = &self.dvr {
            if audio || best.height == self.variants.iter().map(|v| v.height).max().unwrap_or(0) {
                return Ok(dvr.clone());
            }
            let url = dvr.replace("/chunked/", &format!("/{}/", best.name));
            if http(&url, None, None).is_ok() {
                return Ok(url);
            }
            return Err("此 Twitch DVR 沒有指定畫質；請選擇最佳可用畫質".into());
        }
        Ok(best.url.clone())
    }
    pub fn command(
        &self,
        request: &Request,
        tools: &crate::download::Tools,
        path: &Path,
        snapshot: Option<&Path>,
    ) -> Result<Command, String> {
        if request.range.is_some() && self.dvr.is_none() {
            return Err("此 Twitch 直播目前沒有可回溯的 DVR，無法指定時間範圍".into());
        }
        let (start, end) = request.range.unwrap_or((0, 0));
        if let Some((_, end)) = request.range
            && end as f64 > self.duration + 1.
        {
            return Err(format!(
                "時間範圍超出目前可抓取的 Twitch 串流（約 {:.0} 秒）",
                self.duration
            ));
        }
        let mut cmd = crate::process::command(&tools.ffmpeg);
        cmd.args([
            "-nostdin",
            "-y",
            "-hide_banner",
            "-loglevel",
            "error",
            "-progress",
            "pipe:2",
            "-rw_timeout",
            "15000000",
        ]);
        let bounded = request.range.is_some();
        if bounded {
            // A finite playlist enables fast input seeking and accurate decoder preroll trimming.
            cmd.args(["-protocol_whitelist", "file,http,https,tcp,tls,crypto"]);
            if request.format.content == Content::Audio {
                // Audio packets must not wait for the next video keyframe.
                cmd.args(["-seek2any", "1"]);
            }
            if start > 0 {
                cmd.arg("-ss").arg(start.to_string());
            }
            cmd.arg("-i")
                .arg(snapshot.ok_or("Twitch DVR 下載快照遺失")?);
            cmd.arg("-t").arg((end - start).to_string());
        } else {
            if self.dvr.is_some() {
                cmd.args(["-live_start_index", "0"]);
            }
            cmd.arg("-i").arg(self.selected_url(
                request.format.max_height,
                request.format.content == Content::Audio,
            )?);
        }
        if request.format.content != Content::Audio {
            let codec = if request.format.video_codec == VideoCodec::Auto {
                VideoCodec::H264
            } else {
                request.format.video_codec
            };
            cmd.args([
                "-map",
                "0:v:0?",
                "-c:v",
                if bounded { codec.ffmpeg().1 } else { "copy" },
            ]);
        }
        if request.format.content != Content::Video {
            let codec = if request.format.audio_codec == AudioCodec::Auto {
                AudioCodec::Aac
            } else {
                request.format.audio_codec
            };
            cmd.args([
                "-map",
                "0:a:0?",
                "-c:a",
                if bounded { codec.ffmpeg().1 } else { "copy" },
            ]);
        }
        cmd.arg(path);
        Ok(cmd)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_video_audio_and_relative_hls_without_accepting_file_urls() {
        let base = url::Url::parse("https://example.com/path/master.m3u8").unwrap();
        let variants=manifest("#EXTM3U\n#EXT-X-STREAM-INF:RESOLUTION=1280x720\n720.m3u8\n#EXT-X-STREAM-INF:VIDEO=\"audio_only\"\naudio.m3u8",&base).unwrap();
        assert_eq!(variants[0].height, 720);
        assert_eq!(variants[0].url, "https://example.com/path/720.m3u8");
        assert!(variants[1].audio);
        assert!(manifest("#EXT-X-STREAM-INF:RESOLUTION=1x2\nfile:///tmp/x", &base).is_err());
        assert_eq!(
            playlist_duration("#EXTINF:2.5,\na.ts\n#EXTINF:3,\nb.ts"),
            5.5
        );
    }
    #[test]
    fn frozen_dvr_resolves_segments_and_keys_without_live_edge_tags() {
        let base = url::Url::parse("https://cdn.example.com/vod/chunked/index-dvr.m3u8").unwrap();
        let text=freeze_playlist("#EXTM3U\n#EXT-X-PLAYLIST-TYPE:EVENT\n#EXT-X-KEY:METHOD=AES-128,URI=\"key.bin\"\n#EXTINF:3,\n0.ts\n",&base).unwrap();
        assert!(text.contains("https://cdn.example.com/vod/chunked/0.ts"));
        assert!(text.contains("URI=\"https://cdn.example.com/vod/chunked/key.bin\""));
        assert!(!text.contains("TYPE:EVENT"));
        assert!(text.ends_with("#EXT-X-ENDLIST\n"));
        assert!(freeze_playlist("#EXTM3U\nfile:///tmp/private.ts", &base).is_err());
    }
    #[test]
    fn dvr_ranges_seek_precisely_and_live_capture_remains_unbounded() {
        let mut stream = LiveStream {
            variants: vec![Variant {
                url: "https://cdn.example.com/live.m3u8".into(),
                height: 360,
                audio: false,
                name: "chunked".into(),
            }],
            dvr: Some("https://cdn.example.com/chunked/index-dvr.m3u8".into()),
            duration: 120.,
            preview_playlist: None,
        };
        let mut request = Request {
            url: "https://www.twitch.tv/example".into(),
            output: "/tmp".into(),
            range: Some((10, 13)),
            live_from_start: false,
            is_live: true,
            retries: 0,
            cookies: None,
            format: Default::default(),
        };
        let command = stream
            .command(
                &request,
                &crate::download::Tools::default(),
                Path::new("capture.mkv"),
                Some(Path::new("range.m3u8")),
            )
            .unwrap();
        let args: Vec<_> = command
            .get_args()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert!(!args.iter().any(|s| s == "-live_start_index"));
        assert!(args.iter().position(|s| s == "-ss") < args.iter().position(|s| s == "-i"));
        assert!(args.windows(2).any(|v| v == ["-c:v", "libx264"]));
        assert!(args.windows(2).any(|v| v == ["-c:a", "aac"]));
        assert!(args.windows(2).any(|v| v == ["-t", "3"]));
        request.range = Some((100, 130));
        assert!(
            stream
                .command(
                    &request,
                    &crate::download::Tools::default(),
                    Path::new("capture.mkv"),
                    Some(Path::new("range.m3u8"))
                )
                .is_err()
        );
        request.range = Some((61, 64));
        request.format.content = Content::Audio;
        let command = stream
            .command(
                &request,
                &crate::download::Tools::default(),
                Path::new("audio.mka"),
                Some(Path::new("range.m3u8")),
            )
            .unwrap();
        let args: Vec<_> = command
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(args.windows(2).any(|a| a == ["-seek2any", "1"]));
        assert!(!args.iter().any(|a| a == "-c:v"));
        request.format.content = Content::VideoAudio;
        request.range = None;
        let command = stream
            .command(
                &request,
                &crate::download::Tools::default(),
                Path::new("capture.mkv"),
                None,
            )
            .unwrap();
        assert!(!command.get_args().any(|arg| arg == "-t"));
        stream.dvr = None;
        request.range = Some((10, 13));
        assert!(
            stream
                .command(
                    &request,
                    &crate::download::Tools::default(),
                    Path::new("capture.mkv"),
                    Some(Path::new("range.m3u8"))
                )
                .unwrap_err()
                .contains("DVR")
        );
        request.range = None;
        let command = stream
            .command(
                &request,
                &crate::download::Tools::default(),
                Path::new("capture.mkv"),
                None,
            )
            .unwrap();
        assert!(!command.get_args().any(|arg| arg == "-t" || arg == "-ss"));
    }
    #[test]
    fn gql_errors_do_not_turn_network_failures_into_offline_channels() {
        assert!(decode::<ChannelData>(br#"{"errors":[{"message":"permission denied"}]}"#).is_err());
        let data:ChannelData=decode(br#"{"data":{"user":{"displayName":"Example","stream":null,"archive":{"edges":[],"pageInfo":{"hasNextPage":false}}}}}"#).unwrap();
        assert!(data.user.unwrap().stream.is_none());
    }
}
