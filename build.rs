fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(feature = "desktop")]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rerun-if-changed=assets/windows/flowmoor.rc");
        println!("cargo:rerun-if-changed=assets/windows/Flowmoor.ico");
        embed_resource::compile_for(
            "assets/windows/flowmoor.rc",
            ["flowmoor-rust"],
            embed_resource::ParamsIncludeDirs(["assets/windows"]),
        )
        .manifest_required()
        .expect("failed to embed the Flowmoor Windows icon");

        if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
            // GPUI's debug render/event frames exceed the default 1 MiB Windows stack.
            println!("cargo:rustc-link-arg-bin=flowmoor-rust=/STACK:8388608");
        }
    }
}
