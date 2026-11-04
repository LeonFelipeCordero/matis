from ..inventory import stock
from ..orders import models
from ..reporting import report
from ..orders import service


def describe(drink: str, requirements: dict[str, int]) -> str:
    return f"{drink}: {requirements['beans']}g beans, {requirements['water']}ml water"


def brew(drink: str, quantity: int, available: dict[str, int]) -> str:
    order = models.create(drink, quantity)
    if not stock.reserve(available, drink, quantity):
        return "out of stock"
    ticket = service.enqueue(order)
    return report.receipt(drink, quantity) + f";ticket={ticket}"
