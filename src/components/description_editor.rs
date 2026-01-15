//! Description editor component with undo/redo support.
//!
//! This component provides a text editor for editing commit descriptions
//! with built-in undo (Ctrl+Z) and redo (Ctrl+Shift+Z) support.

use iced::keyboard::{self, Event as KeyboardEvent, Key};
use iced::widget::{button, column, container, row, text, text_editor};
use iced::{Element, Fill, Length, Subscription};

/// Messages for the description editor
#[derive(Debug, Clone)]
pub enum Message {
    /// Text editor action (typing, selection, etc.)
    EditorAction(text_editor::Action),
    /// Save the description
    Save,
    /// Cancel editing
    Cancel,
    /// Undo last change
    Undo,
    /// Redo last undone change
    Redo,
}

/// Undo/redo stack for text editing
#[derive(Debug, Clone)]
struct UndoStack {
    history: Vec<String>,     // Past states (oldest first)
    future: Vec<String>,      // Redo states (newest first after undo)
    current: String,          // Current text content
    last_snapshot_len: usize, // Track length at last snapshot for word-boundary detection
}

impl UndoStack {
    fn new(initial: String) -> Self {
        Self {
            last_snapshot_len: initial.len(),
            current: initial,
            history: Vec::new(),
            future: Vec::new(),
        }
    }

    /// Record a new state. Called after text changes.
    /// Only creates snapshot at word boundaries (space/newline entered).
    fn push(&mut self, text: String) {
        let should_snapshot = self.should_snapshot(&text);

        if should_snapshot && self.current != text {
            self.history.push(self.current.clone());
            self.future.clear(); // Clear redo stack on new input
            self.last_snapshot_len = self.current.len();
        }
        self.current = text;
    }

    fn should_snapshot(&self, new_text: &str) -> bool {
        // Snapshot on word boundaries: space or newline just typed
        if new_text.len() > self.current.len() {
            let diff = &new_text[self.current.len()..];
            return diff.contains(' ') || diff.contains('\n');
        }
        // Snapshot on significant deletion (more than 1 char at once)
        if self.current.len() > new_text.len() + 1 {
            return true;
        }
        false
    }

    fn undo(&mut self) -> Option<String> {
        if let Some(prev) = self.history.pop() {
            self.future.push(self.current.clone());
            self.current = prev.clone();
            Some(prev)
        } else {
            None
        }
    }

    fn redo(&mut self) -> Option<String> {
        if let Some(next) = self.future.pop() {
            self.history.push(self.current.clone());
            self.current = next.clone();
            Some(next)
        } else {
            None
        }
    }
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new(String::new())
    }
}

/// State for the description editor
pub struct State {
    /// Whether the editor is active
    pub editing: bool,
    /// The text editor content
    content: text_editor::Content,
    /// Original text (for detecting changes)
    original: String,
    /// Undo/redo stack
    undo_stack: UndoStack,
}

impl State {
    /// Create a new inactive description editor state
    pub fn new() -> Self {
        Self {
            editing: false,
            content: text_editor::Content::new(),
            original: String::new(),
            undo_stack: UndoStack::default(),
        }
    }

    /// Start editing with the given description
    pub fn start_editing(&mut self, description: &str) {
        self.editing = true;
        self.original = description.to_string();
        self.content = text_editor::Content::with_text(description);
        self.undo_stack = UndoStack::new(description.to_string());
    }

    /// Cancel editing and reset state
    pub fn cancel(&mut self) {
        self.editing = false;
        self.content = text_editor::Content::new();
        self.original.clear();
        self.undo_stack = UndoStack::default();
    }

    /// Get the current text content
    pub fn get_text(&self) -> String {
        self.content.text()
    }

    /// Check if there are unsaved changes
    pub fn has_changes(&self) -> bool {
        self.content.text() != self.original
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

use crate::state_wrapper::{StateMut, StateRef};

/// Update the editor state based on a message
pub fn update(mut state: StateMut<'_, State>, message: Message) {
    match message {
        Message::EditorAction(action) => {
            state.content.perform(action);
            let text = state.content.text();
            state.undo_stack.push(text);
        }
        Message::Undo => {
            if let Some(text) = state.undo_stack.undo() {
                state.content = text_editor::Content::with_text(&text);
            }
        }
        Message::Redo => {
            if let Some(text) = state.undo_stack.redo() {
                state.content = text_editor::Content::with_text(&text);
            }
        }
        Message::Save | Message::Cancel => {
            // These are handled by the parent component
        }
    }
}

/// Render the editor view
pub fn view<'a>(state: StateRef<'a, State>) -> Element<'a, Message> {
    let editor = text_editor(&state.state().content)
        .on_action(Message::EditorAction)
        .height(Length::Fixed(120.0));

    let has_changes = state.has_changes();

    let save_button = if has_changes {
        button(text("Save").size(11))
            .on_press(Message::Save)
            .padding([4, 12])
            .style(button::primary)
    } else {
        button(text("Save").size(11))
            .padding([4, 12])
            .style(button::secondary)
    };

    let cancel_button = button(text("Cancel").size(11))
        .on_press(Message::Cancel)
        .padding([4, 12])
        .style(button::secondary);

    let header = row![
        text("Description").size(11).style(text::primary),
        container(text("")).width(Fill),
        save_button,
        cancel_button,
    ]
    .spacing(8);

    container(column![header, editor].spacing(8))
        .padding(12)
        .width(Fill)
        .style(container::bordered_box)
        .into()
}

/// Subscription for keyboard shortcuts (Ctrl+Z, Ctrl+Shift+Z)
/// Only active when editing
pub fn subscription(state: &State) -> Subscription<Message> {
    if state.editing {
        keyboard::listen().filter_map(handle_keyboard_event)
    } else {
        Subscription::none()
    }
}

/// Handle keyboard events for undo/redo
fn handle_keyboard_event(event: KeyboardEvent) -> Option<Message> {
    if let KeyboardEvent::KeyPressed { key, modifiers, .. } = event {
        if modifiers.control() {
            if let Key::Character(c) = key.as_ref() {
                if c == "z" {
                    return Some(if modifiers.shift() {
                        Message::Redo
                    } else {
                        Message::Undo
                    });
                }
            }
        }
    }
    None
}
