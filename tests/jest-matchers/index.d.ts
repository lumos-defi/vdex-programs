import * as anchor from "@project-serum/anchor";
export {};
declare global {
  namespace jest {
    interface Matchers<R> {
      toBNEqual<E = anchor.BN>(expected: E | number): R;
    }
    interface Expect {
      toBNEqual(expected: anchor.BN | number): any;
    }
  }
}
