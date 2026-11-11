# Flowmoor 前端原始碼到 GPUI 的對應

此次依原始碼拆解版面、狀態與事件；實機截圖只用於確認差異。原專案只讀，不改動。

| 原版來源 | GPUI 對應 | 本次範圍 |
|---|---|---|
| `src/components/Sidebar.jsx` | src/sidebar_view.rs／Page | clamp(180px,18vw,200px) 展開、48px 收合，下載／佇列／媒體庫／設定；不放未實作功能的空按鈕 |
| `src/config/theme.js` | src/theme.rs | 深淺色 background、surface、ink、border、accent、10px 控件圓角與系統字體；透過 Theme::update 同步 GPUI 控件 tokens |
| `src/components/download/DownloadFormPanels.jsx` 的 ClassicDownloadForm | main.rs 下載內容區 | 網址 → 影片資訊 → 格式／檔名 → 時間範圍 → 資料夾 → 操作區 |
| `src/components/download/FormatControls.jsx` | 來源樣式選單、Input 與 FormatOptions | 真實來源畫質、多內容勾選、視訊／音訊編碼與檔名；選項送入共用 Rust 核心 |
| `src/components/download/VideoInfoPanel.jsx` | src/metadata.rs、src/video_info_view.rs | 真實標題、頻道、觀看數、長度／直播狀態與縮圖；來源異動丟棄舊查詢結果 |
| `src/components/TimeRangeInput.jsx` | 開始／結束 Input、重置 | 同步到 Request.range；原生 WebView 播放器、播放／暫停、定位、音量、放大、拖曳範圍與目前時間帶入 |
| `DownloadOutputFolder`／`PathDropdown.jsx` | src/output_controls.rs、原生資料夾選擇器 | 路徑下拉、暫存／明確保存／移除、新增資料夾 |
| `DownloadQueuePanel.jsx` | src/queue_view.rs、6px 進度列 | 實際進度、錯誤、取消、停止、重新下載；影片／VOD 暫停續傳與排序；最多 3 路並行 |
| `src/components/ui/FlowmoorEmptyIllustration.jsx` | assets/boat.svg、boat.png | 原版紙船幾何直接移植，沒有重新生成替代圖 |

## 元件庫調查

採用既有 GPUI Kit 0.9.0：Input、Button、Progress、Slider、原生焦點與資料夾對話框。下拉選單以原版幾何與互動實作；圖示將原版 Phosphor SVG 靜態移植，透過 AssetSource 對應目前使用的 47 個名稱，其餘資產交給 AllAssets。

- 官方：https://gpui-kit.com/component/
- gpuicn：https://github.com/karyalabs/gpuicn — source-distribution 方式、45 個元件，官方標示 experimental，也以 GPUI Kit 為基底；此階段不增加第二套主題與元件來源。

## 差異與後續

只移植下載範圍。多內容勾選、下載預設、指定編碼與播放器預覽已接入。畫質以來源高度上限選擇，音訊模式忽略影片高度。聊天室、精彩紀錄、雲端、ASR、剪輯與自動出片未加入。頻道列表納入本輪；依使用者確認，排程與自訂版面暫不移植。原生版以 GPUI 漸層近似 CSS 縮圖遮罩；backdrop-filter 與折射效果仍有差異，不宣稱像素完全相同。

## 檢查

Hallmark app-screen 規則用於控件、焦點、真實資料與狀態。其 sloplint --render 已執行，但工具拒絕 .rs（只支援網頁格式），因此沒有 HTML gate 通過的宣稱。改以 Rust 編譯、Clippy、核心測試、CUA 原生 AX／畫面及獨立 reviewer 檢查。

## 本輪架構

