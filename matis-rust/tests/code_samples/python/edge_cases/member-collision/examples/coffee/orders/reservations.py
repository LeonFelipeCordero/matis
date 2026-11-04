from coffee.inventory.stock import Inventory
from coffee.orders.models import Order


def reserve(order: Order) -> str:
    drinks = ", ".join(f"{line.quantity} {line.drink}" for line in order.lines)
    return f"Reserved for {order.customer}: {drinks}"


def queue_order(inventory: Inventory, order: Order) -> str:
    if not inventory.reserve(order):
        return f"Waiting for stock: {order.customer}"
    return reserve(order)


def request_supplier(supplier, order: Order) -> str:
    if not supplier.reserve(order):
        return f"Supplier cannot fill: {order.customer}"
    return supplier.delivery_label(order)
