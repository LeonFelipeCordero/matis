package coffee.inventory

import coffee.orders.Order

fun deliveryLabel(order: Order): String {
    val units = order.lines.sumOf { it.quantity }
    return "Deliver $units drinks to ${order.customer}"
}

fun manifest(orders: List<Order>): List<String> = orders.map { deliveryLabel(it) }
