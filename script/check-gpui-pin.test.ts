import { expect, test } from "bun:test";
import { problems, type WorkspaceManifest } from "./check-gpui-pin";

function manifest(): WorkspaceManifest {
  const dependencies: Record<string, unknown> = {};
  for (const alias of [
    "gpui",
    "gpui_platform",
    "gpui_web",
    "gpui_macros",
    "reqwest_client",
    "sum-tree",
  ]) {
    dependencies[alias] = {
      git: "https://github.com/mg-chao/gpui.git",
      rev: "a".repeat(40),
      ...(alias === "sum-tree" ? { package: "sum_tree" } : {}),
    };
  }
  return { workspace: { dependencies } };
}

test("accepts a matching immutable fork revision", () => {
  expect(problems(manifest())).toEqual([]);
});

test("rejects a missing GPUI dependency", () => {
  const value = manifest();
  delete value.workspace!.dependencies!.gpui_web;
  expect(problems(value)).toContain("gpui_web must depend on the GPUI fork");
});

test("rejects a registry snapshot mixed into the fork", () => {
  const value = manifest();
  value.workspace!.dependencies!.gpui = {
    package: "gpui-pre",
    version: "=0.3.9",
  };
  expect(
    problems(value).some((problem) =>
      problem.startsWith("gpui must select package"),
    ),
  ).toBe(true);
});

test("rejects mismatched fork revisions", () => {
  const value = manifest();
  Object.assign(value.workspace!.dependencies!.gpui_web!, {
    rev: "b".repeat(40),
  });
  expect(problems(value)).toContain(
    "the GPUI crates are pinned to more than one Git revision",
  );
});

test("rejects a moving branch or abbreviated revision", () => {
  const value = manifest();
  Object.assign(value.workspace!.dependencies!.gpui!, {
    branch: "main",
    rev: "abcdef0",
  });
  expect(
    problems(value).some((problem) => problem.includes("full Git revision")),
  ).toBe(true);
});

test("rejects a local path override", () => {
  const value = manifest();
  Object.assign(value.workspace!.dependencies!.gpui!, {
    path: "../gpui/crates/gpui",
  });
  expect(
    problems(value).some((problem) =>
      problem.includes("without a branch, tag or path"),
    ),
  ).toBe(true);
});

test("rejects a different repository or package name", () => {
  const value = manifest();
  Object.assign(value.workspace!.dependencies!["sum-tree"]!, {
    git: "https://github.com/zed-industries/zed.git",
    package: "gpui-pre-sum-tree",
  });
  expect(
    problems(value).some((problem) =>
      problem.startsWith("sum-tree must select package"),
    ),
  ).toBe(true);
});
