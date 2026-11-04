package reservations;

import orders.Order;
import java.util.ArrayList;
import java.util.List;

public class OrderCreatedClient {
    private final List<Order> events = new ArrayList<>();

    public Order sendAndWait(Order order) {
        events.add(order);
        return events.get(events.size() - 1);
    }
}
