package reservations

import orders.Order

open class ReservationLedger {
    private val orders = mutableListOf<Order>()

    open fun commit(order: Order): Order {
        orders.add(order)
        return order
    }
}

class DeliveryLedger : ReservationLedger() {
    override fun commit(order: Order): Order = super.commit(order)
}
