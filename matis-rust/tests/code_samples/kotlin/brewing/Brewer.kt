package coffee.brewing

import coffee.reporting.render
import java.lang.StringBuilder

interface CoffeeMaker {
    fun brew(names: List<String>): String
}

class Brewer : CoffeeMaker {
    override fun brew(names: List<String>): String {
        if (names.isEmpty()) return render(emptyList())
        val accepted = names.filter { portions(it) > 0 }
        return render(accepted)
    }

    fun retry(attempts: Int): Int {
        if (attempts <= 0) return 0
        return 1 + retry(attempts - 1)
    }

    fun score(batches: List<List<String>>): Int {
        var total = 0
        for (batch in batches) {
            for (name in batch) {
                val amount = portions(name)
                if (amount > 0 && amount <= 3) total += amount
            }
        }
        return total
    }

    // Deliberately exceed the cyclomatic-complexity limit without deep nesting.
    fun classify(amount: Int): Int {
        var category = 0
        if (amount > 0) category++
        if (amount > 1) category++
        if (amount > 2) category++
        if (amount > 3) category++
        if (amount > 4) category++
        if (amount > 5) category++
        if (amount > 6) category++
        if (amount > 7) category++
        return category
    }

    fun ready(water: Int, beans: Int, cups: Int, temperature: Int): Boolean {
        return water > 0 && beans > 0 && cups > 0 && temperature > 0
    }

    fun calibrate(dose: Int, water: Int, seconds: Int, temperature: Int,
                  grind: Int, pressure: Int, humidity: Int, altitude: Int): Int {
        val extraction = water * pressure / maxOf(dose * seconds, 1)
        return extraction + temperature - grind - humidity / 10 + altitude / 100
    }
}

fun main() {
    val output = StringBuilder(Brewer().brew(listOf("espresso", "filter", "invalid")))
    println(output.toString())
}
