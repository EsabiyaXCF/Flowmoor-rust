Flowmoor Rust third-party disclosures

THIRD_PARTY_NOTICES.html is the self-contained, offline document opened by
Settings > About > Third-party licenses & resources. It contains the Rust
dependency inventory, original license texts, embedded-asset notices, Rust
standard-library notices, and disclosures for external tools and services.
The desktop executable embeds this file. run.sh also copies it to
Contents/Resources/THIRD_PARTY_NOTICES.html in the macOS app bundle.

Regenerate after changing Cargo.lock, embedded resources, tool snapshots or
the Rust toolchain (Python 3.11+ and Cargo/rustc must be on PATH):

  python3 scripts/generate-third-party-notices.py
  python3 scripts/generate-third-party-notices.py --check
  python3 tests/scripts/test-third-party-notices.py

The default target is rustc's host. Use --target <triple> for another target;
the required crate sources must already be in Cargo's local cache. The
generator runs offline and does not download code. run.sh regenerates before
compilation so the embedded notice file matches that build's target/lockfile.

upstream/ preserves license files omitted from crate archives. sources.json
beside each snapshot records the upstream URL, generally pinned to the
package's .cargo_vcs_info.json commit. licenses/ contains additional original
texts and the yt-dlp macOS bundle's third-party notices. sources.json records
their provenance. assets/hls.LICENSE and assets/phosphor/LICENSE remain the
sources for the project's existing player and icon notices.

No standalone upstream license text was found for mac 0.1.1,
pathfinder_geometry 0.5.1, seahash 4.1.0 or simd_helpers 0.1.0. Their declared
licenses, authors and exact crate-source links are disclosed in the document.
Do not invent missing copyright notices. Resolve these source/notice gaps
before claiming the distribution has passed a complete licensing review.

External executables are not copied into the app by run.sh. If a distributor
adds yt-dlp, FFmpeg/ffprobe or curl, they must preserve the actual builds'
notices and fulfill applicable corresponding-source obligations. Merely
linking to a project's homepage is not a substitute for those obligations.
The local macOS yt-dlp snapshot is 2026.10.26.220202 (PyInstaller, GPLv3+);
the Unix zipimport fallback is 2026.10.26. Local FFmpeg 8.1.2 reports GPLv3+.
Different versions/builds must be reviewed separately.

The page is a disclosure, not permission to use internal platform APIs,
download content, use third-party trademarks or redistribute media. The
existing Twitch logo's original acquisition source and the ownership of
existing Flowmoor artwork have not been independently verified here.
