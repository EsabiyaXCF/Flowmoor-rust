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
