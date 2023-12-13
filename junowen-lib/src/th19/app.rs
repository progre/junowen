use getset::{CopyGetters, Getters, MutGetters, Setters};

#[repr(C)]
pub struct ControllerSelect {
    _unknown1: [u8; 0x14],
    pub cursor: u32,
    _prev_cursor: u32,
    pub max_cursor: u32,
    _unknown2: [u8; 0x80],
    pub depth: u32,
    // unknown remains...
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u32)]
pub enum ScreenId {
    TitleLoading,
    Title,
    GameLoading,
    Option,
    ControllerSelect,
    GameSettings,
    Unknown2,
    DifficultySelect,
    PlayerMatchupSelect,
    OnlineVSMode,
    CharacterSelect,
    Unknown3,
    Unknown4,
    Unknown5,
    Unknown6,
    MusicRoom,
    Unknown7,
    Unknown8,
    Manual,
    Manual2,
    Unknown10,
    Archievements,
    Unknown11,
    Unknown12,
    Unknown13,
    Unknown14,
    Unknown15,
    Unknown16,
}

#[repr(C)]
pub struct CharacterCursor {
    pub cursor: u32,
    pub prev_cursor: u32,
    _unknown1: [u8; 0xd0],
}

#[repr(C)]
#[derive(CopyGetters, Getters, MutGetters, Setters)]
pub struct MainMenu {
    _unknown1: [u8; 0x18],
    #[get_copy = "pub"]
    screen_id: ScreenId,
    _prev_screen_id: ScreenId,
    _unknown2: u32,
    _unknown3: u32,
    _unknown4: u32,
    #[getset(get_copy = "pub", get_mut = "pub", set = "pub")]
    cursor: u32,
    _prev_cursor: u32,
    #[get_copy = "pub"]
    max_cursor: u32,
    _unknown5: [u8; 0xcc],
    #[getset(get = "pub", get_mut = "pub")]
    p1_cursor: CharacterCursor,
    #[getset(get = "pub", get_mut = "pub")]
    p2_cursor: CharacterCursor,
}

#[derive(CopyGetters, Setters)]
#[repr(C)]
pub struct Game {
    _unknown1: [u8; 0x0038],
    #[getset(get_copy = "pub", set = "pub")]
    cursor: u32,
    _prev_cursor: u32,
    _max_cursor: u32,
    _unknown2: [u8; 0x0080],
    #[getset(get_copy = "pub")]
    depth: u32,
    _unknown3: [u8; 0x012c],
    // 0x01f4
    #[getset(get_copy = "pub")]
    pause: u32,
    _inverted_pause: u32,
    // unknown remains...
}

#[derive(Copy, Clone, Debug)]
enum MainLoopTaskId {
    ControllerSelect = 0x09,
    Menu = 0x0a,
    Game = 0x0e,
}

#[derive(Debug)]
#[repr(C)]
pub struct MainLoopTasksLinkedListItem {
    pub id: u32,
    _unknown1: u32,
    func: u32,
    _unknown2: [u8; 0x18],
    arg: u32,
}

#[repr(C)]
pub struct MainLoopTasksLinkedList {
    item: *const MainLoopTasksLinkedListItem,
    next: *const MainLoopTasksLinkedList,
}

impl MainLoopTasksLinkedList {
    #[must_use]
    pub fn len(&self) -> usize {
        let mut len = 0;
        let mut p = self as *const MainLoopTasksLinkedList;
        loop {
            len += 1;
            p = unsafe { (*p).next };
            if p.is_null() {
                return len;
            }
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn find_controller_select(&self) -> Option<&ControllerSelect> {
        self.find(MainLoopTaskId::ControllerSelect)
    }
    pub fn find_controller_select_mut(&mut self) -> Option<&mut ControllerSelect> {
        self.find_mut(MainLoopTaskId::ControllerSelect)
    }

    pub fn find_main_menu(&self) -> Option<&MainMenu> {
        self.find(MainLoopTaskId::Menu)
    }
    pub fn find_main_menu_mut(&mut self) -> Option<&mut MainMenu> {
        self.find_mut(MainLoopTaskId::Menu)
    }

    pub fn find_game(&self) -> Option<&Game> {
        self.find(MainLoopTaskId::Game)
    }
    pub fn find_game_mut(&mut self) -> Option<&mut Game> {
        self.find_mut(MainLoopTaskId::Game)
    }

    fn find<T>(&self, id: MainLoopTaskId) -> Option<&T> {
        let arg = self.to_vec().iter().find(|item| item.id == id as u32)?.arg as *const T;
        unsafe { arg.as_ref() }
    }
    fn find_mut<T>(&mut self, id: MainLoopTaskId) -> Option<&mut T> {
        let arg = self.to_vec().iter().find(|item| item.id == id as u32)?.arg as *mut T;
        unsafe { arg.as_mut() }
    }

    pub fn to_vec(&self) -> Vec<&MainLoopTasksLinkedListItem> {
        let mut vec = Vec::new();
        let mut list = self as *const Self;
        loop {
            if list.is_null() {
                return vec;
            }
            vec.push(unsafe { (*list).item.as_ref().unwrap() });
            list = unsafe { (*list).next };
        }
    }
}

#[derive(Getters, MutGetters)]
#[repr(C)]
pub struct App {
    _unknown1: [u8; 0x18],
    #[getset(get = "pub", get_mut = "pub")]
    main_loop_tasks: &'static mut MainLoopTasksLinkedList,
    // unknown remains...
}
