fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        // GPUI's debug render/event frames exceed the default 1 MiB Windows stack.
        println!("cargo:rustc-link-arg-bin=flowmoor-rust=/STACK:8388608");
    }
}
