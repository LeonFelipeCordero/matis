"""External ledger providers are not local reservation constructors."""

from ..inventory.adapters import AdapterLedger, AdapterLedger as VendorLedger
from ..inventory import adapters
from ..inventory.adapters import DottedLedger


def enqueue_vendor(order):
    return order.enqueue("vendor")


def install_vendor(order):
    VendorLedger.commit = enqueue_vendor
    ledger = VendorLedger()
    return ledger.commit(order)


def migrate_vendor(order, ledger: AdapterLedger):
    VendorLedger.commit = enqueue_vendor
    return ledger.commit(order)


def migrate_dotted(order, ledger: DottedLedger):
    adapters.DottedLedger.commit = enqueue_vendor
    return ledger.commit(order)


class RemoteLedger:
    def commit(self, order):
        return order.enqueue("remote")


def RemoteLedger(provider):
    return provider.open_ledger()


def receive_remote(order, provider):
    ledger = RemoteLedger(provider)
    return ledger.commit(order)
