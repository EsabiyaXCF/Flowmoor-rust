# 真實下載驗收

統一入口為 `tests/scripts/test-flowmoor-complete-download.cjs`，使用和桌面相同的 Rust Manager，並透過 FFprobe 檢查實際串流、畫質、指定編碼與時間長度。不是僅檢查程序退出碼；影音時間範圍下載另檢查兩條串流的起點差距小於 250ms，以免總長正確卻缺少開頭畫面。

先安裝 Node.js 20+、yt-dlp、FFmpeg／ffprobe，並建置 CLI：

```sh
cargo build --locked --no-default-features --bin flowmoor-download
node tests/scripts/test-flowmoor-complete-download.cjs 'https://www.youtube.com/watch?v=jNQXAC9IVRw' --start 0 --end 3 --height 144 --video-codec h264 --audio-codec aac
```

把 URL 換成仍可公開下載的 Twitch VOD，亦可驗證其時間範圍。Twitch 頻道正在直播且 DVR 可用時，可使用頻道 URL 及早期時間範圍；不存在的 DVR 區間會失敗，不會以目前直播畫面冒充。

常用選項：`--content video+audio|video-only|audio-only`、`--video-codec auto|h264|vp9|av1`、`--audio-codec auto|mp3|aac|opus`、`--height N`、`--start TIME --end TIME`、`--cookies FILE`。時間接受秒數、MM:SS、HH:MM:SS；起訖須成對。`--live-from-start` 僅供來源支援的 YouTube DVR。

`FLOWMOOR_YTDLP`／`FLOWMOOR_FFMPEG` 指定執行檔；`FLOWMOOR_FFPROBE` 指定測試探測工具。腳本優先使用本機 `.tools/`，再使用 PATH。報告及媒體位於忽略的 `test-output/`，不得提交；Cookie 檔與其內容不得加入儲存庫。

每次測試 120 秒後會透過 CLI 的 `--cancel-on-stdin` 控制通道要求取消，並等待它回收下載程序樹；Windows 不再依賴 Node 的 SIGINT 模擬。這不是長時間直播停止測試：直播開始／停止／後處理另以原生介面驗收，核心測試覆蓋對應生命週期。平台權限、登入、地區、廣告與連線狀況會影響結果，請在報告中區分來源限制與程式失敗。

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
