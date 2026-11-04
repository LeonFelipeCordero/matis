from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Generic, TypeVar

from coffee.brewing.recipe import Recipe
from coffee.orders.models import Order

T = TypeVar("T")


class Ledger(ABC, Generic[T]):
    @abstractmethod
    def record(self, entry: T) -> None:
        raise NotImplementedError


@dataclass(frozen=True)
class StockChange:
    drink: str
    units: int
    reason: str


class Inventory(Ledger[StockChange]):
    @dataclass(frozen=True)
    class Snapshot:
        available: dict[str, int]

        def units(self, drink: str) -> int:
            return self.available.get(drink, 0)

    def __init__(self, available: dict[str, int] | None = None):
        self.available: dict[str, int] = dict(available or {})
        self.changes: list[StockChange] = []

    def record(self, entry: StockChange) -> None:
        self.changes.append(entry)
        self.available[entry.drink] = self.available.get(entry.drink, 0) + entry.units

    def receive(self, *shipments: StockChange, **counts: int) -> None:
        for shipment in shipments:
            self.record(shipment)
        for drink, units in counts.items():
            self.record(StockChange(drink, units, "delivery"))

    def reserve(self, order: Order) -> bool:
        needed: dict[str, int] = {}
        for line in order.lines:
            units = Recipe.portions(line.drink) * line.quantity
            if units == 0:
                return False
            needed[line.drink] = needed.get(line.drink, 0) + units
        if any(self.available.get(drink, 0) < units for drink, units in needed.items()):
            return False
        for drink, units in needed.items():
            self.record(StockChange(drink, -units, "order"))
        return True

    def snapshot(self) -> Snapshot:
        return self.Snapshot(dict(self.available))
