
pub struct FrameInput {
    pub events: Vec<Event>,
    pub elapsed_time: f64,
    pub screen_width: usize,
    pub screen_height: usize,
    pub window_width: usize,
    pub window_height: usize
}

#[derive(Debug, Clone, PartialEq)]
pub enum State
{
    Pressed,
    Released
}

#[derive(Debug, Clone, PartialEq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event
{
    MouseClick {
        state: State,
        button: MouseButton,
        position: (f64, f64)
    },
    MouseMotion {
        delta: (f64, f64),
    },
    MouseWheel {
        delta: f64,
    },
    Key {
        state: State,
        kind: String
    },
}