"""Coffee order schema revisions may create tables, not populate domain rows."""

from alembic import op
import sqlalchemy as sa

revision = "coffee_orders"
down_revision = None


def upgrade():
    op.create_table("coffee_orders", sa.Column("id", sa.Integer(), primary_key=True))
    op.create_index("coffee_order_customer", "coffee_orders", ["customer"])


def downgrade():
    op.drop_index("coffee_order_customer", table_name="coffee_orders")
    op.drop_table("coffee_orders")


def graph_name_collation():
    op.create_index("graph_nodes_name", "graph_nodes", ["repository_id", sa.text("name COLLATE NOCASE")])


def other_name_collation():
    op.create_index("coffee_order_name", "coffee_orders", ["customer_id", sa.text("name COLLATE NOCASE")])


def seed_orders():
    op.bulk_insert("coffee_orders", [{"id": 1, "customer": "Ada"}])


def query_orders():
    op.execute("SELECT id FROM coffee_orders")


def connection_orders():
    return op.get_bind()


def mapped_orders():
    return sa.select("coffee_orders")
