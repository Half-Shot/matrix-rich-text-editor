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

use html5ever::tendril::{StrTendril, TendrilSink};
use html5ever::tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink};
use html5ever::{
    parse_fragment, Attribute, ExpandedName, LocalName, Namespace, QualName,
};

use super::node::TextNode;
use super::{qual_name, DomContainer, DomCreationError, DomHandle, DomNode};

pub type DomCreationResult = Result<DomContainer, DomCreationError>;

pub struct DomCreator {
    state: DomCreationError,
}

impl DomCreator {
    pub fn parse(html: &str) -> DomCreationResult {
        parse_fragment(
            DomCreator::default(),
            Default::default(),
            qual_name(""),
            vec![],
        )
        //parse_document(DomCreator::default(), Default::default())
        .from_utf8()
        .one(html.as_bytes())
    }
}

impl Default for DomCreator {
    fn default() -> Self {
        Self {
            state: DomCreationError::new(),
        }
    }
}

impl TreeSink for DomCreator {
    type Handle = DomHandle;
    type Output = DomCreationResult;

    fn finish(mut self) -> Self::Output {
        self.state.dom.gc();
        if self.state.parse_errors.is_empty() {
            Ok(self.state.dom)
        } else {
            Err(self.state)
        }
    }

    fn parse_error(&mut self, msg: std::borrow::Cow<'static, str>) {
        self.state.parse_errors.push(String::from(msg));
    }

    fn get_document(&mut self) -> Self::Handle {
        self.state.dom.document_handle().clone()
    }

    fn elem_name<'a>(&'a self, target: &'a Self::Handle) -> ExpandedName<'a> {
        self.state.dom.get_node(target).name().expanded()
    }

    fn create_element(
        &mut self,
        name: QualName,
        attrs: Vec<Attribute>,
        flags: ElementFlags,
    ) -> Self::Handle {
        dbg!("create_element");
        dbg!(name.local.as_ref());
        self.state.dom.create_element(name, attrs, flags)
    }

    fn create_comment(&mut self, text: StrTendril) -> Self::Handle {
        todo!()
    }

    fn create_pi(
        &mut self,
        target: StrTendril,
        data: StrTendril,
    ) -> Self::Handle {
        todo!()
    }

    fn append(
        &mut self,
        parent: &Self::Handle,
        child: NodeOrText<Self::Handle>,
    ) {
        match child {
            NodeOrText::AppendNode(child) => {
                match self.state.dom.get_mut_node(parent) {
                    DomNode::Container(p) => p.append(child),
                    DomNode::Document(p) => p.append(child),
                    DomNode::Text(_) => {
                        panic!("Appending node to text! {:?}", parent)
                    }
                }
            }
            NodeOrText::AppendText(tendril) => {
                let mut add_node = false;
                match self.state.dom.get_mut_node(parent) {
                    DomNode::Container(_) => add_node = true,
                    DomNode::Document(_) => add_node = true,
                    DomNode::Text(p) => {
                        p.content += tendril.as_ref();
                    }
                }
                if add_node {
                    let new_handle = self.state.dom.add_node(DomNode::Text(
                        TextNode::new(tendril.as_ref()),
                    ));
                    match self.state.dom.get_mut_node(parent) {
                        DomNode::Container(p) => p.append(new_handle),
                        DomNode::Document(p) => p.append(new_handle),
                        DomNode::Text(_) => {
                            panic!("parent changed from container to text!")
                        }
                    }
                }
            }
        };
    }

    fn append_based_on_parent_node(
        &mut self,
        element: &Self::Handle,
        prev_element: &Self::Handle,
        child: NodeOrText<Self::Handle>,
    ) {
        todo!()
    }

    fn append_doctype_to_document(
        &mut self,
        name: StrTendril,
        public_id: StrTendril,
        system_id: StrTendril,
    ) {
        todo!()
    }

    fn get_template_contents(&mut self, target: &Self::Handle) -> Self::Handle {
        todo!()
    }

    fn same_node(&self, x: &Self::Handle, y: &Self::Handle) -> bool {
        dbg!(x);
        dbg!(y);
        dbg!(x == y);
        x == y
    }

    fn set_quirks_mode(&mut self, _mode: QuirksMode) {
        // Nothing to do here for now
    }

    fn append_before_sibling(
        &mut self,
        sibling: &Self::Handle,
        new_node: NodeOrText<Self::Handle>,
    ) {
        todo!()
    }

    fn add_attrs_if_missing(
        &mut self,
        target: &Self::Handle,
        attrs: Vec<Attribute>,
    ) {
        todo!()
    }

    fn remove_from_parent(&mut self, target: &Self::Handle) {
        todo!()
    }

