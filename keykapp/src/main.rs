use commitlog::message::*;
use commitlog::*;
use rdev::simulate;
use rdev::EventType::*;
use rdev::Key;
use rdev::{grab, Event, EventType};
use std::cmp::min;
use std::collections::HashMap;
use std::error::Error;
use std::u32;

#[derive(Debug, PartialEq, Eq, Hash)]

enum KeyEventType {
    KeyPress(Key),
    KeyRelease(Key),
}

impl KeyEventType {
    fn to_event_type(key_event_type: KeyEventType) -> EventType {
        match key_event_type {
            KeyEventType::KeyPress(key) => KeyPress(key),
            KeyEventType::KeyRelease(key) => KeyRelease(key),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct KeyEvent {
    event_type: KeyEventType,
}

impl KeyEvent {
    fn from(event: &Event) -> Option<KeyEvent> {
        let event = event.clone();
        match event.event_type {
            KeyPress(key) => Some(KeyEvent {
                event_type: KeyEventType::KeyPress(key),
            }),
            KeyRelease(key) => Some(KeyEvent {
                event_type: KeyEventType::KeyRelease(key),
            }),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum KeyEventSexp {
    List(Vec<KeyEventSexp>),
    Atom(KeyEvent),
}

struct AppState {
    event_log: Vec<Event>,
    ngram_counts: HashMap<KeyEventSexp, u32>,
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
    let ngram_counts: HashMap<KeyEventSexp, u32> = HashMap::new();
    let mut app_state = AppState {
        event_log,
        ngram_counts,
    };

    app_state.event_log = read_event_log();

    let event_loop = move |event: Event| {
        // open a directory called 'log' for segment and index storage
        let opts = LogOptions::new("log");
        let mut commit_log = CommitLog::new(opts).unwrap();

        match event.event_type {
            EventType::KeyPress(_) | EventType::KeyRelease(_) => {
                commit_log
                    .append_msg(
                        serde_cbor::to_vec(&event)
                            .expect("Could not serialize event."),
                    )
                    .expect("Could not log serialized event.");

                app_state.event_log.push(event.clone());

                // increment ngrams with current event as suffix
                let ngram_length_max_global = 3;
                let ngram_length_max_current =
                    min(app_state.event_log.len(), ngram_length_max_global);
                for i in (0..ngram_length_max_current).into_iter() {
                    // ngrams[log[(-i)..=(-1)].map(|e| KeyEvent::from(e))]++
                    let event_log = &app_state.event_log;
                    let mut ngrams = &app_state.ngram_counts;

                    let ngram: Vec<KeyEventSexp> = event_log
                        [(event_log.len() - i - 1)..(event_log.len())]
                        .into_iter()
                        .map(|e| {
                            KeyEventSexp::Atom(KeyEvent::from(e).unwrap())
                        })
                        .collect();
                    let item: KeyEventSexp = KeyEventSexp::List(ngram);
                    match ngrams.get(&item) {
                        Some(count) => {
                            // println!("{:#?}", count);
                        }
                        None => {
                            // println!("None");
                        }
                    }
                }

                // use incoming key event to pick (if any) a corresponding outgoing key event (dummy pass-through for now)
                if let KeyEventSexp::Atom(key_event) =
                    KeyEventSexp::Atom(KeyEvent::from(&event).unwrap())
                {
                    let event_type =
                        KeyEventType::to_event_type(key_event.event_type);

                    println!("{:#?}", &event_type);
                    simulate(&event_type).unwrap();

                    None
                } else {
                    // This is a pass-through shouldn't happen at this point as we're grabbing and simulating all key events above, but might be useful if anything goes wrong or in a future "direct input" mode
                    Some(event)
                }
            }
            _ => Some(event),
        }
    };

    grab(event_loop).expect("Could not grab keyboard event.");
    Ok(())
}
