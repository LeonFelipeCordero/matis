"""Dispatch brewed orders and retain pickup notes on the domain model."""

from .models import Order, OrderStatus


def tracking_label(order: Order, channel: str) -> str:
    return f"{order.customer}: {order.status.value} at {channel}"


def track_delivery(order: Order, /, courier: str = "counter", *, fee: int = 0, **metadata: str) -> str:
    order.notes.update(metadata, courier=courier, delivery_total=str(tracking_total(order, fee=fee)))
    return order.transition(OrderStatus.BREWED)


def track_events(*orders: Order, channel: str = "pickup", **metadata) -> list[str]:
    return [track_delivery(order, channel, **metadata) for order in orders if order.status == OrderStatus.QUEUED]


def tracking_total(order: Order, # Order annotations and separators are not inputs.
                   /, *, fee: int = 0):
    return order.total + fee


class PickupQueue:
    def __init__(self):
        self.orders: list[Order] = []

    def record(self: "PickupQueue", order: Order):
        def label(order: Order, channel: str):
            return tracking_label(order, channel)
        self.orders.append(order)
        return label(order, "pickup")

    @classmethod
    def from_orders(cls, orders: list[Order]) -> "PickupQueue":
        queue = cls()
        queue.orders = [order for order in orders if order.status != OrderStatus.CANCELLED]
        return queue

    @staticmethod
    def notification(order: Order, channel: str):
        return tracking_label(order, channel)

    def dispatch(queue: "PickupQueue", courier: str = "counter", *, fee: int = 0) -> list[str]:
        return track_events(*queue.orders, channel=courier, fee=fee)

    def collect(self, *customers: str):
        ready = [order for order in self.orders if order.status == OrderStatus.BREWED and order.customer in customers]
        self.orders = [order for order in self.orders if order not in ready]
        return ready

    def annotate(self, **metadata: str):
        for order in self.orders:
            order.notes.update(metadata)