    fn reparent_children(
        &mut self,
        node: &Self::Handle,
        new_parent: &Self::Handle,
    ) {
        todo!()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::dom::node::{ContainerNode, TextNode};
    use crate::dom::{qual_name, DomContainer, DomNode};

    #[derive(Clone, Debug)]
    struct TestNode {
        dom_node: DomNode,
        children: Vec<TestNode>,
    }

    fn doc<'a>(
        children: impl IntoIterator<Item = &'a TestNode>,
    ) -> DomContainer {
        let mut ret = DomContainer::new();

        fn add(
            ret: &mut DomContainer,
            parent: &DomHandle,
            test_node: TestNode,
        ) -> DomHandle {
            let child = ret.add_node(test_node.dom_node);

            let parent = ret.get_mut_node(&parent);
            match parent {
                DomNode::Container(p) => {
                    p.append(child.clone());
                }
                DomNode::Document(p) => {
                    p.append(child.clone());
                }
                DomNode::Text(_) => panic!("Parent can't be a text node"),
            }

            for ch in test_node.children {
                add(ret, &child, ch);
            }
            /*
                        let handle = ret.add_node(test_node.dom_node);

                        let mut added_children = Vec::new();

                        let parent = ret.get_mut_node(&handle);
                        match parent {
                            DomNode::Container(p) => {
                                for h in added_children {
                                    p.append(h)
                                }
                            }
                            DomNode::Document(p) => {
                                for h in added_children {
                                    p.append(h)
                                }
                            }
                            DomNode::Text(_) => panic!("Parent can't be a text node"),
                        }
            */
            child
        }

        let document_handle = ret.document_handle().clone();
        for ch in children.into_iter() {
            add(&mut ret, &document_handle, ch.clone());
        }

        /*
                let mut added_children = Vec::new();
                for ch in children {
                    added_children.push(add(&mut ret, &document_handle, ch.clone()));
                }
                assert_eq!(added_children.len(), 1); // TODO: so don't take a list!

                let parent = ret.get_mut_document();
                match parent {
                    DomNode::Document(p) => {
                        for h in added_children {
                            p.append(h)
                        }
                    }
                    _ => panic!("Document was not a document!"),
                }
        */
        ret
    }

    fn el<'a>(
        name: &str,
        children: impl IntoIterator<Item = &'a TestNode>,
    ) -> TestNode {
        TestNode {
            dom_node: DomNode::Container(ContainerNode::new(qual_name(name))),
            children: children.into_iter().cloned().collect(),
        }
    }

    fn tx(content: &str) -> TestNode {
        TestNode {
            dom_node: DomNode::Text(TextNode::new(content)),
            children: Vec::new(),
        }
    }
    /*
        fn doc<'a>(
            children: impl IntoIterator<Item = &'a DomNode>,
        ) -> DomContainer {
            let mut ret = DomContainer::new();

            for node in children.into_iter() {
                let handle = ret.add_node(node.clone());
                if let DomNode::Document(doc) = ret.get_mut_document() {
                    doc.append(handle);
                } else {
                    panic!("document was not a Document!");
                }
            }

            ret
        }

        fn el<'a>(
            name: &str,
            children: impl IntoIterator<Item = &'a DomNode>,
        ) -> DomNode {
            DomNode::Container(ContainerNode::new(qual_name(name)))
        }

        fn tx(content: &str) -> DomNode {
            DomNode::Text(TextNode::new(content))
        }
    */
    fn d(node: DomContainer) -> String {
        format!("{:?}", node)
    }

    fn parse(input: &str) -> DomContainer {
        DomCreator::parse(input).unwrap()
    }

    #[test]
    fn parsing_an_empty_string_creates_an_empty_dom() {
        assert_eq!(d(parse("")), d(doc(&[el("html", &[])])));
    }

    #[test]
    fn parsing_a_text_snippet_creates_one_node() {
        assert_eq!(d(parse("foo")), d(doc(&[el("html", &[tx("foo")])])));
    }

    #[test]
    fn parsing_two_tags_creates_two_tags() {
        assert_eq!(
            DomCreator::parse("<i></i>").unwrap(),
            DomContainer::from(DomNode::Container(
                ContainerNode::new(qual_name("i")) /*DivNode::from(&[
                                                       DomNode::Div(DivNode::from(&[DomNode::Text(TextNode::new(
                                                           "a"
                                                       ))])),
                                                       DomNode::Div(DivNode::from(&[DomNode::Text(TextNode::new(
                                                           "b"
                                                       ))]))
                                                   ])*/
            ))
        );
    }
}

/// Some experiments with RcDom, to see how html5ever works
#[cfg(test)]
mod rcdom_test {
    use std::{cell::RefCell, rc::Rc};

    use html5ever::{
        parse_fragment,
        tendril::{Tendril, TendrilSink},
        LocalName, Namespace, QualName,
    };
    use markup5ever_rcdom::{Node, NodeData, RcDom};

    use crate::dom::qual_name;

    fn doc<'a>(children: impl IntoIterator<Item = &'a Rc<Node>>) -> Rc<Node> {
        let ret = Node::new(NodeData::Document);
        ret.children.replace_with(|ch| {
            let mut new_ch = ch.clone();
            new_ch.extend(children.into_iter().cloned());
            new_ch
        });
        ret
    }

    fn el<'a>(
        name: &str,
        children: impl IntoIterator<Item = &'a Rc<Node>>,
    ) -> Rc<Node> {
        let ret = Node::new(NodeData::Element {
            name: qual_name(name),
            attrs: RefCell::new(Vec::new()),
            template_contents: None,
            mathml_annotation_xml_integration_point: false,
        });
        ret.children.replace_with(|ch| {
            let mut new_ch = ch.clone();
            new_ch.extend(children.into_iter().cloned());
            new_ch
        });

        ret
    }

    fn tx(contents: &str) -> Rc<Node> {
        Node::new(NodeData::Text {
            contents: RefCell::new(Tendril::from(contents)),
        })
    }

    fn parse(input: &str) -> Rc<Node> {
        parse_fragment(
            RcDom::default(),
            Default::default(),
            QualName::new(
                None,
                Namespace::from("http://www.w3.org/1999/xhtml"),
                LocalName::from(""),
            ),
            vec![],
        )
        .from_utf8()
        .one(input.as_bytes())
        .document
    }

    fn d(node: Rc<Node>) -> String {
        format!("{:?}", node)
    }

    #[test]
    fn rcdom_parsing_an_empty_string_creates_an_empty_dom() {
        assert_eq!(d(parse("")), d(doc(&[el("html", &[])])));
    }

    #[test]
    fn rcdom_parsing_a_text_snippet_creates_one_node() {
        assert_eq!(d(parse("foo")), d(doc(&[el("html", &[tx("foo")])])));
    }
}
