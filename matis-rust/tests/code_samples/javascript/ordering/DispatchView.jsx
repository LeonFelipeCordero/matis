import { dispatchLabel, Dispatch } from "./Dispatch.js";

export function ReceiptCard({ title, children }) {
  return <article aria-label={title}><h2>{title}</h2>{children}</article>;
}

const DispatchCards = { ReceiptCard };

export function DispatchView({ order }) {
  return (
    <section aria-label="Pickup receipts">
      <ReceiptCard title={dispatchLabel(order)}>
        <DispatchCards.ReceiptCard title={order.customer} />
      </ReceiptCard>
    </section>
  );
}

export function dispatchPreview(order) {
  const dispatcher = new Dispatch();
  return dispatcher.schedule([order]);
}
