// Copyright 2022 The Matrix.org Foundation C.I.C.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::dom::{
    Dom, DomNode, FormattingNode, Range, SameNodeRange, TextNode, ToHtml,
};
use crate::{ActionResponse, ComposerState, ComposerUpdate, Location};

pub struct ComposerModel<C>
where
    C: Clone,
{
    cur_state_index: usize,
    states: Vec<ComposerState<C>>,
}

impl<'a, C> ComposerModel<C>
where
    C: Clone,
    Dom<C>: ToHtml<C>,
    &'a str: ToHtml<C>,
{
    pub fn new() -> Self {
        Self {
            cur_state_index: 0,
            states: vec![ComposerState::new()],
        }
    }

    /**
     * Cursor is at end.
     */
    pub fn select(&mut self, start: Location, end: Location) {
        let cur_state = self.get_current_state_mut();
        cur_state.start = start;
        cur_state.end = end;
    }

    /**
     * Return the start and end of the selection, ensuring the first number
     * returned is <= the second, and they are both 0<=n<=html.len().
     */
    fn safe_selection(&self) -> (usize, usize) {
        // TODO: Does not work with tags, and will probably be obselete when
        // we can look for ranges properly.
        let cur_state = self.get_current_state();
        let html = cur_state.dom.to_html();

        let mut s: usize = cur_state.start.into();
        let mut e: usize = cur_state.end.into();
        s = s.clamp(0, html.len());
        e = e.clamp(0, html.len());
        if s > e {
            (e, s)
        } else {
            (s, e)
        }
    }

    /**
     * Replaces text in the current selection with new_text.
     */
    pub fn replace_text(&mut self, new_text: &[C]) -> ComposerUpdate<C> {
        // TODO: escape any HTML?
        let (s, e) = self.safe_selection();
        self.replace_text_in(&new_text, s, e)
    }

    /**
     * Replaces text in the an arbitrary start..end range with new_text.
     */
    pub fn replace_text_in(
        &mut self,
        new_text: &[C],
        start: usize,
        end: usize,
    ) -> ComposerUpdate<C> {
        let mut cur_state = self.get_current_state_copy().clone();
        // Shrink states list
        self.states.shrink_to(self.cur_state_index as usize);

        let range = cur_state.dom.find_range_mut(start, end);
        match range {
            Range::SameNode(range) => {
                Self::replace_same_node(&mut cur_state, range, new_text);
                cur_state.start = Location::from(start + new_text.len());
                cur_state.end = cur_state.start;
            }

            Range::NoNode => {
                cur_state.dom
                    .append(DomNode::Text(TextNode::from(new_text.to_vec())));

                cur_state.start = Location::from(new_text.len());
                cur_state.end = cur_state.start;
            }

            _ => panic!("Can't replace_text_in in complex object models yet"),
        }

        self.cur_state_index += 1;
        self.states.push(cur_state);

        // TODO: for now, we replace every time, to check ourselves, but
        // at least some of the time we should not
        self.create_update_replace_all()
    }

    pub fn backspace(&mut self) -> ComposerUpdate<C> {
        let cur_state = self.get_current_state_mut();
        if cur_state.start == cur_state.end {
            // Go back 1 from the current location
            cur_state.start -= 1;
        }

        self.replace_text(&[])
    }

    /**
     * Deletes text in an arbitrary start..end range.
     */
    pub fn delete_in(&mut self, start: usize, end: usize) -> ComposerUpdate<C> {
        self.with_cur_state(|state| {
            state.end = Location::from(start);
        });
        self.replace_text_in(&[], start, end)
    }

    /**
     * Deletes the character after the current cursor position.
     */
    pub fn delete(&mut self) -> ComposerUpdate<C> {
        self.with_cur_state(|state| {
            if state.start == state.end {
                state.end += 1;
            }
        });
        self.replace_text(&[])
    }

    pub fn enter(&mut self) -> ComposerUpdate<C> {
        ComposerUpdate::keep()
    }

    pub fn action_response(
        &mut self,
        action_id: String,
        response: ActionResponse,
    ) -> ComposerUpdate<C> {
        drop(action_id);
        drop(response);
        ComposerUpdate::keep()
    }

    pub fn get_selection(&self) -> (Location, Location) {
        let cur_state = self.get_current_state();
        (cur_state.start, cur_state.end)
    }

    pub fn bold(&mut self) -> ComposerUpdate<C> {
        let mut cur_state = self.get_current_state_copy().clone();

        // Shrink states list
        self.states.shrink_to(self.cur_state_index as usize);

        // Temporary: only works if we have a single text node
        if cur_state.dom.children().len() == 1 {
            let (s, e) = self.safe_selection();
            if let DomNode::Text(t) = &mut cur_state.dom.children_mut()[0] {
                let text = t.data();
                let before = text[..s].to_vec();
                let during = text[s..e].to_vec();
                let after = text[e..].to_vec();

                t.set_data(before);

                // TODO: nicer construction of DOM nodes
                cur_state.dom.append(DomNode::Formatting(FormattingNode::new(
                    "strong".to_html(),
                    vec![DomNode::Text(TextNode::from(during))],
                )));

                cur_state.dom.append(DomNode::Text(TextNode::from(after)));

                // TODO: for now, we replace every time, to check ourselves, but
                // at least some of the time we should not

                self.cur_state_index += 1;
                self.states.push(cur_state);

                return self.create_update_replace_all();
            }
        }

        panic!("Can't bold in complex object models yet");
    }

    pub fn get_html(&self) -> Vec<C> {
        self.get_current_state().dom.to_html()
    }

    pub fn undo(&mut self) -> ComposerUpdate<C> {
        if self.cur_state_index > 0 {
            self.cur_state_index -= 1;
            self.create_update_replace_all()
        } else {
            ComposerUpdate::keep()
        }
    }

    pub fn redo(&mut self) -> ComposerUpdate<C> {
        if (self.cur_state_index as usize) < self.states.len()-1 {
            self.cur_state_index += 1;
            self.create_update_replace_all()
        } else {
            ComposerUpdate::keep()
        }
    }

    pub fn get_current_state_copy(&self) -> ComposerState<C> {
        self.states.get(self.cur_state_index as usize).unwrap().clone()
    }

    fn get_current_state(&self) -> &ComposerState<C> {
        self.states.get(self.cur_state_index as usize).unwrap()
    }

    fn with_cur_state<R>(&mut self, block: impl Fn(&mut ComposerState<C>) -> R) -> R {
        block(self.states.get_mut(self.cur_state_index).unwrap())
    }

    fn get_current_state_mut(&mut self) -> &mut ComposerState<C> {
        self.states.get_mut(self.cur_state_index).unwrap()
    }

    // Internal functions
    fn create_update_replace_all(&self) -> ComposerUpdate<C> {
        let cur_state = self.get_current_state();
        ComposerUpdate::replace_all(cur_state.dom.to_html(), cur_state.start, cur_state.end)
    }

    fn replace_same_node(state: &mut ComposerState<C>, range: SameNodeRange, new_text: &[C]) {
        let node = state.dom.lookup_node_mut(range.node_handle);
        if let DomNode::Text(ref mut t) = node {
            let text = t.data();
            let mut n = text[..range.start_offset].to_vec();
            n.extend_from_slice(new_text);
            n.extend_from_slice(&text[range.end_offset..]);
            t.set_data(n);
        } else {
            panic!("Can't deal with ranges containing non-text nodes (yet?)")
        }
    }

    fn get_previous_states(&self) -> &[ComposerState<C>] {
        &self.states[..(self.cur_state_index as usize)]
    }

    fn get_next_states(&self) -> &[ComposerState<C>] {
        &self.states[(self.cur_state_index+1 as usize)..]
    }
}

