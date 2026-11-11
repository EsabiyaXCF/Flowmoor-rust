# 2026-11-09 下載介面與功能驗收

## 自動檢查

- macOS Apple Silicon、Rust 1.99.1、GPUI Kit／gpui-webview 0.9.0、yt-dlp 2026.10.26、FFmpeg 8.1.2。
- `cargo test --no-default-features`：23 項通過；涵蓋來源與時間、編碼失敗保留來源、重試／永久錯誤、取消程序樹、暫停續傳與排序、三路同時啟動、直播停止後編碼、設定保存、頻道來源分類與分頁游標、時間拖曳精度／交叉規則、待開始佇列。
- `cargo clippy --all-targets -- -D warnings`、`cargo build --bins` 通過；`cargo fmt` 完成。
- `node tests/scripts/test-preview.cjs`：首次播放、定位、Twitch 平台按鈕與廣告時間隔離通過。

## 真實媒體

完整下載入口：`tests/scripts/test-flowmoor-complete-download.cjs`，用 FFprobe 檢查實際內容、編碼、高度與長度；報告及媒體留在 Git 忽略的 test-output。

| 來源／內容 | 設定 | 結果 |
|---|---|---|
| YouTube jNQXAC9IVRw 影音 | 0–3 秒、144p、H.264／AAC | 3.024 秒，編碼與雙流通過 |
| 同影片影音 | 0–3 秒、144p、VP9／Opus | 3.010 秒，編碼與雙流通過 |
| 同影片僅影片 | 0–3 秒、144p、AV1 | 3.000 秒，沒有音訊流 |
| 同影片僅音訊 | 0–3 秒、MP3 | 3.000 秒，沒有影片流 |
| Twitch VOD 2923868252 | 10–13 秒、360p、H.264／AAC | 3.029 秒，編碼與雙流通過 |
| Twitch hungry 正在直播 | 0–5 秒、360p、H.264／AAC | 5.037 秒，編碼與雙流通過 |
| YouTube @LofiGirl/live | 0–3 秒、144p、H.264／AAC | 3.027 秒，編碼與雙流通過 |

## 原生 UI

- YouTube 首次點擊 GPUI 播放按鈕即可播放；滑鼠定位、播放／暫停、開始與結束時間帶入確認。4–10 秒選取正確回填表單。
- 預設 144p 保存、套用並重新查詢影片後仍保留，重啟後輸出路徑保留。深淺色也存入 Rust 專用設定檔，設定 round-trip 測試涵蓋該值。
- YouTube @jawed 與 Twitch yuzukiririna 頻道清單、真實縮圖與選取進入下載表單確認。Twitch 第一次 30 筆，載入更多後共 41 筆並結束分頁。
- Twitch 頻道內選第一支設定 10–20 秒，返回後改選第二支，開始／結束皆已清空；換片不會下載錯誤片段。
- 從頻道選片後透過 UI 同時下載影音與 MP3；兩項均顯示完成。FFprobe 確認 3.026 秒 H.264／MP3 MKV 與 3.000 秒純 MP3。
- Twitch hungry 透過 UI 下載，位元組與速度隨擷取更新。手動停止後輸出可播放的 30.200333 秒 H.264／AAC 檔案。
- Twitch hungry 透過 UI 選擇純音訊 MP3，手動停止後完成編碼；FFprobe 確認為 108.181333 秒純 MP3，沒有影片流。
- 啟用官方 gpui-fast 合成後，播放器放回卡片內，與長表單共用捲動並正確裁切。

## 複查與限制

獨立來源複查指出的 4 項下載／預覽問題已修正：暫停排序、預設畫質被重設、廣告時間污染、多內容工作辨識。後續頻道複查的換片範圍殘留與網址分頁判斷亦已修正。

Hallmark `sloplint --scope app --render` 拒絕 .rs，未宣稱網頁 render gates 通過；以 Rust 檢查、JS 行為檢查及 CUA 原生操作驗收。

原專案來源檔案保持只讀；本次新增與修改位於 Flowmoor-rust。沒有改動 landing-page、github-public 或 public/version.json。

