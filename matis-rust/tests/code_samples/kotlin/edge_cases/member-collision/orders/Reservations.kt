package coffee.orders

import coffee.inventory.Inventory

fun reserve(order: Order): String {
    val drinks = order.lines.joinToString(", ") { "${it.quantity} ${it.drink}" }
    return "Reserved for ${order.customer}: $drinks"
}

fun queueOrder(inventory: Inventory, order: Order): String {
    if (!inventory.reserve(order)) return "Waiting for stock: ${order.customer}"
    return reserve(order)
}

fun requestSupplier(supplier: Any, order: Order): String {
    if (!supplier.reserve(order)) return "Supplier cannot fill: ${order.customer}"
    return supplier.deliveryLabel(order)
}
