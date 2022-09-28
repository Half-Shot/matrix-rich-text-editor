import { render } from '@testing-library/react';
import { forwardRef } from 'react';

import { useWysiwyg } from './useWysiwyg';

const Editor = forwardRef<HTMLDivElement>(function Editor(_props, forwardRef) {
    const { ref, isWysiwygReady } = useWysiwyg();
    return <div ref={(node) => {
        if (node) {
            ref.current = node;
            if (typeof forwardRef === 'function') forwardRef(node);
            else if (forwardRef) forwardRef.current = node;
        }
    }}
    contentEditable={isWysiwygReady} />;
});

function toContainHtml(
    // TODO: remove lint ignores? I can't find the right import to be able to use the types.
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    this: any, /* MatcherState */
    editor: HTMLDivElement,
    html: string,
// eslint-disable-next-line @typescript-eslint/no-explicit-any
): any /* ExpectationResult */ {
    const { printReceived, matcherHint } = this.utils;
    const received = editor.innerHTML;
    const expected = html + '<br>';
    const passMessage =
      matcherHint('.not.toContainHtml', 'received', '') +
      '\n\n' +
      `Expected editor inner HTML to be ${printReceived(expected)} but received:\n` +
      `${received}`;

    const failMessage =
      matcherHint('.toContainHtml', 'received', '') +
      '\n\n' +
      `Expected editor inner HTML to be ${printReceived(expected)} but received:\n` +
      `${received}`;

    const pass = received == expected;

    return { pass, message: () => (pass ? passMessage : failMessage) };
}

declare global {
  // TODO: can we avoid disabling this lint?
  // eslint-disable-next-line @typescript-eslint/no-namespace
  namespace jest {
    interface Matchers<R> {
      toContainHtml(
        html: string
      ): R;
    }
  }
}

expect.extend({ toContainHtml });

describe('useWysiwyg', () => {
    describe('Rendering characters', () => {
        let editor: HTMLDivElement;

        function setEditorHtml(html: string) {
            // The editor always needs an extra BR after your HTML
            editor.innerHTML = html + '<br />';
        }

        function deleteRange(start: number, end: number) {
            const textNode = editor.childNodes[0];
            const range = document.createRange();
            range.setStart(textNode, start);
            range.setEnd(textNode, end);
            const sel = document.getSelection();
            if (sel) {
                sel.removeAllRanges();
                sel.addRange(range);
                sel.deleteFromDocument();
            }
        }

        beforeAll(() => {
            render(<Editor ref={(node) => {
                if (node) {
                    editor = node;
                }
            }} />);
        });

        it('Should render ASCII characters with width 1', () => {
            // When
            setEditorHtml('abcd');
            deleteRange(0, 1);

            // Then
            expect(editor).toContainHtml('bcd');

            //When
            setEditorHtml('abcd');
            deleteRange(0, 2);

            //Then
            expect(editor).toContainHtml('cd');
        });

        it.skip('Should render UCS-2 characters with width 1', () => {
            // When
            setEditorHtml('\u{03A9}bcd');
            deleteRange(0, 1);

            // Then
            expect(editor).toContainHtml('bcd');

            // When
            setEditorHtml('\u{03A9}bcd');
            deleteRange(0, 2);

            // Then
            expect(editor).toContainHtml('cd');
        });

        it.skip('Should render Multi-code unit UTF-16 characters with width 2', () => {
            // When
            setEditorHtml('\u{1F4A9}bcd');
            deleteRange(0, 2);

            // Then
            expect(editor).toContainHtml('bcd');

            //When
            setEditorHtml('\u{1F4A9}bcd');
            deleteRange(0, 3);

            //Then
            expect(editor).toContainHtml('cd');
        });

        it.skip('Should render complex characters with width = num UTF-16 code units', () => {
            // When
            setEditorHtml('\u{1F469}\u{1F3FF}\u{200D}\u{1F680}bcd');
            deleteRange(0, 7);

            // Then
            expect(editor).toContainHtml('bcd');

            //When
            setEditorHtml('\u{1F469}\u{1F3FF}\u{200D}\u{1F680}bcd');
            deleteRange(0, 8);

            //Then
            expect(editor).toContainHtml('cd');
        });
    });
});
