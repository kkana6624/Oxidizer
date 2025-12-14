pub mod events;

use self::events::InputEvent;
use crossbeam_channel::{unbounded, Receiver, Sender};

pub struct InputQueue {
    sender: Sender<InputEvent>,
    receiver: Receiver<InputEvent>,
}

impl InputQueue {
    pub fn new() -> Self {
        let (sender, receiver) = unbounded();
        Self { sender, receiver }
    }

    /// Pushes an event into the queue.
    /// This can be called from multiple threads (e.g. input polling thread).
    pub fn push(&self, event: InputEvent) {
        let _ = self.sender.send(event);
    }

    /// Pops an event from the queue.
    /// Non-blocking. Returns None if queue is empty.
    pub fn pop(&self) -> Option<InputEvent> {
        self.receiver.try_recv().ok()
    }

    /// Returns a clone of the sender, allowing it to be passed to other threads/structs.
    pub fn sender(&self) -> Sender<InputEvent> {
        self.sender.clone()
    }
}

impl Default for InputQueue {
    fn default() -> Self {
        Self::new()
    }
}
