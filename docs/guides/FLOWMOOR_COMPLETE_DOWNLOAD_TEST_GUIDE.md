# 真實下載驗收

統一入口為 `tests/scripts/test-flowmoor-complete-download.cjs`，使用和桌面相同的 Rust Manager，並透過 FFprobe 檢查實際串流、畫質、指定編碼、容器與時間長度。不是僅檢查程序退出碼；影音時間範圍下載另檢查兩條串流的起點差距小於 250ms，以免總長正確卻缺少開頭畫面。

先安裝 Node.js 20+、yt-dlp、FFmpeg／ffprobe，並建置 CLI：

```sh
cargo build --locked --no-default-features --bin flowmoor-download
node tests/scripts/test-flowmoor-complete-download.cjs 'https://www.youtube.com/watch?v=jNQXAC9IVRw' --start 0 --end 3 --height 144 --video-codec h264 --audio-codec aac --container mp4
```

把 URL 換成仍可公開下載的 Twitch VOD，亦可驗證其時間範圍。Twitch 頻道正在直播且 DVR 可用時，可使用頻道 URL 及早期時間範圍；不存在的 DVR 區間會失敗，不會以目前直播畫面冒充。

常用選項：`--content video+audio|video-only|audio-only`、`--video-codec auto|h264|vp9|av1`、`--audio-codec auto|mp3|aac|opus`、`--container auto|mp4|mkv`、`--height N`、`--start TIME --end TIME`、`--cookies FILE`。指定 MP4 或 MKV 會核對實際容器和副檔名；純音訊依音訊編碼決定格式。時間接受秒數、MM:SS、HH:MM:SS；起訖須成對。`--live-from-start` 僅供來源支援的 YouTube DVR。

`FLOWMOOR_YTDLP`／`FLOWMOOR_FFMPEG` 指定執行檔；`FLOWMOOR_FFPROBE` 指定測試探測工具。腳本優先使用本機 `.tools/`，再使用 PATH。報告及媒體位於忽略的 `test-output/`，不得提交；Cookie 檔與其內容不得加入儲存庫。

單次測試預設 120 秒後會透過 CLI 的 `--cancel-on-stdin` 控制通道要求取消，並等待它回收下載程序樹；可用 `--timeout-ms` 調整。Windows 不再依賴 Node 的 SIGINT 模擬。這不是長時間直播停止測試：直播開始／停止／後處理另以原生介面驗收，核心測試覆蓋對應生命週期。平台權限、登入、地區、廣告與連線狀況會影響結果，請在報告中區分來源限制與程式失敗。

## 下載參數矩陣

參考原版 Flowmoor 的 `tests/scripts/test-flowmoor-complete-download-matrix.cjs`，沿用公開來源 `https://youtu.be/njX2bu-_Vw4`、完整影片及 30–45 秒區間。每個案例呼叫上面的統一入口，實際經過與桌面相同的 Manager 下載、轉檔，再由 FFprobe 檢查輸出。

```sh
# 原版的 108 組：1080p／2160p × 3 種影片編碼 × 3 種音訊編碼 × 3 種內容 × 2 種時間
node tests/scripts/test-flowmoor-complete-download-matrix.cjs --reference-only

# 192 組：加入影片／音訊 auto
node tests/scripts/test-flowmoor-complete-download-matrix.cjs

# 960 組：來源目前全部 9 種高度，加上最佳畫質，共 10 × 4 × 4 × 3 × 2
node tests/scripts/test-flowmoor-complete-download-matrix.cjs --all-qualities --output test-output/matrix-960 --jobs 2

# 接續同一份矩陣，重測尚未成功的案例
node tests/scripts/test-flowmoor-complete-download-matrix.cjs --all-qualities --resume test-output/matrix-960/matrix-report.json --jobs 2
```

