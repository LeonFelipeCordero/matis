from ..brewing import recipe


def reserve(available: dict[str, int], drink: str, quantity: int) -> bool:
    needed = recipe.ingredients(drink, quantity)
    if any(available.get(item, 0) < units for item, units in needed.items()):
        return False
    for item, units in needed.items():
        available[item] -= units
    return True


def replenish(available: dict[str, int], delivery: dict[str, int]) -> None:
    for item, units in delivery.items():
        available[item] = available.get(item, 0) + units
