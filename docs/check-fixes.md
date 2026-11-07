# Fix check failures

`matis check` links each failure to a section below. Use these suggestions as starting points, not automatic fixes. Keep behavior and tests unchanged when you refactor. If a metric is unavailable, resolve analysis problems before changing the code.

The before-and-after examples use Python. Apply the same approach in your project's language.

## maxParameters

Group related inputs into a small parameter object, record, or struct. Pass that object instead of several separate values. Do not group unrelated inputs only to pass the check.

Before:

```python
def ship_order(order, street, city, postal_code, country):
    label = format_label(street, city, postal_code, country)
    return dispatch(order, label)
```

After:

```python
from dataclasses import dataclass


@dataclass(frozen=True)
class Address:
    street: str
    city: str
    postal_code: str
    country: str


def ship_order(order, address):
    label = format_label(address.street, address.city, address.postal_code, address.country)
    return dispatch(order, label)
```

Update callers to pass an `Address` instead of separate address fields.

## maxFileLines

Split the file by responsibility. Move related functions or types into focused files and update their callers and imports. For a large class, extract a cohesive responsibility into another class. This rule measures files, not class size.

Before, `orders.py` handles both order creation and receipt formatting:

```python
# orders.py
def format_receipt(order):
    return f"Order {order.id}: {order.total}"


def create_order(cart):
    order = save_order(cart)
    send_receipt(format_receipt(order))
    return order
```

After, move receipt formatting into its own file. Keep the existing `save_order` and `send_receipt` imports in `orders.py`:

```python
# receipts.py
def format_receipt(order):
    return f"Order {order.id}: {order.total}"
```

```python
# orders.py
from receipts import format_receipt


def create_order(cart):
    order = save_order(cart)
    send_receipt(format_receipt(order))
    return order
```

## maxFunctionLines

Extract a coherent operation into a named function. Keep each function focused on one task.

Before:

```python
def complete_order(order):
    charge(order)
    lines = [f"{item.name}: {item.price}" for item in order.items]
    lines.append(f"Total: {order.total}")
    send_email(order.email, "\n".join(lines))
```

After:

```python
def receipt_text(order):
    lines = [f"{item.name}: {item.price}" for item in order.items]
    lines.append(f"Total: {order.total}")
    return "\n".join(lines)


def complete_order(order):
    charge(order)
    send_email(order.email, receipt_text(order))
```

## maxWeightedComplexity

Separate independent decisions into focused functions. Replace repeated conditional logic with a clear data lookup when appropriate.

Before:

```python
def shipping_cost(region):
    if region == "local":
        return 5
    if region == "national":
        return 10
    if region == "international":
        return 25
    raise ValueError("Unsupported region")
```

After:

```python
SHIPPING_COSTS = {"local": 5, "national": 10, "international": 25}


def shipping_cost(region):
    if not isinstance(region, str) or region not in SHIPPING_COSTS:
        raise ValueError("Unsupported region")
    return SHIPPING_COSTS[region]
```

Keep validation for unsupported values. Use a lookup only when the conditions compare the same input against fixed values.

## maxDecisionDepth

Use early returns for invalid inputs and special cases. Extract deeply nested decisions into focused functions.

Before:

```python
def can_publish(user, article):
    if user is not None:
        if user.is_editor:
            if article.is_ready:
                return True
    return False
```

After:

```python
def can_publish(user, article):
    if user is None:
        return False
    if not user.is_editor:
        return False
    return bool(article.is_ready)
```

## maxNestedLoops

Use an index, map, or set instead of repeatedly scanning the same collection. Extract independent iterations when this makes the algorithm clearer. Check runtime and memory use; moving a loop alone does not make it faster.

## maxCircularImportGroups

Remove a dependency from the reported cycle. Move shared definitions into a lower-level module, or pass a dependency through a function parameter. Keep dependencies flowing in one direction.

## maxChangeImpact

Review the files that depend on this file or module. Separate stable interfaces from frequently changing implementation. Remove unnecessary dependencies rather than moving them to hide the measurement.

## maxFanIn

Many files depend on this file. Check whether it combines unrelated responsibilities. Split those responsibilities into focused modules and update callers to import only what they need. A stable shared utility can legitimately have many dependents; use a narrow `fanExclude` pattern when that is intentional.

## maxFanOut

This file depends on many other files. Split independent operations into focused modules and remove unused dependencies. Keep each module responsible for one task. Do not add a forwarding module only to hide dependencies.

Fan limits apply to files, not modules. Set `maxFanIn` or `maxFanOut` to `null` to disable that check, or use `fanExclude` gitignore-style patterns for intentional exceptions. Exclusions suppress only fan findings; dependency counts and other checks stay unchanged.

## maxInstability

Review outgoing dependencies. Separate unrelated responsibilities and depend on a small stable interface where useful. High instability can be appropriate for application entry points; choose a limit that fits the module's role.

## analysis

Read the diagnostic and fix the syntax or unsupported construct at the reported path. Run the check again. Missing measurements do not mean that the code passed.
