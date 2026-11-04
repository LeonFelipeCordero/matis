package intake

import orders.Order
import reservations.ReservationLedger
import reservations.ReservationLedger as PickupLedger
import reservations.DeliveryLedger

class ReservationIntake {
    @Inject private lateinit var ledger: ReservationLedger

    fun fulfill(order: Order): Order = ledger.commit(order)
}

class TemporaryIntake(ledger: ReservationLedger) {
    fun fulfill(order: Order): Order = ledger.commit(order)
}

fun openPickup(order: Order): Order {
    val ledger = ReservationLedger()
    return ledger.commit(order)
}

fun openAlias(order: Order): Order {
    val ledger: ReservationLedger = PickupLedger()
    return ledger.commit(order)
}

fun receive(order: Order, ledger: ReservationLedger): Order = ledger.commit(order)

fun receiveUnknown(order: Order, unknown: Any): Order = unknown.commit(order)

fun redirect(order: Order, ledger: ReservationLedger): Order {
    ledger = DeliveryLedger()
    return ledger.commit(order)
}

fun advance(order: Order, ledger: ReservationLedger): Order {
    ledger++
    return ledger.commit(order)
}

fun advanceBefore(order: Order, ledger: ReservationLedger): Order {
    ++ledger
    return ledger.commit(order)
}

fun visitBatch(order: Order, ledger: ReservationLedger, stations: List<ReservationLedger>) {
    stations.forEach { ledger -> ledger.commit(order) }
    ledger.commit(order)
}
