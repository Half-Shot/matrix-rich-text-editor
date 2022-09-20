import { RefObject, useCallback, useMemo, useRef, useState } from "react";

// rust generated bindings
// eslint-disable-next-line camelcase
import { ComposerModel, ComposerUpdate, new_composer_model_from_html } from "../../generated/wysiwyg";
import { getCurrentSelection } from "./dom";

type Actions = Array<[string, any, any?]>;

function traceAction(testNode: HTMLElement | null, actions: Actions, composerModel: ComposerModel | null) {
    return (update: ComposerUpdate | null, name: string, value1?: any, value2?: any) => {
        if (!testNode || !composerModel) {
            return update;
        }

        if (value2 !== undefined) {
            console.debug(`composer_model.${name}(${value1}, ${value2})`);
        } else if (value1 !== undefined) {
            console.debug(`composer_model.${name}(${value1})`);
        } else {
            console.debug(`composer_model.${name}()`);
        }

        actions.push([name, value1, value2]);

        updateTestCase(testNode, composerModel, update, actions);

        return update;
    };
}

function getSelectionAccordingToActions(actions: Actions) {
    return () => {
        for (let i = actions.length - 1; i >= 0; i--) {
            const action = actions[i];
            if (action[0] === "select") {
                return [action[1], action[2]];
            }
        }
        return [-1, -1];
    };
}

function updateTestCase(
    testNode: HTMLElement,
    composerModel: ComposerModel,
    update: ComposerUpdate | null,
    actions: Actions,
) {
    // let html = editor.innerHTML;
    if (update) {
        // TODO: if (replacement_html !== html) SHOW AN ERROR?
        // TODO: handle other types of update (not just replace_all)
        update.text_update();
        //    html = update.text_update().replace_all?.replacement_html;
    }

    testNode.innerText = generateTestCase(
        actions, composerModel.to_example_format(),
    );

    testNode.scrollTo(0, testNode.scrollHeight - testNode.clientHeight);
}

function generateTestCase(actions: Actions, html: string) {
    let ret = "";

    function add(name: string, value1: any, value2: any) {
        if (name === "select") {
            ret += (
                "model.select("
                + `Location::from(${value1}), `
                + `Location::from(${value2}));\n`
            );
        } else if (value2 !== undefined) {
            ret += `model.${name}(${value1 ?? ""}, ${value2});\n`;
        } else if (name === "replace_text") {
            ret += `model.${name}("${value1 ?? ""}");\n`;
        } else {
            ret += `model.${name}(${value1 ?? ""});\n`;
        }
    }

    function start() {
        const text = addSelection(collected, selection[0], selection[1]);
        ret += `let mut model = cm("${text}");\n`;
    }

    let lastName: string | null = null;
    let isCollectingMode = true;
    let collected = "";
    let selection = [0, 0];
    for (const [name, value1, value2] of actions) {
        if (isCollectingMode) {
            if (name === "replace_text") {
                collected += value1;
            } else if (name === "select") {
                selection = [value1, value2];
            } else {
                isCollectingMode = false;
                start();
                add(name, value1, value2);
            }
        } else if (lastName === "select" && name === "select") {
            const nl = ret.lastIndexOf("\n", ret.length - 2);
            if (nl > -1) {
                ret = ret.substring(0, nl) + "\n";
                add(name, value1, value2);
            }
        } else {
            add(name, value1, value2);
        }
        lastName = name;
    }

    if (isCollectingMode) {
        start();
    }

    ret += `assert_eq!(tx(&model), "${html}");\n`;

    return ret;
}

function addSelection(text: string, start: number, end: number) {
    // In the original wysiwyg js, the function is called with one parameter but the TS definition requires 3 params
    // new_composer_model_from_html(text)
    const model = new_composer_model_from_html(text, -1, -1);
    model.select(start, end);
    return model.to_example_format();
}

function resetTestCase(
    editor: HTMLElement,
    testNode: HTMLElement,
    composerModel: ComposerModel,
    actions: Actions,
    html: string,
) {
    const [start, end] = getCurrentSelection(editor);
    actions = [
        ["replace_text", html],
        ["select", start, end],
    ];
    updateTestCase(testNode, composerModel, null, actions);
}

export function useTestCases(editorRef: RefObject<HTMLElement | null>, composerModel: ComposerModel | null) {
    const testRef = useRef<HTMLDivElement>(null);
    const actions = useRef<Array<[string, any, any]>>([]).current;

    const [editorHtml, setEditorHtml] = useState<string>('');

    const memorizedTraceAction = useMemo(
        () => traceAction(testRef.current, actions, composerModel), [testRef, actions, composerModel],
    );

    const memorizedGetSelection = useMemo(() => getSelectionAccordingToActions(actions), [actions]);

    const onResetTestCase = useCallback(() => editorRef.current && testRef.current && composerModel &&
        resetTestCase(
            editorRef.current,
            testRef.current,
            composerModel,
            actions,
            editorHtml,
        ),
    [editorRef, testRef, composerModel, actions, editorHtml],
    );

    return {
        testRef,
        utilities: {
            traceAction: memorizedTraceAction,
            getSelectionAccordingToActions: memorizedGetSelection,
            onResetTestCase,
            setEditorHtml,
        },
    };
}

export type TestUtilities = ReturnType<typeof useTestCases>['utilities'];