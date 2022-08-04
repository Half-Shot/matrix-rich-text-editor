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

use std::collections::HashSet;
use std::fmt::Display;

use super::node::{ContainerNode, TextNode};
use super::{qual_name, DomHandle, DomNode};

#[derive(Clone, Debug, PartialEq)]
pub struct DomContainer {
    nodes: Vec<DomNode>,
    document_handle: DomHandle,
}

impl DomContainer {
    pub fn new() -> Self {
        let document = DomNode::Document(ContainerNode::new(qual_name("")));
        Self::from(document)
    }

    pub fn from(document: DomNode) -> Self {
        Self {
            nodes: vec![document],
            document_handle: DomHandle(0),
        }
    }

    pub fn get_node(&self, handle: &DomHandle) -> &DomNode {
        self.nodes
            .get(handle.0)
            .expect("Invalid handle passed to get_node")
    }

    pub(crate) fn get_mut_node(&mut self, handle: &DomHandle) -> &mut DomNode {
        self.nodes
            .get_mut(handle.0)
            .expect("Invalid handle passed to get_mut_node")
    }

    pub fn get_document(&self) -> &DomNode {
        self.nodes
            .get(self.document_handle.0)
            .expect("document_handle was invalid!")
    }

    pub fn get_mut_document(&mut self) -> &mut DomNode {
        self.nodes
            .get_mut(self.document_handle.0)
            .expect("document_handle was invalid!")
    }

    pub fn document_handle(&self) -> &DomHandle {
        &self.document_handle
    }

    pub fn to_html_string(&self) -> String {
        String::from("")
    }

    pub fn add_node(&mut self, node: DomNode) -> DomHandle {
        let handle = DomHandle(self.nodes.len());
        self.nodes.push(node);
        handle
    }

    pub fn create_element(
        &mut self,
        name: html5ever::QualName,
        _attrs: Vec<html5ever::Attribute>,
        _flags: html5ever::tree_builder::ElementFlags,
    ) -> DomHandle {
        // TODO: attrs and flags
        let node = match name.local.as_ref() {
            "" => DomNode::Text(TextNode::new("")),
            _ => DomNode::Container(ContainerNode::new(name)),
        };

        self.add_node(node)
    }

    /// INVALIDATES ALL HANDLES!
    pub(crate) fn gc(&mut self) {
        //let mut used_indices = HashSet::new();
        let mut deleted_indices = HashSet::from_iter(0..self.nodes.len());

        fn find_used(
            dom_container: &mut DomContainer,
            deleted_indices: &mut HashSet<usize>,
            handle: &DomHandle,
        ) {
            deleted_indices.remove(&handle.0);
            let mut children = Vec::new();
            match dom_container.get_node(handle) {
                DomNode::Container(p) => {
                    children.extend(p.children().iter().cloned());
                }
                DomNode::Document(p) => {
                    children.extend(p.children().iter().cloned());
                }
                DomNode::Text(_) => {}
            }
            for ch in children {
                find_used(dom_container, deleted_indices, &ch)
            }
        }

        let document_handle = self.document_handle().clone();
        find_used(self, &mut deleted_indices, &document_handle);

        // Create a new list of nodes with the deleted ones removed
        let mut new_nodes: Vec<DomNode> = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(i, n)| {
                if deleted_indices.contains(&i) {
                    None
                } else {
                    Some(n)
                }
            })
            .cloned()
            .collect();

        fn remap_handle(
            deleted_indices: &HashSet<usize>,
            handle: &DomHandle,
        ) -> DomHandle {
            // Every deleted node before this one means this one is
            // reduced by one.
            let mut new_index = handle.0;
            for i in deleted_indices {
                if *i < handle.0 {
                    new_index -= 1;
                }
            }
            DomHandle(new_index)
        }

        // Modify the handles in all of those nodes to be correct
        for node in &mut new_nodes {
            match node {
                DomNode::Document(n) => {
                    for c in n.children.iter_mut() {
                        *c = remap_handle(&deleted_indices, c);
                    }
                }
                DomNode::Container(n) => {
                    for c in n.children.iter_mut() {
                        *c = remap_handle(&deleted_indices, c);
                    }
                }
                DomNode::Text(_) => {}
            }
        }

        let new_document_handle =
            remap_handle(&deleted_indices, &self.document_handle);

        self.nodes = new_nodes;
        self.document_handle = new_document_handle;
    }
}

impl Display for DomContainer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("")
    }
}
