package coffee.orders;

import coffee.brewing.PortionSchedule;
import static coffee.brewing.PortionSchedule.portions;

public class TastingOrders {
    public int total(String name) { return PortionSchedule.portions(name) + portions(name); }
}
