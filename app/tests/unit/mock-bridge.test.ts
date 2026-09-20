import { describe, expect, it } from "vitest";
import { MockBridge } from "../../src/shell/mock-bridge";

describe("MockBridge", () => {
  it("round-trips packages and files in memory", async () => {
    const b = new MockBridge();
    await b.writePackage("C:/p/A.comp", { manifest: "{}", images: [{ name: "X.png", bytes: new Uint8Array([1, 2]) }] });
    const pkg = await b.readPackage("C:/p/A.comp");
    expect(pkg.manifest).toBe("{}");
    expect(Array.from(pkg.images[0].bytes)).toEqual([1, 2]);
    await b.writeFile("C:/p/out.png", new Uint8Array([9]));
    expect(Array.from(await b.readFile("C:/p/out.png"))).toEqual([9]);
    b.setNextPick("C:/p/A.comp");
    expect(await b.pickOpenPackage()).toBe("C:/p/A.comp");
    expect(await b.pickOpenPackage()).toBeNull();
    expect(b.baseName("C:/p/A.comp")).toBe("A");
  });
});
