import init from "./generated/wysiwyg.js";
import {
    codeunit_count,
    generate_testcase,
    get_selection_in_editor,
    node_and_offset,
    selection_according_to_actions
} from "./wysiwyg.js";

import {computeSelectionOffset} from './dom.js'

const editor = document.getElementById('editor');

run_tests([
    { name: "ASCII characters have width 1", test: () => {
        set_editor_html("abcd");
        deleteRange(0, 1);
        assert_editor_contains("bcd");

        set_editor_html("abcd");
        deleteRange(0, 2);
        assert_editor_contains("cd");
    }},

    { name: "UCS-2 characters have width 1", test: () => {
        set_editor_html("\u{03A9}bcd");
        deleteRange(0, 1);
        assert_editor_contains("bcd");

        set_editor_html("\u{03A9}bcd");
        deleteRange(0, 2);
        assert_editor_contains("cd");
    }},

    { name: "Multi-code unit UTF-16 characters have width 2", test: () => {
        set_editor_html("\u{1F4A9}bcd");
        deleteRange(0, 2);
        assert_editor_contains("bcd");

        set_editor_html("\u{1F4A9}bcd");
        deleteRange(0, 3);
        assert_editor_contains("cd");
    }},

    { name: "Complex characters width = num UTF-16 code units", test: () => {
        set_editor_html("\u{1F469}\u{1F3FF}\u{200D}\u{1F680}bcd");
        deleteRange(0, 7);
        assert_editor_contains("bcd");

        set_editor_html("\u{1F469}\u{1F3FF}\u{200D}\u{1F680}bcd");
        deleteRange(0, 8);
        assert_editor_contains("cd");
    }},

    { name: "node_and_offset finds at the start of simple text", test: () => {
        set_editor_html("abcdefgh");
        let { node, offset } = node_and_offset(editor, 0);
        assert_same(node, editor.childNodes[0]);
        assert_eq(offset, 0);
    }},

    { name: "node_and_offset finds in the middle of simple text", test: () => {
        set_editor_html("abcdefgh");
        let { node, offset } = node_and_offset(editor, 4);
        assert_same(node, editor.childNodes[0]);
        assert_eq(offset, 4);
    }},

    { name: "node_and_offset finds at the end of simple text", test: () => {
        set_editor_html("abcdefgh");
        let { node, offset } = node_and_offset(editor, 8);
        assert_same(node, editor.childNodes[0]);
        assert_eq(offset, 8);
    }},

    { name: "node_and_offset returns null if off the end", test: () => {
        set_editor_html("abcdefgh");
        // 8 characters, plus the br we always append = 9, so 10 is off end
        let { node, offset } = node_and_offset(editor, 10);
        assert_same(node, null);
        assert_eq(offset, 1);
    }},

    { name: "node_and_offset finds before subnode", test: () => {
        set_editor_html("abc<b>def</b>gh");
        let { node, offset } = node_and_offset(editor, 2);
        assert_same(node, editor.childNodes[0]);
        assert_eq(offset, 2);
    }},

    { name: "node_and_offset finds inside subnode", test: () => {
        set_editor_html("abc<b>def</b>gh");
        let { node, offset } = node_and_offset(editor, 4);
        assert_same(node, editor.childNodes[1].childNodes[0]);
        assert_eq(offset, 1);
    }},

    { name: "node_and_offset finds after subnode", test: () => {
        set_editor_html("abc<b>def</b>gh");
        let { node, offset } = node_and_offset(editor, 7);
        assert_same(node, editor.childNodes[2]);
        assert_eq(offset, 1);
    }},

    { name: "node_and_offset finds before br", test: () => {
        set_editor_html("a<br />b");
        let { node, offset } = node_and_offset(editor, 0);
        assert_same(node, editor.childNodes[0]);
        assert_eq(offset, 0);
    }},

    { name: "node_and_offset finds br start", test: () => {
        set_editor_html("a<br />b");
        let { node, offset } = node_and_offset(editor, 1);
        assert_same(node, editor.childNodes[0]);
        assert_eq(offset, 1);
    }},

    { name: "node_and_offset finds br end", test: () => {
        // We never actually return the br as the node that
        // was selected, unless there are two in a row.

        set_editor_html("a<br />b");
        let { node, offset } = node_and_offset(editor, 2);
        assert_same(node, editor.childNodes[2]);
        assert_eq(offset, 0);
    }},

    { name: "node_and_offset finds between brs", test: () => {
        // Selection falls between the two brs
        set_editor_html("a<br /><br />b");
        let { node, offset } = node_and_offset(editor, 2);
        assert_same(node, editor.childNodes[2]);
        assert_eq(offset, 0);
    }},

    { name: "node_and_offset finds after br", test: () => {
        set_editor_html("a<br />b");
        let { node, offset } = node_and_offset(editor, 3);
        assert_same(node, editor.childNodes[2]);
        assert_eq(offset, 1);
    }},

    { name: "node_and_offset finds inside an empty list", test: () => {
        set_editor_html("<ul><li><li></ul>");
        let { node, offset } = node_and_offset(editor, 0);
        assert_same(node, editor.childNodes[0].childNodes[0]);
        assert_eq(offset, 0);
    }},

    { name: "node_and_offset finds inside two  empty list", test: () => {
        set_editor_html("<ul><li><li></ul><li><li></ul>");
        let { node, offset } = node_and_offset(editor, 0);
        assert_same(node, editor.childNodes[0].childNodes[0]);
        assert_eq(offset, 0);
    }},

    { name: "node_and_offset finds inside a list", test: () => {
        set_editor_html("<ul><li>foo<li></ul>");
        let { node, offset } = node_and_offset(editor, 1);
        assert_same(node, editor.childNodes[0].childNodes[0].childNodes[0]);
        assert_eq(offset, 1);
    }},

    { name: "codeunit_count ASCII", test: () => {
        set_editor_html("abcdefgh");
        let textNode = editor.childNodes[0];
        assert_eq(codeunit_count(editor, textNode, 0), 0);
        assert_eq(codeunit_count(editor, textNode, 3), 3);
        assert_eq(codeunit_count(editor, textNode, 7), 7);
        // Just past the end is allowed
        assert_eq(codeunit_count(editor, textNode, 8), 8);
        // But not past that
        assert_eq(codeunit_count(editor, textNode, 9), -1);
    }},

    { name: "codeunit_count UCS-2", test: () => {
        set_editor_html("a\u{03A9}b\u{03A9}c");
        let textNode = editor.childNodes[0];
        assert_eq(codeunit_count(editor, textNode, 0), 0);
        assert_eq(codeunit_count(editor, textNode, 1), 1);
        assert_eq(codeunit_count(editor, textNode, 4), 4);
        assert_eq(codeunit_count(editor, textNode, 5), 5);
        assert_eq(codeunit_count(editor, textNode, 6), -1);
    }},

    { name: "codeunit_count complex", test: () => {
        set_editor_html("a\u{1F469}\u{1F3FF}\u{200D}\u{1F680}b");
        let textNode = editor.childNodes[0];
        assert_eq(codeunit_count(editor, textNode, 0), 0);
        assert_eq(codeunit_count(editor, textNode, 7), 7);
        assert_eq(codeunit_count(editor, textNode, 8), 8);
        assert_eq(codeunit_count(editor, textNode, 9), 9);
        assert_eq(codeunit_count(editor, textNode, 10), -1);
    }},

    { name: "codeunit_count nested", test: () => {
        set_editor_html("a<b>b</b>c");
        let firstTextNode = editor.childNodes[0];
        let boldTextNode = editor.childNodes[1].childNodes[0];
        let thirdTextNode = editor.childNodes[2];
        assert_eq(codeunit_count(editor, firstTextNode, 0), 0);
        assert_eq(codeunit_count(editor, boldTextNode, 0), 1);
        assert_eq(codeunit_count(editor, thirdTextNode, 0), 2);
    }},

    { name: "codeunit_count treats br as a character", test: () => {
        set_editor_html("a<br />b");
        let firstTextNode = editor.childNodes[0];
        let brNode = editor.childNodes[1];
        let secondTextNode = editor.childNodes[2];
        assert_eq(codeunit_count(editor, firstTextNode, 0), 0);
        assert_eq(codeunit_count(editor, brNode, 0), 2);
        assert_eq(codeunit_count(editor, secondTextNode, 1), 3);
    }},

    { name: "codeunit_count deeply nested", test: () => {
        set_editor_html("aaa<b><i>bbb</i>ccc</b>ddd");
        let firstTextNode = editor.childNodes[0];
        let boldItalicTextNode = editor.childNodes[1].childNodes[0].childNodes[0];
        let boldOnlyTextNode = editor.childNodes[1].childNodes[1];
        let thirdTextNode = editor.childNodes[2];
        assert_eq(codeunit_count(editor, firstTextNode, 1), 1);
        assert_eq(codeunit_count(editor, firstTextNode, 2), 2);
        assert_eq(codeunit_count(editor, firstTextNode, 3), 3);
        // We can supply the text node or its parent
        assert_eq(codeunit_count(editor, boldItalicTextNode, 0), 3);
        assert_eq(codeunit_count(editor, boldItalicTextNode, 1), 4);
        assert_eq(codeunit_count(editor, boldItalicTextNode, 2), 5);
        assert_eq(codeunit_count(editor, boldOnlyTextNode, 0), 6);
        assert_eq(codeunit_count(editor, boldOnlyTextNode, 1), 7);
        assert_eq(codeunit_count(editor, boldOnlyTextNode, 2), 8);
        assert_eq(codeunit_count(editor, thirdTextNode, 0), 9);
        assert_eq(codeunit_count(editor, thirdTextNode, 1), 10);
        assert_eq(codeunit_count(editor, thirdTextNode, 2), 11);
    }},

    { name: "The offset should contain all the characters when the editor node is selected", test: () => {
        // When
        set_editor_html("abc<b>def</b>gh");
        // Use the editor node and a offset as 1 to simulate the FF behavior
        let offset = computeSelectionOffset(editor, 1);

        // Then
        assert_eq(offset, 8);

         // When
        set_editor_html("abc<b>def</b>gh<ul><li>alice</li><li>bob</li>");
        offset = computeSelectionOffset(editor, 1);

         // Then
         assert_eq(offset, 16);
    }},

    { name: "The offset should contain the selected characters", test: () => {
        // When
        set_editor_html("abc<b>def</b>gh<ul><li>alice</li><li>bob</li>");
        let offset = computeSelectionOffset(editor.childNodes[0], 1);

        // Then
        assert_eq(offset, 1);

        // When
        offset = computeSelectionOffset(editor.childNodes[0], 20);

        // Then
        assert_eq(offset, 20);
    }},

    { name: "Selecting back to the beginning of the line works", test: () => {
        // When
        set_editor_html("abc");
        let offset = computeSelectionOffset(editor.childNodes[0], 0);

        // Then
        assert_eq(offset, 0);
    }},

    { name: "Selection according to no actions is -1, 1", test: () => {
        const actions = [];
        assert_eq([-1, -1], selection_according_to_actions(actions));
    }},

    { name: "Selection is found from the last action", test: () => {
        const actions = [
            ["foo", "bar", "baz"],
            ["select", 10, 10],
            ["foo", "bar", "baz"],
            ["select", 12, 13],
            ["foo", "bar", "baz"],
        ];
        assert_eq([12, 13], selection_according_to_actions(actions));
    }},

    { name: "Getting selection cursor after BR", test: () => {
        // Place the cursor on the empty line
        set_editor_html("para 1<br /><br />para 2");
        const secondbr = document.querySelector("#editor br").nextSibling;
        cursor_to_node(secondbr, 0);
        const sel = document.getSelection();

        // The focusNode and anchorNode are the editor object, not one of the
        // text nodes inside it, and the offset tells you which node inside
        // editor is immediately after the cursor.
        assert_same(sel.anchorNode, editor);
        assert_eq(sel.anchorOffset, 2);
        assert_same(sel.focusNode, editor);
        assert_eq(sel.focusOffset, 2);

        // We should see ourselves as on code unit 7, because the BR
        // counts as 1.
        assert_eq(
            [7, 7],
            get_selection_in_editor(editor)
        );
    }},

    { name: "Getting backward selection ending after BR", test: () => {
        set_editor_html("para 1<br /><br />para 2");
        const secondbr = document.querySelector("#editor br").nextSibling;
        select_end_to(secondbr, 0);
        const sel = document.getSelection();

        // The focusNode and anchorNode are the editor object, not one of the
        // text nodes inside it, and the offset tells you which node inside
        // editor is immediately after the cursor.
        assert_same(sel.anchorNode, editor.childNodes[3]);
        assert_eq(sel.anchorOffset, 6);
        assert_same(sel.focusNode, editor);
        assert_eq(sel.focusOffset, 2);

        // Selection backwards from the end to the blank line
        assert_eq(
            [14, 7],
            get_selection_in_editor(editor)
        );
    }},

    { name: "Select all with ctrl-a", test: () => {
        set_editor_html("para 1<br /><br />para 2");
        select_all();
        assert_eq(
            [0, 15], // Not 14 here because the last BR gets counted?
            get_selection_in_editor(editor)
        );
    }},

    { name: "Select all by dragging", test: () => {
        set_editor_html("para 1<br /><br />para 2");
        select_start_to_end();
        assert_eq(
            [0, 14],
            get_selection_in_editor(editor)
        );
    }},

    { name: "Select all by dragging backwards", test: () => {
        set_editor_html("para 1<br /><br />para 2");
        select_end_to_start();
        assert_eq(
            [14, 0],
            get_selection_in_editor(editor)
        );
    }},

    { name: "Select across multiple newlines", test: () => {
        set_editor_html("para 1<br /><br />para 2");
        const p1 = editor.childNodes[0];
        const p2 = editor.childNodes[3];
        select(p1, 2, p2, 3);
        assert_eq(
            [2, 11],
            get_selection_in_editor(editor)
        );
    }},

    { name: "Selection after pressing down at end", test: () => {
        set_editor_html("para 1<br /><br />para 2");
        // Simulate going to end of doc and pressing down arrow
        cursor_to_after_end();
        assert_eq(
            [14, 14],
            get_selection_in_editor(editor)
        );
    }},

    { name: "Selection at beginning", test: () => {
        set_editor_html("para 1<br /><br />para 2");
        cursor_to_beginning();
        assert_eq(
            [0, 0],
            get_selection_in_editor(editor)
        );
    }},

    { name: "Generate testcase from 1 character and selection", test: () => {
        const actions = [
            ["replace_text", "a", undefined],
            ["select", 1, 1]
        ];

        const expected = (
            'let mut model = cm("a|");\n'
            + 'assert_eq!(tx(&model), "a|");\n'
        );

        assert_eq(expected, generate_testcase(actions, "a|"));
    }},

    { name: "Generate testcase with cursor at the beginning", test: () => {
        const actions = [
            ["replace_text", "a", undefined],
            ["select", 0, 0]
        ];

        const expected = (
            'let mut model = cm("|a");\n'
            + 'assert_eq!(tx(&model), "|a");\n'
        );

        assert_eq(expected, generate_testcase(actions, "|a"));
    }},

    { name: "Generate testcase from multiple typed characters", test: () => {
        const actions = [
            ["replace_text", "a", undefined],
            ["replace_text", "b", undefined],
            ["replace_text", "c", undefined],
            ["replace_text", "d", undefined],
            ["select", 4, 4]
        ];

        const expected = (
            'let mut model = cm("abcd|");\n'
            + 'assert_eq!(tx(&model), "abcd|");\n'
        );

        assert_eq(expected, generate_testcase(actions, "abcd|"));
    }},

    { name: "Generate testcase collecting initial selections", test: () => {
        const actions = [
            ["replace_text", "a", undefined],
            ["select", 1, 1],
            ["replace_text", "b", undefined],
            ["select", 2, 2],
            ["replace_text", "c", undefined],
            ["select", 3, 3],
            ["replace_text", "d", undefined],
            ["select", 4, 4]
        ];

        const expected = (
            'let mut model = cm("abcd|");\n'
            + 'assert_eq!(tx(&model), "abcd|");\n'
        );

        assert_eq(expected, generate_testcase(actions, "abcd|"));
    }},

    { name: "Generate testcase with pasted start", test: () => {
        const actions = [
            ["replace_text", "abcd", undefined],
            ["select", 4, 4]
        ];

        const expected = (
            'let mut model = cm("abcd|");\n'
            + 'assert_eq!(tx(&model), "abcd|");\n'
        );

        assert_eq(expected, generate_testcase(actions, "abcd|"));
    }},

    { name: "Generate testcase by typing and bolding", test: () => {
        const actions = [
            ["replace_text", "a", undefined],
            ["replace_text", "b", undefined],
            ["replace_text", "c", undefined],
            ["replace_text", "d", undefined],
            ["select", 1, 3],
            ["bold"]
        ];

        const expected = (
            'let mut model = cm("a{bc}|d");\n'
            + 'model.bold();\n'
            + 'assert_eq!(tx(&model), "a<strong>{bc}|</strong>d");\n'
        );

        assert_eq(
            expected,
            generate_testcase(actions, "a<strong>{bc}|</strong>d")
        );
    }},

    { name: "Generate testcase with backward selection", test: () => {
        const actions = [
            ["replace_text", "a", undefined],
            ["replace_text", "b", undefined],
            ["replace_text", "c", undefined],
            ["replace_text", "d", undefined],
            ["select", 3, 1],
            ["bold"]
        ];

        const expected = (
            'let mut model = cm("a|{bc}d");\n'
            + 'model.bold();\n'
            + 'assert_eq!(tx(&model), "a<strong>|{bc}</strong>d");\n'
        );

        assert_eq(
            expected,
            generate_testcase(actions, "a<strong>|{bc}</strong>d")
        );
    }},

    { name: "Generate testcase with backward to beginning", test: () => {
        const actions = [
            ["replace_text", "a", undefined],
            ["replace_text", "b", undefined],
            ["replace_text", "c", undefined],
            ["replace_text", "d", undefined],
            ["select", 3, 0],
            ["bold"]
        ];

        const expected = (
            'let mut model = cm("|{abc}d");\n'
            + 'model.bold();\n'
            + 'assert_eq!(tx(&model), "<strong>|{abc}</strong>d");\n'
        );

        assert_eq(
            expected,
            generate_testcase(actions, "<strong>|{abc}</strong>d")
        );
    }},

    { name: "Generate testcase with backward from end", test: () => {
        const actions = [
            ["replace_text", "abc", undefined],
            ["select", 3, 2]
        ];

        const expected = (
            'let mut model = cm("ab|{c}");\n'
            + 'assert_eq!(tx(&model), "<strong>ab|{c}</strong>");\n'
        );

        assert_eq(
            expected,
            generate_testcase(actions, "<strong>ab|{c}</strong>")
        );
    }},

    { name: "Generate testcase with tags on selection boundary", test: () => {
        const actions = [
            ["replace_text", "aa<strong>bbbb</strong>cc", undefined],
            ["select", 2, 6]
        ];

        const expected = (
            'let mut model = cm("aa<strong>{bbbb}|</strong>cc");\n'
            + 'assert_eq!(tx(&model), "aa<strong>{bbbb}|</strong>cc");\n'
        );

        assert_eq(
            expected,
            generate_testcase(actions, "aa<strong>{bbbb}|</strong>cc")
        );
    }},

    { name: "Generate testcase with multiple later selections", test: () => {
        const actions = [
            ["replace_text", "aa<strong>bbbb</strong>cc", undefined],
            ["select", 2, 6],
            ["bold"],
            ["select", 3, 3],
            ["select", 3, 5],
            ["select", 4, 4],
            ["select", 3, 6]
        ];

        const expected = (
            'let mut model = cm("aa<strong>{bbbb}|</strong>cc");\n'
            + 'model.bold();\n'
            + 'model.select(Location::from(3), Location::from(6));\n'
            + 'assert_eq!(tx(&model), "aa<strong>{bbbb}|</strong>cc");\n'
        );

        assert_eq(
            expected,
            generate_testcase(actions, "aa<strong>{bbbb}|</strong>cc")
        );
    }},

    { name: "Generate testcase later selections to beginning", test: () => {
        const actions = [
            ["replace_text", "aa<strong>bbbb</strong>cc", undefined],
            ["select", 2, 6],
            ["bold"],
            ["select", 3, 0]
        ];

        const expected = (
            'let mut model = cm("aa<strong>{bbbb}|</strong>cc");\n'
            + 'model.bold();\n'
            + 'model.select(Location::from(3), Location::from(0));\n'
            + 'assert_eq!(tx(&model), "|{aa<strong>b}bbb</strong>cc");\n'
        );

        assert_eq(
            expected,
            generate_testcase(actions, "|{aa<strong>b}bbb</strong>cc")
        );
    }},

    { name: "Generate testcase multi-backspaces not suppressed", test: () => {
        const actions = [
            ["replace_text", "aa<strong>bbbb</strong>cc", undefined],
            ["select", 8, 8],
            ["backspace"],
            ["backspace"]
        ];

        const expected = (
            'let mut model = cm("aa<strong>bbbb</strong>cc|");\n'
            + 'model.backspace();\n'
            + 'model.backspace();\n'
            + 'assert_eq!(tx(&model), "aa<strong>bbbb|</strong>");\n'
        );

        assert_eq(
            expected,
            generate_testcase(actions, "aa<strong>bbbb|</strong>")
        );
    }}
]);