#[cfg(test)]
mod test {
    use speculoos::{prelude::*, AssertionFailure, Spec};

    use crate::{
        dom::{Dom, DomNode, TextNode, ToHtml},
        Location,
    };

    use super::ComposerModel;
    use crate::ComposerState;

    fn utf8(utf16: &[u16]) -> String {
        String::from_utf16(&utf16).expect("Invalid UTF-16!")
    }

    #[test]
    fn typing_a_character_into_an_empty_box_appends_it() {
        let mut model = cm("|");
        replace_text(&mut model, "v");
        assert_eq!(tx(&model), "v|");
    }

    #[test]
    fn typing_a_character_at_the_end_appends_it() {
        let mut model = cm("abc|");
        replace_text(&mut model, "d");
        assert_eq!(tx(&model), "abcd|");
    }

    #[test]
    fn typing_a_character_in_the_middle_inserts_it() {
        let mut model = cm("|abc");
        replace_text(&mut model, "Z");
        assert_eq!(tx(&model), "Z|abc");
    }

    #[test]
    fn selecting_past_the_end_is_harmless() {
        let mut model = cm("|");
        model.select(Location::from(7), Location::from(7));
        replace_text(&mut model, "Z");
        assert_eq!(tx(&model), "Z|");
    }

    #[test]
    fn replacing_a_selection_with_a_character() {
        let mut model = cm("abc{def}|ghi");
        replace_text(&mut model, "Z");
        assert_eq!(tx(&model), "abcZ|ghi");
    }

