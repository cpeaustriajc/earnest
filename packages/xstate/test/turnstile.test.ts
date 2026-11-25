import { createTestModel } from "@xstate/graph";
import { earnestPaths } from "@earnestjs/xstate";
import { expect, test } from "vitest";
import { turnstile } from "./machines/turnstile.ts";
import { Gate } from "./gate.ts";

const model = createTestModel(turnstile);

for (const path of model.getPaths(earnestPaths)) {
  test(path.description, async () => {
    const gate = new Gate();
    await path.test({
      events: { PAY: () => gate.pay(), PUSH: () => gate.push(), ENTER: () => gate.enter() },
      states: {
        locked: () => expect(gate.state).toBe("locked"),
        paid: () => expect(gate.state).toBe("paid"),
        open: () => expect(gate.state).toBe("open"),
        full: () => expect(gate.state).toBe("full"),
      },
    });
  });
}
