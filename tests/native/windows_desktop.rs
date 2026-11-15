//! Opt-in native regression test; no mouse/keyboard automation.
//! Run with an isolated APPDATA and FLOWMOOR_DESKTOP_TEST_OUTPUT.
use super::*;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[::core::prelude::v1::test]
#[ignore = "Requires a Windows desktop, WebView2, media tools and network"]
fn native_download_and_preview() {
    let output = PathBuf::from(
        std::env::var_os("FLOWMOOR_DESKTOP_TEST_OUTPUT")
            .expect("Set FLOWMOOR_DESKTOP_TEST_OUTPUT to a test directory"),
    );
    let profile = PathBuf::from(std::env::var_os("APPDATA").expect("APPDATA"));
    assert!(
        profile.starts_with(&output),
        "APPDATA must be isolated beneath test output"
    );
    std::fs::create_dir_all(&output).unwrap();
    let outcome = Arc::new(Mutex::new(None::<Result<(), String>>));
    let result = outcome.clone();
    gpui_kit::application()
        .with_assets(flowmoor_assets::FlowmoorAssets)
        .run(move |cx| {
            gpui_kit::init(cx);
            chrome_view::init(cx);
            theme::apply(true, cx);
            let (handle, app) = gpui_kit::open_window(
                WindowOptions {
                    show: true,
                    focus: false,
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        point(px(0.), px(0.)), size(px(1100.), px(800.)),
                    ))),
                    ..Default::default()
                },
                cx,
                |window, cx| cx.new(|cx| DownloadApp::new(window, cx)),
            ).unwrap();
            cx.spawn(async move |cx| {
                let started = Instant::now();
                let mut phase = 0;
                let mut ticks = 0;
                loop {
                    cx.background_executor().timer(Duration::from_millis(250)).await;
                    let step = handle.update(cx, |_, window, cx| {
                        app.update(cx, |this, cx| -> Result<bool, String> {
                            if started.elapsed() > Duration::from_secs(80) {
                                return Err(format!("native smoke timeout in phase {phase}: {:?}", this.library_preview.as_ref().map(|p| &p.read(cx).state)));
                            }
                            if let Some(error) = &this.error {
                                return Err(error.clone());
                            }
                            match phase {
                                0 => {
                                    this.url.update(cx, |v, cx| v.set_value(
                                        "https://www.youtube.com/watch?v=jNQXAC9IVRw", window, cx));
                                    phase = 1;
                                }
                                1 => {
                                    this.url.read(cx).focus_handle(cx).focus(window, cx);
                                    this.inspect(window, cx);
                                    phase = 2;
                                }
                                2 if this.info.is_some() => {
                                    println!("Native metadata: {}", this.info.as_ref().unwrap().title);
                                    this.open_preview(window, cx);
                                    phase = 3;
                                }
                                3 => {
                                    ticks += 1;
                                    if ticks < 12 { return Ok(false); }
                                    println!("Remote preview observed; loaded={}",
                                        this.preview.as_ref().is_some_and(|p| p.read(cx).state.loaded));
                                    this.close_preview(cx);
                                    this.output.update(cx, |v, cx| v.set_value(
                                        output.to_string_lossy().into_owned(), window, cx));
                                    this.start.update(cx, |v, cx| v.set_value("0", window, cx));
                                    this.end.update(cx, |v, cx| v.set_value("3", window, cx));
                                    for (selector, value) in [
                                        (&this.quality, "144p"), (&this.video_codec, "H.264"),
                                        (&this.audio_codec, "AAC"),
                                    ] {
                                        selector.update(cx, |v, cx| v.set_selected_value(
                                            &SharedString::from(value), window, cx));
                                    }
                                    this.preferences.language = "en".into();
                                    this.save_preferences(window, cx)?;
                                    phase = 4;
                                }
                                4 => {
                                    this.add(true, cx);
                                    phase = 5;
                                }
                                5 => {
                                    if let Some(job) = this.manager.jobs.last() {
                                        let s = job.snapshot();
                                        if s.phase.terminal() && s.phase != Phase::Completed {
                                            return Err(format!("Native download failed: {s:?}"));
                                        }
                                        if s.phase == Phase::Completed
                                            && this.library.entries.iter().any(|e| e.result.id == s.id)
                                        {
                                            println!("Native download completed: {:?}", s.files);
                                            let saved = flowmoor_rust::library::Library::load(
                                                &Preferences::path().with_file_name("library.json"))?;
                                            if !saved.entries.iter().any(|e| e.result.id == s.id) {
                                                return Err("Completed download not persisted".into());
                                            }
                                            if Preferences::load(&Preferences::path())?.language != "en" {
                                                return Err("Settings were not persisted".into());
                                            }
                                            this.navigate_to(Page::Library, cx);
                                            this.library_selected = Some(s.id);
                                            this.prepare_library_player(s.id, cx);
                                            phase = 6;
                                        }
                                    }
                                }
                                6 => {
                                    if let Some(player) = &this.library_preview
                                        && player.read(cx).state.loaded
                                    {
                                        player.update(cx, |p, cx| p.toggle(cx));
                                        phase = 7;
                                    }
                                }
                                7 => {
                                    if let Some(player) = &this.library_preview
                                        && player.read(cx).state.playing
                                        && player.read(cx).state.time > 0.25
                                    {
                                        println!("Native WebView2 playback advancing: {:.3}s",
                                            player.read(cx).state.time);
                                        player.update(cx, |p, cx| p.seek_to(1., cx));
                                        phase = 8;
                                    }
                                }
                                8 => {
                                    if let Some(player) = &this.library_preview
                                        && player.read(cx).state.time >= 1.
                                    {
                                        this.navigate_to(Page::Settings, cx);
                                        if this.library_preview.is_some() {
                                            return Err("Player remained after navigation".into());
                                        }
                                        phase = 9;
                                        ticks = 0;
                                    }
                                }
                                9 => {
                                    ticks += 1;
                                    if ticks >= 4 {
                                        println!("Native persistence, playback, seek and teardown passed");
                                        return Ok(true);
                                    }
                                }
                                _ => {}
                            }
                            cx.notify();
                            Ok(false)
                        })
                    }).map_err(|e| e.to_string()).and_then(|r| r);
                    match step {
                        Ok(false) => continue,
                        terminal => {
                            *result.lock().unwrap() = Some(terminal.map(|_| ()));
                            cx.update(|cx| cx.quit());
                            break;
                        }
                    }
                }
            }).detach();
        });
    outcome
        .lock()
        .unwrap()
        .take()
        .expect("Native app exited before test completion")
        .unwrap();
}