- `download.rs`：型別化內容／編碼選項，3 路共用工作佇列；下載與 FFmpeg 共用程序生命週期、取消、進度與錯誤讀取。暫停 VOD 後沿用同一個輸出資料夾續傳；直播使用停止操作。
- `preview.rs` + `assets/preview.js`：GPUI 控件與原生 video／平台播放器的時間／播放狀態橋接。URL 或頁面切換關閉預覽；廣告期間保留正片時間基準並停用範圍操作。
- `preferences.rs`：Rust 專用設定檔，保存下載預設、重試次數、Cookie 路徑、語言與最近路徑；下載結果另存 library.json，不保存執行中佇列。
- 明確指定編碼會於下載後執行 FFmpeg，失敗保留來源；停止直播後仍處理選定編碼。
- WebView 使用 GPUI Kit／gpui-webview 0.9.0，啟用官方 gpui-fast 合成；預覽放回影片資訊區，和表單共用捲動與裁切。

播放器相關限制依 [GPUI Kit WebView 官方文件](https://gpui-kit.com/docs/webview/)；本機驗收目標為 macOS。下載 Cookie 檔與平台嵌入播放器登入狀態分開，來源禁用嵌入時會顯示預覽錯誤。

頻道清單對照 `YouTubeChannelPanel.jsx`／`TwitchChannelPanel.jsx`。YouTube 使用 yt-dlp 平面清單，Twitch 一般 VOD 與直播資訊使用 GraphQL；`channel.rs`／`twitch.rs` 管理資料，`channel_view.rs`／`twitch_channel_view.rs` 管理分頁、縮圖、選取與返回。縮圖另行背景載入，不阻塞文字清單。

## 2026-11-09 互動與間距對照

本輪直接對照 `TimeRangeInput.jsx`、`timeRangeDrag.js`、`PreviewTimeline.jsx`、`ClassicDownloadForm` 與原版 App 截圖。

- 欄位順序：內容／品質 → 名稱／下載預設 → 影片／音訊編碼；兩欄等寬，44px 控件、8px 標籤間距、12px 區域間距。預設名稱移入對話框。
- 縮圖可開啟預覽，播放器使用 16:9；播放鍵置中，前往起訖與設為起訖分開，音量與放大在左。
- 時間尺整合播放頭與雙範圍把手；垂直滾輪以游標為中心縮放、水平滾輪平移，畫面外把手不攔截 seek。
- 時間欄位單擊編輯，拖曳前後調整；向上 96px 逐級細化，沿用原版時長精度級距、3px／步與垂直阻尼。
- 上下鍵 ±1 秒、Shift ±10 秒、PageUp／PageDown ±10 秒；文字編輯模式同樣支援。空值表示預設，Enter／失焦提交，Escape 回復編輯前兩端。
- 起訖交叉會連動另一端，保留至少一秒並限制影片時長；欄位、播放器設點與把手共用規則。
- GPUI 視窗事件在 paint 註冊，MouseUp 常駐接收並依最終座標結算，涵蓋點擊快於下一畫面的情況。
- 時間尺、範圍把手與時間按鈕保留原生無障礙動作。

時間尺的浮動影格縮圖尚未移植。音量列已支援 hover／focus 展開及 200ms 淡入，寬度仍立即展開；CSS 折射及 100deg 多停點遮罩仍為原生近似。尺寸與間距依原始碼對齊，沒有全畫面逐像素差分通過的宣稱。

## 側邊欄、影片卡片與下載底列

- GPUI 主題 rem 改為 16px，與原版瀏覽器一致；內容文字另用原本字級，避免相同 Tailwind 尺寸在原生端縮小。
- 導覽按鈕 40px 高、9px 內側 padding、18px 圖示；主列文字左緣 38px，子列 34px，外層 6px、子列縮排 15px 與 6px。改用滿寬內容列，避免 GPUI Button 的內部置中。
- 下載群組可收合；側欄收合時點擊下載群組會展開側欄與子項目。
- 影片卡片沿用 30% 縮圖、50% 淡背景、深淺色 mask 停點及染色；標題與面板高度沿用原版 container width 公式，14px 內側 padding。染色先加入影像，再共同淡出，避免左緣硬邊。
- 縮圖與右上箭頭可切換卡片內預覽；箭頭停止事件傳播，避免與下層縮圖重複切換。展開與還原後依實際捲動錨點定位，捲動完成要求重繪。
- 輸出欄位改為單一下拉與新增按鈕；選單寬度使用持續保存的實測欄位寬度，重繪不會退回預設值。暫存路徑可明確保存或移除，移除選中路徑後同步目前設定並關閉舊選單。
- 底列為 52px 圓角下載／佇列分段按鈕；加入佇列先維持待開始，個別或批次開始才交給工作程序。直播保留直接下載。

## 2026-11-10 頂列、選單與 Twitch DVR

- `chrome_view.rs` 對照原版標題列及下載導覽快照：僅保留左右返回／前進，頂列移除 Flowmoor icon。macOS 使用原生交通燈，Windows 交由 GPUI TitleBar 提供視窗按鈕；此輪僅實測 macOS。下載快照保留欄位、編碼、畫質、頻道分頁與捲動位置。
- 側欄保留原版錨標誌，SVG alpha mask 依目前文字主題染色；深淺色入口移入左下設定。`hover_glow.rs` 以原版 stiffness 1400／damping 30／mass 0.16 移動高亮，支援 reduced motion。
- `menu_view.rs`／`dropdown_controls.rs`：44px 選項、來源 padding／圓角／陰影、勾選、鍵盤上下與 Enter、Escape／外點關閉、關閉後恢復焦點。開啟 240ms、關閉 120ms；路徑選單 80／40ms，沿用來源曲線，只改繪製位置與透明度。
- `paste_control.rs`：貼上並取得最小寬 130px、高 44px、左右 20px；180ms 文字與剪貼簿圖示位移。
- `metadata.rs` 使用獨立的官方 yt-dlp macOS onedir 執行檔；資訊查詢不再等待縮圖轉檔。VOD 快取 180 秒、直播 5 秒，Cookie 路徑及修改時間納入快取。縮圖背景下載、60 秒快取與原子更新。
- `twitch_channel_view.rs` 對照 `TwitchChannelPanel.jsx` 的頻道標頭、LIVE 卡片、左側 VOD 標題／數量、112×63 縮圖、右下時間、觀看數／日期及 hover。依確認不加入 emotes 或載入未公開。
- `twitch.rs` 對照原版 `gqlClient.cjs`／`downloadEngine.cjs`：取得播放 token、usher HLS 品質與 SHA-1／CDN DVR 探測，快取同一場直播的探測結果；使用 FFmpeg 直接擷取並沿用共用工作進度、停止、重試及後編碼。DVR 範圍從清單起點定位，不把輸入秒數當作 live edge 偏移。重試保留前次片段，輸出加上 attempt 名稱。
- Twitch 直播預覽使用取得的串流，沒有 Twitch live iframe。`playlist_server.rs` 把當下 DVR 固定為 ENDLIST，並在 loopback 代理該清單明列的片段／金鑰；保留片段副檔名與 HTTP Range，避免 Safari 跳向直播末端及跨來源讀取失敗。只接受數字索引，不提供任意 URL 代理。WebView 預載影格；macOS App 僅開放本機網路 ATS。

下載仍直連來源；本機預覽代理不是下載中繼。尚未實測長時間斷線／直播切換新場次、Windows／Linux 與登入受限來源。現有 CSS 遮罩與折射差異仍保留於上節，不宣稱逐像素相同。

macOS 的 App Info.plist 與相對路徑啟動器保存於 assets/macos；`./run.sh` 會編譯並建立同一 bundle。工具、資產與偏好設定皆屬 Rust 專案，不在啟動時讀取原版 Flowmoor。

## 2026-11-10 下載媒體庫與基本設定

- 移除內容區「返回頻道」；頂列共用 24 步導覽快照，保存頁面、影片／頻道、載入中查詢、表單、媒體庫選取與捲動。頻道選 VOD 尚未載完的返回／前進，以實際影片 URL 重新查詢。
- 進度頁只有原版「處理中／待處理」兩區，保留調整分隔、取消、重試、停止、待處理排序與清除已結束；移除全部／下載中／已完成／需要處理分類。列 padding 16×12、標題 14、狀態 20 高、圖示操作 44、進度 6px。
- 媒體庫對照 MediaLibraryTab：max-width 1152、內容 px32／pt24；搜尋與排序 48px 高，basis 320／224 並一起伸縮。縮圖 96×56、間隔 16；列外 px8／py12、主操作內 px8／py4；右側資料夾／檔案／選單為 32／44／32。搜尋無結果為 py40 的純文字。
- 媒體庫只保存下載結果與型別化 Request；支援搜尋、日期／語言標題排序、本機影音播放、重新下載、修改／複製參數、貼回下載頁、複製路徑、開檔／資料夾與移除清單項目。日期使用完成時間；既有項目缺少該時間時保留原日期。語言排序採 [ICU4X](https://docs.rs/icu_collator/2.4.0/icu_collator/struct.CollatorBorrowed.html)，避免以 Unicode 碼位代替原版 localeCompare。
- library.json 與設定同目錄；以暫存檔換名保存，讀取失敗不覆寫原資料。從清單移除不刪媒體檔；清除進度也保留媒體庫。未開始就取消的工作不加入媒體庫。
- 本機預覽沿用同一播放器及 Range 串流服務。MKV／MKA 首次會快取 MP4 remux，沒有重新編碼；大型檔案會有額外準備時間及磁碟用量，播放仍依系統 WebView 的解碼支援。放大時只有一個原生 WebView，離開頁面會隱藏，Escape 還原。
- 頻道名稱與 32px 真實圓形頭像可點擊前往頻道；Twitch VOD 缺少 channel_url 時以 uploader_id 補足。一般 VOD 使用原版遞增查詢視窗，避免 after 游標的 IntegrityCheckFailed；底部哨兵自動載入，失敗保留重試操作。
- 設定只保留一般／下載／語言／外觀／關於。導覽寬 224、padding12、列高46、icon20／gap12；頁首 px16／pt16／pb8、標題18，內容 p24／gap24、欄位 max576；全寬底列 p24、按鈕44。左側與設定內側使用獨立 spring 高亮狀態。
- 設定草稿支援保存／取消、資料夾清單還原、繁中／英文與深淺色即時預覽及重開保留。沒有精彩分數、轉錄、字幕、剪輯、聊天室或雲端相關頁面與空入口。

## 2026-11-10 預覽按鈕外觀

- 對照 TimeRangeInput、PreviewPlayButton 與 MediaPreviewControls，下載頁與媒體庫共用按鈕樣式。圖示按鈕固定 44×44、圓角 22；起點／終點至少 96×44、左右 padding 12、內容間隔 6、圖示 14 與文字 11。播放圖示 16 並右移 2。
- hover 以 Button 本身的圓角繪製；一般圖示使用來源 surface-raised／motion-hover-highlight 色彩，起點為 emerald-500/10、終點為 rose-500/10，文字同步變色。停用透明度 35%，不觸發子圖示 hover。
- 控制列圓角 16、padding 8×6、間隔 6、surface-raised/80；音量按鈕內層為 36px 圓形，外層與 160px 展開區共用 surface/70 膠囊。音量展開時隱藏放大按鈕；放大狀態切換還原圖示。保留按鈕無障礙名稱。
- 音量拖曳、1% 鍵盤步進與靜音共用更新入口，保存上次正音量；點擊播放／跳轉時清除音量區殘留焦點。
- 本輪對齊按鈕形狀與色彩；音量軌道與滑塊仍採 GPUI 元件，未完成整條控制列的像素差分或動畫逐幀比較。
