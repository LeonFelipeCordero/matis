package coffee.reporting

import coffee.inventory.Inventory
import coffee.orders.Order
import coffee.orders.OrderLine
import coffee.orders.OrderStatus
import coffee.orders.receipt

typealias OrderFilter = (Order) -> Boolean

class SalesReport(private val inventory: Inventory) {
    private val include: OrderFilter = { it.status == OrderStatus.BREWED }

    fun totals(orders: List<Order>, minimum: Int = 0): Map<String, Int> {
        val totals = mutableMapOf<String, Int>()
        orders.forEach order@ { order ->
            if (!include(order)) return@order
            fun add(line: OrderLine) {
                totals[line.drink] = (totals[line.drink] ?: 0) + line.subtotal()
            }
            order.lines.forEach(::add)
        }
        return totals.filterValues { it >= minimum }
    }

    fun discounts(orders: List<Order>, percent: Int): List<List<Int>> {
        return orders.map { order ->
            order.lines.map { line -> line.subtotal() * (100 - percent) / 100 }
        }
    }

    fun projected(orders: List<Order>, percent: Int): Int {
        val adjusted = fun(line: OrderLine): Int {
            return line.subtotal() * (100 - percent) / 100
        }
        return orders.filter(include).sumOf { order -> order.lines.sumOf(adjusted) }
    }

    fun résumé(orders: List<Order>): String {
        val names = orders.filter(include).flatMap { it.names() }
        return render(names) + ";espresso-stock=" + inventory.units("espresso")
    }

    fun résumé(order: Order): String = order.receipt("café")
}
