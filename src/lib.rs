// #![no_std]
extern crate alloc;

use alloc::format;
use asr::{
    future::{next_tick, retry},
    game_engine::unity::mono::{self, UnityPointer},
    time::Duration,
    watcher::Watcher,
    Process,
};

asr::async_main!(stable);
// asr::panic_handler!();

const GAME_FIXED_DELTA_TIME: f32 = 1.0 / 60.0;

static PROCESS_NAMES: [&str; 1] = ["Throes of the Javelin.exe"];

async fn main() {
    // Set up some general state and settings.
    let splits = vec![
        Split::Start,
        Split::Key,
        Split::ScreenTransition,
        Split::ScreenTransition,
        Split::ScreenTransition,
        Split::ScreenTransition,
        Split::Berry,
        Split::Berry,
        Split::ScreenTransition,
        Split::Berry,
        Split::Berry,
        Split::Berry,
        Split::Berry,
        Split::Berry,
        Split::Berry,
        Split::BigBerry,
    ];

    loop {
        asr::print_message("Trying to attach to the game...");

        let process = retry(|| {
            PROCESS_NAMES.into_iter().find_map(|name| {
                let p = Process::attach(name);
                if p.is_some() {
                    asr::print_message(&format!("Attached to {:?}", name))
                }
                p
            })
        })
        .await;
        process
            .until_closes(async {
                // Load some initial information from the process.

                let game = GameManagerFinder::wait_attach(&process).await;

                let mut controller = Controller::new(splits.clone());

                let mut key_collected = Watcher::<bool>::new();
                let mut berries = Watcher::<i32>::new();
                let mut transitions = Watcher::<i32>::new();
                let mut started = Watcher::<bool>::new();
                let mut finished = Watcher::<bool>::new();
                let mut current_frame = Watcher::<i32>::new();
                let mut speedrun_frame_start = Watcher::<i32>::new();

                asr::print_message("Entering loop");
                loop {
                    if let Some(pair) = current_frame.update(game.get_current_frame(&process)) {
                        if pair.changed() {
                            let frame = pair.current;
                            if let Some(pair) = speedrun_frame_start.pair {
                                let start = pair.current;
                                let game_time = (frame - start) as f32 * GAME_FIXED_DELTA_TIME;
                                asr::timer::set_game_time(Duration::seconds_f32(game_time));
                            }
                        }
                    }
                    speedrun_frame_start.update(game.get_speedrun_frame_start(&process));

                    if let Some(pair) = started.update(game.started(&process)) {
                        if pair.changed_to(&true) {
                            controller.split(Split::Start);
                        } else if pair.changed_to(&false) {
                            controller.reset();
                        }
                    }
                    if let Some(pair) = finished.update(game.finished(&process)) {
                        if pair.changed_to(&true) {
                            controller.split(Split::BigBerry);
                        }
                    }

                    if let Some(pair) = key_collected.update(game.get_key_collected(&process)) {
                        if pair.changed_to(&true) {
                            controller.split(Split::Key);
                        }
                    }

                    if let Some(pair) = berries.update(game.get_berry_count(&process)) {
                        if pair.increased() {
                            // controller.split(Split::Berry(pair.current));
                            controller.split(Split::Berry);
                        }
                    }

                    if let Some(pair) = transitions.update(game.get_transition_count(&process)) {
                        if pair.increased() {
                            controller.split(Split::ScreenTransition);
                        }
                    }

                    next_tick().await;
                }
            })
            .await;
    }
}

struct Controller {
    splits: Vec<Split>,
    next_split: usize,
}

impl Controller {
    pub fn new(splits: Vec<Split>) -> Self {
        Self {
            splits,
            next_split: 0,
        }
    }

    pub fn split(&mut self, split: Split) {
        asr::print_message(&format!("split {:?}", split));
        if let Some(&current) = self.splits.get(self.next_split) {
            if current == split {
                if let Split::Start = split {
                    asr::timer::start();
                } else {
                    asr::timer::split();
                }
                self.next_split += 1;
            }
        }
    }

