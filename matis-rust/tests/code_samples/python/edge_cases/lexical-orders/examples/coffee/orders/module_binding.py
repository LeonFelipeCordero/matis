"""A module-level seasonal override survives an ordinary conditional block."""

from coffee.brewing.recipe import Recipe as Drink
from coffee.inventory.stock import Inventory

name = "espresso"
if name in Inventory({name: 12}).available:
    Drink = Inventory
print(Drink.portions(name))