function deleteRange(start, end) {
    let textNode = editor.childNodes[0];
    const range = document.createRange();
    range.setStart(textNode, start);
    range.setEnd(textNode, end);
    var sel = document.getSelection();
    sel.removeAllRanges();
    sel.addRange(range);
    sel.deleteFromDocument();
}

async function run_tests(tests) {
    await init();

    log("Running tests:");
    for (const test of tests) {
        try {
            test.test();
            log(` - ok - ${test.name}`);
        } catch (e) {
            error(` - Failed - ${test.name}`);
            throw e;
        }
    }
}

function assert_eq(left, right) {
    const le = JSON.stringify(left);
    const ri = JSON.stringify(right);
    if (le !== ri) {
        throw_error(`Unequal:
${le}
${ri}`);
    }
}

function assert_same(left, right) {
    if (left !== right) {
        throw_error(`Assertion failed: ${left} is not ${right}`);
    }
}

/*function assert(condition, explanation) {
    if (!condition) {
        throw_error(`Assertion failed: ${explanation}`);
    }
}*/

function log(msg) {
    let div = document.createElement("div");
    div.innerText = msg;
    document.body.appendChild(div);

    console.log(msg);
}

function error(msg) {
    let div = document.createElement("div");
    div.innerText = msg;
    div.style.color = "red";
    document.body.appendChild(div);

    console.error(msg);
}

