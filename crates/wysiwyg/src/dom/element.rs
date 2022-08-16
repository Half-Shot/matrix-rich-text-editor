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

use crate::dom::nodes::dom_node::DomNode;

pub trait Element<'a, C> {
    fn name(&'a self) -> &'a Vec<C>;
    fn children(&'a self) -> &'a Vec<DomNode<C>>;
    fn children_mut(&'a mut self) -> &'a mut Vec<DomNode<C>>;
}