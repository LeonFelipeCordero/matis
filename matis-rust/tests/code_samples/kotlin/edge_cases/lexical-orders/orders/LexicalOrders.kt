package coffee.orders

import coffee.brewing.portions as dose

fun single(recipes: List<(String) -> Int>, name: String, local: String): Int {
    recipes.forEach { dose -> println(dose(local)) }
    return dose(name)
}

fun destructured(recipes: Map<String, (String) -> Int>, name: String, local: String): Int {
    recipes.forEach { (label, dose) -> println("$label: ${dose(local)}") }
    return dose(name)
}

fun loop(recipes: List<(String) -> Int>, name: String, local: String): Int {
    for (dose in recipes) { println(dose(local)) }
    return dose(name)
}

fun pairedLoop(recipes: Map<String, (String) -> Int>, name: String, local: String): Int {
    for ((label, dose) in recipes) { println("$label: ${dose(local)}") }
    return dose(name)
}

fun nested(batches: List<List<(String) -> Int>>, name: String, local: String): Int {
    batches.forEach { recipes -> recipes.forEach { dose -> println(dose(local)) } }
    return dose(name)
}
