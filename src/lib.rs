// #![no_std]
extern crate alloc;

use alloc::format;
use asr::{
    future::next_tick,
    game_engine::unity::mono::{self, UnityPointer},
    Process,
};

asr::async_main!(stable);
// asr::panic_handler!();

async fn main() {
    // TODO: Set up some general state and settings.

    asr::print_message("Hello, World!");

    loop {
        let process = Process::wait_attach("Throes of the Javelin.exe").await;
        process
            .until_closes(async {
                // Load some initial information from the process.
                let player_data_store = GameManagerFinder::wait_attach(&process).await;

                let module = &player_data_store.module;
                let class = player_data_store
                    .image
                    .wait_get_class(&process, module, "GameManager")
                    .await;
                asr::print_message(&format!(
                    "{:?}",
                    class.get_field_offset(&process, module, "berryCount")
                ));

                let mut berries = Some(999);

                asr::print_message("Entering loop");
                loop {
                    let new_berries = player_data_store.get_berry_count(&process);

                    if berries != new_berries {
                        asr::print_message(&format!("player has {:?} berries", new_berries));
                    }
                    berries = new_berries;

                    next_tick().await;
                }
            })
            .await;
    }
}

struct GameManagerFinder {
    module: mono::Module,
    image: mono::Image,

    player_data_pointers: PlayerDataPointers,
}

impl GameManagerFinder {
    pub fn new(module: mono::Module, image: mono::Image) -> Self {
        Self {
            module,
            image,

            player_data_pointers: PlayerDataPointers::new(),
        }
    }

    pub async fn wait_attach(process: &Process) -> Self {
        // let pointer_size = process_pointer_size(process).unwrap_or(PointerSize::Bit64);
        // asr::print_message(&format!(
        //     "GameManagerFinder wait_attach: pointer_size = {:?}",
        //     pointer_size
        // ));
        asr::print_message("GameManagerFinder wait_attach: Module wait_attach_auto_detect...");
        next_tick().await;
        let mut found_module = false;
        let mut needed_retry = false;
        loop {
            let module = mono::Module::wait_attach_auto_detect(process).await;
            if !found_module {
                found_module = true;
                asr::print_message("GameManagerFinder wait_attach: module get_default_image...");
                next_tick().await;
            }
            for _ in 0..0x10 {
                if let Some(image) = module.get_default_image(process) {
                    asr::print_message("GameManagerFinder wait_attach: got module and image");
                    next_tick().await;
                    // return GameManagerFinder::new(pointer_size, module, image);
                    return GameManagerFinder::new(module, image);
                }
                next_tick().await;
            }
            if !needed_retry {
                needed_retry = true;
                asr::print_message("GameManagerFinder wait_attach: retry...");
                next_tick().await;
            }
        }
    }

    pub fn get_berry_count(&self, process: &Process) -> Option<i32> {
        self.player_data_pointers
            .berries
            .deref(process, &self.module, &self.image)
            .ok()
    }
}

struct PlayerDataPointers {
    berries: UnityPointer<3>,
}

impl PlayerDataPointers {
    pub fn new() -> Self {
        Self {
            berries: UnityPointer::new("GameManager", 0, &["instance", "berryCount"]),
        }
    }
}
