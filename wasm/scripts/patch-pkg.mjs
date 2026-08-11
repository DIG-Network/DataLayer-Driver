// NOTE: the bundler build uses --no-opt because the bundled wasm-opt rejects blst's bulk-memory ops; re-enable optimization only with a wasm-opt that supports --enable-bulk-memory.
import { readFile, writeFile, copyFile } from "node:fs/promises";
import path from "node:path";

const pkgDir = path.resolve("pkg");
const pkgJsonPath = path.join(pkgDir, "package.json");

const pkg = JSON.parse(await readFile(pkgJsonPath, "utf8"));

pkg.name = "@dignetwork/datalayer-driver-wasm";
pkg.description = "WebAssembly bindings for the Chia DataLayer driver (offline DIGStore spend-bundle construction).";
pkg.repository = { type: "git", url: "https://github.com/DIG-Network/DataLayer-Driver.git" };
pkg.license = "MIT";
pkg.types = "datalayer-driver-wasm.d.ts";
pkg.files = Array.from(new Set([...(pkg.files ?? []), "datalayer-driver-wasm.d.ts"]));

await writeFile(pkgJsonPath, JSON.stringify(pkg, null, 2) + "\n");
await copyFile(path.resolve("types/datalayer-driver-wasm.d.ts"), path.join(pkgDir, "datalayer-driver-wasm.d.ts"));

console.log(`Patched ${pkg.name}@${pkg.version}`);