    #[test]
    fn replacing_a_backwards_selection_with_a_character() {
        let mut model = cm("abc|{def}ghi");
        replace_text(&mut model, "Z");
        assert_eq!(tx(&model), "abcZ|ghi");
    }

    #[test]
    fn typing_a_character_after_a_multi_codepoint_character() {
        // Woman Astronaut:
        // Woman+Dark Skin Tone+Zero Width Joiner+Rocket
        let mut model = cm("\u{1F469}\u{1F3FF}\u{200D}\u{1F680}|");
        replace_text(&mut model, "Z");
        assert_eq!(tx(&model), "\u{1F469}\u{1F3FF}\u{200D}\u{1F680}Z|");
    }

    #[test]
    fn typing_a_character_in_a_range_inserts_it() {
        let mut model = cm("0123456789|");
        let new_text = "654".encode_utf16().collect::<Vec<u16>>();
        model.replace_text_in(&new_text, 4, 7);
        assert_eq!(tx(&model), "0123654|789");
    }

    #[test]
    fn backspacing_a_character_at_the_end_deletes_it() {
        let mut model = cm("abc|");
        model.backspace();
        assert_eq!(tx(&model), "ab|");
    }

    #[test]
    fn backspacing_a_character_at_the_beginning_does_nothing() {
        let mut model = cm("|abc");
        model.backspace();
        assert_eq!(tx(&model), "|abc");
    }

    #[test]
    fn backspacing_a_character_in_the_middle_deletes_it() {
        let mut model = cm("ab|c");
        model.backspace();
        assert_eq!(tx(&model), "a|c");
    }

    #[test]
    fn backspacing_a_selection_deletes_it() {
        let mut model = cm("a{bc}|");
        model.backspace();
        assert_eq!(tx(&model), "a|");
    }

    #[test]
    fn backspacing_a_backwards_selection_deletes_it() {
        let mut model = cm("a|{bc}");
        model.backspace();
        assert_eq!(tx(&model), "a|");
    }

    #[test]
    fn deleting_a_character_at_the_end_does_nothing() {
        let mut model = cm("abc|");
        model.delete();
        assert_eq!(tx(&model), "abc|");
    }

    #[test]
    fn deleting_a_character_at_the_beginning_deletes_it() {
        let mut model = cm("|abc");
        model.delete();
        assert_eq!(tx(&model), "|bc");
    }

    #[test]
    fn deleting_a_character_in_the_middle_deletes_it() {
        let mut model = cm("a|bc");
        model.delete();
        assert_eq!(tx(&model), "a|c");
    }

    #[test]
    fn deleting_a_selection_deletes_it() {
        let mut model = cm("a{bc}|");
        model.delete();
        assert_eq!(tx(&model), "a|");
    }

    #[test]
    fn deleting_a_backwards_selection_deletes_it() {
        let mut model = cm("a|{bc}");
        model.delete();
        assert_eq!(tx(&model), "a|");
    }

    #[test]
    fn deleting_a_range_removes_it() {
        let mut model = cm("abcd|");
        model.delete_in(1, 3);
        assert_eq!(tx(&model), "a|d");
    }

