package coffee.brewing

import coffee.reporting.render

fun portions(name: String): Int = when (name) {
    "espresso" -> 2
    "filter" -> 3
    else -> 0
}

fun preview(): String = render(listOf("espresso", "filter"))
