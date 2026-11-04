from coffee.orders.models import Order


def delivery_label(order: Order) -> str:
    units = sum(line.quantity for line in order.lines)
    return f"Deliver {units} drinks to {order.customer}"


def manifest(orders: list[Order]) -> list[str]:
    return [delivery_label(order) for order in orders]
