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

// increment ngrams with current event as suffix. LATER: create and use a Sexp
// (kapp) log instead of the event log and use the kapp log tail to learn
// ngrams from for now, since we're just proxying, these two are the same
fn update_ngrams_from_log_tail(app_state: &mut AppState) {
    let event_log = &app_state.event_log;
    let ngrams = &mut app_state.ngram_counts;
    let ngram_length_max_global = 8;
    let ngram_length_max_current =
        min(app_state.event_log.len(), ngram_length_max_global);
    for i in (0..ngram_length_max_current).into_iter() {
        let ngram: Vec<KeyEventSexp> = event_log
            [(event_log.len() - i - 1)..(event_log.len())]
            .into_iter()
            .map(|e| KeyEventSexp::Atom(KeyEvent::from(e).unwrap()))
            .collect();
        // note: here single-kapp items are also added as a List
        // rather than an Atom, which is fine for now but good to
        // keep in mind.
        let item = KeyEventSexp::List(ngram);
        ngrams.push_increase(
            item.clone(),
            ngrams.get_priority(&item).unwrap_or(&0) + i as u32 + 1,
        );
    }
}

fn load_app_state_from_event_log(app_state: &mut AppState) {
    // open a directory called 'log' for segment and index storage
    let opts = LogOptions::new("log");
    let log = CommitLog::new(opts).unwrap();

    // read the messages
    let messages = log.read(0, ReadLimit::max_bytes(usize::MAX)).unwrap();
    messages.iter().for_each(|message| {
        let event = serde_cbor::from_reader::<Event, &[u8]>(message.payload())
            .expect("Could not deserialize event from log message.");
        app_state.event_log.push(event);
        update_ngrams_from_log_tail(app_state);
    });
}

fn main() -> Result<(), Box<dyn Error>> {
    let event_log: Vec<Event> = Vec::new();
    let ngram_counts: PriorityQueue<KeyEventSexp, u32> = PriorityQueue::new();
    let mut app_state = AppState {
        event_log,
        ngram_counts,
    };

    load_app_state_from_event_log(&mut app_state);

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

                update_ngrams_from_log_tail(&mut app_state);

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
                    let ngrams = &app_state.ngram_counts;
                    println!("---------- choose your adventure ----------");
                    ngrams.clone().into_sorted_iter().take(1).for_each(
                        |(sexp, count)| println!("{:#?}: {}", &sexp, count),
                    );

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
