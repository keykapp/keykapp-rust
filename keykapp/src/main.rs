use commitlog::*;
use rdev::{grab, Event, EventType};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let grab_keyboard = |event: Event| {
        // open a directory called 'log' for segment and index storage
        let opts = LogOptions::new("log");
        let mut log = CommitLog::new(opts).unwrap();

        match event.event_type {
            EventType::KeyPress(_) | EventType::KeyRelease(_) => {
                log.append_msg(
                    serde_cbor::to_vec(&event)
                        .expect("Could not serialize event."),
                )
                .expect("Could not log serialized event.");
                Some(event)
            }
            _ => Some(event),
        }
    };

    grab(grab_keyboard).expect("Could not grab");
    Ok(())
}
