"""Compose order services within privately owned repository contexts."""

from contextlib import contextmanager
from matis.persistence.repositories import repository_store as _repository_store
from matis.persistence.navigation import navigation_reader as _navigation_reader
from matis.persistence.database import migrate_database as _migrate_database
from matis.services.repositories import RepositoryService
from matis.services.navigation import NavigationService


@contextmanager
def repository_service(path):
    with _repository_store(path) as repository:
        yield RepositoryService(repository)


@contextmanager
def navigation_service(path):
    with _navigation_reader(path) as reader:
        yield NavigationService(reader)


def migrate_database(path):
    _migrate_database(path)


@contextmanager
def expose_repository(path):
    with _repository_store(path) as repository:
        yield repository


@contextmanager
def expose_nested_repository(path):
    with _repository_store(path) as repository:
        yield {"coffee": [repository]}


def return_repository(path):
    with _repository_store(path) as repository:
        return repository


def expose_reader(path):
    with _navigation_reader(path) as reader:
        alias = reader
        return alias


def use_session(path):
    with _repository_store(path) as repository:
        return repository.session.query("coffee")
