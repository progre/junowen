use derive_new::new;
use getset::CopyGetters;

#[derive(Debug, CopyGetters, new)]
pub struct Action {
    #[get_copy = "pub"]
    id: u8,
    #[get_copy = "pub"]
    play_sound: bool,
    value: Option<String>,
}

impl Action {
    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }
}

pub enum OnMenuInputResult<S> {
    None,
    Cancel,
    Action(Action),
    SubScene(S),
}
