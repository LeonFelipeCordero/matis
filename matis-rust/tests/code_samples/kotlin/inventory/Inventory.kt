package coffee.inventory

import coffee.brewing.portions
import coffee.orders.Order

abstract class StockLedger<T> {
    protected val entries: MutableList<T> = mutableListOf()

    open fun record(entry: T) {
        entries.add(entry)
    }

    fun <R> snapshot(transform: (T) -> R): List<R> = entries.map(transform)
}

interface Reservable {
    fun reserve(order: Order): Boolean
}

class Inventory(private val available: MutableMap<String, Int> = mutableMapOf()) :
    StockLedger<Inventory.Change>(), Reservable {
    data class Change(val drink: String, val units: Int, val reason: String)

    override fun record(entry: Change) {
        super.record(entry)
        available[entry.drink] = units(entry.drink) + entry.units
    }

    fun receive(vararg shipments: Change) {
        shipments.forEach { record(it) }
    }

    override fun reserve(order: Order): Boolean {
        val needed = mutableMapOf<String, Int>()
        for (line in order.lines) {
            val amount = portions(line.drink) * line.quantity
            if (amount == 0) return false
            needed[line.drink] = (needed[line.drink] ?: 0) + amount
        }
        if (needed.any { (drink, count) -> units(drink) < count }) return false
        needed.forEach { (drink, count) -> record(Change(drink, -count, "order")) }
        return true
    }

    fun units(drink: String): Int = available[drink] ?: 0

    companion object {
        fun stocked(espresso: Int = 40, filter: Int = 60): Inventory {
            val inventory = Inventory()
            inventory.receive(Change("espresso", espresso, "opening"), Change("filter", filter, "opening"))
            return inventory
        }
    }
}
