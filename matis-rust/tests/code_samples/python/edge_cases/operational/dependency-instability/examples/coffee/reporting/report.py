from ..brewing import recipe


def receipt(drink: str, quantity: int) -> str:
    ingredients = recipe.ingredients(drink, quantity)
    return f"coffee:{drink};cups={quantity};beans={ingredients['beans']}"


def daily_totals(orders: list[tuple[str, int]]) -> dict[str, int]:
    totals: dict[str, int] = {}
    for drink, quantity in orders:
        totals[drink] = totals.get(drink, 0) + quantity
    return totals
