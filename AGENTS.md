# Flowmoor-rust

使用 GPUI＋Rust，先只做下載；記錄、雲端同步、ASR、剪輯等未經要求不要實作。
風格延續 Flowmoor：通透、典雅、簡潔、扁平，以分隔線與留白建立層級。
不要讀取或改動舊專案 landing-page、github-public、public/version.json。
業務邏輯放在 library；GPUI 與 CLI 共用下載核心。未知輸入在 Request 邊界驗證一次。
不要為未來模組預先抽象、加空實作或無效按鈕。
下載驗收入口：tests/scripts/test-flowmoor-complete-download.cjs。
雲端功能未啟用；若未來接入，先遵守 ../RecordTimeLabel/docs/ARCHITECTURE_CLOUD_SYNC.md，共享 Core 不得分叉。
修改下載核心後執行 cargo test --no-default-features；桌面介面執行 cargo check --all-targets 並實機驗證。
