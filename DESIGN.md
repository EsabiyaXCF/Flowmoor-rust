# Flowmoor 原生下載介面

以原版 YouTwitchDownloader/src/config/theme.js、Sidebar.jsx、DownloadFormPanels.jsx、FormatControls.jsx 為設計與互動來源，不另發明主題。

所有色彩與字體集中於 src/theme.rs 的 GPUI Theme block；控件讀取同一主題。深色背景 #020408、表面 #05080E、文字 #F8FCFF、次要文字 #AEC2DB、分隔線 #223048、重點色 #B0C6E8。淺色背景 #F9FBFF、表面 #FFFFFF、文字 #101A2B、次要文字 #334560、分隔線 #DAE4F2、重點色 #306BE0。

側邊欄 clamp(180px,18vw,200px)，可收為 48px，沿用錨、下載與佇列圖示。工作區先貼連結查詢，成功後才展開原版順序的設定；下載佇列獨立頁面。控件圓角 10px，主要操作高 44px，版面以 8／16／20px 間距和細分隔線分組。原版紙船空狀態直接移植。

不加入未支援功能的空選單。Hallmark 原生調整：CSS/HTML 規則以 GPUI theme、焦點、彈出選單與原生對話框對應；動畫只用於來源既有的 hover 高亮、選單及貼上互動，尊重 reduced motion，不依賴動畫辨識狀態。

詳細來源與差異見 docs/UI_PORT.md。

頂列高 44px，使用系統視窗控制與返回／前進，沒有第二個 Flowmoor 標誌；錨與字標在側欄，依主題文字染色。深淺色入口位於左下設定。圖示使用原版 Phosphor SVG。Twitch 頻道延續原版 LIVE 卡片、分隔線與緊湊 VOD 列。

媒體庫延續原版 1152px 內容上限、32px 水平 padding、48px 搜尋／排序與 96×56 縮圖；設定使用 224px 內側導覽、24px 內容 padding 及全寬保存底列。下載進度只保留處理中／待處理，使用 6px 進度列，沒有狀態分類 tabs。
