"""Pure mappings convert saved coffee facts to domain-shaped dictionaries."""

from __future__ import annotations
from typing import Any


def order_record(order: dict[str, Any]) -> dict[str, Any]:
    return {"id": order["id"], "customer": order["customer"], "items": list(order["items"])}


def order_names(orders):
    return [order["customer"] for order in orders]


def export_records(destination, orders):
    destination.write_text(str([order_record(order) for order in orders]))


def read_records(source):
    return source.read_text()


def lookup_record(connection, key):
    return connection.execute(f"SELECT id FROM coffee_orders WHERE id = {key}")
