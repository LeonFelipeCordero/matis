package coffee.orders

class TrackingRates(private val deliveryFee: Int = 50) {
    fun quote(order: Order): Int = order.total() + deliveryFee
}

fun <T> TrackingRates.track(order: Order, vararg events: T): String {
    order.transition(OrderStatus.BREWED)
    return "${order.customer}: ${quote(order)} (${events.joinToString()})"
}
val trackingLabel = { (order, counter): Pair<Order, String>, fee: Int -> "${order.customer} at $counter: ${order.total() + fee}" }
val trackingQueue: () -> MutableList<Order> = { -> mutableListOf<Order>() }
val trackingReady: (Order) -> Boolean = { it.status == OrderStatus.BREWED }

fun dispatchPickup(orders: List<Order>, rates: TrackingRates): List<String> {
    val queue = trackingQueue()
    for (order in orders) if (order.status == OrderStatus.QUEUED) rates.track(order, "brewed")
    queue.addAll(orders.filter(trackingReady))
    return queue.map { order -> trackingLabel(order to "counter", rates.quote(order) - order.total()) }
}
