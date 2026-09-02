#![no_std]
extern crate alloc;

#[global_allocator]
static ALLOC: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

mod memory;

use alloc::format;
use asr::{future::next_tick, print_message, timer::TimerState, Address64, Process};

use crate::memory::Memory;

asr::async_main!(stable);
asr::panic_handler!();

macro_rules! check_state_changed {
    ($var:ident, $state:ident) => {
        if $var != $state.$var {
            print_message(&format!("{} = {}", stringify!($var), $var));
        }
    };

    ($var:ident, $state:ident.$state_var:ident) => {
        if $var != $state.$state_var {
            print_message(&format!("{} = {}", stringify!($var), $var));
        }
    };
}

macro_rules! check_update_state_changed {
    ($var:ident, $state:ident) => {
        check_state_changed!($var, $state);
        $state.$var = $var;
    };

    ($var:ident, $state:ident.$state_var:ident) => {
        check_state_changed!($var, $state.$state_var);
        $state.$state_var = $var;
    };
}

#[allow(unused_macros)]
macro_rules! read_and_check_update_state {
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

        check_update_state_changed!($var, $state);
    };
}

struct AutosplitterState {
    was_loading_screen: bool,

    #[cfg(debug_assertions)]
    streaming_paused: bool,
    #[cfg(debug_assertions)]
    is_teleporting: bool,

    #[cfg(debug_assertions)]
    is_faded_out: bool,

    #[cfg(debug_assertions)]
    surrogate_active: bool,
    #[cfg(debug_assertions)]
    surrogate_currently_open_panel: i32,
    #[cfg(debug_assertions)]
    surrogate_allow_menu_close: bool,

    #[cfg(debug_assertions)]
    player_unloaded: bool,

    #[cfg(debug_assertions)]
    game_time_paused: bool,
}

impl AutosplitterState {
    pub fn new() -> AutosplitterState {
        AutosplitterState {
            was_loading_screen: false,

            #[cfg(debug_assertions)]
            streaming_paused: false,
            #[cfg(debug_assertions)]
            is_teleporting: false,

            #[cfg(debug_assertions)]
            is_faded_out: false,

            #[cfg(debug_assertions)]
            surrogate_active: false,
            #[cfg(debug_assertions)]
            surrogate_currently_open_panel: -1,
            #[cfg(debug_assertions)]
            surrogate_allow_menu_close: false,

            #[cfg(debug_assertions)]
            player_unloaded: false,

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
    let is_teleporting: bool = memory
        .deref(&memory.level_streaming.is_teleporting)
        .unwrap_or_default();

    let is_faded_out: bool = memory
        .deref(&memory.gui_menu.fade_out_time)
        .is_ok_and(|t: f32| t == -1.0);

    let surrogate_active: bool = memory
        .deref(&memory.checkpoint_menu.is_active)
        .unwrap_or_default();
    let surrogate_currently_open_panel: i32 = memory
        .deref(&memory.checkpoint_menu.currently_open_panel)
        .unwrap_or_default();
    let surrogate_allow_menu_close: bool = memory
        .deref(&memory.checkpoint_menu.allow_menu_close)
        .unwrap_or_default();

    let player_unloaded: bool = memory
        .deref(&memory.player_controller.instance)
        .is_ok_and(|addr: Address64| addr.is_null());

    let is_loading_screen =
        is_faded_out && (is_teleporting || player_unloaded || state.was_loading_screen);

    let game_time_paused = (streaming_paused && !player_unloaded)
        || is_loading_screen
        || (surrogate_currently_open_panel == -1 && !surrogate_allow_menu_close);
    if game_time_paused {
        asr::timer::pause_game_time();
    } else {
        asr::timer::resume_game_time();
    }

    #[cfg(debug_assertions)]
    {
        check_state_changed!(is_loading_screen, state.was_loading_screen);

        check_update_state_changed!(streaming_paused, state);
        check_update_state_changed!(is_teleporting, state);

        check_update_state_changed!(is_faded_out, state);

        check_update_state_changed!(surrogate_active, state);
        check_update_state_changed!(surrogate_currently_open_panel, state);
        check_update_state_changed!(surrogate_allow_menu_close, state);

        check_update_state_changed!(player_unloaded, state);

        check_update_state_changed!(game_time_paused, state);
    }

    state.was_loading_screen = is_loading_screen;
}
