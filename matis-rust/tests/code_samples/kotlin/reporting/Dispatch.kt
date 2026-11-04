package coffee.reporting

import coffee.orders.Order
import coffee.orders.OrderStatus
import coffee.orders.receipt

typealias DispatchFormatter = (Order) -> String

class Dispatch(private val send: (String) -> Unit) {
    private fun eligible(order: Order): Boolean = order.status == OrderStatus.BREWED

    fun deliver(orders: List<Order>, formatter: DispatchFormatter) {
        val fallback = fun(order: Order): String { return order.receipt("pickup") }
        orders.filter(::eligible).forEach dispatch@ { order ->
            if (order.customer.isEmpty()) return@dispatch
            fun formatReady(): String = formatter(order).ifEmpty { fallback(order) }
            send(formatReady())
        }
        preview(orders, fun(order: Order) = order.receipt("preview"))
    }

    fun preview(orders: List<Order>, formatter: DispatchFormatter): List<String> {
        return orders.map(formatter)
    }

    companion object Factory {
        fun console(): Dispatch = Dispatch(::println)
    }
}
