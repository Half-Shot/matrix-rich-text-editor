use widestring::Utf16String;

use crate::ffi_menu_state::MenuState;
use crate::ffi_text_update::TextUpdate;

pub struct ComposerUpdate {
    inner: wysiwyg::ComposerUpdate<Utf16String>,
}

impl ComposerUpdate {
    pub fn from(inner: wysiwyg::ComposerUpdate<Utf16String>) -> Self {
        Self { inner }
    }

    pub fn text_update(&self) -> TextUpdate {
        TextUpdate::from(self.inner.text_update.clone())
    }

    pub fn menu_state(&self) -> MenuState {
        MenuState::from(self.inner.menu_state.clone())
    }
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, sync::Arc};

    use crate::{ActionState, ComposerAction, ComposerModel, MenuState};

    #[test]
    fn initial_menu_update_is_populated() {
        let model = Arc::new(ComposerModel::new());
        let update = model.replace_text(String::from(""));

        // Only Redo is disabled
        assert_eq!(
            update.menu_state(),
            MenuState::Update {
                action_states: redo_disabled()
            }
        );
    }

    #[test]
    fn after_set_content_from_html_menu_is_updated() {
        let model = Arc::new(ComposerModel::new());
        let update = model.set_content_from_html(String::from(""));

        // Undo and Redo are disabled
        assert_eq!(
            update.menu_state(),
            MenuState::Update {
                action_states: undo_and_redo_disabled()
            }
        );
    }

    #[test]
    fn after_later_set_content_from_html_menu_is_updated() {
        let model = Arc::new(ComposerModel::new());
        model.replace_text(String::from("foo"));
        model.replace_text(String::from("bar"));
        model.undo();
        let update = model.set_content_from_html(String::from(""));

        // Undo and Redo are disabled
        assert_eq!(
            update.menu_state(),
            MenuState::Update {
                action_states: undo_and_redo_disabled()
            }
        );
    }

    #[test]
    fn after_set_content_from_markdown_menu_is_updated() {
        let model = Arc::new(ComposerModel::new());
        let update = model.set_content_from_markdown(String::from(""));

        // Undo and Redo are disabled
        assert_eq!(
            update.menu_state(),
            MenuState::Update {
                action_states: undo_and_redo_disabled()
            }
        );
    }

    fn redo_disabled() -> HashMap<ComposerAction, ActionState> {
        HashMap::from([
            (ComposerAction::Bold, ActionState::Enabled),
            (ComposerAction::Indent, ActionState::Enabled),
            (ComposerAction::InlineCode, ActionState::Enabled),
            (ComposerAction::Italic, ActionState::Enabled),
            (ComposerAction::Link, ActionState::Enabled),
            (ComposerAction::OrderedList, ActionState::Enabled),
            (ComposerAction::Redo, ActionState::Disabled),
            (ComposerAction::StrikeThrough, ActionState::Enabled),
            (ComposerAction::UnIndent, ActionState::Enabled),
            (ComposerAction::Underline, ActionState::Enabled),
            (ComposerAction::Undo, ActionState::Enabled),
            (ComposerAction::UnorderedList, ActionState::Enabled),
        ])
    }

    fn undo_and_redo_disabled() -> HashMap<ComposerAction, ActionState> {
        HashMap::from([
            (ComposerAction::Bold, ActionState::Enabled),
            (ComposerAction::Indent, ActionState::Enabled),
            (ComposerAction::InlineCode, ActionState::Enabled),
            (ComposerAction::Italic, ActionState::Enabled),
            (ComposerAction::Link, ActionState::Enabled),
            (ComposerAction::OrderedList, ActionState::Enabled),
            (ComposerAction::Redo, ActionState::Disabled),
            (ComposerAction::StrikeThrough, ActionState::Enabled),
            (ComposerAction::UnIndent, ActionState::Enabled),
            (ComposerAction::Underline, ActionState::Enabled),
            (ComposerAction::Undo, ActionState::Disabled),
            (ComposerAction::UnorderedList, ActionState::Enabled),
        ])
    }
}
