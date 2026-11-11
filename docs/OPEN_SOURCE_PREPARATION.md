# 原始碼開源準備

本次僅準備本機可檢閱的原始碼，沒有建立遠端儲存庫、提交、推送或發布 Release。

## 已準備

- 本 Rust 專案採 MIT，Copyright (c) 2026 Flowmoor；散布時保留 LICENSE。原版 Flowmoor 的專有授權不受影響。
- README、貢獻與真實下載測試指南；下載功能、平台驗收及原生渲染差異均依實際狀態標示。
- Linux 核心、macOS 桌面與 Windows 核心／桌面 GitHub Actions；Windows 含實際 FFmpeg 正常停止檢查，尚未在 GitHub 遠端執行。CI 不自動下載真實平台影片，也不發布二進位。
- `.gitignore` 排除工具、建置產物、下載報告／媒體、Cookie、設定與本機設計工作紀錄。文件中的個人絕對路徑已移除。
- `python3 scripts/prepare-source.py --check` 檢查 Git 候選檔案；不帶 `--check` 產生含 SHA-256 清單的原始碼 ZIP。此掃描涵蓋常見權杖與個人路徑，不取代人工檢閱，也不掃描尚不存在的 Git 歷史。

## 發布前仍須核對

- 目前沒有 Git 歷史與 remote；公開帳號、儲存庫名稱及首次推送另行決定。
- 上游 `mac 0.1.1`、`pathfinder_geometry 0.5.1`、`seahash 4.1.0`、`simd_helpers 0.1.0` 未附可取得的獨立授權文本，目前保留宣告與精確來源連結；不可宣稱完整二進位授權審查已通過。詳見 third-party/README.txt。
- Flowmoor 素材來自原專案；Twitch 標誌的原始取得來源尚未獨立核實。MIT 不會授予他人的商標權，第三方標誌與素材仍需依各自條款確認。
- 此原始碼包不附 yt-dlp、FFmpeg、ffprobe、curl 或下載媒體；若未來附送外部工具，須依具體建置重新處理授權與對應原始碼義務。
- App 目前使用專案內資產，尚不是可任意搬移的安裝包。發行二進位前仍需處理資產封裝、平台實測及 macOS 簽署／公證。

原始碼包準備與下載驗收結果記於 docs/VALIDATION.md；不將本機通過當作所有平台、帳號權限或所有未來直播均可下載的保證。
