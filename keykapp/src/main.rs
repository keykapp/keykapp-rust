use commitlog::message::*;
use commitlog::*;
use rdev::EventType::*;
use rdev::{grab, Event, EventType};
use std::collections::HashMap;
use std::error::Error;
use std::u32;

struct KeyEvent {
    pub name: Option<String>,
    pub event_type: EventType,
}

enum Item<T> {
    Collection(Vec<Item<T>>),
    Value(T),
}

struct AppState {
    event_log: Vec<Event>,
    ngram_counts: HashMap<Item<KeyEvent>, u32>,
}

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

fn main() -> Result<(), Box<dyn Error>> {
    let event_log: Vec<Event> = Vec::new();
    let ngram_counts: HashMap<Item<KeyEvent>, u32> = HashMap::new();
    let mut app_state = AppState {
        event_log,
        ngram_counts,
    };

    app_state.event_log = read_event_log();
    println!("event_log.len() -> {}", app_state.event_log.len());

    let event_loop = move |event: Event| {
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

                app_state.event_log.push(event.clone());
                println!("event_log.len() -> {}", app_state.event_log.len());

                Some(event)
            }
            _ => Some(event),
        }
    };

    grab(event_loop).expect("Could not grab keyboard event.");
    Ok(())
}
