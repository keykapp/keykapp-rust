use commitlog::message::*;
use commitlog::*;
use rdev::{grab, Event, EventType};
use std::error::Error;

fn read_event_log() -> Vec<Event> {
    // open a directory called 'log' for segment and index storage
    let opts = LogOptions::new("log");
    let log = CommitLog::new(opts).unwrap();

    // read the messages
    let messages = log.read(0, ReadLimit::max_bytes(usize::MAX)).unwrap();
    let events: Vec<Event> = messages
        .iter()
        .map(|message| {
            serde_cbor::from_reader::<Event, &[u8]>(message.payload())
                .expect("Could not deserialize event from log message.")
        })
        .collect();
    events
}

static mut EVENT_LOG: Vec<Event> = Vec::new();

fn main() -> Result<(), Box<dyn Error>> {
    unsafe {
        EVENT_LOG = read_event_log();
        println!("EVENT_LOG.len() -> {}", EVENT_LOG.len());
    }

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
                unsafe {
                    EVENT_LOG.push(event.clone());
                    println!("EVENT_LOG.len() -> {}", EVENT_LOG.len());
                }
                Some(event)
            }
            _ => Some(event),
        }
    };

    grab(grab_keyboard).expect("Could not grab");
    Ok(())
}
