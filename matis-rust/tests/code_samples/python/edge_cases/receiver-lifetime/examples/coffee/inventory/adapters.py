"""Vendor adapters whose commit methods can be replaced at runtime."""

class AdapterLedger:
    def commit(self, order):
        return order.receipt()


class DottedLedger:
    def commit(self, order):
        return order.receipt()
