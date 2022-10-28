package io.element.android.wysiwyg.test.utils

import android.text.Editable
import android.view.View
import androidx.core.widget.addTextChangedListener
import androidx.test.espresso.UiController
import androidx.test.espresso.ViewAction
import androidx.test.espresso.matcher.ViewMatchers.isDisplayed
import io.element.android.wysiwyg.EditorEditText
import io.element.android.wysiwyg.inputhandlers.models.InlineFormat
import org.hamcrest.Matcher

object Editor {

    class SetText(
        private val text: String,
    ) : ViewAction {
        override fun getConstraints(): Matcher<View> = isDisplayed()

        override fun getDescription(): String = "Set text to $text"

        override fun perform(uiController: UiController?, view: View?) {
            val editor = view as? EditorEditText ?: return
            editor.setText(text)
        }
    }

    class SetHtml(
        private val html: String,
    ) : ViewAction {
        override fun getConstraints(): Matcher<View> = isDisplayed()

        override fun getDescription(): String = "Set html to $html"

        override fun perform(uiController: UiController?, view: View?) {
            val editor = view as? EditorEditText ?: return
            editor.setHtml(html)
        }
    }

    class SetLink(
        private val url: String,
    ) : ViewAction {
        override fun getConstraints(): Matcher<View> = isDisplayed()

        override fun getDescription(): String = "Set link to $url"

        override fun perform(uiController: UiController?, view: View?) {
            val editor = view as? EditorEditText ?: return
            editor.setLink(url)
        }
    }

    class ToggleList(
        private val ordered: Boolean,
    ) : ViewAction {
        override fun getConstraints(): Matcher<View> = isDisplayed()

        override fun getDescription(): String {
            val type = if (ordered) "ordered" else "unordered"
            return "Create $type list"
        }

        override fun perform(uiController: UiController?, view: View?) {
            val editor = view as? EditorEditText ?: return
            editor.toggleList(ordered)
        }
    }

    object Undo : ViewAction {
        override fun getConstraints(): Matcher<View> = isDisplayed()

        override fun getDescription(): String = "Performs undo action"

        override fun perform(uiController: UiController?, view: View?) {
            val editor = view as? EditorEditText ?: return
            editor.undo()
        }
    }

    object Redo : ViewAction {
        override fun getConstraints(): Matcher<View> = isDisplayed()

        override fun getDescription(): String = "Performs undo action"

        override fun perform(uiController: UiController?, view: View?) {
            val editor = view as? EditorEditText ?: return
            editor.redo()
        }
    }

    class ToggleFormat(
        private val format: InlineFormat
    ) : ViewAction {
        override fun getConstraints(): Matcher<View> = isDisplayed()

        override fun getDescription(): String = "Toggle format to $format"

        override fun perform(uiController: UiController?, view: View?) {
            val editor = view as? EditorEditText ?: return
            editor.toggleInlineFormat(format)
        }

    }

    class AddTextWatcher(
        private val textWatcher: (Editable?) -> Unit,
    ) : ViewAction {
        override fun getConstraints(): Matcher<View> = isDisplayed()

        override fun getDescription(): String = "Add a text watcher"

        override fun perform(uiController: UiController?, view: View?) {
            val editor = view as? EditorEditText ?: return
            editor.addTextChangedListener(
                onTextChanged = {_,_,_,_ -> },
                beforeTextChanged = {_,_,_,_ -> },
                afterTextChanged = textWatcher,
            )
        }
    }
}

object EditorActions {
    fun setText(text: String) = Editor.SetText(text)
    fun setHtml(html: String) = Editor.SetHtml(html)
    fun setLink(url: String) = Editor.SetLink(url)
    fun toggleList(ordered: Boolean) = Editor.ToggleList(ordered)
    fun undo() = Editor.Undo
    fun redo() = Editor.Redo
    fun toggleFormat(format: InlineFormat) = Editor.ToggleFormat(format)
    fun addTextWatcher(watcher : (Editable?) -> Unit) = Editor.AddTextWatcher(watcher)
}
