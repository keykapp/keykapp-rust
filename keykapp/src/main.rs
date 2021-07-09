use commitlog::message::*;
use commitlog::*;
use core::time;
use priority_queue::PriorityQueue;
use rdev::simulate;
use rdev::EventType::*;
use rdev::Key;
use rdev::Key::*;
use rdev::{grab, Event, EventType};
use serde::{Deserialize, Serialize};
use std::cmp::min;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::error::Error;
use std::fmt;
use std::fmt::Display;
use std::io::Write;
use std::thread;
use std::u32;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Serialize, Deserialize)]
enum KeyEventType {
    KeyPress(Key),
    KeyRelease(Key),
}

impl KeyEventType {
    fn from(event: &Event) -> Option<Self> {
        match KeyEvent::from(event) {
            Some(key_event) => Some(key_event.event_type),
            None => None,
        }
    }
    fn to_event_type(self) -> EventType {
        match self {
            KeyEventType::KeyPress(key) => KeyPress(key),
            KeyEventType::KeyRelease(key) => KeyRelease(key),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Serialize, Deserialize)]
struct KeyEvent {
    event_type: KeyEventType,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Serialize, Deserialize)]
enum KappAtom {
    KeyAction(KeyEvent),
}

impl KappAtom {
    fn to_event_type(self) -> Option<EventType> {
        match self {
            KappAtom::KeyAction(key_event) => {
                Some(key_event.event_type.to_event_type())
            }
        }
    }
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

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
enum KappSexp {
    List(Vec<KappSexp>),
    Atom(KappAtom),
}

impl Display for KappSexp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            KappSexp::Atom(kapp_atom) => kapp_atom.fmt(f),
            KappSexp::List(kapp_list) => {
                write!(f, "(")?;
                kapp_list
                    .iter()
                    .try_for_each(|sexp| write!(f, "{} ", sexp))?;
                write!(f, ")")
            }
        }
    }
}

impl Display for KappAtom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            KappAtom::KeyAction(key_event) => match key_event.event_type {
                KeyEventType::KeyPress(key) => {
                    write!(f, "<")?;
                    let key = format!("{:#?}", key)
                        .replace("\n", "")
                        .replace(" ", "")
                        .replace(",", "");
                    write!(f, "{}", key)
                }
                KeyEventType::KeyRelease(key) => {
                    let key = format!("{:#?}", key)
                        .replace("\n", "")
                        .replace(" ", "")
                        .replace(",", "");
                    write!(f, "{}", key)?;
                    write!(f, ">")
                }
            },
        }
    }
}

