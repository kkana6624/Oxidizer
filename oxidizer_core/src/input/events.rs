#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Button {
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    ScratchClockwise,
    ScratchCounterClockwise,
    Start,
    Select,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InputEvent {
    /// Absolute audio time when the event occurred
    pub timestamp: f64,
    pub button: Button,
    pub pressed: bool,
}
