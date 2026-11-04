"""Order intake, recovery and ledger migration paths."""

from ..inventory import ledger as ledgers
from ..inventory.ledger import (
    DeliveryLedger,
    ReservationLedger,
    ReservationLedger as PickupLedger,
    StableStation,
)


def fulfill(order, ledger: ledgers.ReservationLedger):
    return ledger.commit(order)


def accept_unknown(order, unknown):
    return unknown.commit(order)


def redirect_inferred(order, replacement):
    ledger = ReservationLedger()
    ledger = replacement
    return ledger.commit(order)


def redirect_declared(order, ledger: ReservationLedger, replacement):
    ledger = replacement
    return ledger.commit(order)


def receive_before_opening(order):
    ledger.commit(order)
    ledger: ReservationLedger = ReservationLedger()
    return ledger


def open_on_demand(order, ready):
    if ready:
        ledger = ReservationLedger()
    return ledger.commit(order)


def open_declared_on_demand(order, ready):
    if ready:
        ledger: ReservationLedger = ReservationLedger()
    return ledger.commit(order)


def restore_batch(order, replacements):
    ledger = ReservationLedger()
    ledger, pending = replacements
    ledger.commit(order)
    return pending


def replace_field(order, station: StableStation, replacement):
    station.ledger = replacement
    return station.ledger.commit(order)


def replace_alias(order, station: StableStation, replacement):
    alias = station
    alias.ledger = replacement
    return station.ledger.commit(order)


def replace_chain(order, station: StableStation, replacement):
    alias = station
    supervisor = alias
    supervisor.ledger = replacement
    return station.ledger.commit(order)


def write_original(order, station: StableStation, replacement):
    alias: StableStation = station
    station.ledger = replacement
    return alias.ledger.commit(order)


def install_adapter(order, station: StableStation, replacement):
    adapter = station.ledger
    adapter.commit = replacement
    return station.ledger.commit(order)


def configure_remote(station, replacement):
    station.ledger = replacement
    return station


def fulfill_unchanged(order, station: StableStation):
    return station.ledger.commit(order)


def open_pickup(order):
    ledger: ReservationLedger = ReservationLedger()
    return ledger.commit(order)


def open_delivery(order):
    ledger: ReservationLedger = DeliveryLedger()
    return ledger.commit(order)


def resume_pickup(order):
    opened = ReservationLedger()
    ledger: ReservationLedger = opened
    return ledger.commit(order)


def resume_delivery(order):
    opened = DeliveryLedger()
    ledger: ReservationLedger = opened
    return ledger.commit(order)


def open_alias(order):
    ledger: ReservationLedger = PickupLedger()
    return ledger.commit(order)


def visit_batch(order, station: ReservationLedger, batches):
    def visit(station):
        return station.commit(order)
    results = list(map(visit, batches))
    station.commit(order)
    return results
