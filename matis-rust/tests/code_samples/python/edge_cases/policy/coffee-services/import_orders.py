"""Order import paths cannot conceal cross-layer storage ownership."""

from typing import TYPE_CHECKING
from ..persistence import repositories as order_storage
import matis.persistence.database as order_database
from matis.persistence.models import RepositoryEntity as CoffeeOrder

if TYPE_CHECKING:
    from ..persistence.navigation import GraphReader


def open_orders(path):
    from ..persistence.repositories import repository_store
    return repository_store(path)


def database_for_orders(path):
    factory = order_database.create_engine
    alias = factory
    return alias(path)


def model_for_orders():
    return CoffeeOrder


def list_orders(store):
    return order_storage.RepositoryStore(store).list()
