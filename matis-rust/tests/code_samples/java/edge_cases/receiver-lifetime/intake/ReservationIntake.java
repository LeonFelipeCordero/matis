package intake;

import orders.Order;
import reservations.ReservationLedger;
import reservations.DeliveryLedger;
import reservations.OrderCreatedClient;
import java.util.List;
import java.util.function.Consumer;

class ReservationIntake {
    private final ReservationLedger ledger;
    private final OrderCreatedClient client;
    static ReservationLedger openingLedger;

    ReservationIntake(ReservationLedger ledger, OrderCreatedClient client) {
        this.ledger = ledger;
        this.client = client;
    }

    Order fulfill(Order order) {
        Order saved = ledger.commit(order);
        return client.sendAndWait(saved);
    }

    Order open(Order order) {
        var local = new ReservationLedger();
        return local.commit(order);
    }

    static Order opening(Order order) {
        return openingLedger.commit(order);
    }

    Order receive(Order order, ReservationLedger parameter) {
        return parameter.commit(order);
    }

    Order unknown(Order order, Object unknown) {
        return unknown.commit(order);
    }

    Order redirect(Order order, ReservationLedger local) {
        local = new DeliveryLedger();
        return local.commit(order);
    }

    Order advance(Order order, ReservationLedger local) {
        local++;
        return local.commit(order);
    }

    void visit(Order order, ReservationLedger local, List<ReservationLedger> stations) {
        stations.forEach(local -> local.commit(order));
        local.commit(order);
    }

    void visitTyped(Order order, ReservationLedger local, List<ReservationLedger> stations) {
        stations.forEach((ReservationLedger local) -> local.commit(order));
        local.commit(order);
    }

    void visitFinal(Order order, ReservationLedger local, List<ReservationLedger> stations) {
        stations.forEach((final ReservationLedger local) -> local.commit(order));
        local.commit(order);
    }

    void visitInferred(Order order, ReservationLedger local, List<ReservationLedger> stations) {
        stations.forEach((var local) -> local.commit(order));
        local.commit(order);
    }

    Order shadow(Order order, MissingProvider ledger) {
        return ledger.commit(order);
    }
}

class StaticDispatch {
    ReservationLedger ledger;

    static Order fulfill(Order order) {
        return ledger.commit(order);
    }
}

class DeliveryStation {
    ReservationLedger ledger;

    DeliveryStation() {
        this.ledger = new DeliveryLedger();
    }

    Order fulfill(Order order) {
        return ledger.commit(order);
    }
}

class AuditLedger {
    Order commit(Order order) {
        return order;
    }

    Order commit(Order order, String reason) {
        System.out.println(reason);
        return order;
    }
}

class AuditedIntake {
    Order fulfill(Order order, AuditLedger ledger) {
        return ledger.commit(order);
    }
}
