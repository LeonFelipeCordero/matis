/** @jsx createElement */
/** @jsxFrag createFragment */
import { createElement, createFragment } from "./elements.js";

export function ReceiptCard({ title, children }) {
  return <article><h2>{title}</h2>{children}</article>;
}

// The receipt exporter uses XML namespaces, not React's DOM renderer.
export function ReceiptPreview({ customer, status }) {
  return <>
    <ReceiptCard title={`Order for ${customer}`}></ReceiptCard>
    <coffee-status>{status}</coffee-status>
    <svg:svg xmlns:svg="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
      <svg:path d="M2 2 H12 V10 H2 Z" />
    </svg:svg>
  </>;
}

export function BrandedReceipt({ ReceiptCard, customer }) {
  return <ReceiptCard title={`Pickup for ${customer}`} />;
}
