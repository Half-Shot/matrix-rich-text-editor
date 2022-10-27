package io.element.android.wysiwyg.spans

import android.os.Parcel
import android.text.style.ForegroundColorSpan
import androidx.annotation.ColorInt

class LinkSpan : ForegroundColorSpan {
    constructor(@ColorInt color: Int) : super(color)
    constructor(parcel: Parcel) : super(parcel)
}
