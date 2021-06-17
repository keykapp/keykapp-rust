use rdev::{grab, Event, EventType};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let grab_keyboard = |event: Event| {
        match event.event_type {
            // EventType::KeyPress(Key::Tab) => {
            //     println!("Blocked a Tab!");
            //     simulate(&EventType::KeyPress(Key::KeyK))
            //         .expect("Failed to simulate");
            //     println!("Replaced Tab with k!");
            //     // Return `None` to stop original event from going through
            //     None
            // }
            // EventType::KeyRelease(Key::Tab) => None,
            EventType::KeyPress(_) | EventType::KeyRelease(_) => {
                println!("{:?}", event);
                Some(event)
            }
            _ => Some(event),
        }
    };

    grab(grab_keyboard).expect("Could not grab");
    Ok(())
}