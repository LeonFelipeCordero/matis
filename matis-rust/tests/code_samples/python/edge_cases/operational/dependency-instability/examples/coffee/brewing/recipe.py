from . import brewer
from . import brewer as brewing_service
from . import recipe as recipe_catalog
import sys


RECIPES = {"espresso": (18, 36), "filter": (20, 300)}


def ingredients(drink: str, quantity: int = 1) -> dict[str, int]:
    beans, water = RECIPES.get(drink, (0, 0))
    return {"beans": beans * quantity, "water": water * quantity}


def preview(drink: str) -> str:
    return brewer.describe(drink, ingredients(drink))


def print_menu(drinks: list[str]) -> None:
    for drink in drinks:
        requirements = recipe_catalog.ingredients(drink)
        sys.stdout.write(brewing_service.describe(drink, requirements) + "\n")
