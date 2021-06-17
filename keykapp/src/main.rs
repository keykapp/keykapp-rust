// use commitlog::message::*;
use commitlog::*;
use rdev::{grab, Event, EventType};
use std::error::Error;

// fn print_log() {
//     // open a directory called 'log' for segment and index storage
//     let opts = LogOptions::new("log");
//     let log = CommitLog::new(opts).unwrap();

//     // read the messages
//     let messages = log.read(0, ReadLimit::max_bytes(usize::MAX)).unwrap();
//     for msg in messages.iter() {
//         println!(
//             "{} - {:#?}",
//             msg.offset(),
//             serde_cbor::from_reader::<Event, &[u8]>(msg.payload())
//         );
//     }
// }

fn main() -> Result<(), Box<dyn Error>> {
    let grab_keyboard = |event: Event| {
        // open a directory called 'log' for segment and index storage
        let opts = LogOptions::new("log");
        let mut log = CommitLog::new(opts).unwrap();

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