    #[test]
    fn selecting_ascii_characters() {
        let mut model = cm("abcdefgh|");
        model.select(Location::from(0), Location::from(1));
        assert_eq!(tx(&model), "{a}|bcdefgh");

        model.select(Location::from(1), Location::from(3));
        assert_eq!(tx(&model), "a{bc}|defgh");

        model.select(Location::from(4), Location::from(8));
        assert_eq!(tx(&model), "abcd{efgh}|");

        model.select(Location::from(4), Location::from(9));
        assert_eq!(tx(&model), "abcd{efgh}|");
    }

    #[test]
    fn selecting_single_utf16_code_unit_characters() {
        let mut model = cm("\u{03A9}\u{03A9}\u{03A9}|");

        model.select(Location::from(0), Location::from(1));
        assert_eq!(tx(&model), "{\u{03A9}}|\u{03A9}\u{03A9}");

        model.select(Location::from(0), Location::from(3));
        assert_eq!(tx(&model), "{\u{03A9}\u{03A9}\u{03A9}}|");

        model.select(Location::from(1), Location::from(2));
        assert_eq!(tx(&model), "\u{03A9}{\u{03A9}}|\u{03A9}");
    }

    #[test]
    fn selecting_multiple_utf16_code_unit_characters() {
        let mut model = cm("\u{1F4A9}\u{1F4A9}\u{1F4A9}|");

        model.select(Location::from(0), Location::from(2));
        assert_eq!(tx(&model), "{\u{1F4A9}}|\u{1F4A9}\u{1F4A9}");

        model.select(Location::from(0), Location::from(6));
        assert_eq!(tx(&model), "{\u{1F4A9}\u{1F4A9}\u{1F4A9}}|");

        model.select(Location::from(2), Location::from(4));
        assert_eq!(tx(&model), "\u{1F4A9}{\u{1F4A9}}|\u{1F4A9}");
    }

    #[test]
    fn selecting_complex_characters() {
        let mut model =
            cm("aaa\u{03A9}bbb\u{1F469}\u{1F3FF}\u{200D}\u{1F680}ccc|");

        model.select(Location::from(0), Location::from(3));
        assert_eq!(
            tx(&model),
            "{aaa}|\u{03A9}bbb\u{1F469}\u{1F3FF}\u{200D}\u{1F680}ccc"
        );

        model.select(Location::from(0), Location::from(4));
        assert_eq!(
            tx(&model),
            "{aaa\u{03A9}}|bbb\u{1F469}\u{1F3FF}\u{200D}\u{1F680}ccc"
        );

        model.select(Location::from(7), Location::from(14));
        assert_eq!(
            tx(&model),
            "aaa\u{03A9}bbb{\u{1F469}\u{1F3FF}\u{200D}\u{1F680}}|ccc"
        );

        model.select(Location::from(7), Location::from(15));
        assert_eq!(
            tx(&model),
            "aaa\u{03A9}bbb{\u{1F469}\u{1F3FF}\u{200D}\u{1F680}c}|cc"
        );
    }

    #[test]
    fn bolding_ascii_adds_strong_tags() {
        let mut model = cm("aa{bb}|cc");
        model.bold();
        // TODO: because it's not an AST
        assert_eq!(tx(&model), "aa{<s}|trong>bb</strong>cc");

        let mut model = cm("aa|{bb}cc");
        model.bold();
        assert_eq!(tx(&model), "aa|{<s}trong>bb</strong>cc");
    }

    #[test]
    fn undoing_action_restores_previous_state() {
        let mut model = cm("hello |");
        let mut prev = model.get_current_state_copy();
        let prev_text_node = TextNode::from("world!".encode_utf16().collect::<Vec<u16>>());
        prev.dom.append(DomNode::Text(prev_text_node));
        model.states.insert(0, prev.clone());
        model.cur_state_index += 1;

        model.undo();

        assert_eq!(prev.dom.children().len(), model.get_current_state().dom.children().len());
    }

    #[test]
    fn inserting_text_creates_previous_state() {
        let mut model = cm("|");
        assert!(model.get_previous_states().is_empty());

        replace_text(&mut model, "hello world!");
        assert!(!model.get_previous_states().is_empty());
    }