impl KappSexp {
    fn from_event(event: &Event) -> Option<KappSexp> {
        match KeyEvent::from(event) {
            Some(key_event) => {
                Some(KappSexp::Atom(KappAtom::KeyAction(key_event)))
            }
            None => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct KeyboardState {
    pressed_keys: HashSet<Key>,
}

impl KeyboardState {
    fn new() -> Self {
        KeyboardState {
            pressed_keys: HashSet::new(),
        }
    }

    fn update(&mut self, key_event_type: KeyEventType) {
        match key_event_type {
            KeyEventType::KeyPress(key) => {
                self.pressed_keys.insert(key);
            }
            KeyEventType::KeyRelease(key) => {
                self.pressed_keys.remove(&key);
            }
        }
    }
    fn meta(&self) -> bool {
        self.pressed_keys.contains(&MetaLeft)
            || self.pressed_keys.contains(&MetaRight)
    }

    fn command(&self) -> bool {
        self.meta()
    }

    fn control(&self) -> bool {
        self.pressed_keys.contains(&ControlLeft)
            || self.pressed_keys.contains(&ControlRight)
    }

    fn shift(&self) -> bool {
        self.pressed_keys.contains(&ShiftLeft)
            || self.pressed_keys.contains(&ShiftRight)
    }

    fn pressed(&self, key: Key) -> bool {
        self.pressed_keys.contains(&key)
    }

    fn enter_insert_mode(&self) -> bool {
        self.command() && self.control() && self.pressed(KeyJ)
    }

    fn enter_command_mode(&self) -> bool {
        self.command() && self.control() && self.pressed(KeyK)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
enum InputMode {
    Insert,
    LeavingInputEnteringCommand,
    Command,
    LeavingCommandEnteringInsert,
}

#[derive(Debug, PartialEq, Clone)]
enum Effect {
    Simulate(EventType),
}

struct AppState {
    kapp_log: Vec<KappSexp>,
    ngram_counts: PriorityQueue<KappSexp, u32>,
    keyboard_state: KeyboardState,
    input_mode: InputMode,
    keybindings: HashMap<Key, KappSexp>,
    pending_effects: VecDeque<Effect>,
    is_interactive: bool,
}

impl AppState {
    fn new() -> Self {
        AppState {
            kapp_log: Vec::new(),
            ngram_counts: PriorityQueue::new(),
            keyboard_state: KeyboardState::new(),
            input_mode: InputMode::Insert,
            keybindings: HashMap::new(),
            pending_effects: VecDeque::new(),
            is_interactive: false,
        }
    }
}

const KEYSWITCHES: [Key; 8] =
    [KeyJ, KeyF, KeyK, KeyD, KeyL, KeyS, SemiColon, KeyA];

fn update_ngrams_from_log_tail(app_state: &mut AppState) {
    let kapp_log = &app_state.kapp_log;
    let ngram_length_max_global = 32;
    let ngram_length_min_global = 1;
    let ngram_length_max_current =
        min(app_state.kapp_log.len(), ngram_length_max_global);
    for ngram_length in
        (ngram_length_min_global..=ngram_length_max_current).into_iter()
    {
        let ngram: Vec<KappSexp> = kapp_log
            [(kapp_log.len() - ngram_length)..(kapp_log.len())]
            .to_vec();
        // note: here single-kapp items are also added as a List
        // rather than an Atom, which is fine for now but good to
        // keep in mind.
        let item = KappSexp::List(ngram);

        let ngrams = &app_state.ngram_counts;
        let initial_or_previous_priority =
            ngrams.get_priority(&item).unwrap_or(&0).clone();
        let priority_increment = item.num_atoms();

        let ngrams = &mut app_state.ngram_counts;
        ngrams.push_increase(
            item.clone(),
            initial_or_previous_priority + priority_increment,
        );
    }
}

impl KappSexp {
    fn num_atoms(&self) -> u32 {
        match &self {
            &KappSexp::Atom(_atom) => 1,
            &KappSexp::List(list) => {
                list.iter().fold(0, |sum, item| sum + item.num_atoms())
            }
        }
    }
}

fn load_app_state_from_log(app_state: &mut AppState) {
    // open a directory called 'log' for segment and index storage
    let opts = LogOptions::new("log");
    let log = CommitLog::new(opts).unwrap();

    // read the messages
    let messages = log.read(0, ReadLimit::max_bytes(usize::MAX)).unwrap();
    messages.iter().for_each(|message| {
        let kapp =
            serde_cbor::from_reader::<KappSexp, &[u8]>(message.payload())
                .expect("Could not deserialize event from log message.");

        logged_eval(app_state, &kapp);
    });

    // dump pending side effects (very important!)
    app_state.pending_effects.clear();
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut app_state = AppState::new();

    load_app_state_from_log(&mut app_state);
    recompute_derived_data_if_needed(&mut app_state);
    render_ui(&app_state);
    app_state.is_interactive = true;

    let event_loop = move |incoming_event: Event| {
        let event = incoming_event.clone();

        let outgoing_event = reduce(&mut app_state, incoming_event);

        perform_effects(&mut app_state);

        if should_ui_rerender(&app_state, &event) {
            render_ui(&app_state);
        }

        outgoing_event
    };

    grab(event_loop).expect("Could not grab keyboard event.");
    Ok(())
}

fn should_ui_rerender(app_state: &AppState, event: &Event) -> bool {
    match event.event_type {
        KeyRelease(_) => app_state.keyboard_state.pressed_keys.is_empty(),
        _ => false,
    }
}

fn perform_effects(app_state: &mut AppState) -> () {
    let delay = time::Duration::from_millis(5);
    while let Some(effect) = app_state.pending_effects.pop_front() {
        match effect {
            Effect::Simulate(event_type) => {
                simulate(&event_type).expect("Could not simulate event.");
                thread::sleep(delay);
            }
        }
    }
}

fn reduce(app_state: &mut AppState, event: Event) -> Option<Event> {
    match event.event_type {
        EventType::KeyPress(_) | EventType::KeyRelease(_) => {
            // use incoming key event to pick (if any) a corresponding
            // outgoing kapp
            match app_state.input_mode {
                InputMode::Insert => {
                    app_state.keyboard_state.update(
                        KeyEventType::from(&event)
                            .expect("Could not update keyboard state."),
                    );
                    let kbd = &app_state.keyboard_state;
                    if kbd.enter_command_mode() {
                        app_state.input_mode =
                            InputMode::LeavingInputEnteringCommand;
                    } else {
                        let kapp = KappSexp::from_event(&event).unwrap();
                        logged_eval(app_state, &kapp);
                    }
                }
                InputMode::LeavingInputEnteringCommand => {
                    // FIXME: this block assumes that every event after the
                    // KeyPress that triggered entering CommandMode is a
                    // KeyRelease
                    let kbd = &app_state.keyboard_state;
                    // drop the non-modifier KeyRelease from the
                    // `enter_command_mode()` keybinding
                    if !kbd.enter_command_mode() {
                        // pass through KeyRelease events before changing
                        // mode
                        let kapp = KappSexp::from_event(&event).unwrap();
                        logged_eval(app_state, &kapp);

                        let kbd = &app_state.keyboard_state;
                        // on the last KeyRelease we pass it through then
                        // change the mode
                        if kbd.pressed_keys.len() == 1 {
                            app_state.input_mode = InputMode::Command;
                        }
                    }
                    &app_state.keyboard_state.update(
                        KeyEventType::from(&event)
                            .expect("Could not update keyboard state."),
                    );
                }
                InputMode::LeavingCommandEnteringInsert => {
                    // FIXME: this block assumes that every event after the
                    // KeyPress that triggered entering InsertMode is a
                    // KeyRelease
                    // drop every event without emitting until last KeyRelease, then change mode
                    let kbd = &app_state.keyboard_state;
                    if kbd.pressed_keys.len() == 1 {
                        app_state.input_mode = InputMode::Insert;
                    }
                    &app_state.keyboard_state.update(
                        KeyEventType::from(&event)
                            .expect("Could not update keyboard state."),
                    );
                }
                InputMode::Command => {
                    app_state.keyboard_state.update(
                        KeyEventType::from(&event)
                            .expect("Could not update keyboard state."),
                    );
                    let kbd = &app_state.keyboard_state;
                    if kbd.enter_insert_mode() {
                        app_state.input_mode =
                            InputMode::LeavingCommandEnteringInsert;
                    } else if !kbd.enter_command_mode() {
                        if app_state.keyboard_state.pressed_keys.len() == 1 {
                            let keybindings = app_state.keybindings.clone();
                            let pressed_key: Vec<Key> = app_state
                                .keyboard_state
                                .pressed_keys
                                .clone()
                                .into_iter()
                                .collect();
                            let pressed_key = pressed_key.first().unwrap();
                            if KEYSWITCHES.contains(pressed_key) {
                                let kapp =
                                    keybindings.get(pressed_key).unwrap();
                                logged_eval(app_state, &kapp);
                            }
                        }
                    }
                }
            }
            recompute_derived_data_if_needed(app_state);

            None
        }
        _ => Some(event),
    }
}

fn recompute_derived_data_if_needed(app_state: &mut AppState) {
    if app_state.keyboard_state.pressed_keys.is_empty() {
        update_keybindings(app_state);
    }
}

fn update_keybindings(app_state: &mut AppState) {
    let ngrams = &app_state.ngram_counts;
    let suggested_kapps: Vec<KappSexp> = ngrams
        .clone()
        .into_sorted_iter()
        .take(KEYSWITCHES.len())
        .map(|(kapp, _priority)| kapp)
        .collect();
    let keybindings: HashMap<Key, KappSexp> = KEYSWITCHES
        .to_vec()
        .clone()
        .into_iter()
        .zip(suggested_kapps.into_iter())
        .collect();
    app_state.keybindings = keybindings;
}

fn render_ui(app_state: &AppState) {
    println!("\n---- Keykapp ----");
    println!("- InputMode::{:#?}", app_state.input_mode);
    println!(
        "- Pressed Keys: {:#?}",
        app_state.keyboard_state.pressed_keys
    );
    render_keybindings(app_state);
}

fn render_keybindings(app_state: &AppState) {
    let items: Vec<(&Key, &KappSexp, &u32)> = KEYSWITCHES
        .iter()
        .filter_map(|key| {
            app_state.keybindings.get(key).and_then(|kapp| {
                app_state
                    .ngram_counts
                    .get_priority(kapp)
                    .and_then(|priority| Some((key, kapp, priority)))
            })
        })
        .collect();
    items.into_iter().for_each(|(key, kapp, priority)| {
        let button: String = format!("{:#?} [{}]: {}", key, priority, kapp);
        println!("{}", button);
    });
}

fn logged_eval(app_state: &mut AppState, kapp: &KappSexp) -> () {
    eval(app_state, kapp);

    app_state.kapp_log.push(kapp.clone());
    update_ngrams_from_log_tail(app_state);

    if app_state.is_interactive {
        persist_eval(kapp);

        print!(" {}", kapp);
        std::io::stdout().flush().unwrap();
    }
}

fn persist_eval(kapp: &KappSexp) {
    let opts = LogOptions::new("log");
    let mut commit_log = CommitLog::new(opts).unwrap();
    commit_log
        .append_msg(
            serde_cbor::to_vec(kapp).expect("Could not serialize event."),
        )
        .expect("Could not log serialized event.");
}

fn eval(app_state: &mut AppState, kapp: &KappSexp) -> () {
    match kapp {
        KappSexp::Atom(atom) => match atom {
            KappAtom::KeyAction(_) => {
                let event_type = &atom.to_event_type().unwrap();
                app_state
                    .pending_effects
                    .push_back(Effect::Simulate(*event_type));
            }
        },
        KappSexp::List(list) => list.into_iter().for_each(|sexp| {
            eval(app_state, sexp);
        }),
    }
}
