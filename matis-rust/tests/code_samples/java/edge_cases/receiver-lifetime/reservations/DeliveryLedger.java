package reservations;

import orders.Order;

public class DeliveryLedger extends ReservationLedger {
    @Override
    public Order commit(Order order) {
        return super.commit(order);
    }
}
