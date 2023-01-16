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

//! Methods on Dom that modify its contents and are guaranteed to conform to
//! our invariants e.g. no empty text nodes, no adjacent text nodes.

use crate::dom::nodes::dom_node::DomNodeKind::{Generic, ListItem, Paragraph};
use crate::dom::unicode_string::UnicodeStr;
use crate::dom::DomLocation;
use crate::{DomHandle, DomNode, UnicodeString};

use super::action_list::{DomAction, DomActionList};
use super::nodes::dom_node::DomNodeKind;
use super::nodes::{ContainerNode, TextNode};
use super::unicode_string::{UnicodeStrExt, UnicodeStringExt};
use super::{Dom, Range};

impl<S> Dom<S>
where
    S: UnicodeString,
{
    /// Removes node at given handle from the dom, and if it has children
    /// moves them to its parent container children.
    pub fn remove_and_keep_children(&mut self, node_handle: &DomHandle) {
        #[cfg(any(test, feature = "assert-invariants"))]
        self.assert_invariants();

        let parent = self.parent_mut(node_handle);
        let index = node_handle.index_in_parent();
        let node = parent.remove_child(index);
        let mut last_index = index;
        if let DomNode::Container(mut node) = node {
            for i in (0..node.children().len()).rev() {
                let child = node.remove_child(i);
                parent.insert_child(index, child);
                last_index += 1;
            }
        }

        // Clean up any adjacent text nodes
        if last_index > 0 {
            merge_if_adjacent_text_nodes(parent, last_index - 1);
        }
        if index > 0 {
            merge_if_adjacent_text_nodes(parent, index - 1);
        }

        #[cfg(any(test, feature = "assert-invariants"))]
        self.assert_invariants();
    }

    pub fn replace_text_in(&mut self, new_text: S, start: usize, end: usize) {
        #[cfg(any(test, feature = "assert-invariants"))]
        self.assert_invariants();

        let range = self.find_range(start, end);
        let (start_block, end_block) =
            self.top_most_block_nodes_in_range(&range);
        let deleted_handles = if range.is_empty() {
            if !new_text.is_empty() {
                self.append_at_end_of_document(DomNode::new_text(new_text));
            }
            Vec::new()
        // We check for the first starting_link_handle if any
        // Because for links we always add the text to the next sibling
        } else if let Some(starting_link_handle) =
            first_shrinkable_link_node_handle(&range)
        {
            // We replace and delete as normal with an empty string on the current range
            let deleted_handles =
                self.replace_multiple_nodes(&range, "".into());
            // Then we set the new text value in the next sibling node (or create a new one if none exists)
            self.set_new_text_in_next_sibling_node(
                starting_link_handle,
                new_text,
            );
            deleted_handles
        } else {
            self.replace_multiple_nodes(&range, new_text)
        };

        self.merge_adjacent_text_nodes_after_replace(range, deleted_handles);

        // If text was replaced, not inserted
        let needs_to_merge_block_nodes =
            if let (Some(start_block), Some(end_block)) =
                (start_block, end_block)
            {
                start_block != end_block
            } else {
                false
            };
        if start != end && needs_to_merge_block_nodes {
            let (start_block, end_block) =
                self.top_most_block_nodes_in_boundary(start);
            if let (Some(start_block_loc), Some(end_block_loc)) =
                (start_block, end_block)
            {
                // If there are adjacent block nodes as a result of replacing text
                if start_block_loc != end_block_loc {
                    let end_block = self.remove(&end_block_loc.node_handle);
                    let DomNode::Container(end_block) = end_block else {
                        panic!("Ending block node must be a container node")
                    };
                    let removed_items = end_block.take_children();
                    if let DomNode::Container(start_block) =
                        self.lookup_node_mut(&start_block_loc.node_handle)
                    {
                        // Merge contents in `start_block`
                        start_block.append_children(removed_items);
                    } else {
                        panic!("Starting block node must be a container node");
                    }

                    if end_block_loc.node_handle.has_parent() {
                        self.remove_empty_nodes_recursively(
                            &end_block_loc.node_handle.parent_handle(),
                        );
                    }

                    self.join_nodes_in_container(&start_block_loc.node_handle);
                }
            }
        }
        #[cfg(any(test, feature = "assert-invariants"))]
        self.assert_invariants();
    }

    fn top_most_block_nodes_in_boundary(
        &self,
        selection_pos: usize,
    ) -> (Option<DomLocation>, Option<DomLocation>) {
        let range = self.find_range(selection_pos + 1, selection_pos + 1);
        self.top_most_block_nodes_in_range(&range)
    }

    fn top_most_block_nodes_in_range(
        &self,
        range: &Range,
    ) -> (Option<DomLocation>, Option<DomLocation>) {
        let mut start = None;
        let mut end = None;
        let mut sorted_locations = range.locations.clone();
        sorted_locations.sort();
        for location in sorted_locations {
            if location.kind.is_block_kind() {
                if location.is_start() && location.end_offset == location.length
                {
                    start = Some(location.clone());
                } else if location.is_end() {
                    end = Some(location.clone());
                }
            }
        }
        (start, end)
    }

    fn remove_empty_nodes_recursively(&mut self, handle: &DomHandle) {
        let needs_removal = if let DomNode::Container(container) =
            self.lookup_node_mut(&handle)
        {
            container.is_empty()
        } else {
            false
        };
        if needs_removal {
            self.remove(&handle);
        }

        if handle.has_parent() {
            self.remove_empty_nodes_recursively(&handle.parent_handle());
        }
    }

    /// Deletes the given [to_delete] nodes and then removes any given parent nodes that became
    /// empty, recursively.
    /// Returns a list of all the nodes that were deleted
    /// TODO: This function does not preserve invariants - the functions that
    /// call it should be moved into dom_methods, and they should grow
    /// assertions at the beginning and end that they preserve invariants,
    /// then this function can become non-public. (All public methods in
    /// dom_methods should preserve invariants.)
    pub(crate) fn delete_nodes(
        &mut self,
        mut to_delete: Vec<DomHandle>,
    ) -> Vec<DomHandle> {
        let mut deleted = Vec::new();

        // Delete in reverse order to avoid invalidating handles
        to_delete.sort();

        // We repeatedly delete to ensure anything that became empty because
        // of deletions is itself deleted.
        while !to_delete.is_empty() {
            let handle = to_delete.pop().unwrap();
            if handle.is_root() {
                continue;
            }

            self.remove(&handle);
            deleted.push(handle.clone());
            let parent = self.parent(&handle);
            let parent_handle = parent.handle();
            if parent.children().is_empty()
                && !to_delete.contains(&parent_handle)
                && !deleted.contains(&parent_handle)
            {
                to_delete.push(parent_handle);
            }
        }
        deleted
    }

    fn set_new_text_in_next_sibling_node(
        &mut self,
        node_handle: DomHandle,
        new_text: S,
    ) {
        if let Some(sibling_text_node) =
            self.first_next_sibling_text_node_mut(&node_handle)
        {
            let mut data = sibling_text_node.data().to_owned();
            data.insert(0, &new_text);
            sibling_text_node.set_data(data);
        } else if !new_text.is_empty() {
            let new_child = DomNode::new_text(new_text);
            let parent = self.parent_mut(&node_handle);
            let index = node_handle.index_in_parent() + 1;
            parent.insert_child(index, new_child);
        }
    }

    fn first_next_sibling_text_node_mut(
        &mut self,
        node_handle: &DomHandle,
    ) -> Option<&mut TextNode<S>> {
        let parent = self.parent(node_handle);
        let children_number = parent.children().len();
        if node_handle.index_in_parent() < children_number - 1 {
            let sibling = self.lookup_node_mut(&node_handle.next_sibling());
            let DomNode::Text(sibling_text_node) = sibling else {
                return None
            };
            Some(sibling_text_node)
        } else {
            None
        }
    }

    /// Returns a list of handles to all the nodes that we deleted
    fn replace_multiple_nodes(
        &mut self,
        range: &Range,
        new_text: S,
    ) -> Vec<DomHandle> {
        let len = new_text.len();
        let action_list = self.replace_in_text_nodes(range.clone(), new_text);

        let (to_add, to_delete, _) = action_list.grouped();
        let to_delete: Vec<DomHandle> =
            to_delete.into_iter().map(|a| a.handle).collect();

        // We only add nodes in one special case: when the selection ends at
        // a BR tag. In that case, the only nodes that might be deleted are
        // going to be before the one we add here, so their handles won't be
        // invalidated by the add we do here.
        for add_action in to_add.into_iter().rev() {
            let parent_handle = &add_action.parent_handle;
            let parent = self.lookup_node_mut(parent_handle);
            if let DomNode::Container(parent) = parent {
                parent.insert_child(add_action.index, add_action.node);
            } else {
                panic!("Parent was not a container!");
            }
        }

        // Delete the nodes marked for deletion
        let deleted_handles = if !to_delete.is_empty() {
            self.delete_nodes(to_delete)
        } else {
            Vec::new()
        };

        // If our range has length and covered multiple text-like nodes, join together
        // the two sides of the range.
        if range.start() != range.end() && range.leaves().count() > 1 {
            // join_nodes will use the first location of our range, so we must
            // check whether we deleted it!
            if let Some(first_loc) = range.locations.first() {
                if !deleted_handles.contains(&first_loc.node_handle) {
                    // Calculate the position 1 code unit after the end of the
                    // range, after the in-between characters have been
                    // deleted, and the new characters have been inserted.
                    let new_pos = range.start() + len + 1;

                    // join_nodes only requires that the first location in
                    // the supplied range has a valid handle.
                    // We think it's OK to pass in a range where later
                    // locations have been deleted.
                    // TODO: can we just pass in this handle, to avoid the
                    // ambiguity here?
                    self.join_nodes(range, new_pos);
                }
            }
        } else if let Some(first_leave) = range.leaves().next() {
            if let Some(ancestor_handle) = self
                .find_first_non_matching_ancestor_in(
                    &deleted_handles,
                    &first_leave.node_handle,
                )
            {
                self.join_nodes_in_container(&ancestor_handle);
            }
        }

        deleted_handles
    }

    fn find_first_non_matching_ancestor_in(
        &self,
        list: &Vec<DomHandle>,
        node_handle: &DomHandle,
    ) -> Option<DomHandle> {
        fn parent_handle_in_list(
            list: &Vec<DomHandle>,
            handle: &DomHandle,
        ) -> Option<DomHandle> {
            if handle.has_parent() {
                let parent_handle = handle.parent_handle();
                if !list.contains(&parent_handle) {
                    Some(parent_handle)
                } else {
                    parent_handle_in_list(list, &parent_handle)
                }
            } else {
                None
            }
        }
        parent_handle_in_list(list, node_handle)
    }

    pub(crate) fn join_nodes_in_container(
        &mut self,
        container_handle: &DomHandle,
    ) {
        let child_count = if let DomNode::Container(container) =
            self.lookup_node(container_handle)
        {
            container.children().len()
        } else {
            panic!("Parent node should be a container");
        };

        if child_count > 0 {
            for i in (0..child_count - 1).rev() {
                let handle = container_handle.child_handle(i);
                let next_handle = container_handle.child_handle(i + 1);
                let next_node = self.lookup_node(&next_handle);
                let node = self.lookup_node(&handle);

                if node.can_push(next_node) {
                    let mut next_node = self.remove(&next_handle);
                    let node_mut = self.lookup_node_mut(&handle);
                    node_mut.push(&mut next_node);
                }
            }
        }
    }

    /// Given a range to replace and some new text, modify the nodes in the
    /// range to replace the text with the supplied text.
    /// Returns a list of actions to be done to the Dom (add or remove nodes).
    /// NOTE: all nodes to be created are later in the Dom than all nodes to
    /// be deleted, so you can safely add them before performing the
    /// deletions, and the handles of the deletions will remain valid.
    fn replace_in_text_nodes(
        &mut self,
        range: Range,
        new_text: S,
    ) -> DomActionList<S> {
        let mut action_list = DomActionList::default();
        let mut first_text_node = true;

        for loc in range.locations.iter() {
            let mut node = self.lookup_node_mut(&loc.node_handle);
            match &mut node {
                DomNode::Container(_) => {
                    if loc.length == 0
                        && loc.kind.is_block_kind()
                        && loc.kind != Generic
                    {
                        // Empty block node
                        if new_text.is_empty() {
                            action_list.push(DomAction::remove_node(
                                loc.node_handle.clone(),
                            ));
                            first_text_node = false;
                        } else if first_text_node {
                            match loc.kind {
                                Paragraph | ListItem => {
                                    let text_node = DomNode::new_text(new_text.clone());
                                    action_list.push(DomAction::add_node(
                                        loc.node_handle.clone(),
                                        0,
                                        text_node,
                                    ));
                                    first_text_node = false;
                                },
                                _ => panic!("A block node that can't contain inline nodes was selected, text can't be added to it."),
                            }
                        }
                    }
                }
                DomNode::LineBreak(_) => {
                    match (loc.start_offset, loc.end_offset) {
                        (0, 1) => {
                            // Whole line break is selected, delete it
                            action_list.push(DomAction::remove_node(
                                loc.node_handle.clone(),
                            ));
                        }
                        (1, 1) => {
                            // Cursor is after line break, no need to delete
                        }
                        (0, 0) => {
                            if first_text_node && !new_text.is_empty() {
                                action_list.push(DomAction::add_node(
                                    loc.node_handle.parent_handle(),
                                    loc.node_handle.index_in_parent(),
                                    DomNode::new_text(new_text.clone()),
                                ));
                                first_text_node = false;
                            }
                        }
                        _ => panic!(
                            "Tried to insert text into a line break with offset != 0 or 1. \
                            Start offset: {}, end offset: {}",
                            loc.start_offset,
                            loc.end_offset,
                        ),
                    }
                }
                DomNode::Text(node) => {
                    let old_data = node.data();

                    // If this is not the first node, and the selections spans
                    // it, delete it.
                    if loc.start_offset == 0
                        && loc.end_offset == old_data.len()
                        && !first_text_node
                    {
                        action_list.push(DomAction::remove_node(
                            loc.node_handle.clone(),
                        ));
                    } else {
                        // Otherwise, delete the selected text
                        let mut new_data =
                            old_data[..loc.start_offset].to_owned();

                        // and replace with the new content
                        if first_text_node {
                            new_data.push(new_text.deref());
                        }

                        new_data.push(&old_data[loc.end_offset..]);
                        if new_data.is_empty() {
                            action_list.push(DomAction::remove_node(
                                loc.node_handle.clone(),
                            ));
                        } else {
                            node.set_data(new_data);
                        }
                    }

                    first_text_node = false;
                }
                _ => panic!("Unknown type of leaf node found"),
            }
        }

        // If text wasn't added in any previous iteration, just append it next to the last leaf
        if first_text_node && !new_text.is_empty() {
            let last_leaf = range.leaves().last().unwrap();
            action_list.push(DomAction::add_node(
                last_leaf.node_handle.parent_handle(),
                last_leaf.node_handle.index_in_parent() + 1,
                DomNode::new_text(new_text.clone()),
            ));
        }

        action_list
    }

    fn merge_adjacent_text_nodes_after_replace(
        &mut self,
        replaced_range: Range,
        deleted_handles: Vec<DomHandle>,
    ) {
        // If we've ended up with adjacent text nodes, merge them
        if let Some(first_location) = replaced_range.locations.first() {
            let first_handle = &first_location.node_handle;
            if deleted_handles.contains(first_handle) {
                // If we deleted the first node in the range ...
                if first_handle.index_in_parent() > 0 {
                    // ... and that was not the first in its parent,
                    // then merge the node before with the next.
                    let prev_handle = first_handle.prev_sibling();
                    self.merge_text_nodes_around(&prev_handle);
                }
            } else {
                // If the first node of the range still exists, then
                // merge it with the next, and potentially also the
                // previous.
                self.merge_text_nodes_around(&first_location.node_handle);
            }
        }
    }

    fn merge_text_nodes_around(&mut self, handle: &DomHandle) {
        // TODO: make this method not public because it is used to make
        // the invariants true, instead of assuming they are true at the
        // beginning!
        // Instead, move another method into here, and make it call this one.

        let parent = self.parent_mut(handle);
        let idx = handle.index_in_parent();
        if idx > 0 {
            merge_if_adjacent_text_nodes(parent, idx - 1);
        }
        merge_if_adjacent_text_nodes(parent, idx);

        #[cfg(any(test, feature = "assert-invariants"))]
        self.assert_invariants();
    }

    /// Returns two new subtrees as the result of splitting the Dom symmetrically without mutating
    /// itself. Also returns the new handles of node that was split.
    ///
    /// Only returns nodes that are modified by the split and ignores any nodes which were not
    /// either split or contain a node that was split.
    pub(crate) fn split_new_sub_trees(
        &self,
        from_handle: &DomHandle,
        offset: usize,
        depth: usize,
    ) -> (Dom<S>, DomHandle, Dom<S>, DomHandle) {
        let mut clone = self.clone();
        let right = clone.split_sub_tree_from(from_handle, offset, depth);

        // Remove unmodified children of the right split
        let mut right = right.into_container().take_children();
        right.truncate(1);

        // Remove unmodified children of the left split
        let mut left = clone
            .into_node(&from_handle.sub_handle_up_to(depth))
            .into_container()
            .unwrap()
            .take_children();
        let left = left.split_off(left.len() - 1);

        // Reset the handle roots after unmodified siblings were removed
        let mut right_handle =
            from_handle.sub_handle_down_from(depth).raw().to_owned();
        right_handle[0] = 0;
        let right_handle = DomHandle::from_raw(right_handle);

        let mut left_handle =
            from_handle.sub_handle_down_from(depth).raw().to_owned();
        left_handle[0] = 0;
        let left_handle = DomHandle::from_raw(left_handle);

        (Dom::new(left), left_handle, Dom::new(right), right_handle)
    }

    /// Splits the current tree at the given handle, returning the 'right' side of the split tree, after the given handle to the end of the Dom.
    /// The 'left' part will remain in the Dom.
    ///
    /// * `from_handle` - the position of the first node to extract.
    /// * `start_offset` - the position within the given first node to split.
    /// * `depth` - the depth within the original tree at which to make the returned tree's root
    pub(crate) fn split_sub_tree_from(
        &mut self,
        from_handle: &DomHandle,
        start_offset: usize,
        depth: usize,
    ) -> Dom<S> {
        self.split_sub_tree(from_handle, start_offset, None, usize::MAX, depth)
    }

    /// Extract the tree between the given 2 handles, splitting the Dom in 2 parts. The previous and next parts stays in the tree and this
    /// function returns the extracted sub-tree.
    ///
    /// * `from_handle` - the position of the first node to extract.
    /// * `start_offset` - the position within the given first node to split.
    /// * `to_handle` - the position of the last node to extract.
    /// * `end_offset` - the position within the given last node to split. If if should cover the whole node, use `usize::MAX`.
    /// * `depth` - the depth within the original tree at which to make the returned tree's root
    pub(crate) fn split_sub_tree_between(
        &mut self,
        from_handle: &DomHandle,
        start_offset: usize,
        to_handle: &DomHandle,
        end_offset: usize,
        depth: usize,
    ) -> Dom<S> {
        self.split_sub_tree(
            from_handle,
            start_offset,
            Some(to_handle.clone()),
            end_offset,
            depth,
        )
    }

    /// Extract the tree between the given 2 handles if `to_handle` is not `None`.
    /// Otherwise, splits the Dom into 2 parts, extracting this second part and returning it.
    ///
    /// * `from_handle` - the position of the first node to extract.
    /// * `start_offset` - the position within the given first node to split.
    /// * `to_handle` - the position of the last node to extract, if any. Use `None` otherwise.
    /// * `end_offset` - the position within the given last node to split. If if should cover the whole node, use `usize::MAX`.
    /// * `depth` - the depth within the original tree at which to make the returned tree's root
    pub fn split_sub_tree(
        &mut self,
        from_handle: &DomHandle,
        start_offset: usize,
        to_handle: Option<DomHandle>,
        end_offset: usize,
        depth: usize,
    ) -> Dom<S> {
        let cur_handle = from_handle.sub_handle_up_to(depth);
        let mut subtree_children = self.split_sub_tree_at_index(
            cur_handle,
            start_offset,
            end_offset,
            from_handle,
            to_handle,
        );

        // Create new 'root' node to contain the split sub-tree
        let new_subtree = subtree_children.remove(0);
        Dom::new_with_root(new_subtree)
    }

    fn split_sub_tree_at_index<'a>(
        &'a mut self,
        cur_handle: DomHandle,
        start_offset: usize,
        end_offset: usize,
        from_handle: &'a DomHandle,
        to_handle: Option<DomHandle>,
    ) -> Vec<DomNode<S>> {
        let mut nodes = Vec::new();

        let is_container_node: bool;
        let is_text_node: bool;
        {
            let node = self.lookup_node(&cur_handle);
            is_container_node = node.is_container_node();
            is_text_node = node.is_text_node();
        }

        if is_container_node {
            nodes.extend(self.split_sub_tree_at_container(
                cur_handle,
                start_offset,
                end_offset,
                from_handle,
                to_handle,
            ));
        } else if is_text_node {
            nodes.extend(self.split_sub_tree_at_text_node(
                cur_handle,
                start_offset,
                end_offset,
                from_handle,
                to_handle,
            ));
        } else {
            nodes.push(self.remove(&cur_handle));
        }

        nodes
    }

    fn split_sub_tree_at_container<'a>(
        &'a mut self,
        cur_handle: DomHandle,
        start_offset: usize,
        end_offset: usize,
        from_handle: &'a DomHandle,
        to_handle: Option<DomHandle>,
    ) -> Vec<DomNode<S>> {
        let depth = cur_handle.depth();
        let mut child_count = 0;
        let min_child_index: usize =
            if is_ancestor_or_self(&cur_handle, from_handle) {
                sub_handle_up_to_or_none(from_handle, depth + 1)
                    .map_or(0, |h| h.index_in_parent())
            } else {
                0
            };
        let max_child_index = if let DomNode::Container(container) =
            self.lookup_node(&cur_handle)
        {
            child_count = container.children().len();
            to_handle.clone().map_or(child_count, |to_handle| {
                if is_ancestor_or_self(&cur_handle, &to_handle) {
                    sub_handle_up_to_or_none(&to_handle, depth + 1)
                        .map_or(child_count, |h| h.index_in_parent() + 1)
                } else {
                    child_count
                }
            })
        } else {
            usize::MAX
        };

        let mut child_nodes = Vec::new();
        for i in (min_child_index..max_child_index).rev() {
            let child_path = cur_handle.child_handle(i);
            let mut new_children = self.split_sub_tree_at_index(
                child_path,
                start_offset,
                end_offset,
                from_handle,
                to_handle.clone(),
            );
            new_children.extend(child_nodes);
            child_nodes = new_children;
        }

        let result: Vec<DomNode<S>>;
        let mut needs_to_remove_container = false;
        if let DomNode::Container(container) = self.lookup_node(&cur_handle) {
            if !container.handle().is_root()
                && container.is_empty()
                && child_count > 0
            {
                needs_to_remove_container = true;
            }
            result = vec![DomNode::Container(
                container.clone_with_new_children(child_nodes),
            )]
        } else {
            result = Vec::new();
        }

        if needs_to_remove_container {
            self.remove(&cur_handle);
        }

        result
    }

    fn split_sub_tree_at_text_node<'a>(
        &'a mut self,
        cur_handle: DomHandle,
        start_offset: usize,
        end_offset: usize,
        from_handle: &'a DomHandle,
        to_handle: Option<DomHandle>,
    ) -> Vec<DomNode<S>> {
        let mut nodes = Vec::new();
        let DomNode::Text(text_node) = self.lookup_node_mut(&cur_handle) else {
            panic!("Found node must be a TextNode");
        };
        if (cur_handle == *from_handle
            || (from_handle.is_ancestor_of(&cur_handle)
                && cur_handle.index_in_parent() == 0))
            && (1..=text_node.data().chars().count()).contains(&start_offset)
        {
            let left_data = text_node.data()[..start_offset].to_owned();
            let right_data = text_node.data()[start_offset..].to_owned();
            text_node.set_data(left_data);
            if !right_data.is_empty() {
                nodes.push(DomNode::new_text(right_data));
            }
        } else if to_handle.is_some()
            && cur_handle == to_handle.unwrap()
            && (1..=text_node.data().chars().count()).contains(&end_offset)
        {
            let left_data = text_node.data()[..end_offset].to_owned();
            let right_data = text_node.data()[end_offset..].to_owned();
            text_node.set_data(left_data);
            if !right_data.is_empty() {
                nodes.push(DomNode::new_text(right_data));
            }
        } else {
            nodes.push(self.remove(&cur_handle));
        }
        nodes
    }

    pub fn adds_line_break(&self, handle: &DomHandle) -> bool {
        let node = self.lookup_node(&handle);
        let is_block_node = node.is_block_node();
        if !is_block_node || handle.is_root() {
            return false;
        }

        let parent = self.parent(&handle);
        let child_count = parent.children().len();

        node.handle().index_in_parent() + 1 < child_count
    }
}