完整高度清單為最佳、2160、1440、1080、720、608、480、360、240、144；608 是這個來源實際提供的高度。影片編碼為 auto／H.264／VP9／AV1，音訊編碼為 auto／AAC／Opus／MP3。內容分為影音、僅影片、僅音訊。前 108 個案例的參數與順序維持原版，方便比較。來源可用格式變更時，須重新探測並調整高度清單。

矩陣關閉應用層重試，要求一次完成、串流數量與指定編碼正確、指定高度完全符合；最佳畫質在此來源須保留 2160p。完整影片預期 126 秒，容許誤差小於 3 秒；區間須為 15 秒，誤差小於 250ms。影音起點差距須小於 250ms。這能攔截指定 4K 卻自動退回 1080p，以及裁切長度錯誤。

每組預設最多兩小時，容納 4K 軟體編碼；`--case-timeout-ms` 可調整。`--jobs` 預設 2，範圍 1–4。`--list` 只列參數，`--case 8` 或 `--from-case 55 --to-case 56` 可縮小測試範圍。

`matrix-report.json` 保存彙總、實際串流及命令；每個 `case-NNN/` 保留 `download-report.json`、`download.log`。成功媒體自動刪除，失敗媒體保留；需要成功檔案時加 `--keep-success-files`。CLI 執行檔與兩個測試腳本的 SHA-256 指紋相同時，續跑才會略過已成功案例；程式修改後會重新驗證。重測前請另存報告及失敗案例目錄，避免覆寫首次失敗證據。Ctrl+C 會要求目前案例取消並等待程序樹回收。

此矩陣覆蓋固定 YouTube 來源的畫質、編碼、內容、完整／區間組合；不等同 Twitch 直播／DVR、登入 Cookie、所有來源、任意起訖時間或跨作業系統驗收。矩陣使用預設容器；指定 MP4 或 MKV 請用上方單次驗收。YouTube 進階調校選項不列入矩陣；亦未獨立量測網路速度或逐幀解碼全部成品。下載日誌可能含短效 CDN URL，分享前應移除。

## Windows 原生回歸

桌面測試包含實際 GPUI／WebView2、YouTube 資訊查詢及三秒下載、設定／媒體庫持久化、本機播放時間前進、定位與換頁關閉播放器。它不注入鍵鼠輸入，但會建立不搶焦點的測試視窗；不可用隱藏視窗的結果推論播放能力。一般測試預設忽略此項，需有可用桌面、WebView2、網路與媒體工具才執行。

在專案根目錄啟動獨立 PowerShell 子程序，以免改變目前 shell 的 APPDATA：

```powershell
pwsh -NoProfile -Command {
    $testOutput = Join-Path (Get-Location) 'test-output/windows-native'
    $env:FLOWMOOR_DESKTOP_TEST_OUTPUT = $testOutput
    $env:APPDATA = Join-Path $testOutput 'profile'
    $env:FLOWMOOR_YTDLP = (Get-Command yt-dlp.exe).Source
    $env:FLOWMOOR_FFMPEG = (Get-Command ffmpeg.exe).Source
    $env:RUST_MIN_STACK = '8388608'
    cargo test --locked --bin flowmoor-rust native_download_and_preview -- --ignored --nocapture --test-threads=1
    exit $LASTEXITCODE
}
```

RUST_MIN_STACK 設定的是測試執行緒。Windows MSVC 桌面執行檔的主執行緒由 build.rs 設為 8 MiB，避免 debug 的 GPUI 呼叫堆疊超過預設空間。

下載程序整合測試另可帶入真實 FFmpeg：

```powershell
$env:FLOWMOOR_TEST_FFMPEG = (Get-Command ffmpeg.exe).Source
cargo test --locked --no-default-features --test platform_process
```

這會驗證 Ctrl+C 收尾、並行停止隔離、暫停續傳、程序樹回收及 CLI stdin 取消。遠端預覽的 loaded 狀態只記錄，不視為播放驗收通過；本機播放器則必須真的回報播放時間前進。
