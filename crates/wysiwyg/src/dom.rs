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

use std::collections::{Bound, BTreeMap, HashMap, HashSet};
use std::fmt::Display;
use std::ops::Bound::{Excluded, Included};
use std::ops::RangeBounds;
use crate::dom_traverser::{FindResult, NodePosition};

fn utf8(input: &[u16]) -> String {
    String::from_utf16(input).expect("Invalid UTF-16!")
}

pub trait Element<'a, C>
where
C: Clone {
    fn name(&'a self) -> &'a Vec<C>;
    fn children(&'a self) -> &'a Vec<DomNode<C>>;
    fn children_mut(&'a mut self) -> &'a mut Vec<DomNode<C>>;
}

fn fmt_element<'a, C>(
    element: &'a impl Element<'a, C>,
    lt: C,
    gt: C,
    fwd_slash: C,
    f: &mut HtmlFormatter<C>,
) where
    C: 'static + Clone,
    DomNode<C>: ToHtml<C>,
{
    let name = element.name();
    if !name.is_empty() {
        f.write_char(&lt);
        f.write(element.name());
        f.write_char(&gt);
    }
    for child in element.children() {
        child.fmt_html(f);
    }
    if !name.is_empty() {
        f.write_char(&lt);
        f.write_char(&fwd_slash);
        f.write(element.name());
        f.write_char(&gt);
    }
}

fn fmt_element_u16<'a>(
    element: &'a impl Element<'a, u16>,
    f: &mut HtmlFormatter<u16>,
) {
    fmt_element(element, '<' as u16, '>' as u16, '/' as u16, f);
}

pub struct HtmlFormatter<C> {
    chars: Vec<C>,
}

impl<C> HtmlFormatter<C>
where
    C: Clone,
{
    pub fn new() -> Self {
        Self { chars: Vec::new() }
    }

    pub fn write_char(&mut self, c: &C) {
        self.chars.push(c.clone());
    }

    pub fn write(&mut self, slice: &[C]) {
        self.chars.extend_from_slice(slice);
    }

    pub fn write_iter(&mut self, chars: impl Iterator<Item = C>) {
        self.chars.extend(chars)
    }

    pub fn finish(self) -> Vec<C> {
        self.chars
    }
}

pub trait ToHtml<C>
where
    C: Clone,
{
    fn fmt_html(&self, f: &mut HtmlFormatter<C>);

    fn to_html(&self) -> Vec<C> {
        let mut f = HtmlFormatter::new();
        self.fmt_html(&mut f);
        f.finish()
    }
}

impl ToHtml<u16> for &str {
    fn fmt_html(&self, f: &mut HtmlFormatter<u16>) {
        f.write_iter(self.encode_utf16());
    }
}

impl ToHtml<u16> for String {
    fn fmt_html(&self, f: &mut HtmlFormatter<u16>) {
        f.write_iter(self.encode_utf16());
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DomHandle {
    // Later, we will want to allow continuing iterating from this handle, and
    // comparing handles to see which is first in the iteration order. This
    // will allow us to walk the tree from earliest to latest of 2 handles.
    path: Vec<usize>,
}

impl DomHandle {
    pub fn from_raw(path: Vec<usize>) -> Self {
        Self { path }
    }

    fn parent_handle(&self) -> DomHandle {
        assert!(self.path.len() > 0);

        let mut new_path = self.path.clone();
        new_path.pop();
        DomHandle::from_raw(new_path)
    }

    fn child_handle(&self, child_index: usize) -> DomHandle {
        let mut new_path = self.path.clone();
        new_path.push(child_index);
        DomHandle::from_raw(new_path)
    }

    fn index_in_parent(&self) -> usize {
        assert!(self.path.len() > 0);

        self.path.last().unwrap().clone()
    }

    fn has_parent(&self) -> bool {
        !self.path.is_empty()
    }

    fn prev_sibling_handle(&self) -> DomHandle {
        assert!(self.has_parent() && self.index_in_parent() > 0);

        let sibling_index = self.index_in_parent()-1;
        let mut new_path = self.path.clone();
        new_path.pop();
        new_path.push(sibling_index);
        DomHandle { path: new_path }
    }

    fn next_sibling_handle(&self) -> DomHandle {
        assert!(self.has_parent());

        let sibling_index = self.index_in_parent()+1;
        let mut new_path = self.path.clone();
        new_path.pop();
        new_path.push(sibling_index);
        DomHandle { path: new_path }
    }

    pub fn raw(&self) -> &Vec<usize> {
        &self.path
    }

    /// Create a new INVALID handle
    ///
    /// Don't use this to lookup_node(). It will return the wrong node
    fn new_invalid() -> Self {
        Self {
            path: vec![usize::MAX],
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.path.contains(&usize::MAX)
    }
}

/// The answer supplied when you ask where a range is in the DOM, and the start
/// and end are both inside the same node.
#[derive(Debug, PartialEq)]
pub struct SameNodeRange {
    /// The node containing the range
    pub node_handle: DomHandle,

    /// The position within this node that corresponds to the start of the range
    pub start_offset: usize,

    /// The position within this node that corresponds to the end of the range
    pub end_offset: usize,
}

#[derive(Debug, PartialEq)]
pub enum Range {
    SameNode(SameNodeRange),

    // The range is too complex to calculate (for now)
    TooDifficultForMe,

    // The DOM contains no nodes at all!
    NoNode,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dom<C>
where
C: Clone {
    document: DomNode<C>,
    handles_for_start: BTreeMap<usize, HashSet<DomHandle>>,
    handles_for_end: BTreeMap<usize, HashSet<DomHandle>>,
    positions_for_handles: HashMap<DomHandle, NodePosition>,
}

impl<C> Dom<C>
where
C: Clone {

    // fn update_positions_internal(
    //     handle: DomHandle,
    //     old_len: usize,
    //     update_next_nodes: bool,
    //     handles_for_positions: &mut BTreeMap<usize, HashSet<DomHandle>>,
    //     positions_for_handles: &mut HashMap<DomHandle, NodePosition>,
    // ) {
    //
    // }

    pub fn new(top_level_items: Vec<DomNode<C>>) -> Self {
        let mut document = ContainerNode::new(Vec::new(), top_level_items);
        let handle = DomHandle::from_raw(Vec::new());
        document.set_handle(handle.clone());
        let mut instance = Self {
            document: DomNode::Container(document),
            handles_for_start: BTreeMap::new(),
            handles_for_end: BTreeMap::new(),
            positions_for_handles: HashMap::new(),
        };
        instance.update_positions(handle, 0, false);
        instance
    }

    fn document(&self) -> &ContainerNode<C> {
        // Would be nice if we could avoid this, but it is really convenient
        // in several places to be able to treat document as a DomNode.
        if let DomNode::Container(ret) = &self.document {
            ret
        } else {
            panic!("Document should always be a Container!")
        }
    }

    fn document_mut(&mut self) -> &mut ContainerNode<C> {
        // Would be nice if we could avoid this, but it is really convenient
        // in several places to be able to treat document as a DomNode.
        if let DomNode::Container(ret) = &mut self.document {
            ret
        } else {
            panic!("Document should always be a Container!")
        }
    }

    pub fn children(&self) -> &Vec<DomNode<C>> {
        self.document().children()
    }

    pub fn children_mut(&mut self) -> &mut Vec<DomNode<C>> {
        self.document_mut().children_mut()
    }

    pub fn append(&mut self, child: DomNode<C>) {
        let handle = self.document_mut().append(child);
        self.update_positions(handle, 0, false);
    }

    fn update_positions(&mut self, handle: DomHandle, old_len: usize, update_next_nodes: bool) {
        // TODO: maybe refactor it to only update children positions and next nodes
        let node = self.lookup_node(handle.clone()).clone();
        let index = if handle.has_parent() { handle.index_in_parent() } else { 0 };
        let node_len = node.len();
        let diff_size = (node_len as i32) - (old_len as i32);
        let mut start: usize = usize::MAX;
        let mut end: usize = usize::MAX;

        // Calculate positions
        if handle.has_parent() {
            if index == 0 {
                if let Some(parent_position) = self.positions_for_handles.get(&handle.parent_handle()) {
                    start = parent_position.start;
                    end = start + node_len;
                } else {
                    // Assume Root node
                    start = 0;
                    end = start + node_len;
                }
            } else {
                let prev_sibling_handle = handle.prev_sibling_handle();
                if let Some(sibling_position) = self.positions_for_handles.get(&prev_sibling_handle) {
                    start = sibling_position.end;
                    end = start + node_len;
                }
            }
        } else {
            start = 0;
            end = node_len;
        }

        assert_ne!(start, usize::MAX);

        // Set positions
        if handle.has_parent() {
            self.positions_for_handles.insert(handle.clone(), NodePosition { start, end });
            if let Some(mut cached_handles) = self.handles_for_start.get_mut(&start) {
                cached_handles.replace(handle.clone());
            } else {
                self.handles_for_start.insert(start, HashSet::from([handle.clone()]));
            }
            if let Some(mut cached_handles) = self.handles_for_end.get_mut(&start) {
                cached_handles.replace(handle);
            } else {
                self.handles_for_end.insert(start, HashSet::from([handle]));
            }
        }

        // Update children too
        match node {
            DomNode::Container(node) => {
                for mut child in node.children() {
                    self.update_positions(child.handle(), 0, false)
                }
            }
            DomNode::Formatting(node) => {
                for mut child in node.children() {
                    self.update_positions(child.handle(), 0, false)
                }
            }
            _ => {}
        }

        // Update next nodes if needed
        if update_next_nodes {
            let results = self.handles_for_start.range((Included(end), Excluded(usize::MAX)));
            for (_, handles) in results {
                for handle in handles {
                    if let Some(mut position) = self.positions_for_handles.get_mut(&handle) {
                        if diff_size.is_positive() {
                            position.start += diff_size as usize;
                            position.end += diff_size as usize;
                        } else {
                            position.start -= diff_size as usize;
                            position.end -= diff_size as usize;
                        }
                    }
                }
            }
        }
    }

    pub fn position_for_handle(&self, handle: &DomHandle) -> Option<&NodePosition> {
        self.positions_for_handles.get(handle)
    }

    pub fn handles_for_range(&self, start: &usize, end: &usize) -> HashSet<&DomHandle> {
        let mut results = HashSet::new();
        // let mut start_results = self.handles_for_start.range(range.clone())
        //     .flat_map(|(_, handles)| {
        //         handles
        //     }).fold(HashSet::new(), |mut acc, handle| {
        //         acc.insert(handle);
        //         acc
        // });
        // let end_results = self.handles_for_end.range(range)
        //     .flat_map(|(_, handles)| {
        //         handles
        //     }).fold(HashSet::new(), |mut acc, handle| {
        //     acc.insert(handle);
        //     acc
        // });
        // start_results.extend(end_results);
        // start_results
        for (i, handles) in self.handles_for_start.iter() {
            // We already passed the limit
            if i > end {
                return results;
            }
            for handle in handles {
                if let Some(pos) = self.position_for_handle(handle) {
                    if &pos.end >= start {
                        results.insert(handle);
                    }
                }
            }
        }
        results
    }

    pub fn replace(&mut self, node_handle: DomHandle, nodes: Vec<DomNode<C>>) {
        let parent_handle = node_handle.parent_handle();
        let parent_node = self.lookup_node_mut(parent_handle.clone());
        let parent_len = parent_node.len();
        let index = node_handle.index_in_parent();
        let result = match parent_node {
            DomNode::Text(_n) => panic!("Text nodes can't have children"),
            DomNode::Formatting(n) => n.replace_child(index, nodes),
            DomNode::Container(n) => n.replace_child(index, nodes),        
        };
        // It should be better to only update the replaced nodes
        self.update_positions(parent_handle, parent_len, true);
        result
    }

    pub fn find_range_mut(&mut self, start: usize, end: usize) -> Range {
        if self.children().is_empty() {
            return Range::NoNode;
        }

        // Potentially silly to walk the tree twice to find both parts, but
        // care will be needed since end may be before start. Very unlikely to
        // be a performance bottleneck, so it's probably fine like this.
        let mut results = Vec::new();
        self.find_pos(self.document_handle(), start, end, 0, &mut results);
        let found: Vec<&FindResult> = results.iter()
            .filter(|result| {
                if let DomNode::Text(node) = self.lookup_node(result.handle().clone()) {
                    true
                } else {
                    false
                }
            })
            .collect();

        // TODO: needs careful handling when on the boundary of 2 ranges:
        // we want to be greedy about when we state something is the same range
        // - maybe find_pos should return 2 nodes when we are on the boundary?
        match found.len() {
            1 => {
                if let FindResult::Found { node_handle, position, offset} = found[0] {
                    Range::SameNode(SameNodeRange {
                        node_handle: node_handle.clone(),
                        start_offset: start - position.start,
                        end_offset: end - position.start,
                    })
                } else {
                    panic!("There should be a single Found result, but there isn't.")
                }
            }
            0 => {
                Range::NoNode
            }
            _ => Range::TooDifficultForMe
        }
    }

    pub fn document_handle(&self) -> DomHandle {
        self.document.handle()
    }

    /// Find the node based on its handle.
    /// Panics if the handle is invalid
    pub fn lookup_node(&self, node_handle: DomHandle) -> &DomNode<C> {
        fn nth_child<'a, C: Clone>(
            element: &'a impl Element<'a, C>,
            idx: usize,
        ) -> &DomNode<C> {
            element.children().get(idx).expect(&format!(
                "This DomHandle wants child {} of this node, but it does \
                not have that many children.",
                idx
            ))
        }

        let mut node = &self.document;
        if !node_handle.is_valid() {
            panic!(
                "Attempting to lookup a node using an invalid DomHandle ({:?})",
                node_handle.raw()
            );
        }
        for idx in node_handle.raw() {
            node = match node {
                DomNode::Container(n) => nth_child(n, *idx),
                DomNode::Formatting(n) => nth_child(n, *idx),
                DomNode::Text(_) => panic!(
                    "Handle path looks for the child of a text node, but text \
                    nodes cannot have children."
                ),
            }
        }
        node
    }

    /// Find the node based on its handle and returns a mutable reference.
    /// Panics if the handle is invalid
    pub fn lookup_node_mut(
        &mut self,
        node_handle: DomHandle,
    ) -> &mut DomNode<C> {
        // TODO: horrible that we repeat lookup_node's logic. Can we share?
        fn nth_child<'a, C: Clone>(
            element: &'a mut impl Element<'a, C>,
            idx: usize,
        ) -> &mut DomNode<C> {
            element.children_mut().get_mut(idx).expect(&format!(
                "This DomHandle wants child {} of this node, but it does \
                not have that many children.",
                idx
            ))
        }

        let mut node = &mut self.document;
        for idx in node_handle.raw() {
            node = match node {
                DomNode::Container(n) => nth_child(n, *idx),
                DomNode::Formatting(n) => nth_child(n, *idx),
                DomNode::Text(_) => panic!(
                    "Handle path looks for the child of a text node, but text \
                    nodes cannot have children."
                ),
            }
        }
        node
    }
}

impl<C> ToHtml<C> for Dom<C>
where
    C: Clone,
    ContainerNode<C>: ToHtml<C>,
{
    fn fmt_html(&self, f: &mut HtmlFormatter<C>) {
        self.document().fmt_html(f)
    }
}

impl Display for Dom<u16> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&utf8(&self.to_html()))?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContainerNode<C>
where
C: Clone {
    name: Vec<C>,
    children: Vec<DomNode<C>>,
    handle: DomHandle,
}

impl<C> ContainerNode<C>
where
C: Clone {
    /// Create a new ContainerNode
    ///
    /// NOTE: Its handle() will be invalid until you call set_handle() or
    /// append() it to another node.
    pub fn new(name: Vec<C>, children: Vec<DomNode<C>>) -> Self {
        Self {
            name,
            children,
            handle: DomHandle::new_invalid(),
        }
    }

    pub fn append(&mut self, mut child: DomNode<C>) -> DomHandle {
        assert!(self.handle.is_valid());

        let child_index = self.children.len();
        let child_handle = self.handle.child_handle(child_index);
        child.set_handle(child_handle.clone());
        self.children.push(child);
        child_handle
    }

    pub fn len(&self) -> usize {
        let mut total_length = 0;
        for child in &self.children {
            total_length += child.len()
        }
        total_length
    }

    fn replace_child(&mut self, index: usize, nodes: Vec<DomNode<C>>) {
        assert!(self.handle.is_valid());
        assert!(index < self.children().len());

        self.children.remove(index);
        let mut current_index = index;
        for mut node in nodes {
            let child_handle = self.handle.child_handle(current_index);
            node.set_handle(child_handle);
            self.children.insert(current_index, node);
            current_index += 1;
        }

        for child_index in current_index..self.children.len() {
            let new_handle = self.handle.child_handle(child_index);
            self.children[child_index].set_handle(new_handle);
        }
    }

    fn handle(&self) -> DomHandle {
        self.handle.clone()
    }

    fn set_handle(&mut self, handle: DomHandle) {
        self.handle = handle;
        for (i, child) in self.children.iter_mut().enumerate() {
            child.set_handle(self.handle.child_handle(i))
        }
    }
}

impl<'a, C> Element<'a, C> for ContainerNode<C>
where
C: Clone {
    fn name(&'a self) -> &'a Vec<C> {
        &self.name
    }

    fn children(&'a self) -> &'a Vec<DomNode<C>> {
        &self.children
    }

    fn children_mut(&'a mut self) -> &'a mut Vec<DomNode<C>> {
        // TODO: replace with soemthing like get_child_mut - we want to avoid
        // anyone pushing onto this, because the handles will be invalid
        &mut self.children
    }
}

impl ToHtml<u16> for ContainerNode<u16> {
    fn fmt_html(&self, f: &mut HtmlFormatter<u16>) {
        fmt_element_u16(self, f)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FormattingNode<C>
where
C: Clone{
    name: Vec<C>,
    children: Vec<DomNode<C>>,
    handle: DomHandle,
}

impl<C> FormattingNode<C>
where
C: Clone {
    /// Create a new FormattingNode
    ///
    /// NOTE: Its handle() will be invalid until you call set_handle() or
    /// append() it to another node.
    pub fn new(name: Vec<C>, children: Vec<DomNode<C>>) -> Self {
        Self {
            name,
            children,
            handle: DomHandle::new_invalid(),
        }
    }

    pub fn len(&self) -> usize {
        let mut total_length = 0;
        for child in &self.children {
            total_length += child.len()
        }
        total_length
    }

    fn handle(&self) -> DomHandle {
        self.handle.clone()
    }

    fn set_handle(&mut self, handle: DomHandle) {
        // TODO: copied into 2 places - move into Element?
        self.handle = handle;
        for (i, child) in self.children.iter_mut().enumerate() {
            child.set_handle(self.handle.child_handle(i))
        }
    }

    pub fn append(&mut self, mut child: DomNode<C>) {
        assert!(self.handle.is_valid());
        // TODO: copied into 2 places - move into Element?

        let child_index = self.children.len();
        let child_handle = self.handle.child_handle(child_index);
        child.set_handle(child_handle);
        self.children.push(child);
    }

    fn replace_child(&mut self, index: usize, nodes: Vec<DomNode<C>>) {
        assert!(self.handle.is_valid());
        assert!(index < self.children().len());
        // TODO: copied into 2 places - move into Element?

        self.children.remove(index);
        let mut current_index = index;
        for mut node in nodes {
            let child_handle = self.handle.child_handle(current_index);
            node.set_handle(child_handle);
            self.children.insert(current_index, node);
            current_index += 1;
        }

        for child_index in current_index..self.children.len() {
            let new_handle = self.handle.child_handle(child_index);
            self.children[child_index].set_handle(new_handle);
        }
    }
}

impl<'a, C: Clone> Element<'a, C> for FormattingNode<C> {
    fn name(&'a self) -> &'a Vec<C> {
        &self.name
    }

    fn children(&'a self) -> &'a Vec<DomNode<C>> {
        &self.children
    }

    fn children_mut(&'a mut self) -> &'a mut Vec<DomNode<C>> {
        &mut self.children
    }
}

impl ToHtml<u16> for FormattingNode<u16> {
    fn fmt_html(&self, f: &mut HtmlFormatter<u16>) {
        fmt_element_u16(self, f)
    }
}

/* TODO
#[derive(Clone, Debug, PartialEq)]
struct ItemNode {}

impl Display for ItemNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}
*/

#[derive(Clone, Debug, PartialEq)]
pub struct TextNode<C> {
    data: Vec<C>,
    handle: DomHandle,
}

impl<C> TextNode<C> {
    /// Create a new TextNode
    ///
    /// NOTE: Its handle() will be invalid until you call set_handle() or
    /// append() it to another node.
    pub fn from(data: Vec<C>) -> Self
    where
        C: Clone,
    {
        Self {
            data,
            handle: DomHandle::new_invalid(),
        }
    }

    pub fn data(&self) -> &[C] {
        &self.data
    }

    pub fn set_data(&mut self, data: Vec<C>) {
        self.data = data;
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    fn handle(&self) -> DomHandle {
        self.handle.clone()
    }

    fn set_handle(&mut self, handle: DomHandle) {
        self.handle = handle;
    }
}

impl ToHtml<u16> for TextNode<u16> {
    fn fmt_html(&self, f: &mut HtmlFormatter<u16>) {
        f.write(&self.data)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum DomNode<C>
where
C: Clone {
    Container(ContainerNode<C>),   // E.g. html, div
    Formatting(FormattingNode<C>), // E.g. b, i
    // TODO Item(ItemNode<C>),             // E.g. a, pills
    Text(TextNode<C>),
}

impl<C> DomNode<C>
where
C: Clone {
    pub fn handle(&self) -> DomHandle {
        match self {
            DomNode::Container(n) => n.handle(),
            DomNode::Formatting(n) => n.handle(),
            DomNode::Text(n) => n.handle(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Text(node) => node.len(),
            Self::Formatting(node) => node.len(),
            Self::Container(node) => node.len(),
        }
    }

    fn set_handle(&mut self, handle: DomHandle) {
        match self {
            DomNode::Container(n) => n.set_handle(handle),
            DomNode::Formatting(n) => n.set_handle(handle),
            DomNode::Text(n) => n.set_handle(handle),
        }
    }
}
impl ToHtml<u16> for DomNode<u16> {
    fn fmt_html(&self, f: &mut HtmlFormatter<u16>) {
        match self {
            DomNode::Container(s) => s.fmt_html(f),
            DomNode::Formatting(s) => s.fmt_html(f),
            // TODO DomNode::Item(s) => s.fmt_html(f),
            DomNode::Text(s) => s.fmt_html(f),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

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

    fn dom<'a, C>(children: impl IntoIterator<Item = &'a DomNode<C>>) -> Dom<C>
    where
        C: 'static + Clone,
    {
        Dom::new(clone_children(children))
    }

    fn b<'a>(
        children: impl IntoIterator<Item = &'a DomNode<u16>>,
    ) -> DomNode<u16> {
        DomNode::Formatting(FormattingNode::new(
            utf16("b"),
            clone_children(children),
        ))
    }

    fn i<'a>(
        children: impl IntoIterator<Item = &'a DomNode<u16>>,
    ) -> DomNode<u16> {
        DomNode::Formatting(FormattingNode::new(
            utf16("i"),
            clone_children(children),
        ))
    }

    fn tx(data: &str) -> DomNode<u16> {
        DomNode::Text(TextNode::from(utf16(data)))
    }

    /// If this node is an element, return its children - otherwise panic
    fn kids<C: Clone>(node: &DomNode<C>) -> &Vec<DomNode<C>> {
        match node {
            DomNode::Container(n) => n.children(),
            DomNode::Formatting(n) => n.children(),
            DomNode::Text(_) => {
                panic!("We expected an Element, but found Text")
            }
        }    
    }   

    // Creation and handles

    #[test]
    fn can_create_a_dom_and_add_nodes() {
        // Create a simple DOM
        let dom = Dom::new(vec![
            DomNode::Text(TextNode::from("a".to_html())),
            DomNode::Formatting(FormattingNode::new(
                "b".to_html(),
                vec![DomNode::Text(TextNode::from("b".to_html()))],
            )),
        ]);

        // The DOM was created successfully
        assert_eq!(dom.to_string(), "a<b>b</b>");
    }

    #[test]
    fn can_find_toplevel_nodes_via_handles() {
        // Create a simple DOM
        let dom = Dom::new(vec![
            DomNode::Text(TextNode::from("a".to_html())),
            DomNode::Formatting(FormattingNode::new(
                "b".to_html(),
                vec![DomNode::Text(TextNode::from("b".to_html()))],
            )),
        ]);

        let child0 = &dom.children()[0];
        let child1 = &dom.children()[1];

        // The handles point to the right nodes
        assert_eq!(dom.lookup_node(child0.handle()), child0);
        assert_eq!(dom.lookup_node(child1.handle()), child1);
    }

    #[test]
    fn can_find_deep_nodes_via_handles() {
        let dom = dom(&[
            tx("foo"),
            b(&[tx("BOLD"), b(&[tx("uberbold")])]),
            tx("bar"),
        ]);

        // Given a DOM with a nested node
        let nested_node = &kids(&kids(&dom.children()[1])[1])[0];

        // When we ask for its handle
        let handle = nested_node.handle();

        // Then we can look it up and find the same node
        assert_eq!(dom.lookup_node(handle), nested_node);
    }

    #[test]
    fn can_replace_toplevel_node_with_multiple_nodes() {
        let mut dom = dom(&[
            tx("foo"),
            tx("bar"),
        ]);

        let node = &dom.children()[0];
        let inserted_nodes = vec![
            tx("ab"),
            b(&[tx("cd")]),
            tx("ef"),
        ];

        dom.replace(node.handle(), inserted_nodes);

        // Node is replaced by new insertion
        assert_eq!(dom.to_string(), "ab<b>cd</b>efbar");
        // Subsequent node handle is properly updated
        let bar_node = &dom.children()[3];
        assert_eq!(bar_node.handle().index_in_parent(), 3);
    }

    #[test]
    fn can_replace_deep_node_with_multiple_nodes() {
        let mut dom = dom(&[
            b(&[tx("foo")]),
        ]);

        let node = &kids(&dom.children()[0])[0];
        let inserted_nodes = vec![
            tx("f"),
            i(&[tx("o")]),
            tx("o"),
        ];

        dom.replace(node.handle(), inserted_nodes);

        // Node is replaced by new insertion
        assert_eq!(dom.to_string(), "<b>f<i>o</i>o</b>");
    }

    // Serialisation

    #[test]
    fn empty_dom_serialises_to_empty_string() {
        assert_eq!(dom(&[]).to_string(), "");
    }

    #[test]
    fn plain_text_serialises_to_just_the_text() {
        assert_eq!(dom(&[tx("foo")]).to_string(), "foo");
    }

    #[test]
    fn mixed_text_and_tags_serialises() {
        assert_eq!(
            dom(&[tx("foo"), b(&[tx("BOLD")]), tx("bar")]).to_string(),
            "foo<b>BOLD</b>bar"
        );
    }

    #[test]
    fn nested_tags_serialise() {
        assert_eq!(
            dom(&[
                tx("foo"),
                b(&[tx("BO"), i(&[tx("LD")])]),
                i(&[tx("it")]),
                tx("bar")
            ])
            .to_string(),
            "foo<b>BO<i>LD</i></b><i>it</i>bar"
        );
    }

    #[test]
    fn empty_tag_serialises() {
        assert_eq!(dom(&[b(&[]),]).to_string(), "<b></b>");
    }

    #[test]
    fn new_adds_cached_positions() {
        let mut d = dom(&[tx("Node"), tx("Another")]);
        assert_eq!(1, d.handles_for_start.get(&0).unwrap().len()); // Root & 'Node'
        assert_eq!(1, d.handles_for_start.get(&4).unwrap().len()); // 'Another'
        assert_eq!(2, d.positions_for_handles.len());

        let start_handle = DomHandle { path: vec![0] };
        let text_node = d.lookup_node(start_handle.clone());
        assert_eq!(1, d.handles_for_start.get(&0).unwrap().len());
        assert_eq!(0, d.positions_for_handles.get(&start_handle).unwrap().start);
        assert_eq!(4, d.positions_for_handles.get(&DomHandle { path: vec![1] }).unwrap().start);
    }

    #[test]
    fn append_adds_cached_positions() {
        let mut d = dom(&[]);
        d.append(tx("Node"));
        assert_eq!(1, d.handles_for_start.get(&0).unwrap().len());
        assert_eq!(1, d.positions_for_handles.len());

        let dom_handle = DomHandle { path: vec![0] };
        let text_node = d.lookup_node(dom_handle.clone());
        assert_eq!(1, d.handles_for_start.get(&0).unwrap().len());
        assert_eq!(0, d.positions_for_handles.get(&dom_handle).unwrap().start);
    }

    #[test]
    fn replace_adds_cached_positions() {
        let mut d = dom(&[tx("Old"), tx("Node")]);
        let handle = DomHandle { path: vec![0] };
        d.replace(handle, vec![tx("BrandNew")]);

        assert_eq!(1, d.handles_for_start.get(&0).unwrap().len());
        let start = d.positions_for_handles.get(&DomHandle { path: vec![1] }).unwrap().start;
        assert_eq!(8, start);
    }

    /*#[test]
    fn finding_range_within_complex_tags_doesnt_work_yet() {
        // TODO: we can't do this yet
        let d = dom(&[tx("foo "), b(&[tx("bar")]), tx(" baz")]);
        let range = d.find_range(4, 7);
        assert_eq!(range, Range::TooDifficultForMe);
    }*/

    // TODO: copy tests from examples/example-web/test.js
    // TODO: improve tests when we have HTML parsing
}
