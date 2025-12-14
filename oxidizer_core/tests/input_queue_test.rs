use oxidizer_core::input::events::{Button, InputEvent};
use oxidizer_core::input::InputQueue;
use std::thread;

#[test]
fn test_input_queue_transmission() {
    let queue = InputQueue::new();
    let sender = queue.sender();

    // Spawn a producer thread
    let handle = thread::spawn(move || {
        let event1 = InputEvent {
            timestamp: 1.0,
            button: Button::Key1,
            pressed: true,
        };
        sender.send(event1).unwrap();

        let event2 = InputEvent {
            timestamp: 1.5,
            button: Button::Key1,
            pressed: false,
        };
        sender.send(event2).unwrap();
    });

    handle.join().unwrap();

    // Consumer (main test thread)
    let received1 = queue.pop().expect("Should receive first event");
    assert_eq!(received1.timestamp, 1.0);
    assert_eq!(received1.button, Button::Key1);
    assert!(received1.pressed);

    let received2 = queue.pop().expect("Should receive second event");
    assert_eq!(received2.timestamp, 1.5);
    assert_eq!(received2.button, Button::Key1);
    assert!(!received2.pressed);

    // Queue should be empty now
    assert!(queue.pop().is_none());
}

#[test]
fn test_input_queue_ordering() {
    let queue = InputQueue::new();

    // Push events with increasing timestamps
    queue.push(InputEvent { timestamp: 10.0, button: Button::Start, pressed: true });
    queue.push(InputEvent { timestamp: 11.0, button: Button::Start, pressed: false });

    let e1 = queue.pop().unwrap();
    let e2 = queue.pop().unwrap();

    assert_eq!(e1.timestamp, 10.0);
    assert_eq!(e2.timestamp, 11.0);
}
