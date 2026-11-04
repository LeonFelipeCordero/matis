"""Seasonal overrides and unconfigured order printers must not select house code."""

from coffee.brewing.recipe import Recipe as Drink
from coffee.brewing.brewer import Brewer


def class_receipt(names):
    return Brewer.brew(names)


def supplied_receipt(names, unknown, factory):
    return unknown.brew(names), factory().brew(names)


def parameter(name, Drink):
    return Drink.portions(name)


def unpacked(name, recipes):
    Drink, spare = recipes
    return Drink.portions(name), spare


def rebound(name, replacement):
    Drink = replacement
    return Drink.portions(name)


def replaced_import(name):
    from seasonal_supplier import Recipe as Drink
    return Drink.portions(name)


def local_recipe(name):
    from coffee.brewing.recipe import Recipe as LocalDrink
    return LocalDrink.portions(name)


def missing_recipe(name):
    return LocalDrink.portions(name)