    #[test]
    fn backspacing_text_creates_previous_state() {
        let mut model = cm("hello world!|");
        assert!(model.get_previous_states().is_empty());
        
        model.backspace();
        assert!(!model.get_previous_states().is_empty());
    }

    #[test]
    fn deleting_text_creates_previous_state() {
        let mut model = cm("hello |world!");
        assert!(model.get_previous_states().is_empty());
        
        model.delete();
        assert!(!model.get_previous_states().is_empty());
    }

    #[test]
    fn formatting_text_creates_previous_state() {
        let mut model = cm("hello {world}|!");
        assert!(model.get_previous_states().is_empty());
        
        model.bold();
        assert!(!model.get_previous_states().is_empty());
    }

    #[test]
    fn undoing_action_removes_last_previous_state() {
        let mut model = cm("hello {world}|!");
        model.states.push(model.get_current_state().clone());

        model.undo();

        // TODO:
    }

    #[test]
    fn undoing_action_adds_popped_state_to_next_states() {
        let mut model = cm("hello {world}|!");
        model.states.push(model.get_current_state().clone());
        model.cur_state_index = 1;

        model.undo();

        assert_eq!(model.get_next_states().get(0).unwrap(), model.get_current_state());
    }

    #[test]
    fn redo_pops_state_from_next_states() {
        let mut model = cm("hello {world}|!");
        model.states.push(model.get_current_state().clone());

        model.redo();

        assert!(model.get_next_states().is_empty());
    }

    #[test]
    fn redoing_action_adds_popped_state_to_previous_states() {
        let mut model = cm("hello {world}|!");
        model.states.push(model.get_current_state_copy());

        model.redo();

        assert_eq!(model.get_previous_states().last().unwrap(), model.get_current_state());
    }

    // Test utils

    fn replace_text(model: &mut ComposerModel<u16>, new_text: &str) {
        model.replace_text(&new_text.encode_utf16().collect::<Vec<u16>>());
    }

    trait Roundtrips<T> {
        fn roundtrips(&self);
    }

    impl<'s, T> Roundtrips<T> for Spec<'s, T>
    where
        T: AsRef<str>,
    {
        fn roundtrips(&self) {
            let subject = self.subject.as_ref();
            let output = tx(&cm(subject));
            if output != subject {
                AssertionFailure::from_spec(self)
                    .with_expected(String::from(subject))
                    .with_actual(output)
                    .fail();
            }
        }
    }

    /**
     * Create a ComposerModel from a text representation.
     */
    fn cm(text: &str) -> ComposerModel<u16> {
        let text: Vec<u16> = text.encode_utf16().collect();

        fn find(haystack: &[u16], needle: &str) -> Option<usize> {
            let needle = needle.encode_utf16().collect::<Vec<u16>>()[0];
            for (i, &ch) in haystack.iter().enumerate() {
                if ch == needle {
                    return Some(i);
                }
            }
            None
        }

        let curs = find(&text, "|").expect(&format!(
            "ComposerModel text did not contain a '|' symbol: '{}'",
            String::from_utf16(&text)
                .expect("ComposerModel text was not UTF-16"),
        ));

        let s = find(&text, "{");
        let e = find(&text, "}");

        let mut ret_text;
        let mut state = ComposerState::new();

        if let (Some(s), Some(e)) = (s, e) {
            if curs == e + 1 {
                // Cursor after end: foo{bar}|baz
                // The { made an extra codeunit - move the end back 1
                state.start = Location::from(s);
                state.end = Location::from(e - 1);
                ret_text = utf8(&text[..s]);
                ret_text += &utf8(&text[s + 1..e]);
                ret_text += &utf8(&text[curs + 1..]);
            } else if curs == s - 1 {
                // Cursor before beginning: foo|{bar}baz
                // The |{ made an extra 2 codeunits - move the end back 2
                state.start = Location::from(e - 2);
                state.end = Location::from(curs);
                ret_text = utf8(&text[..curs]);
                ret_text += &utf8(&text[s + 1..e]);
                ret_text += &utf8(&text[e + 1..]);
            } else {
                panic!(
                    "The cursor ('|') must always be directly before or after \
                    the selection ('{{..}}')! \
                    E.g.: 'foo|{{bar}}baz' or 'foo{{bar}}|baz'."
                )
            }
        } else {
            state.start = Location::from(curs);
            state.end = Location::from(curs);
            ret_text = utf8(&text[..curs]);
            ret_text += &utf8(&text[curs + 1..]);
        }

        state.dom = Dom::new(vec![DomNode::Text(TextNode::from(ret_text.to_html()))]);
        ComposerModel {
            cur_state_index: 0,
            states: vec![state],
        }
    }

