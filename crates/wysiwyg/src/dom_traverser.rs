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

use crate::dom::{Dom, DomNode, DomHandle, Element};

#[derive(Debug, PartialEq, Clone)]
pub struct NodePosition {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, PartialEq)]
pub enum FindResult {
    Found {
        node_handle: DomHandle,
        position: NodePosition,
        offset: usize,
    },
    NotFound {
        position: NodePosition,
    },
}

impl FindResult {
    pub fn is_found(&self) -> bool {
        if let FindResult::Found { .. } = self {
            true
        } else {
            false
        }
    }

    pub fn position(&self) -> &NodePosition {
        match self {
            FindResult::Found { node_handle, position, offset } => position,
            FindResult::NotFound { position } => position
        }
    }
}

impl <C> Dom<C>
where
C: Clone {
    pub fn find_pos(&mut self,
                node_handle: DomHandle,
                start: usize,
                end: usize,
                offset: usize,
                results: &mut Vec<FindResult>
    ) {

        fn process_element<'a, C: 'a + Clone>(
            dom: &mut Dom<C>,
            element: &'a impl Element<'a, C>,
            start: usize,
            end: usize,
            offset: usize,
            results: &mut Vec<FindResult>,
        ) {
            let mut off = offset;
            for child in element.children() {
                let child_handle = child.handle();
                assert!(
                    !child_handle.raw().is_empty(),
                    "Invalid child handle!"
                );
                match results.last() {
                    Some(find_child) => {
                        off = find_child.position().end.clone();
                    }
                    _ => {}
                }
                dom.find_pos(child_handle, start, end, off, results);
            }
        }

        // TODO: consider whether cloning DomHandles is damaging performance,
        // and look for ways to pass around references, maybe.
        if offset > end {
            return;
        }
        let node = self.lookup_node(node_handle.clone()).clone();
        match node {
            DomNode::Text(n) => {
                let len = n.data().len();
                let position = if let Some(position) = self.get_cached_position(&node_handle) {
                    position.clone()
                } else {
                    NodePosition { start: offset, end: offset + len }
                };
                if start <= offset + len {
                    let new_offset = if start >= offset {
                        start - offset
                    } else { 0 };
                    self.set_cached_position(node_handle.clone(), position.clone());
                    results.push(
                        FindResult::Found {
                            node_handle,
                            position: NodePosition { start: offset, end: offset + len },
                            offset: new_offset, // TODO: this offset might be wrong
                        }
                    )
                } else {
                    self.set_cached_position(node_handle.clone(), position.clone());
                    results.push(
                        FindResult::NotFound {
                            position,
                        }
                    )
                }
            }
            DomNode::Formatting(n) => process_element(self, &n, start, end, offset, results),
            DomNode::Container(n) => process_element(self, &n, start, end, offset, results),
        };
    }
}

#[cfg(test)]
mod test {
    use crate::dom::{FormattingNode, Range, TextNode};
    use crate::ToHtml;
    use super::*;

    #[test]
    fn finding_a_node_within_an_empty_dom_returns_empty_results() {
        let mut d: Dom<u16> = dom(&[]);
        let mut results = Vec::new();
        d.find_pos(d.document_handle(), 0, 0, 0, &mut results);
        assert!(results.is_empty());
    }

    #[test]
    fn finding_a_node_within_a_single_text_node_is_found() {
        let mut d: Dom<u16> = dom(&[tx("foo")]);
        let mut results = Vec::new();
        d.find_pos(d.document_handle(), 1, 1, 0, &mut results);
        assert_eq!(
            *results.last().unwrap(),
            FindResult::Found {
                node_handle: DomHandle::from_raw(vec![0]),
                position: NodePosition { start: 0, end: 3 },
                offset: 1
            }
        );
    }

