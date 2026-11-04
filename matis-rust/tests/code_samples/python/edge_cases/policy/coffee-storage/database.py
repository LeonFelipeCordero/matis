"""Coffee database connection setup is not a domain repository."""

from sqlalchemy import create_engine, select, text
from .models import CoffeeOrder


def configure_connection(connection):
    connection.exec_driver_sql("PRAGMA foreign_keys = ON")
    connection.exec_driver_sql("PRAGMA busy_timeout = 30000")
    connection.exec_driver_sql("PRAGMA journal_mode = WAL")


def begin_connection(connection, immediate):
    connection.exec_driver_sql("BEGIN IMMEDIATE" if immediate else "BEGIN")


def readonly_connection(connection):
    connection.execute("PRAGMA query_only=ON")


def setup_with_parameters(connection):
    connection.execute("PRAGMA foreign_keys = ON", {})


def dynamic_setup(connection, setting):
    connection.exec_driver_sql(setting)


def chained_setup(connection):
    connection.executescript("PRAGMA foreign_keys = ON; SELECT order_id FROM coffee_orders")


def read_orders(connection):
    return connection.execute("SELECT order_id FROM coffee_orders")


def mapped_orders(session):
    return session.scalars(select(CoffeeOrder)).all()


def query_orders(session):
    return session.query(CoffeeOrder).all()


def load_order(session, key):
    return session.get(CoffeeOrder, key)


def save_order(session, order):
    session.add(order)
    session.commit()


def initialize_schema(engine):
    CoffeeOrder.metadata.create_all(engine)


def textual_orders(session):
    return session.execute(text("SELECT order_id FROM coffee_orders"))


def formatted_orders(session, customer):
    return session.execute(f"SELECT order_id FROM coffee_orders WHERE customer = {customer}")


def prefixed_orders(session):
    return session.execute(select(CoffeeOrder).prefix_with("EXPLAIN"))


def literal_orders(session):
    return session.execute(select(text("order_id")))


def driver_orders(connection):
    return connection.cursor()


def raw_operator(order):
    return order.customer.op("LIKE")("espresso%")
