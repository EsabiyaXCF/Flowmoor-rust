# 貢獻指南

本版聚焦 YouTube／Twitch 下載與下載媒體庫。修正應盡量落在共用 Rust 核心，讓桌面介面與 CLI 有一致行為；不要加入未實作的頁面或空服務。

## 本機檢查

```sh
cargo fmt --all -- --check
cargo test --locked --no-default-features
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --lib --bin flowmoor-rust
node tests/scripts/test-preview.cjs
cargo fetch --locked
python3 scripts/generate-third-party-notices.py
python3 tests/scripts/test-third-party-notices.py
python3 scripts/prepare-source.py --check
```

核心工作測試不連線真實平台；包含失敗重試、取消程序樹、暫停續傳、直播停止、串流輸出與編碼。桌面更動另需原生實機驗收。真實下載為明確啟動的手動測試，使用 [統一測試入口](docs/guides/FLOWMOOR_COMPLETE_DOWNLOAD_TEST_GUIDE.md)。

`tests/platform_process.rs` 在 Windows／macOS／Linux 使用真正的子程序，驗證參數傳遞、重試、停止隔離、暫停續傳、取消程序樹及 CLI 的 stdin 取消。設定 `FLOWMOOR_TEST_FFMPEG` 為 FFmpeg 路徑後，會另用本機產生的音訊驗證正常停止與 FFprobe 可讀性；Windows CI 包含這項驗證。`tests/worker.rs` 的 shell fixture 仍限定 Unix。

變更 Cargo.lock、Rust 工具鏈或素材後，重新產生第三方聲明，提交相關來源與授權。CI 會為自己的工具鏈重新產生聲明，因此標準函式庫盤點可能與本機快照不同。

回報問題時提供作業系統、工具版本、操作步驟與預期／實際結果；診斷內容先移除 Cookie、權杖、簽章 URL 與個人絕對路徑。不要提交 `.tools/`、`target/`、`test-output/` 或使用者設定與媒體庫。

提交的原創貢獻適用本專案 MIT 授權；第三方內容須保留自己的授權與來源。