    #[test]
    fn finding_a_node_within_flat_text_nodes_is_found() {
        let mut d: Dom<u16> = dom(&[tx("foo"), tx("bar")]);
        let mut results = Vec::new();
        d.find_pos(d.document_handle(), 0, 0, 0, &mut results);
        assert_eq!(
            *results.last().unwrap(),
            FindResult::Found {
                node_handle: DomHandle::from_raw(vec![0]),
                position: NodePosition { start: 0, end: 3 },
                offset: 0
            }
        );
        results.clear();
        d.find_pos(d.document_handle(), 1, 1, 0, &mut results);
        assert_eq!(
            *results.last().unwrap(),
            FindResult::Found {
                node_handle: DomHandle::from_raw(vec![0]),
                position: NodePosition { start: 0, end: 3 },
                offset: 1
            }
        );
        results.clear();
        d.find_pos(d.document_handle(), 2, 2, 0, &mut results);
        assert_eq!(
            *results.last().unwrap(),
            FindResult::Found {
                node_handle: DomHandle::from_raw(vec![0]),
                position: NodePosition { start: 0, end: 3 },
                offset: 2
            }
        );
        // TODO: selections at boundaries need work
        /*
        assert_eq!(
            d.find_pos(d.document_handle(), 3),
            FindResult::Found {
                node_handle: DomHandle::from_raw(vec![1]),
                offset: 0
            }
        );
        assert_eq!(
            d.find_pos(d.document_handle(), 4),
            FindResult::Found {
                node_handle: DomHandle::from_raw(vec![1]),
                offset: 1
            }
        );
        assert_eq!(
            d.find_pos(d.document_handle(), 5),
            FindResult::Found {
                node_handle: DomHandle::from_raw(vec![1]),
                offset: 2
            }
        );
        */
        results.clear();
        d.find_pos(d.document_handle(), 6, 6, 0, &mut results);
        assert_eq!(
            *results.last().unwrap(),
            FindResult::Found {
                node_handle: DomHandle::from_raw(vec![1]),
                position: NodePosition { start: 3, end: 6 },
                offset: 3
            }
        );
    }

    // TODO: comprehensive test like above for non-flat nodes

    #[test]
    fn finding_a_range_within_an_empty_dom_returns_no_node() {
        let mut d: Dom<u16> = dom(&[]);
        let range = d.find_range_mut(0, 0);
        assert_eq!(range, Range::NoNode);
    }

    #[test]
    fn finding_a_range_within_the_single_text_node_works() {
        let mut d = dom(&[tx("foo bar baz")]);
        let range = d.find_range_mut(4, 7);

        if let Range::SameNode(range) = range {
            assert_eq!(range.start_offset, 4);
            assert_eq!(range.end_offset, 7);

            if let DomNode::Text(t) = d.lookup_node(range.node_handle.clone()) {
                assert_eq!(t.data(), "foo bar baz".to_html());
            } else {
                panic!("Should have been a text node!")
            }

            assert_eq!(range.node_handle.raw(), &vec![0]);
        } else {
            panic!("Should have been a SameNodeRange: {:?}", range)
        }
    }

    #[test]
    fn finding_a_range_that_includes_the_end_works_simple_case() {
        let mut d = dom(&[tx("foo bar baz")]);
        let range = d.find_range_mut(4, 11);

        if let Range::SameNode(range) = range {
            assert_eq!(range.start_offset, 4);
            assert_eq!(range.end_offset, 11);

            if let DomNode::Text(t) = d.lookup_node(range.node_handle.clone()) {
                assert_eq!(t.data(), "foo bar baz".to_html());
            } else {
                panic!("Should have been a text node!")
            }

            assert_eq!(range.node_handle.raw(), &vec![0]);
        } else {
            panic!("Should have been a SameNodeRange: {:?}", range)
        }
    }

    #[test]
    fn finding_a_range_within_some_nested_node_works() {
        let mut d = dom(
            &[
                tx("foo "),
                b(&[
                    tx("bar")
                ]),
                tx(" baz")
            ]);
        let range = d.find_range_mut(5, 6);

        if let Range::SameNode(range) = range {
            assert_eq!(range.start_offset, 1);
            assert_eq!(range.end_offset, 2);

            if let DomNode::Text(t) = d.lookup_node(range.node_handle.clone()) {
                assert_eq!(t.data(), "bar".to_html());
            } else {
                panic!("Should have been a text node!")
            }

            assert_eq!(range.node_handle.raw(), &vec![1, 0]);
        } else {
            panic!("Should have been a SameNodeRange: {:?}", range)
        }
    }

    fn dom<'a, C>(children: impl IntoIterator<Item = &'a DomNode<C>>) -> Dom<C>
        where
            C: 'static + Clone,
    {
        Dom::new(clone_children(children))
    }

    fn tx(data: &str) -> DomNode<u16> {
        DomNode::Text(TextNode::from(utf16(data)))
    }

    fn b<'a>(
        children: impl IntoIterator<Item = &'a DomNode<u16>>,
    ) -> DomNode<u16> {
        DomNode::Formatting(FormattingNode::new(
            utf16("b"),
            clone_children(children),
        ))
    }

    fn utf16(input: &str) -> Vec<u16> {
        input.encode_utf16().collect()
    }

    fn clone_children<'a, C>(
        children: impl IntoIterator<Item = &'a DomNode<C>>,
    ) -> Vec<DomNode<C>>
        where
            C: 'static + Clone,
    {
        children.into_iter().cloned().collect()
    }
}