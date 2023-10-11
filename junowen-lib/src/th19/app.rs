#[repr(C)]
pub struct ControllerSelect {
    _unknown1: [u8; 0x14],
    pub cursor: u32,
    _prev_cursor: u32,
    pub max_cursor: u32,
    // ... unknown remains
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
    OnlineMenu,
    CharacterSelect,
    Unknown3,
    Unknown4,
    Unknown5,
    Unknown6,
    MusicRoom,
    Unknown7,
    Unknown8,
    Manual,
    Unknown9,
    Archievements,
}

#[repr(C)]
pub struct CharacterCursor {
    pub cursor: u32,
    pub prev_cursor: u32,
    _unknown1: [u8; 0xd0],
}

#[repr(C)]
pub struct Menu {
    _unknown1: [u8; 0x18],
    pub screen_id: ScreenId,
    _prev_screen_id: u32,
    _unknown2: u32,
    _unknown3: u32,
    _unknown4: u32,
    pub cursor: u32,
    _prev_cursor: u32,
    pub max_cursor: u32,
    _unknown5: [u8; 0xcc],
    pub p1_cursor: CharacterCursor,
    pub p2_cursor: CharacterCursor,
}

#[derive(Debug)]
enum MainLoopTaskId {
    ControllerSelect = 0x09,
    Menu = 0x0a,
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
        let arg = self
            .to_vec()
            .iter()
            .find(|item| item.id == MainLoopTaskId::ControllerSelect as u32)?
            .arg as *const ControllerSelect;
        unsafe { arg.as_ref() }
    }
    pub fn find_controller_select_mut(&self) -> Option<&mut ControllerSelect> {
        let arg = self
            .to_vec()
            .iter()
            .find(|item| item.id == MainLoopTaskId::ControllerSelect as u32)?
            .arg as *mut ControllerSelect;
        unsafe { arg.as_mut() }
    }

    pub fn find_menu(&self) -> Option<&'static Menu> {
        let arg = self
            .to_vec()
            .iter()
            .find(|item| item.id == MainLoopTaskId::Menu as u32)?
            .arg as *const Menu;
        unsafe { arg.as_ref() }
    }
    pub fn find_menu_mut(&self) -> Option<&'static mut Menu> {
        let arg = self
            .to_vec()
            .iter()
            .find(|item| item.id == MainLoopTaskId::Menu as u32)?
            .arg as *mut Menu;
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

#[repr(C)]
pub struct App {
    _unknown1: [u8; 0x18],
    pub main_loop_tasks: &'static MainLoopTasksLinkedList,
}
