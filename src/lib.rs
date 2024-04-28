#![no_std]

use asr::{future::next_tick, Process};

asr::async_main!(stable);
asr::panic_handler!();

async fn main() {
    // TODO: Set up some general state and settings.

    asr::print_message("Hello, World!");

    loop {
        let process = Process::wait_attach("Throes of the Javelin.exe").await;
        process
            .until_closes(async {
                // TODO: Load some initial information from the process.
                loop {
                    // TODO: Do something on every tick.
                    next_tick().await;
                }
            })
            .await;
    }
}
