"""Integration annotations without an imported provider are unresolved."""


def receive(order, ledger: ReservationLedger):
    return ledger.commit(order)
