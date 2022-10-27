package io.element.android.wysiwyg.spans

import android.graphics.Typeface
import android.os.Parcel
import android.text.style.StyleSpan
import android.text.style.TypefaceSpan

class ItalicSpan : StyleSpan, RichTextSpan {
    constructor() : super(Typeface.ITALIC)
    constructor(parcel: Parcel) : super(parcel)
}
