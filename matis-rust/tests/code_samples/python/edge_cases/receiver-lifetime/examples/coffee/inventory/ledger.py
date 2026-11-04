"""Reservation records shared by pickup and delivery workflows."""

class ReservationLedger:
    def __init__(self):
        self.orders = []

    def commit(self, order):
        self.orders.append(order)
        return order


class DeliveryLedger:
    def __init__(self):
        self.deliveries = []

    def commit(self, order):
        self.deliveries.append(order)
        return order


class InjectedStation:
    def __init__(self, ledger: ReservationLedger):
        self.ledger = ledger

    def fulfill(self, order):
        return self.ledger.commit(order)


class StableStation:
    ledger: ReservationLedger

    def fulfill(self, order):
        return self.ledger.commit(order)


class PickupStation:
    ledger: ReservationLedger

    def transfer(self, order, replacement):
        alias = self
        alias.ledger = replacement
        return self.ledger.commit(order)


class RecoveryStation:
    def __init__(self):
        self.ledger = ReservationLedger()

    def redirect(self, order, replacement):
        self.ledger = replacement
        return self.ledger.commit(order)


class RemoteRecoveryStation:
    def __init__(self):
        self.ledger = ReservationLedger()

    def replace_station(self, order, replacement):
        self = replacement
        return self.ledger.commit(order)


class OpeningStation:
    def __init__(self):
        self.ledger = ReservationLedger()

    def fulfill(self, order):
        return self.ledger.commit(order)


class RemoteStation:
    def __init__(self, remote):
        self.ledger = remote


class PickupWindow:
    station: StableStation

    def transfer(self, order, replacement):
        alias = self.station
        alias.ledger = replacement
        return self.station.ledger.commit(order)
