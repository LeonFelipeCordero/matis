package coffee.orders;

import coffee.inventory.Inventory;
import java.util.stream.Collectors;

public final class Reservations {
    public static String reserve(Order order) {
        String drinks = order.lines.stream()
            .map(line -> line.quantity + " " + line.drink)
            .collect(Collectors.joining(", "));
        return "Reserved for " + order.customer + ": " + drinks;
    }

    public static String queueOrder(Inventory inventory, Order order) {
        if (!inventory.reserve(order)) return "Waiting for stock: " + order.customer;
        return reserve(order);
    }

    public static String requestSupplier(Object supplier, Order order) {
        if (!supplier.reserve(order)) return "Supplier cannot fill: " + order.customer;
        return supplier.deliveryLabel(order);
    }
}
