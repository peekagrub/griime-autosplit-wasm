use alloc::{string::String, vec::Vec};
use asr::{
    future::next_tick,
    game_engine::unity::il2cpp::{Image, Module, UnityPointer},
    Address64, Process,
};
use bytemuck::CheckedBitPattern;

macro_rules! declare_pointers {
    ($g:ident {$( $f:ident : $t:ty = $e:expr ),*,}) => {
        pub struct $g {
            $(pub $f: $t),*,
        }

        impl $g {
            pub fn new() -> $g {
                $g {
                    $($f: $e),*,
                }
            }
        }
    };
}

declare_pointers!(LevelStreamingPointers {
    is_level_streaming_paused: UnityPointer<2> = UnityPointer::new("LevelStreaming_PauseHandler", 0, &["instance", "isLevelStreamingPaused"]),
    is_teleporting: UnityPointer<2> = UnityPointer::new("LevelStreaming_Handler", 0, &["_instance", "isTeleportingToNewPosition"]),
});

declare_pointers!(GUIMenuPointers {
    fade_out_time: UnityPointer<2> = UnityPointer::new("GUI_LoadingScreenHandler", 0, &["instance", "fadeOutTime"]),
});

declare_pointers!(GUICheckpointOptionsMenu {
    is_active: UnityPointer<2> = UnityPointer::new("GUI_CheckpointOptionsMenu", 0, &["instance", "isActive"]),
    currently_open_panel: UnityPointer<2> = UnityPointer::new("GUI_CheckpointOptionsMenu", 0, &["instance", "currentlyOpenPanel"]),
    allow_menu_close: UnityPointer<3> = UnityPointer::new("GUI_CheckpointOptionsMenu", 0, &["instance", "closeMenuButton", "m_Interactable"]),
});

declare_pointers!(PlayerController {
    instance: UnityPointer<1> = UnityPointer::new("CharacterScript_Player_PlayerController", 0, &["instance"]),
});

pub struct Memory<'a> {
    pub process: &'a Process,
    pub module: Module,
    pub image: Image,
    pub level_streaming: LevelStreamingPointers,
    pub gui_menu: GUIMenuPointers,
    pub checkpoint_menu: GUICheckpointOptionsMenu,
    pub player_controller: PlayerController,
}

impl Memory<'_> {
    pub async fn wait_attach<'a>(process: &'a Process) -> Memory<'a> {
        asr::print_message("Memory wait_attach: Module wait_attach...");
        next_tick().await;

        loop {
            let module = Module::wait_attach_auto_detect(process).await;

            asr::print_message("Memory wait_attach: Module get_image...");
            next_tick().await;

            for _ in 0..0x10 {
                if let Some(image) = module.get_image(process, "AD_Scripts") {
                    asr::print_message("Memory wait_attach: got module and image");
                    return Memory {
                        process,
                        module,
                        image,
                        level_streaming: LevelStreamingPointers::new(),
                        gui_menu: GUIMenuPointers::new(),
                        checkpoint_menu: GUICheckpointOptionsMenu::new(),
                        player_controller: PlayerController::new(),
                    };
                }
                next_tick().await;
            }
        }
    }

    pub fn deref<T: CheckedBitPattern, const CAP: usize>(
        &self,
        p: &UnityPointer<CAP>,
    ) -> Result<T, asr::Error> {
        p.deref(self.process, &self.module, &self.image)
    }

    #[allow(dead_code)]
    pub fn read_string<const CAP: usize>(&self, p: &UnityPointer<CAP>) -> Option<String> {
        let addr: Address64 = self.deref(p).ok()?;
        let length: u32 = self.process.read(addr + 0x10).ok()?;

        if length >= 2048 {
            return None;
        }

        let wide_str: Vec<u16> = self.process.read_vec(addr + 0x14, length as usize).ok()?;
        String::from_utf16(&wide_str).ok()
    }
}
