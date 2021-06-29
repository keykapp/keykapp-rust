use commitlog::message::*;
use commitlog::*;
use priority_queue::PriorityQueue;
use rdev::simulate;
use rdev::EventType::*;
use rdev::Key;
use rdev::{grab, Event, EventType};
use std::cmp::min;
use std::error::Error;
use std::u32;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]

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

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
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

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
enum KeyEventSexp {
    List(Vec<KeyEventSexp>),
    Atom(KeyEvent),
}

struct AppState {
    event_log: Vec<Event>,
    ngram_counts: PriorityQueue<KeyEventSexp, u32>,
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
    let ngram_counts: PriorityQueue<KeyEventSexp, u32> = PriorityQueue::new();
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
                // LATER: create and use a Sexp (kapp) log instead of the event
                // log and use the kapp log tail to learn ngrams from
                // for now, since we're just proxying, these two are the same
                // LATER: move into function and use on log loaded on startup.
                let event_log = &app_state.event_log;
                let ngrams = &mut app_state.ngram_counts;
                let ngram_length_max_global = 3;
                let ngram_length_max_current =
                    min(app_state.event_log.len(), ngram_length_max_global);
                for i in (0..ngram_length_max_current).into_iter() {
                    let ngram: Vec<KeyEventSexp> = event_log
                        [(event_log.len() - i - 1)..(event_log.len())]
                        .into_iter()
                        .map(|e| {
                            KeyEventSexp::Atom(KeyEvent::from(e).unwrap())
                        })
                        .collect();
                    // note: here single-kapp items are also added as a List
                    // rather than an Atom, which is fine for now but good to
                    // keep in mind.
                    let item = KeyEventSexp::List(ngram);
                    ngrams.push_increase(
                        item.clone(),
                        ngrams.get_priority(&item).unwrap_or(&0)
                            + i as u32
                            + 1,
                    );
                }

                // use incoming key event to pick (if any) a corresponding
                // outgoing key event (dummy pass-through for now)
                if let KeyEventSexp::Atom(key_event) =
                    KeyEventSexp::Atom(KeyEvent::from(&event).unwrap())
                {
                    let event_type =
                        KeyEventType::to_event_type(key_event.event_type);

                    simulate(&event_type).unwrap();

                    // TODO: replace `println!` with concise representation (by
                    // implementing `Display`?)
                    ngrams.clone().into_sorted_iter().take(1).for_each(
                        |(sexp, count)| println!("{:#?}: {}", &sexp, count),
                    );
                    // what planet is this?
                    // what is your name?
                    // what is your quest?

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