/// Look at the children of parent at index and index + 1. If they are both
/// text nodes, merge them into the first and delete the second.
/// If either child does not exist, do nothing.
fn merge_if_adjacent_text_nodes<S>(parent: &mut ContainerNode<S>, index: usize)
where
    S: UnicodeString,
{
    let previous_child = parent.children().get(index);
    let after_child = parent.children().get(index + 1);
    if let (Some(DomNode::Text(t1)), Some(DomNode::Text(t2))) =
        (previous_child, after_child)
    {
        let mut data = t1.data().to_owned();
        data.push(t2.data());
        if let Some(DomNode::Text(t1_mut)) = parent.get_child_mut(index) {
            t1_mut.set_data(data);
            parent.remove_child(index + 1);
        } else {
            unreachable!("t1 was a text node but t1_mut was not!");
        }
    }
}

fn first_shrinkable_link_node_handle(range: &Range) -> Option<DomHandle> {
    let Some(link_loc) = range.locations.iter().find(|loc| {
            loc.kind == DomNodeKind::Link && !loc.is_covered() && loc.is_start()
        }) else {
            return None
        };
    Some(link_loc.node_handle.clone())
}

fn sub_handle_up_to_or_none(
    handle: &DomHandle,
    depth: usize,
) -> Option<DomHandle> {
    if handle.depth() >= depth {
        Some(handle.sub_handle_up_to(depth))
    } else {
        None
    }
}
fn is_ancestor_or_self(ancestor: &DomHandle, handle: &DomHandle) -> bool {
    ancestor.is_ancestor_of(handle) || ancestor == handle
}

