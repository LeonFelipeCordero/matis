from ..brewing import recipe


def create(drink: str, quantity: int) -> dict[str, str | int]:
    requirements = recipe.ingredients(drink, quantity)
    if not requirements['beans']:
        return {"drink": drink, "quantity": 0, "status": "unavailable"}
    return {"drink": drink, "quantity": quantity, "status": "queued"}


def cancel(order: dict[str, str | int]) -> bool:
    if order['status'] != 'queued':
        return False
    order['status'] = 'cancelled'
    return True
