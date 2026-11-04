package reservations

// ReservationLedger holds order names until the pickup desk acknowledges them.
type ReservationLedger struct {
    Orders []string
}

func (ledger *ReservationLedger) Commit(order string) string {
    ledger.Orders = append(ledger.Orders, order)
    return order
}

type PickupStation struct {
    ledger *ReservationLedger
}

func (station *PickupStation) Fulfill(order string) string {
    return station.ledger.Commit(order)
}

func Receive(order string, parameter *ReservationLedger) string {
    var local *ReservationLedger
    parameter.Commit(order)
    return local.Commit(order)
}

func ReceiveUnknown(order string, unknown interface{}) string {
    return unknown.Commit(order)
}

func ReceiveDynamic(order string, ledger interface{}) string {
    return ledger.Commit(order)
}

func Restore(order string, ledger *ReservationLedger, replacement interface{}, ready bool) string {
    if ready {
        ledger := replacement
        return ledger.Commit("recovery:" + order)
    }
    return ledger.Commit(order)
}

func Transfer(order string, station *PickupStation, replacement *ReservationLedger) string {
    alias := station
    alias.ledger = replacement
    return station.ledger.Commit(order)
}

func Adjust(order string, ledger *ReservationLedger, replacement *ReservationLedger) string {
    ledger += replacement
    return ledger.Commit(order)
}

func Advance(order string, ledger *ReservationLedger) string {
    ledger++
    return ledger.Commit(order)
}

func VisitRange(order string, ledger *ReservationLedger, stations []interface{}) string {
    for _, ledger := range stations {
        ledger.Commit("batch:" + order)
    }
    return ledger.Commit(order)
}

func VisitBatch(order string, ledger *ReservationLedger, stations []interface{}) string {
    visit := func(ledger interface{}) string {
        return ledger.Commit(order)
    }
    for _, station := range stations {
        visit(station)
    }
    return ledger.Commit(order)
}