function throw_error(msg) {
    let div = document.createElement("div");
    div.innerText = msg;
    div.style.color = "red";
    document.body.appendChild(div);

    throw new Error(msg);
}

function set_editor_html(html) {
    // The editor always needs an extra BR after your HTML
    editor.innerHTML = html + "<br />";
}

function assert_editor_contains(html) {
    assert_eq(editor.innerHTML, html + "<br>");
}

function last_text_node() {
    for (let i = editor.childNodes.length - 1; i >= 0; i--) {
        const n = editor.childNodes[i];
        if (n.nodeType === Node.TEXT_NODE && n.textContent !== "\n") {
            return n;
        }
    }
    return null;
}

function index_of(child, parent) {
    let i = 0;
    for (let ch of parent.childNodes) {
        if (ch.isSameNode(child)) {
            return i;
        }
        i++;
    }
    return -1;
}

/** Like clicking at the beginning */
function cursor_to_beginning() {
    const sel = document.getSelection();
    sel.selectAllChildren(editor.firstChild);
    sel.collapseToStart();
}

/** Click at the end then press down arrow */
function cursor_to_after_end() {
    const offset = editor.childNodes.length - 1;
    const sel = document.getSelection();
    sel.setBaseAndExtent(editor, offset, editor, offset);
}

