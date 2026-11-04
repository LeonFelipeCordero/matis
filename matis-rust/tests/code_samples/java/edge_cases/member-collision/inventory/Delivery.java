package coffee.inventory;

import coffee.orders.Order;
import java.util.List;
import java.util.stream.Collectors;

public final class Delivery {
    public static String deliveryLabel(Order order) {
        int units = order.lines.stream().mapToInt(line -> line.quantity).sum();
        return "Deliver " + units + " drinks to " + order.customer;
    }

    public static List<String> manifest(List<Order> orders) {
        return orders.stream().map(order -> deliveryLabel(order)).collect(Collectors.toList());
    }
}
