use std::{
    collections::VecDeque,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::Duration,
};
/// A loopback-only DVR snapshot and media proxy; browsers never receive an arbitrary proxy URL.
pub(super) struct PlaylistServer {
    pub url: String,
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}
impl PlaylistServer {
    pub fn new(body: String, page: impl FnOnce(&str) -> String) -> Result<Self, String> {
        Self::serve(body, None, page)
    }
    pub fn file(
        path: std::path::PathBuf,
        page: impl FnOnce(&str) -> String,
    ) -> Result<Self, String> {
        Self::serve(String::new(), Some(path), page)
    }
    fn serve(
        body: String,
        file: Option<std::path::PathBuf>,
        page: impl FnOnce(&str) -> String,
    ) -> Result<Self, String> {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .map_err(|e| format!("無法建立 DVR 預覽：{e}"))?;
        let address = listener.local_addr().map_err(|e| e.to_string())?;
        let origin = format!("http://{address}");
        let media_path = file.as_ref().map(|p| {
            format!(
                "/media.{}",
                p.extension().and_then(|e| e.to_str()).unwrap_or("mp4")
            )
        });
        let html = Arc::new(
            page(&format!(
                "{origin}{}",
                media_path.as_deref().unwrap_or("/snapshot.m3u8")
            ))
            .into_bytes(),
        );
        let media_url = |index: usize, url: &str| {
            let extension = url::Url::parse(url)
                .ok()
                .and_then(|u| {
                    std::path::Path::new(u.path())
                        .extension()
                        .map(|s| s.to_string_lossy().into_owned())
                })
                .filter(|s| s.len() <= 8 && s.bytes().all(|c| c.is_ascii_alphanumeric()))
                .unwrap_or_else(|| "bin".into());
            format!("{origin}/media/{index}.{extension}")
        };
        let mut urls = Vec::new();
        let mut playlist = String::new();
        for line in body.lines() {
            let mut line = line.to_string();
            if line.starts_with("https://") {
                let index = urls.len();
                let proxy = media_url(index, &line);
                urls.push(line);
                line = proxy;
            } else if let Some(start) = line.find("URI=\"") {
                let start = start + 5;
                if let Some(end) = line[start..].find('"') {
                    let end = start + end;
                    let index = urls.len();
                    let proxy = media_url(index, &line[start..end]);
                    urls.push(line[start..end].to_string());
                    line.replace_range(start..end, &proxy);
                }
            }
            playlist.push_str(&line);
            playlist.push('\n');
        }
        let urls = Arc::new(urls);
        let playlist = Arc::new(playlist.into_bytes());
        let cache = Arc::new(Mutex::new(VecDeque::<(usize, Arc<Vec<u8>>)>::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let closing = stop.clone();
        let worker = std::thread::spawn(move || {
            for stream in listener.incoming() {
                if closing.load(Ordering::Relaxed) {
                    break;
                }
                let Ok(mut stream) = stream else {
                    break;
                };
                let urls = urls.clone();
                let playlist = playlist.clone();
                let html = html.clone();
                let cache = cache.clone();
                let file = file.clone();
                let media_path = media_path.clone();
                std::thread::spawn(move || {
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
                    let mut bytes = [0; 8192];
                    let mut count = 0;
                    loop {
                        if count == bytes.len() {
                            return;
                        }
                        let Ok(read) = stream.read(&mut bytes[count..]) else {
                            return;
                        };
                        if read == 0 {
                            return;
                        }
                        count += read;
                        if bytes[..count].windows(4).any(|v| v == b"\r\n\r\n") {
                            break;
                        }
                    }
                    let request = String::from_utf8_lossy(&bytes[..count]);
                    let first = request.lines().next().unwrap_or_default();
                    let mut parts = first.split_whitespace();
                    let method = parts.next().unwrap_or_default();
                    let path = parts.next().unwrap_or_default();
                    let head = method == "HEAD";
                    let mut local = if media_path.as_deref() == Some(path) {
                        file.as_ref().and_then(|p| std::fs::File::open(p).ok())
                    } else {
                        None
                    };
                    let mut status = "200 OK";
                    let mut mime = "application/vnd.apple.mpegurl";
                    let data = if local.is_some() {
                        mime = match file
                            .as_ref()
                            .and_then(|p| p.extension())
                            .and_then(|e| e.to_str())
                        {
                            Some("webm") => "video/webm",
                            Some("mp3") => "audio/mpeg",
                            Some("wav") => "audio/wav",
                            Some("m4a") => "audio/mp4",
                            Some("aac") => "audio/aac",
                            _ => "video/mp4",
                        };
                        Some(Arc::new(Vec::new()))
                    } else if method != "GET" && !head {
                        None
                    } else if path == "/" {
                        mime = "text/html; charset=utf-8";
                        Some(html)
                    } else if path == "/snapshot.m3u8" {
                        Some(playlist)
                    } else if let Some(index) = path
                        .strip_prefix("/media/")
                        .and_then(|v| v.split_once('.'))
                        .and_then(|(index, _)| index.parse::<usize>().ok())
                        && let Some(url) = urls.get(index)
                    {
                        mime = if url.split('?').next().is_some_and(|s| s.ends_with(".ts")) {
                            "video/mp2t"
                        } else {
                            "application/octet-stream"
                        };
                        let cached = cache
                            .lock()
                            .unwrap()
                            .iter()
                            .find(|(i, _)| *i == index)
                            .map(|(_, bytes)| bytes.clone());
                        if let Some(bytes) = cached {
                            Some(bytes)
                        } else {
                            match flowmoor_rust::twitch::fetch_media(url) {
                                Ok(bytes) => {
                                    let bytes = Arc::new(bytes);
                                    let mut cache = cache.lock().unwrap();
                                    cache.push_back((index, bytes.clone()));
                                    if cache.len() > 3 {
                                        cache.pop_front();
                                    }
                                    Some(bytes)
                                }
                                Err(_) => {
                                    status = "502 Bad Gateway";
                                    None
                                }
                            }
                        }
                    } else {
                        status = "404 Not Found";
                        None
                    };
                    let data = data.unwrap_or_else(|| Arc::new(Vec::new()));
                    let length = local
                        .as_ref()
                        .and_then(|f| f.metadata().ok())
                        .map_or(data.len(), |m| m.len() as usize);
                    let mut start = 0;
                    let mut end = length;
                    let mut range_header = String::new();
                    if length > 0
                        && let Some(range) = request.lines().find_map(|l| {
                            l.strip_prefix("Range: bytes=")
                                .or_else(|| l.strip_prefix("range: bytes="))
                        })
                        && let Some((a, b)) = range.split_once('-')
                    {
                        let range = if a.is_empty() {
                            b.parse::<usize>()
                                .ok()
                                .map(|n| (length.saturating_sub(n), length))
                        } else {
                            a.parse::<usize>().ok().and_then(|a| {
                                if b.trim().is_empty() {
                                    Some((a, length))
                                } else {
                                    b.trim()
                                        .parse::<usize>()
                                        .ok()
                                        .map(|b| (a, b.saturating_add(1).min(length)))
                                }
                            })
                        };
                        if let Some((a, b)) = range
                            && a < b
                            && a < length
                        {
                            start = a;
                            end = b;
                            status = "206 Partial Content";
                            range_header =
                                format!("Content-Range: bytes {start}-{}/{length}\r\n", end - 1);
                        } else {
                            status = "416 Range Not Satisfiable";
                            end = 0;
                            range_header = format!("Content-Range: bytes */{length}\r\n");
                        }
                    }
                    let header = format!(
                        "HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\n{range_header}Accept-Ranges: bytes\r\nAccess-Control-Allow-Origin: *\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
                        end - start
                    );
                    if stream.write_all(header.as_bytes()).is_ok() && !head {
                        if let Some(file) = &mut local {
                            use std::io::Seek;
                            if file.seek(std::io::SeekFrom::Start(start as u64)).is_ok() {
                                let _ = std::io::copy(
                                    &mut file.take((end - start) as u64),
                                    &mut stream,
                                );
                            }
                        } else {
                            let _ = stream.write_all(&data[start..end]);
                        }
                    }
                });
            }
        });
        Ok(Self {
            url: format!("{origin}/"),
            address,
            stop,
            worker: Some(worker),
        })
    }
}
impl Drop for PlaylistServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(self.address);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_files_stream_ranges_and_keep_the_source_file() {
        let path = std::env::temp_dir().join(format!("flowmoor-range-{}.mp4", std::process::id()));
        std::fs::write(&path, b"0123456789").unwrap();
        let server = PlaylistServer::file(path.clone(), |source| source.into()).unwrap();
        for (range, status, body) in [("bytes=2-5", "206", "2345"), ("bytes=20-30", "416", "")] {
            let mut stream = TcpStream::connect(server.address).unwrap();
            stream.write_all(b"GET /media.mp4 HTTP/1.1\r\n").unwrap();
            std::thread::sleep(Duration::from_millis(20));
            write!(stream, "Host: localhost\r\nRange: {range}\r\n\r\n").unwrap();
            let mut result = String::new();
            stream.read_to_string(&mut result).unwrap();
            assert!(result.starts_with(&format!("HTTP/1.1 {status}")));
            assert_eq!(result.split_once("\r\n\r\n").unwrap().1, body);
        }
        drop(server);
        assert_eq!(std::fs::read(&path).unwrap(), b"0123456789");
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn snapshot_supports_ranges_without_exposing_arbitrary_urls_and_closes_the_listener() {
        let server = PlaylistServer::new("#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI=\"https://cdn.example.com/key.key\"\n#EXTINF:3,\nhttps://cdn.example.com/0.ts\n#EXT-X-ENDLIST\n".into(), |_| {
            "<video></video>".into()
        })
        .unwrap();
        let address = server.address;
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .write_all(
                b"GET /snapshot.m3u8 HTTP/1.1\r\nHost: localhost\r\nRange: bytes=0-6\r\n\r\n",
            )
            .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 206"));
        assert!(response.ends_with("#EXTM3U"));
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .write_all(b"GET /snapshot.m3u8 HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(response.contains("/media/0.key"));
        assert!(response.contains("/media/1.ts"));
        assert!(!response.contains("https://cdn.example.com"));
        drop(server);
        assert!(TcpStream::connect(address).is_err());
    }
}
