package reservations;

import orders.Order;
import java.util.ArrayList;
import java.util.List;

public class ReservationLedger {
    private final List<Order> orders = new ArrayList<>();

    public Order commit(Order order) {
        orders.add(order);
        return order;
    }
}
