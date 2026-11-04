package coffee.inventory;

import coffee.orders.Order;
import java.util.HashMap;
import java.util.Map;
import static coffee.brewing.Recipe.portions;

interface Reservable {
    boolean reserve(Order order);
}

public final class Inventory extends StockLedger<Inventory.Change> implements Reservable {
    public static final class Change {
        public final String drink;
        public final int units;
        public final String reason;

        public Change(String drink, int units, String reason) {
            this.drink = drink;
            this.units = units;
            this.reason = reason;
        }
    }

    private final Map<String, Integer> available = new HashMap<>();

    @Override
    public void record(Change change) {
        super.record(change);
        available.merge(change.drink, change.units, Integer::sum);
    }

    public void receive(Change... shipments) {
        for (Change shipment : shipments) record(shipment);
    }

    @Override
    public boolean reserve(Order order) {
        Map<String, Integer> needed = new HashMap<>();
        for (Order.Line line : order.lines) {
            int units = portions(line.drink) * line.quantity;
            if (units == 0) return false;
            needed.merge(line.drink, units, Integer::sum);
        }
        if (needed.entrySet().stream().anyMatch(entry ->
                units(entry.getKey()) < entry.getValue())) return false;
        needed.forEach((drink, count) -> record(new Change(drink, -count, "order")));
        return true;
    }

    public int units(String drink) {
        return available.getOrDefault(drink, 0);
    }
}
