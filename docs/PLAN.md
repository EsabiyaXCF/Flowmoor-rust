# Flowmoor-rust：下載第一階段

範圍：GPUI 原生桌面介面、Rust 下載工作管理、YouTube／Twitch 影片與 VOD、直播、進度、錯誤重試、時間範圍。本輪加入播放器、編碼、下載預設、頻道清單、下載媒體庫與基本設定。暫不加入 RecordTimeLabel、雲端、ASR、剪輯、聊天室、精彩紀錄、排程與自訂版面。

## 架構與擴充位置

- `src/main.rs`：GPUI 輸入、狀態顯示、操作。其他頁面未實作前不顯示無效導覽。
- `src/download.rs`：Request 驗證、Manager 並行佇列、Job 狀態、yt-dlp 子程序、進度解析與取消。任何未來 UI 都共用此入口。
- `src/cli.rs`：相同核心的命令列入口，供完整下載驗收與排錯。
- 播放器位於 `src/preview.rs`；頻道清單位於 `src/channel.rs`／`src/channel_view.rs`。下載媒體庫由 src/library.rs／src/library_view.rs 接合 Request／Job；設定由 src/settings_view.rs 管理草稿。排程仍留待後續，不建立空 trait 或假服務。
- 未來記錄頁另行接入共享 `@recordtimelabel/core`，不可在此複製同步 reducer；此次完全不涉及雲端或舊用戶資料。

## 實作順序與驗收

1. 建立獨立 Cargo 專案，鎖定 GPUI Kit；UI 與服務分離。
2. 以 yt-dlp／FFmpeg 作平台解析與媒體處理工具。Rust 擁有工作、重試、取消與程序生命週期。
3. GPUI 表單：網址、儲存位置、時間範圍、重試、選填 cookies；佇列顯示進度、速度、ETA、錯誤與操作。
4. 核心測試：來源／時間驗證、重試、永久失敗、合併期間狀態、取消程序樹、停止、關閉時清理。
5. 真實 YouTube／Twitch 下載使用 `tests/scripts/test-flowmoor-complete-download.cjs`，以 ffprobe 驗證實際檔案；直播另測停止與重連。
6. Windows／Linux 實機驗收與發行打包另列後續，不將 macOS 結果當成跨平台通過。

## 執行

先安裝 Rust stable、平台 GPUI 建置環境、yt-dlp、FFmpeg／FFprobe；確保工具可在 PATH 找到。

```sh
./run.sh
cargo run --no-default-features --bin flowmoor-download -- 'https://www.youtube.com/watch?v=VIDEO_ID' /path/to/downloads --start 00:10 --end 00:20
cargo test --no-default-features
cargo check --all-targets
cargo build --bins
node tests/scripts/test-flowmoor-complete-download.cjs 'https://www.youtube.com/watch?v=VIDEO_ID' --start 00:10 --end 00:20
```

可用 `FLOWMOOR_YTDLP`、`FLOWMOOR_FFMPEG` 指定工具完整路徑；測試可用 `FLOWMOOR_FFPROBE`。Cookie 檔案需為 yt-dlp 支援的 Netscape 格式；不自動讀取瀏覽器憑證。

## 下載語意與限制

- 每個工作使用獨立子資料夾，避免不同工作範圍覆蓋同名影片；自動重試沿用相同資料夾與 `.part` 檔。手動重新下載建立新工作。
- 最多 3 個工作同時執行；可加入、取消、排序等待工作。影片／VOD 可以暫停並於原資料夾續傳，直播使用停止。此次不做持久化佇列，關閉後不恢復工作列表。
- 影片與 VOD 可多選影音／影片／音訊內容，依來源高度上限與選定編碼輸出；影音合併為 MKV。時間範圍接受秒數、MM:SS、HH:MM:SS；使用 FFmpeg 切割與關鍵影格重編碼，因此切割可能較慢。
- 一般直播從目前開始錄製；YouTube 可選從開頭（來源需支援 DVR）。Twitch 可用的直播 DVR 由直連 HLS／CDN 擷取，過去直播亦可選一般 VOD。無 DVR 的過去區間不能憑空取得。
- 直播沒有總長度時顯示已下載位元組；影音分流進度為目前串流進度，不假裝是整體百分比。
- yt-dlp 顯示 finished 只代表單串流已下載，必須等程序成功且 after_move 指向實際檔案才標為完成。
- 錯誤採最多指定次數的退避重試；登入、私人影片、已刪除、權限、磁碟不足等已辨識永久錯誤不自動重試。
- 直播重連受來源與 yt-dlp 支援影響，可能有缺段；不承諾無縫錄製。網路恢復不等於資料完整。
- 取消強制終止程序樹並保留 partial；停止先送中斷、最多等待 15 秒，標記「已停止，請確認輸出」，不冒充完整下載成功。
- macOS／Linux 用 process group 管理子程序；Windows 目前用 taskkill 終止程序樹，尚未驗證 graceful finalize 與完整生命週期，不宣稱 Windows 已正式可用。
- 程式不內建工具下載器、更新器、登入服務或安裝包。使用者環境的 yt-dlp 版本與網站要求仍會影響成功率。

本次工作已在 `.tools/yt-dlp` 安裝官方 2026.10.26 版本。`run.sh` 與驗收腳本優先使用它；不修改系統既有 yt-dlp。此工具目錄不納入 Git。

頻道縮圖可使用系統 curl；缺少時顯示可操作的文字清單。原生播放器透過 gpui-webview／WKWebView 承載平台播放器。播放器的廣告／禁止嵌入限制與下載權限分開處理。
