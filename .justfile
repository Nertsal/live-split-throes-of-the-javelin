list:
    just --list

debug:
    cargo build
    ~/.cargo/bin/asr-debugger --debug target/wasm32-unknown-unknown/debug/live_split_throes_of_the_javelin.wasm