尚未驗證 Windows／Linux、長時間直播斷線恢復、私人／會員影片的登入預覽與 YouTube 從直播開頭的完整 DVR 邊界。嵌入播放權限與 Cookie 下載權限分開；不承諾直播缺段可補回。排程、自訂版面、聊天室、精彩紀錄與記錄同步依本輪範圍不實作。

## 2026-11-09 間距與互動補驗

- 時間欄位：單擊編輯、水平拖曳到 18–19 秒邊界、上下鍵調秒、Shift 調十秒，Escape 從被推動的 14–15 秒還原為編輯前的 4–8 秒，實機確認。
- 預覽：首次播放、內嵌裁切、範圍把手回填、放大與 Escape 還原確認；開啟及還原後會捲至播放器，整組時間尺與控制列可見。
- 新底列透過 UI 加入 5–8 秒 YouTube 工作，先顯示待開始／0 bytes；按開始後完成。FFprobe 確認輸出 3.000000 秒、H.264 與 AC3 雙流（自動編碼），媒體保留於 test-output/ui/flowmoor-1794234667849。
- 原生資料夾選擇器加入 test-output/ui-folder，選單顯示暫存；保存後移除暫存標記，移除後選單關閉且輸出回到 使用者原有下載資料夾。只移除本次測試清單項目，未刪除實體資料夾。
- 側欄展開／收合、收合模式點下載群組可展開、群組子項目收合、深淺色縮圖外觀已實機確認。
- 本輪獨立來源複查的時間欄位還原、範圍外把手、路徑移除保存與舊選單問題皆已修正；實機另發現並修正重疊預覽點擊、捲動未重繪及選單寬度在重繪時重設。

- 最終 16px rem 版本再次確認深淺色、資料夾選單與欄位等寬、清單文字與移除按鈕未溢出，以及底部下載／佇列分段按鈕。測試後還原淺色與原輸出路徑。

## 2026-11-10 原版導覽、選單與直連 Twitch 補驗

- `cargo test --no-default-features --offline`：29 項（15 library、14 worker）；`cargo test --bin flowmoor-rust --offline`：2 項（選單曲線、loopback Range／片段改寫／關閉）。嚴格 all-target Clippy、bins build、格式化與 JS bridge 檢查通過。新增播放拒絕錯誤不再被吞掉。
- 同一公開 YouTube 影片單次本機比較：舊資訊流程 4.104 秒，獨立 onedir 純資訊流程 1.737 秒，取得 25 個格式；這是本機樣本，不是跨來源效能保證。縮圖已與資訊查詢分離。
- 透過統一下載腳本重新驗證 YouTube jNQXAC9IVRw 的 0–3 秒／144p／H.264+AAC，實際 3.024000 秒，位於 test-output/flowmoor-1794240339291。
- 新 Twitch 直連 DVR：p1lepe1el1 的 10–13 秒／360p／H.264+AAC，實際 3.029 秒，位於 test-output/flowmoor-1794238861379。
- 新直連直播停止：共用 Manager 發出 stop 後，狀態為 Stopped，仍完成指定後編碼；FFprobe 確認 60.053 秒、360p、H.264+AAC，位於 test-output/live-stop-direct/flowmoor-1794240552637。一次性驗證程式放在 /tmp，沒有加入產品 CLI 選項。
- macOS CUA 確認一般 VOD 清單、真實縮圖／時長／觀看數／日期、LIVE 下載入口、來源圖示與側欄標誌染色；頂列沒有多餘標誌或模式按鈕。
- 固定 DVR 預覽經 CUA 確認從 0 播到 01:02，有真實影格，總長 4:48:00，沒有跳到直播末端；同一 loopback playlist 經 FFprobe 確認 17280.383 秒。先前只用 metadata 預載不呈現首影格，已改為 auto。
- 獨立 finish reviewer 檢查導覽、範圍、重試與 proxy；本輪修正重試覆蓋、大小寫頻道 DVR、快照自訂品質與捲動後高亮幾何。最後 proxy review 為 ship，未另有確定功能問題。

此輪沒有移植未公開 VOD 探測、emotes、聊天室、記錄、排程或自訂版面。Windows／Linux、長時間斷線恢復與跨直播場次仍未實測；目前原生玻璃與縮圖遮罩是近似，沒有宣稱像素完全一致。