/** Click at the end */
function cursor_to_end() {
    const lastTextNode = last_text_node();
    const len = lastTextNode.textContent.length;
    const sel = document.getSelection();
    sel.setBaseAndExtent(lastTextNode, len, lastTextNode, len);
}

/** Moves to the supplied node at the supplied offset. Ignores the offset
 * if you supply a non-text node, and places you immediately BEFORE the
 * supplied node. */
function cursor_to_node(node, offset) {
    if (node.nodeType === Node.TEXT_NODE) {
        // Text node - refer to it, with index at end
        const sel = document.getSelection();
        sel.setBaseAndExtent(node, offset, node, offset);
    } else {
        // Find parent and point to this node within the parent
        const parent = node.parentNode;
        const idx = index_of(node, parent);
        const sel = document.getSelection();
        sel.setBaseAndExtent(parent, idx, parent, idx);
    }
}

function select_all() {
    const sel = document.getSelection();
    sel.selectAllChildren(editor);
}

function select_start_to_end() {
    const lastTextNode = last_text_node();

    const sel = document.getSelection();
    sel.setBaseAndExtent(
        editor.firstChild,
        0,
        lastTextNode,
        lastTextNode.textContent.length
    );
}

function select_end_to_start() {
    cursor_to_end();
    document.getSelection().extend(editor.firstChild);
}

function select(node1, offset1, node2, offset2) {
    cursor_to_node(node1, offset1);

    let n2;
    let o2;
    if (node2.nodeType === Node.TEXT_NODE) {
        o2 = offset2;
        n2 = node2;
    } else {
        o2 = index_of(node2, node2.parentNode);
        n2 = node2.parentNode;
    }

    document.getSelection().extend(n2, o2);
}

/** Select from the end to the supplied node. If node is not a text node,
 * offset is ignored, and the selection starts BEFORE node. */
function select_end_to(node, offset) {
    cursor_to_end();

    let n;
    let o;
    if (node.nodeType === Node.TEXT_NODE) {
        o = offset;
        n = node;
    } else {
        o = index_of(node, node.parentNode);
        n = node.parentNode;
    }

    document.getSelection().extend(n, o);
}