#[cfg(test)]
mod test {
    use crate::dom::DomHandle;
    use crate::tests::testutils_composer_model::{cm, tx};
    use crate::ToHtml;

    #[test]
    fn delete_nodes_refuses_to_delete_root() {
        let mut model = cm("|");
        model
            .state
            .dom
            .delete_nodes(vec![model.state.dom.document_handle()]);
        assert_eq!(tx(&model), "|")
    }

    #[test]
    fn delete_nodes_refuses_recursively_to_delete_root() {
        let mut model = cm("a|");
        model
            .state
            .dom
            .delete_nodes(vec![DomHandle::from_raw(vec![0])]);
        assert_eq!(tx(&model), "|")
    }

    #[test]
    fn split_dom_simple() {
        let mut model = cm("Text|<b>bold</b><i>italic</i>");
        let ret = model.state.dom.split_sub_tree_from(
            &DomHandle::from_raw(vec![1, 0]),
            2,
            0,
        );
        assert_eq!(model.state.dom.to_html(), "Text<b>bo</b>");
        assert_eq!(ret.to_html().to_string(), "<b>ld</b><i>italic</i>");
    }

    #[test]
    fn split_dom_with_nested_formatting() {
        let mut model = cm("<u>Text|<b>bold</b><i>italic</i></u>");
        let ret = model.state.dom.split_sub_tree_from(
            &DomHandle::from_raw(vec![0, 1, 0]),
            2,
            0,
        );
        assert_eq!(model.state.dom.to_html(), "<u>Text<b>bo</b></u>");
        assert_eq!(ret.to_html().to_string(), "<u><b>ld</b><i>italic</i></u>");
    }

