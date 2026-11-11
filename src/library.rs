use crate::download::{Request, Snapshot};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
#[derive(Clone, Serialize, Deserialize)]
pub struct Entry {
    pub request: Request,
    pub result: Snapshot,
    pub completed_at: Option<u128>,
    pub thumbnail: Option<PathBuf>,
    pub uploader: Option<String>,
}
#[derive(Default, Serialize, Deserialize)]
pub struct Library {
    pub entries: Vec<Entry>,
}
impl Library {
    pub fn load(path: &Path) -> Result<Self, String> {
        match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| format!("媒體庫無法讀取：{e}")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.to_string()),
        }
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        std::fs::create_dir_all(path.parent().ok_or("媒體庫路徑無效")?)
            .map_err(|e| e.to_string())?;
        let temporary = path.with_extension("tmp");
        std::fs::write(
            &temporary,
            serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        std::fs::rename(temporary, path).map_err(|e| e.to_string())
    }
    pub fn remember(&mut self, mut entry: Entry) {
        if let Some(source) = &entry.thumbnail
            && entry.result.directory.is_dir()
        {
            let target = entry.result.directory.join("thumbnail.jpg");
            if source != &target && std::fs::copy(source, &target).is_ok() {
                entry.thumbnail = Some(target);
            }
        }

        if let Some(current) = self
            .entries
            .iter_mut()
            .find(|e| e.result.id == entry.result.id)
        {
            *current = entry;
        } else {
            self.entries.push(entry);
        }
    }
}
pub fn prepare_preview(path: &Path, ffmpeg: &Path) -> Result<(PathBuf, f64), String> {
    if !path.is_file() {
        return Err("媒體檔案已移動或不存在".into());
    }
    let mut preview = path.to_path_buf();
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("mkv") || e.eq_ignore_ascii_case("mka"))
    {
        use std::hash::{Hash, Hasher};
        let metadata = path.metadata().map_err(|e| e.to_string())?;
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        path.hash(&mut hash);
        metadata.len().hash(&mut hash);
        metadata.modified().ok().hash(&mut hash);
        // shortcut: cache the full remux once; use streaming remuxes when large-file startup becomes a bottleneck.
        preview = std::env::temp_dir().join(format!("flowmoor-local-{:x}.mp4", hash.finish()));
        if !preview.is_file() {
            let pending = preview.with_extension("part.mp4");
            let output = crate::process::command(ffmpeg)
                .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
                .arg(path)
                .args([
                    "-map",
                    "0:v:0?",
                    "-map",
                    "0:a:0?",
                    "-c",
                    "copy",
                    "-movflags",
                    "+frag_keyframe+empty_moov+default_base_moof",
                ])
                .arg(&pending)
                .output()
                .map_err(|e| e.to_string())?;
            if !output.status.success() {
                let _ = std::fs::remove_file(pending);
                return Err(format!(
                    "預覽無法準備：{}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
            std::fs::rename(pending, &preview).map_err(|e| e.to_string())?;
        }
    }
    let probe = ffmpeg.with_file_name(if cfg!(windows) {
        "ffprobe.exe"
    } else {
        "ffprobe"
    });
    let output = crate::process::command(probe)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "json",
        ])
        .arg(&preview)
        .output()
        .map_err(|e| e.to_string())?;
    #[derive(Deserialize)]
    struct Format {
        duration: String,
    }
    #[derive(Deserialize)]
    struct Info {
        format: Format,
    }
    let info: Info = serde_json::from_slice(&output.stdout).map_err(|_| "無法讀取媒體時長")?;
    let duration = info
        .format
        .duration
        .parse::<f64>()
        .map_err(|e| e.to_string())?;
    if !duration.is_finite() || duration <= 0. {
        return Err("媒體沒有可播放內容".into());
    }
    Ok((preview, duration))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_updates_without_deleting_media() {
        let directory =
            std::env::temp_dir().join(format!("flowmoor-library-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let media = directory.join("video.mp4");
        std::fs::write(&media, b"media").unwrap();
        let path = directory.join("library.json");
        let mut library = Library::load(&path).unwrap();
        let request = crate::download::Request {
            url: "https://www.youtube.com/watch?v=jNQXAC9IVRw".into(),
            output: directory.clone(),
            range: Some((10, 13)),
            live_from_start: false,
            is_live: false,
            retries: 2,
            cookies: None,
            format: Default::default(),
        };
        let result = Snapshot {
            id: 1,
            url: request.url.clone(),
            title: "first".into(),
            phase: crate::download::Phase::Completed,
            attempt: 1,
            downloaded: 5,
            total: Some(5),
            speed: None,
            eta: None,
            files: vec![media.clone()],
            error: None,
            directory: directory.clone(),
        };
        let mut entry = Entry {
            request,
            result,
            completed_at: Some(20),
            thumbnail: None,
            uploader: Some("channel".into()),
        };
        library.remember(entry.clone());
        entry.result.title = "updated".into();
        library.remember(entry);
        library.save(&path).unwrap();
        let restored = Library::load(&path).unwrap();
        assert_eq!(restored.entries.len(), 1);
        assert_eq!(restored.entries[0].completed_at, Some(20));
        let mut previous = serde_json::to_value(&restored.entries[0]).unwrap();
        previous.as_object_mut().unwrap().remove("completed_at");
        assert!(
            serde_json::from_value::<Entry>(previous)
                .unwrap()
                .completed_at
                .is_none()
        );
        assert_eq!(restored.entries[0].result.title, "updated");
        assert_eq!(restored.entries[0].request.range, Some((10, 13)));
        library.entries.clear();
        library.save(&path).unwrap();
        assert!(media.is_file());
        std::fs::remove_dir_all(directory).unwrap();
    }
}
