use crate::download::{Request, Tools};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct ChannelSource {
    pub base: String,
    pub youtube: bool,
    pub tab: usize,
}
impl ChannelSource {
    pub fn parse(source: &str) -> Option<Self> {
        let url = url::Url::parse(source).ok()?;
        if !matches!(url.scheme(), "http" | "https")
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return None;
        }
        let host = url.host_str()?;
        let parts: Vec<_> = url.path_segments()?.filter(|s| !s.is_empty()).collect();
        let youtube = host == "youtube.com" || host.ends_with(".youtube.com");
        let base = if youtube {
            if parts.first().is_some_and(|s| s.starts_with('@')) {
                if parts.len() > 2
                    || parts
                        .get(1)
                        .is_some_and(|p| !matches!(*p, "videos" | "shorts" | "streams"))
                {
                    return None;
                }
                format!("https://www.youtube.com/{}", parts[0])
            } else if matches!(parts.first().copied(), Some("channel" | "c" | "user"))
                && parts.len() >= 2
                && parts.len() <= 3
            {
                if parts
                    .get(2)
                    .is_some_and(|p| !matches!(*p, "videos" | "shorts" | "streams"))
                {
                    return None;
                }
                format!("https://www.youtube.com/{}/{}", parts[0], parts[1])
            } else {
                return None;
            }
        } else if (host == "twitch.tv" || host == "www.twitch.tv" || host == "m.twitch.tv")
            && !parts.is_empty()
            && parts.len() <= 2
            && parts.get(1).is_none_or(|p| *p == "videos")
            && !matches!(parts[0], "videos" | "directory" | "downloads" | "settings")
        {
            format!("https://www.twitch.tv/{}", parts[0].to_ascii_lowercase())
        } else {
            return None;
        };
        let tab = match parts.last().copied() {
            Some("shorts") => 1,
            Some("streams") => 2,
            _ => 0,
        };
        Some(Self { base, youtube, tab })
    }
    pub fn listing_url(&self, tab: usize) -> String {
        if self.youtube {
            format!(
                "{}/{}",
                self.base,
                ["videos", "shorts", "streams"][tab.min(2)]
            )
        } else {
            format!("{}/videos?filter=archives&sort=time", self.base)
        }
    }
    pub fn live_url(&self) -> String {
        if self.youtube {
            format!("{}/live", self.base)
        } else {
            self.base.clone()
        }
    }
}
#[derive(Clone, Deserialize)]
pub struct Entry {
    pub id: String,
    pub url: String,
    #[serde(default)]
    pub title: String,
    pub duration: Option<f64>,
    pub live_status: Option<String>,
    pub thumbnail: Option<String>,
    #[serde(default)]
    pub(crate) thumbnails: Vec<Thumbnail>,
    #[serde(skip)]
    pub image: Option<PathBuf>,
    pub view_count: Option<u64>,
    pub upload_date: Option<String>,
}
#[derive(Clone, Deserialize)]
pub(crate) struct Thumbnail {
    url: String,
}
#[derive(Clone, Deserialize)]
struct Listing {
    title: Option<String>,
    channel: Option<String>,
    uploader: Option<String>,
    #[serde(default)]
    entries: Vec<Option<Entry>>,
}
#[derive(Clone)]
pub struct Page {
    pub title: String,
    pub entries: Vec<Entry>,
    pub next: usize,
    pub has_more: bool,
    pub live: Option<crate::twitch::LiveSummary>,
    pub avatar_url: Option<String>,
    pub avatar: Option<PathBuf>,
}

pub fn inspect(request: &Request, tools: &Tools, start: usize) -> Result<Page, String> {
    request.validate()?;
    let source = ChannelSource::parse(&request.url).ok_or("無效的頻道網址")?;
    let mut page = if !source.youtube {
        crate::twitch::channel(&source, start, request.cookies.as_deref())?
    } else {
        let mut cmd = crate::process::command(&tools.ytdlp);
        cmd.args([
            "--ignore-config",
            "--flat-playlist",
            "--skip-download",
            "--dump-single-json",
            "--no-warnings",
            "--socket-timeout",
            "15",
            "--retries",
            "0",
            "--playlist-start",
        ])
        .arg(start.to_string())
        .arg("--playlist-end")
        .arg((start + 29).to_string());
        if let Some(cookies) = &request.cookies {
            cmd.arg("--cookies").arg(cookies);
        }
        let output = cmd
            .arg("--")
            .arg(&request.url)
            .output()
            .map_err(|e| format!("無法取得頻道清單：{e}"))?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
        }
        let listing: Listing =
            serde_json::from_slice(&output.stdout).map_err(|e| format!("頻道清單格式錯誤：{e}"))?;
        let fetched = listing.entries.len();
        Page {
            title: listing
                .channel
                .or(listing.uploader)
                .or(listing.title)
                .unwrap_or_else(|| "頻道影片".into()),
            entries: listing.entries.into_iter().flatten().collect(),
            next: start + fetched,
            has_more: fetched == 30,
            live: None,
            avatar_url: None,
            avatar: None,
        }
    };
    for entry in &mut page.entries {
        if entry.thumbnail.is_none() {
            entry.thumbnail = entry.thumbnails.last().map(|t| t.url.clone());
        }
        if let Some(url) = &mut entry.thumbnail {
            *url = url
                .replace("%{width}", "320")
                .replace("%{height}", "180")
                .replace("{width}", "320")
                .replace("{height}", "180");
        }
    }
    Ok(page)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn channel_urls_do_not_capture_video_or_live_urls() {
        assert_eq!(
            ChannelSource::parse("https://youtube.com/@jawed/shorts/")
                .unwrap()
                .tab,
            1
        );
        assert_eq!(
            ChannelSource::parse("https://youtube.com/@jawed/streams?view=0")
                .unwrap()
                .tab,
            2
        );
        let source = ChannelSource::parse("https://www.youtube.com/@jawed/videos").unwrap();
        assert_eq!(
            source.listing_url(2),
            "https://www.youtube.com/@jawed/streams"
        );
        assert_eq!(
            ChannelSource::parse("https://twitch.tv/example/videos")
                .unwrap()
                .live_url(),
            "https://www.twitch.tv/example"
        );
        assert_eq!(
            ChannelSource::parse("https://twitch.tv/Example")
                .unwrap()
                .base,
            "https://www.twitch.tv/example"
        );
        for url in [
            "https://youtube.com/watch?v=jNQXAC9IVRw",
            "https://youtube.com/@jawed/live",
            "https://twitch.tv/videos/123",
            "https://youtube.com.evil/@x",
            "file://youtube.com/@x",
        ] {
            assert!(ChannelSource::parse(url).is_none(), "{url}");
        }
    }
}
