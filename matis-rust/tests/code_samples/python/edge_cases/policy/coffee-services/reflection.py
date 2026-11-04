"""Order plugins must not bypass private service ports by reflection."""

import builtins
import importlib
import operator as inspect_order
import sys
from importlib import import_module as load_plugin


def dynamic_plugin(name):
    return load_plugin(name)


def builtin_plugin(name):
    return __import__(name)


def evaluate_discount(expression):
    return eval(expression)


def execute_discount(expression):
    exec(expression)


def builtin_evaluate(expression):
    return builtins.eval(expression)


def registry_plugin(name):
    return sys.modules[name]


def reflective_storage(service):
    return getattr(service, "_repository")


def reflective_write(service, storage):
    setattr(service, "_repository", storage)


def dictionary_storage(service):
    return service.__dict__["_repository"]


def attribute_storage(service):
    return service.__getattribute__("_repository")


def dotted_storage(service):
    return inspect_order.attrgetter("_repository.session")(service)


def aliased_storage(service):
    lookup = inspect_order.attrgetter
    return lookup("_reader")(service)


def method_storage(service):
    return inspect_order.methodcaller("execute", "SELECT order_id FROM coffee_orders")(service)


def public_customer(order):
    return inspect_order.attrgetter("customer.name")(order)


def public_label(order):
    return inspect_order.methodcaller("label")(order)


def declared_import():
    return importlib
