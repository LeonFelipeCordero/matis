import scheduleDrink from "./Scheduling.js";

export function Scheduling({ names, customer }) {
  return <section aria-label={`Pickup for ${customer}`}>
    {names.map(name => <p key={name}>{name}: {scheduleDrink(name)}</p>)}
  </section>;
}