- 最後版 CUA 再確認 auto 預載於 00:00 顯示真實首影格、時間尺 Home／Shift+Right 定位、設點 10–20 秒；頂列返回頻道／前進後該範圍保留，換選一般 VOD 時清除舊範圍。畫質下拉鍵盤選取／Escape 關閉與左下設定切換兩種主題、錨標誌跟隨染色皆確認。
- 一般 Twitch VOD 2923868252 再驗證 10–13 秒／360p／H.264+AAC：3.029000 秒，位於 test-output/flowmoor-1794240765860。
- macOS Info.plist 與 launcher 已保存於 assets/macos，run.sh 建立同一 App bundle；不依賴暫時手工產生的設定。

## 2026-11-10 下載媒體庫、設定與最後功能複查

- 核心檢查：17 library tests（含語言排序）、14 worker tests、3 desktop tests，共 34 個不同測試；no-default-features 路徑、嚴格 all-target Clippy、格式化、bins build 與 preview JS bridge 均納入檢查。標題測試比對 Node Intl.Collator 在繁中／英文的排序結果；媒體庫測試包含保存、更新、舊資料讀取及移除後保留實體檔案。
- 指定入口再次執行公開 YouTube 0–3 秒／144p／H.264+AAC：FFprobe 3.024000 秒，report 為 test-output/report-1794248535529-23244.json，媒體在 test-output/flowmoor-1794248535811。
- 透過原生 UI 建立影片與純音訊下載，兩筆結果重開後仍在媒體庫。影片 3.024000 秒／H.264+AAC，音訊 3.008 秒／Opus；實際播放、音量展開／靜音、定位、放大／Escape 均確認。修正放大重複焦點造成的 panic，以及離開頁面仍殘留原生 WebView。
- 媒體庫重新執行既有三秒下載已完成，FFprobe 確認仍為 3.024000 秒／H.264+AAC，位於 test-output/ui/flowmoor-1794248658654。完成列的綠色 6px 進度、開啟資料夾、關閉與清除已結束操作確認；清除進度後三筆下載項目仍在媒體庫。
- 原生 UI 確認搜尋空結果為原版純文字；下載專用操作選單、複製下載 Request JSON／貼回 URL 欄、修改參數、載入中 Back／Forward 與範圍／編碼還原通過。加入待處理後全部移除，不會產生額外媒體項目。
- Twitch 一般 VOD 原生自動捲底由 30 筆增為 43 筆，沒有 IntegrityCheckFailed；真實頭像、名稱前往頻道及頂列返回確認。原版額外「返回頻道」按鈕已移除。
- 最後 reviewer 指出的頻道 VOD 載入中 Back／Forward 分支已修正；原生重測從頻道選 2919227868，載入中返回再前進，恢復的是該 VOD 與下載表單。
- 設定確認取消還原深淺色及暫存資料夾變更；英文／深色保存後重開保留，最後還原繁中／淺色與 使用者原有下載資料夾。設定頁不殘留播放器，換頁不殘留側欄高亮。關於顯示版本與授權入口。

只改 Flowmoor-rust，原版 Flowmoor 原始碼維持只讀。未移植聊天室、精彩分數、轉錄／字幕、剪輯、記錄／雲端、未公開 VOD、排程與自訂版面。元件幾何依來源對齊並以 macOS 畫面核對；CSS 折射、縮圖遮罩與時間尺浮動影格縮圖仍有差異，沒有全畫面逐像素差分通過的宣稱。Windows／Linux、長時間斷線及跨直播場次仍未實測；大型 MKV 首次預覽需要完整快取 remux。

- 最後 Range 回歸曾重現分段 HTTP 標頭造成 ConnectionReset；共用服務已改為讀至完整標頭才回應，測試刻意分開 request line 與 Range 標頭傳送，以覆蓋實際故障。

- 最終重跑再確認縮圖與 uploader 會沿用並保存，新項目 completed_at 非空；FFprobe 3.024000 秒／H.264+AAC，位於 test-output/ui/flowmoor-1794250345428。原生再開此檔顯示真實首影格，返回清單後播放器正確隱藏。


## 2026-11-10 下載核心與開源準備