    /**
     * Convert a ComposerModel to a text representation.
     */
    fn tx(model: &ComposerModel<u16>) -> String {
        let mut ret;

        let state = model.get_current_state();
        let utf16: Vec<u16> = state.dom.to_string().encode_utf16().collect();
        if state.start == state.end {
            ret = utf8(&utf16[..state.start.into()]);
            ret.push('|');
            ret += &utf8(&utf16[state.start.into()..]);
        } else {
            let (s, e) = model.safe_selection();

            ret = utf8(&utf16[..s]);
            if state.start < state.end {
                ret.push('{');
            } else {
                ret += "|{";
            }
            ret += &utf8(&utf16[s..e]);
            if state.start < state.end {
                ret += "}|";
            } else {
                ret.push('}');
            }
            ret += &utf8(&utf16[e..]);
        }
        ret
    }

    #[test]
    fn can_replace_text_in_an_empty_composer_model() {
        let mut cm = ComposerModel::new();
        cm.replace_text(&"foo".to_html());
        assert_eq!(tx(&cm), "foo|");
    }

    #[test]
    fn cm_creates_correct_component_model() {
        assert_eq!(cm("|").get_current_state().start, 0);
        assert_eq!(cm("|").get_current_state().end, 0);
        assert_eq!(cm("|").get_html(), &[]);

        assert_eq!(cm("a|").get_current_state().start, 1);
        assert_eq!(cm("a|").get_current_state().end, 1);
        assert_eq!(cm("a|").get_html(), "a".to_html());

        assert_eq!(cm("a|b").get_current_state().start, 1);
        assert_eq!(cm("a|b").get_current_state().end, 1);
        assert_eq!(cm("a|b").get_html(), "ab".to_html());

        assert_eq!(cm("|ab").get_current_state().start, 0);
        assert_eq!(cm("|ab").get_current_state().end, 0);
        assert_eq!(cm("|ab").get_html(), "ab".to_html());

        assert_eq!(cm("foo|").get_current_state().start, 3);
        assert_eq!(cm("foo|").get_current_state().end, 3);
        assert_eq!(cm("foo|").get_html(), ("foo".to_html()));

        let t1 = cm("foo|\u{1F4A9}bar");
        assert_eq!(t1.get_current_state().start, 3);
        assert_eq!(t1.get_current_state().end, 3);
        assert_eq!(t1.get_html(), ("foo\u{1F4A9}bar").to_html());

        let t2 = cm("foo\u{1F4A9}|bar");
        assert_eq!(t2.get_current_state().start, 5);
        assert_eq!(t2.get_current_state().end, 5);
        assert_eq!(t2.get_html(), ("foo\u{1F4A9}bar").to_html());

        assert_eq!(cm("foo|\u{1F4A9}").get_current_state().start, 3);
        assert_eq!(cm("foo|\u{1F4A9}").get_current_state().end, 3);
        assert_eq!(cm("foo|\u{1F4A9}").get_html(), ("foo\u{1F4A9}").to_html());

        assert_eq!(cm("foo\u{1F4A9}|").get_current_state().start, 5);
        assert_eq!(cm("foo\u{1F4A9}|").get_current_state().end, 5);
        assert_eq!(cm("foo\u{1F4A9}|").get_html(), ("foo\u{1F4A9}").to_html());

        assert_eq!(cm("|\u{1F4A9}bar").get_current_state().start, 0);
        assert_eq!(cm("|\u{1F4A9}bar").get_current_state().end, 0);
        assert_eq!(cm("|\u{1F4A9}bar").get_html(), ("\u{1F4A9}bar").to_html());

        assert_eq!(cm("\u{1F4A9}|bar").get_current_state().start, 2);
        assert_eq!(cm("\u{1F4A9}|bar").get_current_state().end, 2);
        assert_eq!(cm("\u{1F4A9}|bar").get_html(), ("\u{1F4A9}bar").to_html());

        assert_eq!(cm("{a}|").get_current_state().start, 0);
        assert_eq!(cm("{a}|").get_current_state().end, 1);
        assert_eq!(cm("{a}|").get_html(), ("a").to_html());

        assert_eq!(cm("|{a}").get_current_state().start, 1);
        assert_eq!(cm("|{a}").get_current_state().end, 0);
        assert_eq!(cm("|{a}").get_html(), ("a").to_html());

        assert_eq!(cm("abc{def}|ghi").get_current_state().start, 3);
        assert_eq!(cm("abc{def}|ghi").get_current_state().end, 6);
        assert_eq!(cm("abc{def}|ghi").get_html(), ("abcdefghi").to_html());

        assert_eq!(cm("abc|{def}ghi").get_current_state().start, 6);
        assert_eq!(cm("abc|{def}ghi").get_current_state().end, 3);
        assert_eq!(cm("abc|{def}ghi").get_html(), ("abcdefghi").to_html());

        let t3 = cm("\u{1F4A9}{def}|ghi");
        assert_eq!(t3.get_current_state().start, 2);
        assert_eq!(t3.get_current_state().end, 5);
        assert_eq!(t3.get_html(), ("\u{1F4A9}defghi").to_html());

        let t4 = cm("\u{1F4A9}|{def}ghi");
        assert_eq!(t4.get_current_state().start, 5);
        assert_eq!(t4.get_current_state().end, 2);
        assert_eq!(t4.get_html(), ("\u{1F4A9}defghi").to_html());

        let t5 = cm("abc{d\u{1F4A9}f}|ghi");
        assert_eq!(t5.get_current_state().start, 3);
        assert_eq!(t5.get_current_state().end, 7);
        assert_eq!(t5.get_html(), ("abcd\u{1F4A9}fghi").to_html());

        let t6 = cm("abc|{d\u{1F4A9}f}ghi");
        assert_eq!(t6.get_current_state().start, 7);
        assert_eq!(t6.get_current_state().end, 3);
        assert_eq!(t6.get_html(), ("abcd\u{1F4A9}fghi").to_html());

        let t7 = cm("abc{def}|\u{1F4A9}ghi");
        assert_eq!(t7.get_current_state().start, 3);
        assert_eq!(t7.get_current_state().end, 6);
        assert_eq!(t7.get_html(), ("abcdef\u{1F4A9}ghi").to_html());

        let t8 = cm("abc|{def}\u{1F4A9}ghi");
        assert_eq!(t8.get_current_state().start, 6);
        assert_eq!(t8.get_current_state().end, 3);
        assert_eq!(t8.get_html(), ("abcdef\u{1F4A9}ghi").to_html());
    }

