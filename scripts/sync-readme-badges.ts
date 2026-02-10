#!/usr/bin/env bun
import { readFileSync, writeFileSync } from "node:fs"
import { resolve } from "node:path"
import { getBadges, type BadgeVariant } from "../packages/opencode/packages/fork-ui/src/readme-badge-catalog"

type Target = "all" | "superproject" | "submodule"
type ReadmeTarget = Exclude<Target, "all">

const ROOT_README_PATH = resolve(import.meta.dir, "..", "README.md")
const SUBMODULE_README_PATH = resolve(import.meta.dir, "..", "packages/opencode/README.md")

const ROOT_BEGIN_MARKER = "<!-- BEGIN:opencode-cloud-readme-badges -->"
const ROOT_END_MARKER = "<!-- END:opencode-cloud-readme-badges -->"
const SUBMODULE_BEGIN_MARKER = "<!-- BEGIN:opencode-submodule-readme-badges -->"
const SUBMODULE_END_MARKER = "<!-- END:opencode-submodule-readme-badges -->"

const DEFAULT_VARIANT_BY_TARGET: Record<ReadmeTarget, BadgeVariant> = {
  superproject: "full",
  submodule: "full",
}

interface Options {
  check: boolean
  target: Target
  variantOverride?: BadgeVariant
}

interface Update {
  target: ReadmeTarget
  filePath: string
  changed: boolean
}

function usage(): never {
  console.error(`Usage: bun scripts/sync-readme-badges.ts [--check] [--target=all|superproject|submodule] [--variant=core|full]\n\nOptions:\n  --check                     Validate files without writing changes.\n  --target=...                Limit sync/check scope. Default: all\n  --variant=core|full         Override per-target variant defaults for this run.\n\nExamples:\n  bun scripts/sync-readme-badges.ts\n  bun scripts/sync-readme-badges.ts --check\n  bun scripts/sync-readme-badges.ts --target=submodule --variant=core`)
  process.exit(1)
}

function parseVariant(value: string): BadgeVariant {
  if (value === "core" || value === "full") return value
  console.error(`Invalid --variant value: ${value}`)
  usage()
}

function parseTarget(value: string): Target {
  if (value === "all" || value === "superproject" || value === "submodule") return value
  console.error(`Invalid --target value: ${value}`)
  usage()
}

function parseArgs(argv: string[]): Options {
  const options: Options = { check: false, target: "all" }

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i]

    if (arg === "--check") {
      options.check = true
      continue
    }

    if (arg === "--target") {
      const value = argv[i + 1]
      if (!value) usage()
      options.target = parseTarget(value)
      i += 1
      continue
    }

    if (arg.startsWith("--target=")) {
      options.target = parseTarget(arg.slice("--target=".length))
      continue
    }

    if (arg === "--variant") {
      const value = argv[i + 1]
      if (!value) usage()
      options.variantOverride = parseVariant(value)
      i += 1
      continue
    }

    if (arg.startsWith("--variant=")) {
      options.variantOverride = parseVariant(arg.slice("--variant=".length))
      continue
    }

    if (arg === "-h" || arg === "--help") {
      usage()
    }

    console.error(`Unknown argument: ${arg}`)
    usage()
  }

  return options
}

function replaceMarkerBlock(content: string, beginMarker: string, endMarker: string, generatedBody: string, filePath: string): string {
  const beginIndex = content.indexOf(beginMarker)
  const endIndex = content.indexOf(endMarker)

  if (beginIndex < 0 || endIndex < 0 || endIndex <= beginIndex) {
    throw new Error(`Markers not found or invalid in ${filePath}: ${beginMarker} ... ${endMarker}`)
  }

  const before = content.slice(0, beginIndex + beginMarker.length)
  const after = content.slice(endIndex)
  return `${before}\n${generatedBody.trimEnd()}\n${after}`
}

function renderRootBadges(variant: BadgeVariant): string {
  return getBadges("opencode-cloud", variant)
    .map((badge) => `[![${badge.label}](${badge.imageUrl})](${badge.linkUrl})`)
    .join("\n")
}

function renderSubmoduleBadges(variant: BadgeVariant): string {
  return getBadges("opencode-submodule", variant)
    .map((badge) => `  <a href="${badge.linkUrl}"><img alt="${badge.label}" src="${badge.imageUrl}" /></a>`)
    .join("\n")
}

function resolveVariant(target: ReadmeTarget, variantOverride?: BadgeVariant): BadgeVariant {
  return variantOverride ?? DEFAULT_VARIANT_BY_TARGET[target]
}

function includesTarget(scope: Target, target: ReadmeTarget): boolean {
  return scope === "all" || scope === target
}

function syncRootReadme(variant: BadgeVariant, check: boolean): Update {
  const current = readFileSync(ROOT_README_PATH, "utf8")
  const generated = renderRootBadges(variant)
  const next = replaceMarkerBlock(current, ROOT_BEGIN_MARKER, ROOT_END_MARKER, generated, ROOT_README_PATH)
  const changed = next !== current

  if (changed && !check) writeFileSync(ROOT_README_PATH, next)

  return { target: "superproject", filePath: ROOT_README_PATH, changed }
}

function syncSubmoduleReadme(variant: BadgeVariant, check: boolean): Update {
  const current = readFileSync(SUBMODULE_README_PATH, "utf8")
  const generated = renderSubmoduleBadges(variant)
  const next = replaceMarkerBlock(current, SUBMODULE_BEGIN_MARKER, SUBMODULE_END_MARKER, generated, SUBMODULE_README_PATH)
  const changed = next !== current

  if (changed && !check) writeFileSync(SUBMODULE_README_PATH, next)

  return { target: "submodule", filePath: SUBMODULE_README_PATH, changed }
}

function main() {
  const options = parseArgs(process.argv.slice(2))
  const updates: Update[] = []

  if (includesTarget(options.target, "superproject")) {
    updates.push(syncRootReadme(resolveVariant("superproject", options.variantOverride), options.check))
  }

  if (includesTarget(options.target, "submodule")) {
    updates.push(syncSubmoduleReadme(resolveVariant("submodule", options.variantOverride), options.check))
  }

  if (options.check) {
    const drifted = updates.filter((update) => update.changed)
    if (drifted.length > 0) {
      console.error("README badge blocks are out of sync:")
      for (const update of drifted) {
        console.error(`- ${update.target}: ${update.filePath}`)
      }
      console.error("Run: just sync-readme-badges")
      process.exit(1)
    }

    console.log("README badge blocks are in sync.")
    return
  }

  const changed = updates.filter((update) => update.changed)
  if (changed.length === 0) {
    console.log("README badge blocks already up to date.")
    return
  }

  for (const update of changed) {
    console.log(`Updated ${update.target} badges: ${update.filePath}`)
  }
}

main()