- MIT 依作者選擇加入此 Rust 專案，Copyright (c) 2026 Flowmoor；原版專有授權檔未改動。README、CONTRIBUTING、下載測試指南及 GitHub Actions 已準備，沒有推送或發布。
- 下載完成前排除不存在、非普通檔案及空檔；完成位元組改用最終檔案實際大小，清除殘留速度／ETA。找不到工具、權限拒絕及未提供的編碼器不進入無效重試。
- 後處理以共用受控程序探測來源編碼，相同編碼直接重新封裝；取消仍會回收探測程序。既有重試、暫停、停止、保留來源與程序樹測試持續通過。
- 補驗找到兩項 Twitch 差異並修正：無 DVR 不再把過去區間套到滑動直播視窗；未設定時間範圍時不再用當下 DVR 長度自動結束，持續到手動停止。
- 真正 DVR 的 1–4 秒原先只有 60 個影片影格，且影片起點比音訊晚 1.013 秒。改用有限播放清單快照、輸入端定位與一次必要編碼；修正後 3.029 秒、360p／H.264+AAC，影音起點皆 0。指定驗收腳本已加入影音起點差距檢查，時間長度誤差縮至 250ms。純音訊另重現等待下一個視訊關鍵影格造成的一秒截短，以 FFmpeg seek2any 修正，沒有增加視訊轉碼。

本輪實際下載（報告與媒體位於忽略的 test-output）：

| 來源／內容 | 設定 | 實際結果 |
|---|---|---|
| YouTube jNQXAC9IVRw 影音 | 0–3 秒／144p／H.264+AAC | 3.024 秒 |
| 同影片純影片 | 1–3 秒／144p／VP9 | 1.999 秒，無音訊 |
| 同影片純音訊 | 0–3 秒／MP3 | 3.000 秒，無影片 |
| Twitch VOD 2919227868 | 0–3 秒／360p／H.264+AAC | 3.054 秒 |
| YouTube @LofiGirl/live | 0–3 秒／144p／H.264+AAC | 3.027 秒 |
| Twitch caseoh_，解析確認 DVR=true | 1–4 秒／360p／H.264+AAC | 3.029 秒，影音起點均為 0 |
| 同一 DVR，較晚區段 | 601–604 秒／360p／H.264+AAC | 3.029 秒，影音起點均為 0 |
| 同一 DVR，純影片 | 11–14 秒／360p／VP9 | 3.000 秒，無音訊 |
| 同一 DVR，純音訊 | 61–64 秒／Opus | 3.0075 秒，無影片 |
| Twitch moistcr1tikal，解析確認 DVR=false | 無範圍，收到進度後手動停止 | Stopped、10.005 秒／H.264+AAC，最終 281418 bytes、無殘留速度與 ETA |

p1lepe1el1、hungry 在本輪連線時離線，正確回報而未宣稱成功。moistcr1tikal 早期 3.100 秒的範圍樣本實際來自滑動直播視窗，已列為發現問題的案例，不列為 DVR 通過證據；修正後這類請求明確拒絕。無 DVR 直播可能包含平台插入的廣告，沒有實作去廣告。

- 36 個不同 Rust 測試（17 library、16 worker、3 desktop），含無 default features 路徑、嚴格 all-target Clippy 與格式檢查。preview bridge JS、第三方聲明與 debug／release 封裝分支另行驗證。
- `./run.sh --release` 增加最佳化建置入口，34MB 原生程式成功建置。Cmd+Q 使用全域退出動作。
- 公開來源掃描排除工具、下載、編譯產物、設定／Cookie、常見憑證及個人絕對路徑；以刻意放入的假權杖與個人路徑確認會拒絕，再移除驗證用檔案。

尚未完成 GitHub 遠端 CI、Windows／Linux 桌面與長時間直播／斷線驗收。四個上游授權文本與 Twitch 標誌取得來源仍需核實，詳見 OPEN_SOURCE_PREPARATION.md；不能宣稱可攜式二進位發行或完整授權審查已通過。

