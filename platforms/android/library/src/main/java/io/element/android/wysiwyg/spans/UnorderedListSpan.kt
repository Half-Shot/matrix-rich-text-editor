package io.element.android.wysiwyg.spans

import android.os.Build
import android.os.Parcel
import android.text.style.BulletSpan
import androidx.annotation.ColorInt
import androidx.annotation.IntRange
import androidx.annotation.RequiresApi

class UnorderedListSpan : BulletSpan, RichTextSpan {
    constructor() : super()
    constructor(gapWidth: Int) : super(gapWidth)
    constructor(gapWidth: Int, @ColorInt color: Int) : super(gapWidth, color)
    @RequiresApi(Build.VERSION_CODES.P)
    constructor(
        gapWidth: Int,
        @ColorInt color: Int,
        @IntRange(from = 0) radius: Int
    ) : super(gapWidth, color, radius)
    constructor(parcel: Parcel) : super(parcel)
}
