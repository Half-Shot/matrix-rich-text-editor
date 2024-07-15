/*
Copyright 2022 The Matrix.org Foundation C.I.C.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

import { ComposerUpdate } from '../generated/wysiwyg';
import { ACTION_TYPES, SUGGESTIONS } from './constants';
import { AllowedMentionAttributes, LinkEvent } from './useListeners/types';

export type BlockType = InputEvent['inputType'] | 'formatInlineCode' | 'clear';

export type WysiwygInputEvent =
    | ClipboardEvent
    | LinkEvent
    | (InputEvent & {
          inputType: BlockType;
          data?: string | null;
      });

export type WysiwygEvent = WysiwygInputEvent | KeyboardEvent;

export type ActionTypes = (typeof ACTION_TYPES)[number];

export type ActionState = 'enabled' | 'reversed' | 'disabled';

export type AllActionStates = Record<ActionTypes, ActionState>;

export type FormattingFunctions = Record<
    Exclude<ActionTypes, 'link'>,
    () => void
> & {
    insertText: (text: string) => void;
    link: (url: string, text?: string) => void;
    mention: (
        url: string,
        text: string,
        attributes: AllowedMentionAttributes,
    ) => void;
    mentionAtRoom: (attributes: AllowedMentionAttributes) => void;
    command: (text: string) => void;
    removeLinks: () => void;
    getLink: () => string;
};

export type Wysiwyg = {
    actions: FormattingFunctions;
    content: () => string;
    messageContent: () => string;
};

export type InputEventProcessor = (
    event: WysiwygEvent,
    wysiwyg: Wysiwyg,
    editor: HTMLElement,
) => WysiwygEvent | null;

export type SuggestionChar = (typeof SUGGESTIONS)[number] | '';
export type SuggestionType = 'mention' | 'command' | 'unknown';
export type MappedSuggestion = {
    keyChar: SuggestionChar;
    text: string;
    type: SuggestionType;
};
export type TraceAction = (
    update: ComposerUpdate | null,
    name: string,
    value1?: string | number,
    value2?: string | number,
) => ComposerUpdate | null;
