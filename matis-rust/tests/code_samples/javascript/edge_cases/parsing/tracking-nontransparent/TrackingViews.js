import { consumeTicket, drinkQueue, pickupLabel, tracking, trackingActions } from "../../../ordering/TrackingCarriers.js";

// Retry receipts cross sequence, invocation, and conditional callback boundaries.
export function receiptPickup(order, retry) {
  const trackingLabel = (/* pickup label */ (() => pickupLabel(order)));
  const trackingSequence = (retry ? tracking.requeue(order) : order, (() => trackingActions.receipt(order)));
  const trackingImmediate = ((() => [...drinkQueue(order)]))();
  const trackingChoice = (retry ? (() => trackingActions.receipt(order)) : (() => pickupLabel(order)));
  const receipt = consumeTicket(order, ((function explicitTicket(ticket) {
    return trackingActions.receipt(ticket);
  })));
  const notifications = [...((function* ticketGenerator() {
    yield* trackingActions.notifications([order]);
  }))()];
  const counter = consumeTicket(order, ((ticket) => pickupLabel(ticket)));
  return {
    label: trackingLabel(), receipt, drinks: trackingImmediate, notifications, counter,
    routedReceipt: trackingSequence(), nextLabel: trackingChoice(),
  };
}