    #[test]
    fn cm_and_tx_roundtrip() {
        assert_that!("|").roundtrips();
        assert_that!("a|").roundtrips();
        assert_that!("a|b").roundtrips();
        assert_that!("|ab").roundtrips();
        assert_that!("foo|\u{1F4A9}bar").roundtrips();
        assert_that!("foo\u{1F4A9}|bar").roundtrips();
        assert_that!("foo|\u{1F4A9}").roundtrips();
        assert_that!("foo\u{1F4A9}|").roundtrips();
        assert_that!("|\u{1F4A9}bar").roundtrips();
        assert_that!("\u{1F4A9}|bar").roundtrips();
        assert_that!("{a}|").roundtrips();
        assert_that!("|{a}").roundtrips();
        assert_that!("abc{def}|ghi").roundtrips();
        assert_that!("abc|{def}ghi").roundtrips();
        assert_that!("\u{1F4A9}{def}|ghi").roundtrips();
        assert_that!("\u{1F4A9}|{def}ghi").roundtrips();
        assert_that!("abc{d\u{1F4A9}f}|ghi").roundtrips();
        assert_that!("abc|{d\u{1F4A9}f}ghi").roundtrips();
        assert_that!("abc{def}|\u{1F4A9}ghi").roundtrips();
        assert_that!("abc|{def}\u{1F4A9}ghi").roundtrips();
    }
}
