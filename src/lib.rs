// #![no_std]
extern crate alloc;

use alloc::format;
use asr::{
    future::next_tick,
    game_engine::unity::mono::{self, UnityPointer},
    Process,
};
use enum_primitive_derive::Primitive;
use num_traits::FromPrimitive;

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

                // asr::print_message("looking at classes");
                // for class in player_data_store.image.classes(&process, module) {
                //     let field = class
                //         .wait_get_static_instance(&process, module, "Name")
                //         .await;
                //     let name: ArrayCString<10> = process.read(field).unwrap_or_default();
                //     asr::print_message(name.validate_utf8().unwrap_or_default());
                // }

                // let class = player_data_store
                //     .image
                //     .wait_get_class(&process, module, "Player")
                //     .await;
                // asr::print_message(&format!(
                //     "{:?}",
                //     class.get_field_offset(&process, module, "state")
                // ));

                let mut berries = -1;
                let mut player_state = PlayerState::Idk;

                asr::print_message("Entering loop");
                loop {
                    let new_berries = player_data_store.get_berry_count(&process).unwrap_or(0);
                    if berries != new_berries {
                        asr::print_message(&format!("player now has {:?} berries", new_berries));
                        if new_berries > berries {
                            // Berry collected
                            asr::print_message("berry split");
                            // asr::timer::split();
                        } else if new_berries == 0 {
                            asr::print_message("berry reset");
                            // asr::timer::reset();
                        }
                    }
                    berries = new_berries;

                    let new_player_state = player_data_store
                        .get_player_state(&process)
                        .unwrap_or(PlayerState::Idk);
                    if player_state != new_player_state {
                        asr::print_message(&format!("player state is now {:?}", new_player_state));
                        if let PlayerState::AutoMoving = new_player_state {
                            asr::print_message("screen transition");
                        }
                    }
                    player_state = new_player_state;

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

    pub fn get_player_state(&self, process: &Process) -> Option<PlayerState> {
        let state: i32 = self
            .player_data_pointers
            .player_state
            .deref(process, &self.module, &self.image)
            .ok()?;
        PlayerState::from_i32(state)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Primitive)]
enum PlayerState {
    Normal = 0,
    Idk = 1,
    AutoMoving = 2,
}

struct PlayerDataPointers {
    berries: UnityPointer<2>,
    player_state: UnityPointer<3>,
}

impl PlayerDataPointers {
    pub fn new() -> Self {
        Self {
            berries: UnityPointer::new("GameManager", 0, &["instance", "berryCount"]),
            player_state: UnityPointer::new("Player", 0, &["instance", "state"]),
        }
    }
}
