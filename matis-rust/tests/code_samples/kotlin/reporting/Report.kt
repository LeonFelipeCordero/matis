package coffee.reporting

import coffee.brewing.portions

fun render(names: List<String>): String {
    val entries = names.map { name -> "$name=${portions(name)}" }
    val total = names.sumOf { name -> portions(name) }
    return "coffee:${entries.joinToString(",")};total=$total"
}
