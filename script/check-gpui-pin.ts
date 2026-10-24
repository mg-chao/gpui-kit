#!/usr/bin/env bun
/**
 * Verify that all GPUI workspace dependencies use one immutable fork revision.
 *
 *     bun script/check-gpui-pin.ts [path/to/Cargo.toml]
 */
import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";

const REPO_ROOT = resolve(import.meta.dir, "..");
const MANIFEST = "Cargo.toml";
const GPUI_REPOSITORY = "https://github.com/mg-chao/gpui.git";
const GPUI_PACKAGES: Record<string, string> = {
  gpui: "gpui",
  gpui_platform: "gpui_platform",
  gpui_web: "gpui_web",
  gpui_macros: "gpui_macros",
  reqwest_client: "reqwest_client",
  "sum-tree": "sum_tree",
};

export interface WorkspaceManifest {
  workspace?: { dependencies?: Record<string, unknown> };
}

export function problems(manifest: WorkspaceManifest): string[] {
  const dependencies = manifest.workspace?.dependencies ?? {};
  const found: string[] = [];
  const revisions = new Set<string>();
  for (const [alias, packageName] of Object.entries(GPUI_PACKAGES)) {
    const spec = dependencies[alias];
    if (typeof spec !== "object" || spec === null) {
      found.push(`${alias} must depend on the GPUI fork`);
      continue;
    }
    const {
      git,
      rev,
      branch,
      tag,
      path,
      package: actualPackage,
    } = spec as Record<string, unknown>;
    if (git !== GPUI_REPOSITORY || (actualPackage ?? alias) !== packageName) {
      found.push(
        `${alias} must select package ${packageName} from ${GPUI_REPOSITORY}`,
      );
    }
    if (
      typeof rev !== "string" ||
      !/^[0-9a-f]{40}$/.test(rev) ||
      branch ||
      tag ||
      path
    ) {
      found.push(
        `${alias} must pin a full Git revision without a branch, tag or path`,
      );
      continue;
    }
    revisions.add(rev);
  }
  if (revisions.size > 1) {
    found.push("the GPUI crates are pinned to more than one Git revision");
  }
  return found;
}

if (import.meta.main) {
  const manifestPath = process.argv[2] ?? join(REPO_ROOT, MANIFEST);
  const manifest = Bun.TOML.parse(
    readFileSync(manifestPath, "utf8"),
  ) as WorkspaceManifest;
  const found = problems(manifest);
  if (found.length > 0) {
    for (const problem of found)
      console.error(`::error file=${MANIFEST}::${problem}`);
    console.error(
      `\n${MANIFEST} must pin every GPUI crate to the same fork revision.`,
    );
    process.exit(1);
  }
  console.log(`${MANIFEST} pins the GPUI crates to one fork revision`);
}