    #[test]
    fn split_dom_with_nested_formatting_at_sub_level() {
        let mut model = cm("<u>Text|<b>bold</b><i>italic</i></u>");
        let ret = model.state.dom.split_sub_tree_from(
            &DomHandle::from_raw(vec![0, 1, 0]),
            2,
            1,
        );
        assert_eq!(ret.to_html().to_string(), "<u><b>ld</b><i>italic</i></u>");
        assert_eq!(ret.to_html().to_string(), "<u><b>ld</b><i>italic</i></u>");
    }

    #[test]
    fn split_dom_with_lists() {
        let mut model =
            cm("<ul><li>Text|</li><li><b>bold</b><i>italic</i></li></ul>");
        let depth = 0;
        let start_offset = 2;
        let ret = model.state.dom.split_sub_tree_from(
            &DomHandle::from_raw(vec![0, 1, 0, 0]),
            start_offset,
            depth,
        );
        assert_eq!(
            model.state.dom.to_html(),
            "<ul><li>Text</li><li><b>bo</b></li></ul>"
        );
        assert_eq!(
            ret.to_html().to_string(),
            "<ul><li><b>ld</b><i>italic</i></li></ul>"
        )
    }

    #[test]
    fn split_dom_with_lists_at_sub_level() {
        let mut model =
            cm("<ul><li>Text|</li><li><b>bold</b><i>italic</i></li></ul>");
        let depth = 1;
        let start_offset = 2;
        let ret = model.state.dom.split_sub_tree_from(
            &DomHandle::from_raw(vec![0, 1, 0, 0]),
            start_offset,
            depth,
        );
        assert_eq!(
            ret.to_html().to_string(),
            "<ul><li><b>ld</b><i>italic</i></li></ul>"
        );
        assert_eq!(
            ret.to_html().to_string(),
            "<ul><li><b>ld</b><i>italic</i></li></ul>"
        )
    }

