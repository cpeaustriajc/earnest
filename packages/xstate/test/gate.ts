import { CAPACITY } from "./machines/turnstile.ts";

export class Gate {
  state: "locked" | "paid" | "open" | "full" = "locked";
  visitors = 0;

  pay(): void {
    if (this.state === "locked") {
      this.state = "paid";
    }
  }

  push(): void {
    if (this.state === "paid") {
      this.state = "open";
    }
  }

  enter(): void {
    if (this.state === "open") {
      this.visitors += 1;
      this.state = this.visitors === CAPACITY ? "full" : "locked";
    }
  }
}
