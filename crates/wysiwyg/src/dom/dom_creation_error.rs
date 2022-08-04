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

use super::DomContainer;

#[derive(Debug, PartialEq)]
pub struct DomCreationError {
    pub dom: DomContainer,
    pub parse_errors: Vec<String>,
}

impl DomCreationError {
    pub fn new() -> Self {
        Self {
            dom: DomContainer::new(),
            parse_errors: Vec::new(),
        }
    }
}
