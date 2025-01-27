//
// Copyright 2022 The Matrix.org Foundation C.I.C
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

extension ComposerModel {
    /// Apply given action to the composer model.
    ///
    /// - Parameters:
    ///   - action: Action to apply.
    func apply(_ action: WysiwygAction) -> ComposerUpdateProtocol {
        let update: ComposerUpdateProtocol
        switch action {
        case .bold:
            update = bold()
        case .italic:
            update = italic()
        case .strikeThrough:
            update = strikeThrough()
        case .underline:
            update = underline()
        case .inlineCode:
            update = inlineCode()
        case let .link(url: url):
            update = setLink(newText: url)
        case .undo:
            update = undo()
        case .redo:
            update = redo()
        case .orderedList:
            update = orderedList()
        case .unorderedList:
            update = unorderedList()
        }

        return update
    }
}
