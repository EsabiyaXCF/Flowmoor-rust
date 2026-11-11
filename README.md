# Flowmoor Rust

使用 GPUI＋Rust 的原生 YouTube／Twitch 下載工具，可下載影片、VOD 與直播，選擇畫質、編碼與時間範圍，並在媒體庫預覽和重新下載。

目前為來源建置預覽版，已在 Apple Silicon macOS 驗收。尚未提供可攜式、簽署或公證的安裝包；Windows／Linux 桌面版尚未完成實機驗收。

## 啟動

準備 Rust stable、Python 3.11+、yt-dlp、FFmpeg／ffprobe 與 curl。macOS 原生建置需求見 [GPUI Kit 安裝文件](https://gpui-kit.com/docs/installation/)。Node.js 20+ 只在執行 JavaScript 測試時需要。

macOS 可先安裝外部媒體工具：

```sh
brew install yt-dlp ffmpeg
```

若採 Rust 最小安裝，先執行 `rustup component add rust-docs`，供離線授權盤點使用。取得原始碼後，在專案根目錄執行：

```sh
./run.sh --release
```

這會產生並啟動 `target/Flowmoor Rust.app`。目前 App 仍使用專案內資產，請保留原始碼目錄。開發模式可使用 `./run.sh`。

初次下載：貼上 YouTube／Twitch 網址 → 取得資訊 → 選擇畫質、編碼與時間範圍 → 選輸出資料夾 → 開始下載。頻道網址可瀏覽影片清單；Twitch VOD 滾到底部會繼續載入。

## 下載與媒體庫

- 影片、VOD、目前直播及來源提供的 DVR；最多三個工作並行。
- 影音／影片／音訊輸出、H.264／VP9／AV1、MP3／AAC／Opus、畫質上限與下載預設。
- 時間範圍、下載進度、取消、VOD 暫停／續傳、直播停止與失敗重試。
- 媒體庫搜尋與排序、本機影音預覽、重跑下載及參數編輯／複製貼回。
- 基本下載設定、Cookie 檔、繁中／英文與深淺色。

後處理會先確認來源編碼，相同編碼直接重新封裝；精準範圍裁切仍會進行必要編碼。完成狀態以實際輸出檔案確認，空檔不會標記成功。

設定與媒體庫保存在作業系統的使用者應用程式資料目錄；媒體保存在所選資料夾下的獨立工作目錄。執行中的佇列不會在關閉後恢復。移除媒體庫項目不會刪除下載檔案。

聊天室、精彩評分、記錄／雲端同步、轉錄／字幕、剪輯、排程與自訂版面不在本版範圍。

## 命令列

```sh
cargo build --locked --no-default-features --bin flowmoor-download
./target/debug/flowmoor-download 'https://www.youtube.com/watch?v=jNQXAC9IVRw' ./downloads --start 0 --end 3 --height 144 --video-codec h264 --audio-codec aac
```

程式逐行輸出 JSON 進度；Ctrl+C 取消工作。自動化腳本可加 `--cancel-on-stdin`，寫入任一位元組或關閉 stdin 即要求取消並回收下載程序。直播的停止並保存操作可使用桌面介面。Cookie 檔須為 Netscape 格式，不會自動讀取瀏覽器憑證。

`FLOWMOOR_YTDLP`／`FLOWMOOR_FFMPEG` 可指定工具完整路徑。`run.sh` 亦支援本機忽略目錄 `.tools/` 中的工具，但開源原始碼不附工具執行檔。

## 驗證與貢獻

見 [貢獻指南](CONTRIBUTING.md)、[下載測試指南](docs/guides/FLOWMOOR_COMPLETE_DOWNLOAD_TEST_GUIDE.md) 與 [驗收紀錄](docs/VALIDATION.md)。測試不代表所有受限制內容或網路環境都可下載；預覽權限與下載 Cookie 權限亦分開。

原生間距與互動以 Flowmoor 原始碼為準；CSS 折射、遮罩及部分播放器細節仍有差異。大型 MKV／MKA 首次預覽需建立 remux 快取，解碼能力依系統 WebView 而定。

## 授權

本專案採 [MIT](LICENSE)，Copyright © 2026 Flowmoor。散布本專案或其重要部分時，須保留版權及授權聲明；MIT 不要求在衍生 App 的介面中額外署名。

此授權不變更原版 Flowmoor 的專有授權。第三方套件、素材、商標及外部工具依各自條款；見 [第三方聲明](third-party/README.txt)。請僅下載你有權儲存的內容。
