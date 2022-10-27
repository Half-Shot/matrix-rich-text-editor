package io.element.android.wysiwyg.extensions

import android.text.Editable
import androidx.core.text.getSpans
import io.element.android.wysiwyg.spans.RichTextSpan

fun Editable.clearRichTextSpans() {
    val richTextSpans = getSpans<RichTextSpan>(start = 0, end = length)
    for (span in richTextSpans) {
        removeSpan(span)
    }
}