    pub fn reset(&mut self) {
        asr::timer::reset();
        self.next_split = 0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Split {
    Start,
    Key,
    Berry,
    ScreenTransition,
    BigBerry,
}

struct GameManagerFinder {
    module: mono::Module,
    image: mono::Image,

    time_data_pointers: TimeDataPointers,
    player_data_pointers: PlayerDataPointers,
}

impl GameManagerFinder {
    pub fn new(module: mono::Module, image: mono::Image) -> Self {
        Self {
            module,
            image,

            time_data_pointers: TimeDataPointers::new(),
            player_data_pointers: PlayerDataPointers::new(),
        }
    }

    pub async fn wait_attach(process: &Process) -> Self {
        // let pointer_size = process_pointer_size(process).unwrap_or(PointerSize::Bit64);
        // asr::print_message(&format!(
        //     "GameManagerFinder wait_attach: pointer_size = {:?}",
        //     pointer_size
        // ));
        // asr::print_message("GameManagerFinder wait_attach: Module wait_attach_auto_detect...");
        next_tick().await;
        let mut found_module = false;
        let mut needed_retry = false;
        loop {
            let module = mono::Module::wait_attach_auto_detect(process).await;
            if !found_module {
                found_module = true;
                // asr::print_message("GameManagerFinder wait_attach: module get_default_image...");
                next_tick().await;
            }
            for _ in 0..0x10 {
                if let Some(image) = module.get_default_image(process) {
                    // asr::print_message("GameManagerFinder wait_attach: got module and image");
                    next_tick().await;
                    // return GameManagerFinder::new(pointer_size, module, image);
                    return GameManagerFinder::new(module, image);
                }
                next_tick().await;
            }
            if !needed_retry {
                needed_retry = true;
                // asr::print_message("GameManagerFinder wait_attach: retry...");
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

    pub fn get_key_collected(&self, process: &Process) -> Option<bool> {
        self.player_data_pointers
            .key_collected
            .deref(process, &self.module, &self.image)
            .ok()
    }

    pub fn get_transition_count(&self, process: &Process) -> Option<i32> {
        self.player_data_pointers
            .transition_count
            .deref(process, &self.module, &self.image)
            .ok()
    }

    pub fn started(&self, process: &Process) -> Option<bool> {
        self.time_data_pointers
            .started
            .deref(process, &self.module, &self.image)
            .ok()
    }

    pub fn finished(&self, process: &Process) -> Option<bool> {
        self.time_data_pointers
            .finished
            .deref(process, &self.module, &self.image)
            .ok()
    }

    pub fn get_current_frame(&self, process: &Process) -> Option<i32> {
        self.time_data_pointers
            .current_frame
            .deref(process, &self.module, &self.image)
            .ok()
    }

    pub fn get_speedrun_frame_start(&self, process: &Process) -> Option<i32> {
        self.time_data_pointers
            .speedrun_frame_start
            .deref(process, &self.module, &self.image)
            .ok()
    }
}

struct TimeDataPointers {
    started: UnityPointer<2>,
    finished: UnityPointer<2>,
    current_frame: UnityPointer<2>,
    speedrun_frame_start: UnityPointer<2>,
}

impl TimeDataPointers {
    pub fn new() -> Self {
        Self {
            started: UnityPointer::new("UIManager", 0, &["instance", "speedrunStarted"]),
            finished: UnityPointer::new("UIManager", 0, &["instance", "endedSpeedrun"]),
            current_frame: UnityPointer::new("FrameManager", 0, &["instance", "currentFrame"]),
            speedrun_frame_start: UnityPointer::new(
                "UIManager",
                0,
                &["instance", "speedrunFrameStart"],
            ),
        }
    }
}

struct PlayerDataPointers {
    berries: UnityPointer<2>,
    key_collected: UnityPointer<2>,
    transition_count: UnityPointer<2>,
}

impl PlayerDataPointers {
    pub fn new() -> Self {
        Self {
            berries: UnityPointer::new("GameManager", 0, &["instance", "berryCount"]),
            key_collected: UnityPointer::new("GameManager", 0, &["instance", "hasCollectedKey"]),
            transition_count: UnityPointer::new("GameManager", 0, &["instance", "transitionCount"]),
        }
    }
}
