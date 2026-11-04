"""Coffee order service with domain-owned storage and navigation ports."""

from matis.domain.repositories import RepositoryProtocol
from matis.domain.navigation import NavigationReader


class OrderService:
    def __init__(self, repository: RepositoryProtocol, reader: NavigationReader):
        self._repository = repository
        self._reader = reader

    def list_orders(self):
        return self._repository.list()

    def trace_order(self, key):
        return self._reader.get(key)

    def list_through_alias(self):
        storage = self._repository
        return storage.list()

    def return_storage(self):
        return self._repository

    def yield_storage(self):
        yield self._repository

    def tuple_storage(self):
        return ("orders", self._repository)

    def list_storage(self):
        return [self._repository]

    def mapping_storage(self):
        return {"orders": self._repository}

    def alias_storage(self):
        first = self._repository
        second = first
        return second

    def annotated_storage(self):
        storage: RepositoryProtocol = self._repository
        return storage

    def unpack_storage(self):
        storage, label = self._repository, "coffee orders"
        return storage

    def assign_storage(self):
        self.public_orders = self._repository
        return "coffee orders"

    def callback_storage(self):
        publish_orders(self._repository)

    def keyword_storage(self):
        publish_orders(repository=self._repository)

    def copied_storage(self):
        return dict(repository=self._repository)

    def constructor_escape(self, repository: RepositoryProtocol):
        publish_orders(repository)

    def public_attribute(self):
        return self.repository.list()

    def raw_session(self):
        return self._repository.session.execute("SELECT order_id FROM coffee_orders")
