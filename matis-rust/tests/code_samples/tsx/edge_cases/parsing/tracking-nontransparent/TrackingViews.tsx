import type { Order } from "../../../coffee/tsx/TrackingCarriers";
import { consumeTicket, drinkQueue, pickupLabel, tracking, trackingActions } from "../../../coffee/tsx/TrackingCarriers";

// Retry receipts cross sequence, invocation, and conditional callback boundaries.
export function receiptPickup(order: Order, retry: boolean) {
  const trackingLabel = (/* pickup label */ (() => pickupLabel(order)));
  const trackingSequence = (retry ? tracking.requeue(order) : order, (() => trackingActions.receipt(order)));
  const trackingImmediate = ((() => [...drinkQueue(order)]))();
  const trackingChoice = (retry ? (() => trackingActions.receipt(order)) : (() => pickupLabel(order)));
  const receipt = consumeTicket(order, ((function explicitTicket(ticket: Order) {
    return trackingActions.receipt(ticket);
  })));
  const notifications = [...((function* ticketGenerator() {
    yield* trackingActions.notifications([order]);
  }))()];
  const counter = consumeTicket(order, ((ticket: Order) => pickupLabel(ticket)));
  return {
    label: trackingLabel(), receipt, drinks: trackingImmediate, notifications, counter,
    routedReceipt: trackingSequence(), nextLabel: trackingChoice(),
  };
}
