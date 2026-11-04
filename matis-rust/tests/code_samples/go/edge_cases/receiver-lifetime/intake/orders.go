package intake

import "reservations"

func Fulfill(order string, ledger *reservations.ReservationLedger) string {
    return ledger.Commit(order)
}
