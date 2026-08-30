#![no_std]
extern crate alloc;

#[global_allocator]
static ALLOC: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

mod memory;

use alloc::format;
use asr::{future::next_tick, print_message, timer::TimerState, Process};

use crate::memory::Memory;

asr::async_main!(stable);
asr::panic_handler!();

macro_rules! check_state_changed {
    ($var:ident, $state:ident) => {
        if $var != $state.$var {
            print_message(&format!("{} = {}", stringify!($var), $var));
            $state.$var = $var
        }
    };
}

#[allow(unused_macros)]
macro_rules! read_and_check_state {
    ($var:ident, $memory:ident, $class:ident, $inst_var:ident, $state:ident) => {
        let $var = $memory.deref(&$memory.$class.$inst_var);

        if $var.is_err() {
            print_message(&format!(
                "Couldn't read {}: {:?}",
                stringify!($var),
                $var.as_ref().unwrap_err()
            ));
        }

        let $var = $var.unwrap_or_default();

        check_state_changed!($var, $state);
    };
}

struct AutosplitterState {
    #[cfg(debug_assertions)]
    streaming_paused: bool,
    #[cfg(debug_assertions)]
    last_teleport_time: f64,
    #[cfg(debug_assertions)]
    finish_teleport_time: f64,

    #[cfg(debug_assertions)]
    fade_out_time: f32,
    #[cfg(debug_assertions)]
    surrogate_active: bool,
    #[cfg(debug_assertions)]
    surrogate_currently_open_panel: i32,
    #[cfg(debug_assertions)]
    surrogate_allow_menu_close: bool,

    #[cfg(debug_assertions)]
    game_time_paused: bool,
}

impl AutosplitterState {
    pub fn new() -> AutosplitterState {
        AutosplitterState {
            #[cfg(debug_assertions)]
            streaming_paused: false,
            #[cfg(debug_assertions)]
            last_teleport_time: 0.0,
            #[cfg(debug_assertions)]
            finish_teleport_time: 0.0,

            #[cfg(debug_assertions)]
            fade_out_time: 0.0,
            #[cfg(debug_assertions)]
            surrogate_active: false,
            #[cfg(debug_assertions)]
            surrogate_currently_open_panel: -1,
            #[cfg(debug_assertions)]
            surrogate_allow_menu_close: false,

            #[cfg(debug_assertions)]
            game_time_paused: false,
        }
    }
}

async fn main() {
    asr::print_message("Hello, World!");

    let mut state = AutosplitterState::new();

    asr::print_message("main: AutosplitterState new...");

    loop {
        let process = Process::wait_attach("GRIME II.exe").await;

        print_message(&format!(
            "Found process path = {}",
            process.get_path().unwrap_or_default()
        ));
        process
            .until_closes(async {
                print_message("main: until_closes...");
                let memory = Memory::wait_attach(&process).await;
                print_message("main: starting loop...");
                loop {
                    handle_loads(&mut state, &memory);
                    next_tick().await;
                }
            })
            .await;
    }
}

fn handle_loads(state: &mut AutosplitterState, memory: &Memory) {
    if asr::timer::state() != TimerState::Running {
        return;
    }

    let streaming_paused: bool = memory
        .deref(&memory.level_streaming.is_level_streaming_paused)
        .unwrap_or_default();
    let last_teleport_time: f64 = memory
        .deref(&memory.level_streaming.last_teleport_time)
        .unwrap_or_default();
    let finish_teleport_time: f64 = memory
        .deref(&memory.level_streaming.finish_teleport_time)
        .unwrap_or_default();

    let fade_out_time: f32 = memory
        .deref(&memory.gui_menu.fade_out_time)
        .unwrap_or_default();

    let surrogate_active: bool = memory
        .deref(&memory.checkpoint_menu.is_active)
        .unwrap_or_default();
    let surrogate_currently_open_panel: i32 = memory
        .deref(&memory.checkpoint_menu.currently_open_panel)
        .unwrap_or_default();
    let surrogate_allow_menu_close: bool = memory
        .deref(&memory.checkpoint_menu.allow_menu_close)
        .unwrap_or_default();

    let game_time_paused = streaming_paused
        || fade_out_time == -1.0
        || (surrogate_currently_open_panel == -1 && !surrogate_allow_menu_close)
        || (last_teleport_time > finish_teleport_time && !surrogate_active);
    if game_time_paused {
        asr::timer::pause_game_time();
    } else {
        asr::timer::resume_game_time();
    }

    #[cfg(debug_assertions)]
    {
        check_state_changed!(streaming_paused, state);
        check_state_changed!(last_teleport_time, state);
        check_state_changed!(finish_teleport_time, state);

        check_state_changed!(fade_out_time, state);

        check_state_changed!(surrogate_active, state);
        check_state_changed!(surrogate_currently_open_panel, state);
        check_state_changed!(surrogate_allow_menu_close, state);

        check_state_changed!(game_time_paused, state);
    }
}
