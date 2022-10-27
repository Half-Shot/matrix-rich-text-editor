package io.element.android.wysiwyg.spans

import android.graphics.Typeface
import android.os.Parcel
import android.text.style.StyleSpan

class BoldSpan : StyleSpan, RichTextSpan {
    constructor() : super(Typeface.BOLD)
    constructor(parcel: Parcel) : super(parcel)
}
