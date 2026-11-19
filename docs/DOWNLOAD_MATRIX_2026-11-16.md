# 2026-11-16 下載矩陣驗收

最終 **960／960 組通過**，矩陣程序退出碼為 0。原版 Flowmoor 的 108 組參數與順序完整保留，最終亦全部通過。

本次曾有 **44 個不同組合遇到 YouTube CDN HTTP 403**，重測後均成功。因此這是參數與成品驗證通過，不代表來源連線每次都能一次成功。

## 測試範圍

- 來源：[2020 LG OLED l The Black 4K HDR 60fps](https://youtu.be/njX2bu-_Vw4)，沿用原版下載矩陣。
- 10 種畫質：最佳、2160、1440、1080、720、608、480、360、240、144；608 是來源實際提供並經成品驗證的高度。
- 影片：auto、H.264、VP9、AV1；音訊：auto、AAC、Opus、MP3。
- 內容：影音、僅影片、僅音訊；時間：完整影片、30–45 秒。
- 總數：10 × 4 × 4 × 3 × 2 = 960，每組實際呼叫統一下載腳本及與桌面共用的 Rust Manager。
- 使用含中文的檔名；成功媒體清除，FFprobe 資料、狀態、命令與日誌保留。

逐組要求 Completed、應用層 attempt=1、所需串流數量與指定編碼正確、指定高度完全符合。最佳畫質須保留 2160p。完整影片預期 126 秒、誤差小於 3 秒；區間預期 15 秒、誤差小於 250ms。影音串流起點差距須小於 250ms。

| 畫質 | 完整掃描結束時通過 | CDN 403 待重測 | 最終通過 |
|---|---:|---:|---:|
| 最佳 | 90 | 6 | 96 |
| 2160p | 91 | 5 | 96 |
| 1440p | 91 | 5 | 96 |
| 1080p | 94 | 2 | 96 |
| 720p | 92 | 4 | 96 |
| 608p | 93 | 3 | 96 |
| 480p | 93 | 3 | 96 |
| 360p | 92 | 4 | 96 |
| 240p | 93 | 3 | 96 |
| 144p | 90 | 6 | 96 |
| 合計 | 919 | 41 | 960 |

## 發現與修正

### 2160p 指定 H.264 卻下載成 1080p

修正前，055／056 的完整與區間案例都得到 1080p。來源沒有原生 4K H.264，原本優先按編碼排序，讓下載器選了較低高度。

`src/download.rs` 改為 `res,vcodec:…`，先保留解析度，再選擇編碼；必要時從高畫質來源轉成指定編碼。[yt-dlp 格式排序文件](https://github.com/yt-dlp/yt-dlp#sorting-formats)說明排序欄位的優先順序。修正後，指定 2160p 及未指定高度的最佳畫質都通過，沒有退回 1080p。

修正前證據位於 `test-output/matrix-preflight/`。

### 六聲道 Opus 區間轉檔失敗

008（1080p／H.264＋Opus／影音／30–45 秒）曾因 `5.1(side)` 聲道配置被 libopus 拒絕。加入編碼前的聲道配置正規化後，實際成品為 Opus、6 聲道、15.031 秒。

Rust 共用後處理及 Twitch 有界音訊擷取皆使用同一常數。同類問題亦同步修正原版 Flowmoor 的一般轉檔、YouTube 分段轉檔、Twitch 音訊下載與合併；既有時間戳校正與聲道濾鏡合併成一條，避免後面的 `-af` 蓋掉前面的設定。

實際 FFmpeg 校驗 mono、stereo、3.0、quad、5.0、5.1、5.1(side)、6.1、7.1 共 9 種配置：PCM 正規化前後各聲道樣本值相同，Opus 編碼後聲道數正確。失敗來源與重現檔保留於 `test-output/matrix-opus-failure/`。

此問題屬於下載／編碼流程，未涉及 RecordTimeLabel 網頁插件或共享 Core。

## 來源失敗與重測紀錄

完整掃描結束時為 919 通過、41 組 CDN 403；保留報告與失敗案例後，三路重測得到 954 通過、6 組仍為 403。再以單路重測這六組，最終 960 組全部通過。

另有早期調整並行數時已恢復的 004、009、031，共計 44 個不同組合曾遇到 403。這三組已包含於完整掃描的通過數，因此表格不是無重測的一次成功率。操作期間主動取消的案例另行保留，不列為產品故障。各輪都關閉應用層自動重試；測試程序的重新執行有獨立歷史報告。

同一 CLI 執行檔及兩個測試腳本的 SHA-256 指紋：

```text
8f5ea39a829bad08e51ac70fc72a3d9c3715ee6293462bfd356cfbaa92b2ce6d
```

完成後重新計算指紋，確認全部 960 個結果都來自此版本、案例 ID 沒有重複或缺漏，並且沒有驗證錯誤。

Git 忽略的本機證據：

- 最終彙總：`test-output/matrix-960/matrix-report.json`。
- 早期續跑前：`test-output/matrix-960/history/20261115T181625Z-before-resume/`。
- 完整掃描：`test-output/matrix-960/history/20261115T201509Z-complete-scan/`。
- 第一輪重測：`test-output/matrix-960/history/20261115T202057Z-retest-1/`。
- 每組完整狀態、FFprobe 與日誌：`test-output/matrix-960/case-NNN/`。

## 環境與其他檢查

macOS 27.0／arm64、24 GiB RAM、15 個邏輯 CPU，Rust 1.99.1、Node.js 24.22.1、yt-dlp 2026.10.26.220202、FFmpeg／FFprobe 8.1.2。基底 commit 為 `394b533`，包含本次未提交修正。主要矩陣三路並行，初段兩路、最後六組單路；960 案例報告時間為臺北 2026-11-16 02:06–04:23，包含修正、取消與續跑。

已通過：

- `cargo test --no-default-features --offline`：17 個 library、16 個 worker 測試，以及平台程序整合驗證。
- `cargo clippy --all-targets --offline -- -D warnings`、`cargo build --offline --bin flowmoor-rust`。
- `node --test tests/scripts/download-matrix.test.cjs`：案例數量／原版順序及錯誤成品攔截。
- 原版 `opus-channel-layout`、`transcoding-container`、`video-cfr-transcode`、`twitch-video-cfr-transcode`、`youtube-time-range-parallel-execution`、`twitch-audio-codec-resolution`、`twitch-streaming-merger-review-regressions` 測試。

執行、續跑與單案例命令見[下載測試指南](guides/FLOWMOOR_COMPLETE_DOWNLOAD_TEST_GUIDE.md#下載參數矩陣)。

## 限制

此驗收針對固定公開 YouTube 來源的格式組合，未涵蓋 Windows／Linux、Twitch 直播／DVR、登入來源、任意起訖、Rust 尚未提供的容器與 YouTube 進階參數，也未逐幀解碼或逐一驗證各格式的原生播放。沒有獨立網路速度與底層傳輸恢復計數，不能宣稱通過原版所有傳輸品質門檻。

來源仍可能回傳 CDN 403；重測成功不等於已消除此來源風險。4K 精準裁切的軟體重編碼仍慢，本輪最慢為 058 約 543 秒，這是並行驗收下的觀察，不是獨立效能基準。
