package coffee.orders

import coffee.reporting.render as renderCoffee

enum class OrderStatus { QUEUED, BREWED, CANCELLED }

data class OrderLine(val drink: String, val quantity: Int = 1, val price: Int = 300) {
    fun subtotal(): Int = quantity * price

    companion object Factory {
        fun espresso(quantity: Int = 1): OrderLine = OrderLine("espresso", quantity, 250)
    }
}

data class Order(
    val customer: String,
    val lines: List<OrderLine>,
    val notes: Map<String, String> = emptyMap(),
    var status: OrderStatus = OrderStatus.QUEUED,
) {
    fun total(): Int = lines.sumOf { it.subtotal() }

    fun names(): List<String> = lines.flatMap { line -> List(line.quantity) { line.drink } }

    fun transition(next: OrderStatus): String {
        if (status != OrderStatus.QUEUED) return "unchanged"
        val message = when (next) {
            OrderStatus.BREWED -> "collected"
            OrderStatus.CANCELLED -> "refunded"
            else -> "unchanged"
        }
        status = next
        return message
    }
}

fun Order.receipt(prefix: String = "coffee-order"): String = prefix + ":" + renderCoffee(names())
