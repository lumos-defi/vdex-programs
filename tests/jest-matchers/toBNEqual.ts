import * as anchor from "@project-serum/anchor";
import { matcherHint, printReceived } from "jest-matcher-utils";

const passMessage = (received, after) => () =>
  matcherHint(".not.toBNEqual", "received", "") +
  "\n\n" +
  `Expected BN to be equal ${printReceived(after)} but received:\n` +
  `  ${printReceived(received)}`;

const failMessage = (received, after) => () =>
  matcherHint(".toBNEqual", "received", "") +
  "\n\n" +
  `Expected BN to be equal ${printReceived(after)} but received:\n` +
  `  ${printReceived(received)}`;

export function toBNEqual(received: anchor.BN, after: anchor.BN | number) {
  const pass = received.eq(new anchor.BN(after));
  if (pass) {
    return { pass: true, message: passMessage(received, after) };
  }

  return { pass: false, message: failMessage(received, after) };
}