    #[test]
    fn split_dom_with_partial_handle() {
        let mut model = cm("<u>Text|<b>bold</b><i>italic</i></u>");
        let ret = model.state.dom.split_sub_tree_from(
            &DomHandle::from_raw(vec![0, 1]), // Handle of <b>
            2,
            0,
        );
        assert_eq!(model.state.dom.to_html(), "<u>Text<b>bo</b></u>");
        assert_eq!(ret.to_html().to_string(), "<u><b>ld</b><i>italic</i></u>");
    }

    #[test]
    fn split_new_sub_trees() {
        let model = cm("Text|<b>bold</b><i>italic</i>");
        let (left, left_handle, right, right_handle) = model
            .state
            .dom
            .split_new_sub_trees(&DomHandle::from_raw(vec![1, 0]), 2, 0);
        assert_eq!(right.to_html(), "<b>ld</b>");
        assert_eq!(right_handle, DomHandle::from_raw(vec![0, 0]));
        assert_eq!(right.lookup_node(&right_handle).to_html(), "ld");
        assert_eq!(left.to_html(), "<b>bo</b>");
        assert_eq!(left_handle, DomHandle::from_raw(vec![0, 0]));
        assert_eq!(left.lookup_node(&left_handle).to_html(), "bo");
    }

    #[test]
    fn split_new_sub_trees_at_depth() {
        let model = cm("<u>Text|<b>bold</b><i>italic</i></u>");
        let (left, left_handle, right, right_handle) = model
            .state
            .dom
            .split_new_sub_trees(&DomHandle::from_raw(vec![0, 1, 0]), 2, 1);
        assert_eq!(right.to_html(), "<b>ld</b>");
        assert_eq!(right_handle, DomHandle::from_raw(vec![0, 0]));
        assert_eq!(right.lookup_node(&right_handle).to_html(), "ld");
        assert_eq!(left.to_html(), "<b>bo</b>");
        assert_eq!(left_handle, DomHandle::from_raw(vec![0, 0]));
        assert_eq!(left.lookup_node(&left_handle).to_html(), "bo");
    }

    #[test]
    fn replace_text_with_code_block() {
        let mut model = cm("<pre>~Te{st</pre>AA}|BB");
        model.delete();
        assert_eq!(tx(&model), "<pre>~Te|</pre>BB");
    }
}