- 純音訊 AAC 再確認 61–64 秒實際 3.008 秒；Opus 為 3.0075 秒。修正定位後再經較嚴格的 250ms 長度檢查通過。
- 原始碼 ZIP 含 239 個候選來源檔及 SHA-256 清單（不含工具、媒體、設定、建置目錄）；全新解壓與全新 target 目錄的核心 32 項測試及桌面 all-target check 通過，沒有引用原版 Flowmoor 或本機 `.tools/`。
- 使用者解鎖後完成最終 release 原生複驗；App bundle 執行檔 SHA-256 與 target/release/flowmoor-rust 一致。
- 原生表單下載 caseoh_ DVR 的 1–4 秒、360p／H.264+AAC；FFprobe 3.029 秒，影音起點均為 0，最終 148218 bytes。進度列顯示完成與 100%，total 與實體大小一致，speed／ETA 均已清除；媒體位於 test-output/ui/flowmoor-1794288098888。
- 新下載在媒體庫呈現真實首影格，按播放後時間由 0.067 前進到 2.376 秒；放大、Escape 還原與前往設定後關閉播放器均確認，沒有殘留 WebView 或 panic。
- 關於頁授權入口產生與內嵌來源完全相同的離線文件，含 Flowmoor Rust — MIT 及 Copyright (c) 2026 Flowmoor。Cmd+Q 後確認 com.flowmoor.rust.preview 的 isRunning=false。
- 僅還原本輪測試造成的預設輸出資料夾與重試次數，保留其他設定及下載媒體庫。

## 2026-11-10 預覽按鈕複驗

- 僅修改 src/preview.rs；對照來源的按鈕尺寸、圖示、間距、圓角、hover 與停用外觀，下載頁與媒體庫使用同一樣式入口。
- macOS release 原生畫面確認本機媒體的深淺色圓形按鈕與音量展開；設定還原為原本的淺色。控制列的停用按鈕及音量展開時隱藏放大按鈕均確認。
- 最終 release 確認音量 50% → 49% → 48% 的鍵盤調整；靜音歸零後取消靜音恢復 48%，沒有退回舊的 50%。點播放後音量收合、放大按鈕恢復可用。
- cargo clippy --all-targets --offline -- -D warnings 與 release 編譯通過；本次未改下載核心，未重跑網路下載矩陣。
- 本輪 YouTube jNQXAC9IVRw 與 Twitch VOD 2924625705 的內嵌預覽未就緒；原因未確定，遠端啟用後的起點／終點 hover 與播放功能尚未驗收。不可把本機控制列的成功視為遠端播放器已全面通過。
- 沒有完成整個畫面的逐像素差分；音量軌道／滑塊仍使用 GPUI 原生元件樣式。


## 2026-11-11 平台程序適配

- 外部工具統一經共用建構入口處理 Windows 隱藏視窗；桌面執行檔使用 Windows GUI subsystem，CLI 保留主控台。
- Windows 下載程序改用獨立隱藏主控台接收 Ctrl+C，另由 Job Object 管理程序樹；正常停止失敗或逾時不再當作成功保存。此路徑不再呼叫 taskkill，避免工具輸出混入 JSON。Unix 保留原有程序群組與訊號。
- Request 邊界拒絕 Windows 保留裝置檔名；所有平台採一致規則。下載驗收腳本改以 stdin 要求 CLI 取消，並等待程序樹回收。
- macOS 核心 17 library／16 worker 測試及新的 platform_process 整合測試通過；後者包含 Unicode／引號參數、重試、空輸出、停止隔離、暫停續傳、子程序回收與 CLI JSON／stdin 取消。設定 FLOWMOOR_TEST_FFMPEG 後，實際產生錄音、Ctrl+C 收尾與 FFprobe 檢查亦通過。
- x86_64-pc-windows-gnu 的 no-default-features all-targets 編譯檢查與嚴格 Clippy 通過；這是在 macOS 上交叉檢查，沒有執行 Windows 程式。
- 指定完整下載腳本完成公開 YouTube jNQXAC9IVRw，0–3 秒／144p／H.264+AAC；FFprobe 為 3.024000 秒，媒體位於忽略的 test-output/flowmoor-1794391194621。
- 已加入 Windows CI，含原生程序整合測試、FFmpeg 收尾及桌面建置檢查；尚未推送或執行遠端 CI，Windows GUI／播放器也尚未實機驗收。
